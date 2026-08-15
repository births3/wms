use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    decide_lpn_mix, decide_lpn_putaway_bind, lpn_inventory_identity_allows,
    lpn_numbering_document_type, CreateLpnContainerRequest, LpnContainer, LpnContainerTypePolicy,
    LpnContainerValidationError, LpnMixDenied, LpnPutawayBindDecision, UpdateLpnContainerRequest,
    UpsertLpnContainerTypePolicyRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    idempotency,
    operation_context::OperationContext as AuthContext,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LpnContainerRepositoryError {
    NotFound,
    DuplicateCode,
    CodeEmpty,
    CodeTooLong,
    TypeInvalid,
    StatusInvalid,
    IdempotencyConflict,
    NumberingUnavailable,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LpnPutawayBindError {
    NotFound,
    NotUsable,
    LocationConflict,
    MixDeniedSku,
    MixDeniedBatch,
    Audit(String),
    Database(String),
}

impl From<crate::idempotency::IdempotencyError> for LpnContainerRepositoryError {
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

impl From<LpnContainerValidationError> for LpnContainerRepositoryError {
    fn from(error: LpnContainerValidationError) -> Self {
        match error {
            LpnContainerValidationError::CodeEmpty => Self::CodeEmpty,
            LpnContainerValidationError::CodeTooLong => Self::CodeTooLong,
            LpnContainerValidationError::TypeInvalid => Self::TypeInvalid,
            LpnContainerValidationError::StatusInvalid => Self::StatusInvalid,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PgLpnContainerRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, FromRow)]
struct LpnContainerRow {
    id: Uuid,
    owner_id: Uuid,
    lpn_code: String,
    container_type: String,
    capacity_cm3: Option<i64>,
    status: String,
    location_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<LpnContainerRow> for LpnContainer {
    fn from(row: LpnContainerRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            lpn_code: row.lpn_code,
            container_type: row.container_type,
            capacity_cm3: row.capacity_cm3,
            status: row.status,
            location_id: row.location_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl PgLpnContainerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        ctx: &AuthContext,
        req: CreateLpnContainerRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LpnContainer, LpnContainerRepositoryError> {
        let req = CreateLpnContainerRequest {
            container_type: req.container_type.trim().to_string(),
            capacity_cm3: req.capacity_cm3,
        };
        req.validate()?;
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
        let path = "/api/v1/master-data/lpn-containers";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<LpnContainer>(
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
            return Ok(replay);
        }
        let id = Uuid::new_v4();
        let generated_no = crate::document_numbering::PgDocumentNumberingService::new()
            .generate_in_tx(
                &mut tx,
                ctx,
                crate::document_numbering::GenerateDocumentNumberRequest {
                    document_type: lpn_numbering_document_type(&req.container_type),
                    idempotency_key: format!("m1-lpn-create:{id}"),
                    source_module: "M1".to_string(),
                    source_document_id: Some(id),
                },
                now,
            )
            .await
            .map_err(|error| match error {
                crate::document_numbering::DocumentNumberingError::RuleNotFound
                | crate::document_numbering::DocumentNumberingError::DocumentTypeInvalid
                | crate::document_numbering::DocumentNumberingError::InvalidRule => {
                    LpnContainerRepositoryError::NumberingUnavailable
                }
                other => LpnContainerRepositoryError::Database(format!("{other:?}")),
            })?
            .value
            .generated_no;
        let container = req
            .clone()
            .into_new_container(id, ctx.owner_id, generated_no, now)?;
        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            INSERT INTO lpn_containers (
                id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id, created_at, updated_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$8)
            RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id, created_at, updated_at
            "#,
        )
        .bind(container.id)
        .bind(container.owner_id)
        .bind(&container.lpn_code)
        .bind(&container.container_type)
        .bind(container.capacity_cm3)
        .bind(&container.status)
        .bind(container.location_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;
        let created: LpnContainer = row.into();
        append_lpn_audit(&mut tx, ctx, "create_lpn_container", &created, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            "lpn_container",
            &created.id.to_string(),
            &created,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(created)
    }

    pub async fn list(
        &self,
        ctx: &AuthContext,
        keyword: Option<&str>,
        container_type: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<LpnContainer>, LpnContainerRepositoryError> {
        let keyword = keyword.map(str::trim).filter(|value| !value.is_empty());
        let rows = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            SELECT id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id, created_at, updated_at
              FROM lpn_containers
             WHERE owner_id = $1
               AND ($2::text IS NULL OR lpn_code ILIKE '%' || $2 || '%' OR container_type ILIKE '%' || $2 || '%')
               AND ($3::text IS NULL OR container_type = $3)
               AND ($4::text IS NULL OR status = $4)
             ORDER BY lpn_code
            "#,
        )
        .bind(ctx.owner_id)
        .bind(keyword)
        .bind(container_type)
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateLpnContainerRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<LpnContainer, LpnContainerRepositoryError> {
        req.validate()?;
        let request_hash = request_hash(&serde_json::json!({
            "id": id,
            "request": &req,
        }))?;
        let path = "/api/v1/master-data/lpn-containers/{id}";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<LpnContainer>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            path,
            now,
        )
        .await?
        {
            return Ok(replay);
        }
        let before = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            SELECT id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id, created_at, updated_at
              FROM lpn_containers
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(LpnContainerRepositoryError::NotFound)?;
        let status = req
            .status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&before.status);
        let location_id = req.location_id.or(before.location_id);
        let capacity_cm3 = req.capacity_cm3.or(before.capacity_cm3);
        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            UPDATE lpn_containers
               SET status = $3,
                   location_id = $4,
                   capacity_cm3 = $5,
                   updated_at = $6
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(status)
        .bind(location_id)
        .bind(capacity_cm3)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_write_error)?;
        let updated: LpnContainer = row.into();
        append_lpn_audit(&mut tx, ctx, "update_lpn_container", &updated, now).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            path,
            "lpn_container",
            &updated.id.to_string(),
            &updated,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(updated)
    }

    pub async fn bind_lpn_for_putaway(
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        lpn_code: &str,
        location_id: Uuid,
        product_code: &str,
        batch_no: &str,
        now: DateTime<Utc>,
    ) -> Result<LpnContainer, LpnPutawayBindError> {
        let row = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            SELECT id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id, created_at, updated_at
              FROM lpn_containers
             WHERE owner_id = $1 AND lpn_code = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(lpn_code)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| LpnPutawayBindError::Database(error.to_string()))?
        .ok_or(LpnPutawayBindError::NotFound)?;
        match decide_lpn_putaway_bind(&row.status, row.location_id, location_id) {
            LpnPutawayBindDecision::NotUsable => return Err(LpnPutawayBindError::NotUsable),
            LpnPutawayBindDecision::LocationConflict => {
                return Err(LpnPutawayBindError::LocationConflict)
            }
            LpnPutawayBindDecision::Allow => {}
        }
        Self::enforce_putaway_mix(
            tx,
            ctx,
            lpn_code,
            &row.container_type,
            product_code,
            batch_no,
        )
        .await?;
        let updated = sqlx::query_as::<_, LpnContainerRow>(
            r#"
            UPDATE lpn_containers
               SET status = 'in_use',
                   location_id = $3,
                   updated_at = $4
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id, created_at, updated_at
            "#,
        )
        .bind(row.id)
        .bind(ctx.owner_id)
        .bind(location_id)
        .bind(now)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| LpnPutawayBindError::Database(error.to_string()))?;
        let updated: LpnContainer = updated.into();
        append_lpn_audit(tx, ctx, "bind_lpn_container_putaway", &updated, now)
            .await
            .map_err(|error| match error {
                LpnContainerRepositoryError::Audit(message) => LpnPutawayBindError::Audit(message),
                other => LpnPutawayBindError::Database(format!("{other:?}")),
            })?;
        Ok(updated)
    }

    pub async fn enforce_inventory_identity(
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        product_code: &str,
        batch_no: &str,
        location_id: Uuid,
        quality_status: &str,
        incoming_lpn: Option<&str>,
    ) -> Result<(), LpnPutawayBindError> {
        let existing: Option<Option<String>> = sqlx::query_scalar(
            r#"
            SELECT container_lpn
              FROM inventory_batches
             WHERE owner_id = $1
               AND product_code = $2
               AND batch_no = $3
               AND location_id = $4
               AND quality_status = $5
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(product_code)
        .bind(batch_no)
        .bind(location_id)
        .bind(quality_status)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| LpnPutawayBindError::Database(error.to_string()))?;
        if let Some(existing_lpn) = existing {
            if !lpn_inventory_identity_allows(existing_lpn.as_deref(), incoming_lpn) {
                return Err(LpnPutawayBindError::NotUsable);
            }
        }
        Ok(())
    }

    pub async fn enforce_putaway_mix(
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        lpn_code: &str,
        container_type: &str,
        product_code: &str,
        batch_no: &str,
    ) -> Result<(), LpnPutawayBindError> {
        let policy = sqlx::query_as::<_, (bool, bool)>(
            r#"
            SELECT allow_mix_batch, allow_mix_sku
              FROM lpn_container_type_policies
             WHERE owner_id = $1 AND container_type = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(container_type)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| LpnPutawayBindError::Database(error.to_string()))?
        .unwrap_or((false, false));
        let existing = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT product_code, batch_no
              FROM inventory_batches
             WHERE owner_id = $1 AND container_lpn = $2 AND qty_on_hand > 0
            "#,
        )
        .bind(ctx.owner_id)
        .bind(lpn_code)
        .fetch_all(&mut **tx)
        .await
        .map_err(|error| LpnPutawayBindError::Database(error.to_string()))?;
        match decide_lpn_mix(policy.1, policy.0, &existing, product_code, batch_no) {
            Ok(()) => Ok(()),
            Err(LpnMixDenied::Sku) => Err(LpnPutawayBindError::MixDeniedSku),
            Err(LpnMixDenied::Batch) => Err(LpnPutawayBindError::MixDeniedBatch),
        }
    }

    pub async fn list_type_policies(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<LpnContainerTypePolicy>, LpnContainerRepositoryError> {
        let rows = sqlx::query_as::<_, (String, bool, bool)>(
            r#"
            SELECT container_type, allow_mix_batch, allow_mix_sku
              FROM lpn_container_type_policies
             WHERE owner_id = $1
             ORDER BY container_type
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows
            .into_iter()
            .map(
                |(container_type, allow_mix_batch, allow_mix_sku)| LpnContainerTypePolicy {
                    owner_id: ctx.owner_id,
                    container_type,
                    allow_mix_batch,
                    allow_mix_sku,
                },
            )
            .collect())
    }

    pub async fn upsert_type_policy(
        &self,
        ctx: &AuthContext,
        req: UpsertLpnContainerTypePolicyRequest,
    ) -> Result<LpnContainerTypePolicy, LpnContainerRepositoryError> {
        if !wms_domain::is_valid_lpn_container_type(req.container_type.trim()) {
            return Err(LpnContainerRepositoryError::TypeInvalid);
        }
        let now = Utc::now();
        let policy = LpnContainerTypePolicy {
            owner_id: ctx.owner_id,
            container_type: req.container_type.trim().to_string(),
            allow_mix_batch: req.allow_mix_batch,
            allow_mix_sku: req.allow_mix_sku,
        };
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        sqlx::query(
            r#"
            INSERT INTO lpn_container_type_policies (
                owner_id, container_type, allow_mix_batch, allow_mix_sku, updated_at
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (owner_id, container_type)
            DO UPDATE SET
                allow_mix_batch = EXCLUDED.allow_mix_batch,
                allow_mix_sku = EXCLUDED.allow_mix_sku,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(policy.owner_id)
        .bind(&policy.container_type)
        .bind(policy.allow_mix_batch)
        .bind(policy.allow_mix_sku)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "upsert_lpn_type_policy",
            "M1",
            "lpn_container_type_policy",
            policy.container_type.clone(),
            None,
        );
        audit.occurred_at = now;
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| LpnContainerRepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(policy)
    }
}

async fn append_lpn_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    after: &LpnContainer,
    now: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M1",
        "lpn_container",
        after.id.to_string(),
        None,
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| LpnContainerRepositoryError::Audit(format!("{error:?}")))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), LpnContainerRepositoryError> {
    idempotency::lock_key(tx, "lpn_container", owner_id, idempotency_key)
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
) -> Result<Option<T>, LpnContainerRepositoryError> {
    Ok(idempotency::replay::<T>(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        now,
    )
    .await?)
}

async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: &str,
    value: &T,
    now: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    idempotency::store_success(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        resource_id,
        value,
        now,
    )
    .await
    .map_err(Into::into)
}

fn request_hash(value: &serde_json::Value) -> Result<String, LpnContainerRepositoryError> {
    idempotency::request_hash(value).map_err(Into::into)
}

fn map_db_error(error: sqlx::Error) -> LpnContainerRepositoryError {
    LpnContainerRepositoryError::Database(error.to_string())
}

fn map_write_error(error: sqlx::Error) -> LpnContainerRepositoryError {
    if let sqlx::Error::Database(db_error) = &error {
        if db_error.code().as_deref() == Some("23505") {
            return LpnContainerRepositoryError::DuplicateCode;
        }
        if db_error.code().as_deref() == Some("23514") {
            let constraint = db_error.constraint().unwrap_or_default();
            if constraint.contains("status") {
                return LpnContainerRepositoryError::StatusInvalid;
            }
            if constraint.contains("lpn_code") {
                return LpnContainerRepositoryError::CodeEmpty;
            }
            return LpnContainerRepositoryError::TypeInvalid;
        }
    }
    map_db_error(error)
}
