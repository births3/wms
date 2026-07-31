use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    ChangeDrugInspectionPlatformStatusRequest, DrugInspectionConfigValidationError,
    DrugInspectionPlatform, DrugInspectionPlatformListResponse, PageMeta,
    UpsertDrugInspectionPlatformRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    idempotency,
    operation_context::OperationContext as AuthContext,
};

#[derive(Clone)]
pub struct PgDrugInspectionRepository {
    pool: PgPool,
}

impl PgDrugInspectionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        owner_id: Uuid,
        status: Option<String>,
    ) -> Result<DrugInspectionPlatformListResponse, DrugInspectionRepositoryError> {
        if let Some(status) = status.as_deref() {
            validate_status(status)?;
        }
        let rows = sqlx::query_as::<_, DrugInspectionPlatformRow>(
            r#"
            SELECT id, owner_id, platform_code, platform_name, api_url, auth_method,
                   username, (api_key_alias IS NOT NULL) AS api_key_configured,
                   (password_alias IS NOT NULL) AS password_configured,
                   timeout_seconds, status, created_at, updated_at, version
              FROM drug_inspection_platforms
             WHERE owner_id = $1 AND ($2::TEXT IS NULL OR status = $2)
             ORDER BY platform_code ASC
            "#,
        )
        .bind(owner_id)
        .bind(
            status
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let count = u32::try_from(rows.len())
            .map_err(|_| DrugInspectionRepositoryError::Serialize("平台列表过大".to_string()))?;
        Ok(DrugInspectionPlatformListResponse {
            data: rows.into_iter().map(Into::into).collect(),
            page: PageMeta {
                next_cursor: None,
                count,
            },
        })
    }

    pub async fn upsert(
        &self,
        ctx: &AuthContext,
        request: UpsertDrugInspectionPlatformRequest,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<DrugInspectionPlatform>, DrugInspectionRepositoryError> {
        request
            .validate()
            .map_err(DrugInspectionRepositoryError::Invalid)?;
        let request_hash = request_hash(&request)?;
        let now = Utc::now();
        let path = "/api/v1/drug-inspection/platforms";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }

        let before =
            fetch_by_code_for_update(&mut tx, ctx.owner_id, request.platform_code.trim()).await?;
        let row = sqlx::query_as::<_, DrugInspectionPlatformRow>(
            r#"
            INSERT INTO drug_inspection_platforms (
                id, owner_id, platform_code, platform_name, api_url, auth_method,
                api_key_alias, username, password_alias, timeout_seconds, status,
                created_by, updated_by, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12, $13, $13, 1)
            ON CONFLICT (owner_id, platform_code)
            DO UPDATE SET
                platform_name = EXCLUDED.platform_name,
                api_url = EXCLUDED.api_url,
                auth_method = EXCLUDED.auth_method,
                api_key_alias = EXCLUDED.api_key_alias,
                username = EXCLUDED.username,
                password_alias = EXCLUDED.password_alias,
                timeout_seconds = EXCLUDED.timeout_seconds,
                status = EXCLUDED.status,
                updated_by = EXCLUDED.updated_by,
                updated_at = EXCLUDED.updated_at,
                version = drug_inspection_platforms.version + 1
            RETURNING id, owner_id, platform_code, platform_name, api_url, auth_method,
                      username, (api_key_alias IS NOT NULL) AS api_key_configured,
                      (password_alias IS NOT NULL) AS password_configured,
                      timeout_seconds, status, created_at, updated_at, version
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(request.platform_code.trim())
        .bind(request.platform_name.trim())
        .bind(request.api_url.trim())
        .bind(request.auth_method.trim())
        .bind(request.api_key_alias.as_deref().map(str::trim))
        .bind(request.username.as_deref().map(str::trim))
        .bind(request.password_alias.as_deref().map(str::trim))
        .bind(request.timeout_seconds)
        .bind(request.status.trim())
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value: DrugInspectionPlatform = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            &value,
            before.map(|row| audit_snapshot(&DrugInspectionPlatform::from(row))),
            "di.platform.upserted",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn change_status(
        &self,
        ctx: &AuthContext,
        platform_id: Uuid,
        request: ChangeDrugInspectionPlatformStatusRequest,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<DrugInspectionPlatform>, DrugInspectionRepositoryError> {
        request
            .validate()
            .map_err(DrugInspectionRepositoryError::Invalid)?;
        let request_hash = request_hash(&(platform_id, &request))?;
        let now = Utc::now();
        let path = format!("/api/v1/drug-inspection/platforms/{platform_id}/status");
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &path,
            now,
        )
        .await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }

        let before = fetch_by_id_for_update(&mut tx, ctx.owner_id, platform_id)
            .await?
            .ok_or(DrugInspectionRepositoryError::NotFound)?;
        let row = sqlx::query_as::<_, DrugInspectionPlatformRow>(
            r#"
            UPDATE drug_inspection_platforms
               SET status = $3, updated_by = $4, updated_at = $5, version = version + 1
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, platform_code, platform_name, api_url, auth_method,
                      username, (api_key_alias IS NOT NULL) AS api_key_configured,
                      (password_alias IS NOT NULL) AS password_configured,
                      timeout_seconds, status, created_at, updated_at, version
            "#,
        )
        .bind(ctx.owner_id)
        .bind(platform_id)
        .bind(request.status.trim())
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value: DrugInspectionPlatform = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &request_hash,
            "PATCH",
            &path,
            &value,
            Some(audit_snapshot(&DrugInspectionPlatform::from(before))),
            "di.platform.status_changed",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }
}

#[derive(Clone, Debug)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Debug)]
pub enum DrugInspectionRepositoryError {
    Invalid(DrugInspectionConfigValidationError),
    NotFound,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<crate::idempotency::IdempotencyError> for DrugInspectionRepositoryError {
    fn from(error: crate::idempotency::IdempotencyError) -> Self {
        match error {
            crate::idempotency::IdempotencyError::Conflict => Self::IdempotencyConflict,
            crate::idempotency::IdempotencyError::Database(error) => {
                Self::Database(error.to_string())
            }
            crate::idempotency::IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
}

#[derive(Clone, Debug, FromRow)]
struct DrugInspectionPlatformRow {
    id: Uuid,
    owner_id: Uuid,
    platform_code: String,
    platform_name: String,
    api_url: String,
    auth_method: String,
    username: Option<String>,
    api_key_configured: bool,
    password_configured: bool,
    timeout_seconds: i32,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

impl From<DrugInspectionPlatformRow> for DrugInspectionPlatform {
    fn from(row: DrugInspectionPlatformRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            platform_code: row.platform_code,
            platform_name: row.platform_name,
            api_url: row.api_url,
            auth_method: row.auth_method,
            username: row.username,
            api_key_configured: row.api_key_configured,
            password_configured: row.password_configured,
            timeout_seconds: row.timeout_seconds,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

fn audit_snapshot(value: &DrugInspectionPlatform) -> Value {
    serde_json::json!({
        "id": value.id,
        "owner_id": value.owner_id,
        "platform_code": value.platform_code,
        "platform_name": value.platform_name,
        "api_url": value.api_url,
        "auth_method": value.auth_method,
        "username": value.username,
        "api_key_configured": value.api_key_configured,
        "password_configured": value.password_configured,
        "timeout_seconds": value.timeout_seconds,
        "status": value.status,
        "version": value.version,
    })
}

async fn fetch_by_code_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    platform_code: &str,
) -> Result<Option<DrugInspectionPlatformRow>, DrugInspectionRepositoryError> {
    sqlx::query_as::<_, DrugInspectionPlatformRow>(
        r#"
        SELECT id, owner_id, platform_code, platform_name, api_url, auth_method,
               username, (api_key_alias IS NOT NULL) AS api_key_configured,
               (password_alias IS NOT NULL) AS password_configured,
               timeout_seconds, status, created_at, updated_at, version
          FROM drug_inspection_platforms
         WHERE owner_id = $1 AND platform_code = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(platform_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn fetch_by_id_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    platform_id: Uuid,
) -> Result<Option<DrugInspectionPlatformRow>, DrugInspectionRepositoryError> {
    sqlx::query_as::<_, DrugInspectionPlatformRow>(
        r#"
        SELECT id, owner_id, platform_code, platform_name, api_url, auth_method,
               username, (api_key_alias IS NOT NULL) AS api_key_configured,
               (password_alias IS NOT NULL) AS password_configured,
               timeout_seconds, status, created_at, updated_at, version
          FROM drug_inspection_platforms
         WHERE owner_id = $1 AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(platform_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

#[allow(clippy::too_many_arguments)]
async fn finish_mutation(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    value: &DrugInspectionPlatform,
    before: Option<Value>,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), DrugInspectionRepositoryError> {
    idempotency::store_success(
        tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        "drug_inspection_platform",
        &value.id.to_string(),
        value,
        now,
    )
    .await
    .map_err(DrugInspectionRepositoryError::from)?;

    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "DI",
        "drug_inspection_platform",
        value.id.to_string(),
        Some(AuditDiff::compute(
            before.unwrap_or(Value::Null),
            audit_snapshot(value),
        )),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| DrugInspectionRepositoryError::Audit(format!("{error:?}")))?;
    Ok(())
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), DrugInspectionRepositoryError> {
    idempotency::lock_key(tx, "drug-inspection-platform", owner_id, idempotency_key)
        .await
        .map_err(Into::into)
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, DrugInspectionRepositoryError> {
    idempotency::replay(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        now,
    )
    .await
    .map_err(Into::into)
}

fn request_hash<T: Serialize>(value: &T) -> Result<String, DrugInspectionRepositoryError> {
    idempotency::request_hash(value).map_err(Into::into)
}

fn validate_status(value: &str) -> Result<(), DrugInspectionRepositoryError> {
    if wms_domain::DRUG_INSPECTION_PLATFORM_STATUSES.contains(&value.trim()) {
        Ok(())
    } else {
        Err(DrugInspectionRepositoryError::Invalid(
            DrugInspectionConfigValidationError::InvalidStatus,
        ))
    }
}

fn map_db_error(error: sqlx::Error) -> DrugInspectionRepositoryError {
    DrugInspectionRepositoryError::Database(error.to_string())
}
