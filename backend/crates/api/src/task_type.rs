use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    normalize_task_type_code, SetTaskTypeEnabledRequest, TaskType, TaskTypeValidationError,
    UpsertTaskTypeRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

#[derive(Clone, Debug)]
pub struct PgTaskTypeRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentTaskTypeMutation {
    pub value: TaskType,
    pub replayed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TaskTypeError {
    Validation(TaskTypeValidationError),
    NotFound,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, FromRow)]
struct TaskTypeRow {
    id: Uuid,
    owner_id: Uuid,
    task_type_code: String,
    task_type_name: String,
    default_priority: i32,
    estimated_minutes: i32,
    mergeable: bool,
    insertable: bool,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

impl PgTaskTypeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, ctx: &AuthContext) -> Result<Vec<TaskType>, TaskTypeError> {
        let rows = sqlx::query_as::<_, TaskTypeRow>(
            r#"
            SELECT id, owner_id, task_type_code, task_type_name, default_priority,
                   estimated_minutes, mergeable, insertable, enabled,
                   created_at, updated_at, version
              FROM task_types
             WHERE owner_id = $1
             ORDER BY task_type_code
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_database_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn upsert(
        &self,
        ctx: &AuthContext,
        task_type_code: &str,
        request: UpsertTaskTypeRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentTaskTypeMutation, TaskTypeError> {
        let task_type_code =
            normalize_task_type_code(task_type_code).map_err(TaskTypeError::Validation)?;
        let request = request.normalized();
        request.validate().map_err(TaskTypeError::Validation)?;
        let request_hash = request_hash(&serde_json::json!({
            "operation": "upsert",
            "task_type_code": &task_type_code,
            "request": &request,
        }))?;

        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_key(
            &mut tx,
            "task-type-idempotency",
            ctx.owner_id,
            idempotency_key,
        )
        .await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentTaskTypeMutation {
                value,
                replayed: true,
            });
        }
        lock_key(&mut tx, "task-type-resource", ctx.owner_id, &task_type_code).await?;

        let before = load_task_type_for_update(&mut tx, ctx.owner_id, &task_type_code).await?;
        let row = if let Some(existing) = before.as_ref() {
            sqlx::query_as::<_, TaskTypeRow>(
                r#"
                UPDATE task_types
                   SET task_type_name = $1,
                       default_priority = $2,
                       estimated_minutes = $3,
                       mergeable = $4,
                       insertable = $5,
                       enabled = $6,
                       updated_at = $7,
                       version = version + 1
                 WHERE id = $8 AND owner_id = $9
                 RETURNING id, owner_id, task_type_code, task_type_name, default_priority,
                           estimated_minutes, mergeable, insertable, enabled,
                           created_at, updated_at, version
                "#,
            )
            .bind(&request.task_type_name)
            .bind(request.default_priority)
            .bind(request.estimated_minutes)
            .bind(request.mergeable)
            .bind(request.insertable)
            .bind(request.enabled)
            .bind(now)
            .bind(existing.id)
            .bind(ctx.owner_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_database_error)?
        } else {
            sqlx::query_as::<_, TaskTypeRow>(
                r#"
                INSERT INTO task_types (
                    id, owner_id, task_type_code, task_type_name, default_priority,
                    estimated_minutes, mergeable, insertable, enabled, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
                RETURNING id, owner_id, task_type_code, task_type_name, default_priority,
                          estimated_minutes, mergeable, insertable, enabled,
                          created_at, updated_at, version
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(&task_type_code)
            .bind(&request.task_type_name)
            .bind(request.default_priority)
            .bind(request.estimated_minutes)
            .bind(request.mergeable)
            .bind(request.insertable)
            .bind(request.enabled)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_database_error)?
        };
        let value: TaskType = row.into();
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "PUT",
            &format!("/api/v1/task-engine/task-types/{task_type_code}"),
            before.as_ref().map(|row| TaskType::from(row.clone())),
            &value,
            "upsert_task_type",
            now,
        )
        .await?;
        Ok(IdempotentTaskTypeMutation {
            value,
            replayed: false,
        })
    }

    pub async fn set_enabled(
        &self,
        ctx: &AuthContext,
        task_type_code: &str,
        request: SetTaskTypeEnabledRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentTaskTypeMutation, TaskTypeError> {
        let task_type_code =
            normalize_task_type_code(task_type_code).map_err(TaskTypeError::Validation)?;
        let request_hash = request_hash(&serde_json::json!({
            "operation": "set_enabled",
            "task_type_code": &task_type_code,
            "request": &request,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_key(
            &mut tx,
            "task-type-idempotency",
            ctx.owner_id,
            idempotency_key,
        )
        .await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentTaskTypeMutation {
                value,
                replayed: true,
            });
        }
        lock_key(&mut tx, "task-type-resource", ctx.owner_id, &task_type_code).await?;
        let before = load_task_type_for_update(&mut tx, ctx.owner_id, &task_type_code)
            .await?
            .ok_or(TaskTypeError::NotFound)?;
        let row = sqlx::query_as::<_, TaskTypeRow>(
            r#"
            UPDATE task_types
               SET enabled = $1, updated_at = $2, version = version + 1
             WHERE id = $3 AND owner_id = $4
             RETURNING id, owner_id, task_type_code, task_type_name, default_priority,
                       estimated_minutes, mergeable, insertable, enabled,
                       created_at, updated_at, version
            "#,
        )
        .bind(request.enabled)
        .bind(now)
        .bind(before.id)
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let value: TaskType = row.into();
        finish_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/task-engine/task-types/{task_type_code}/enabled"),
            Some(before.into()),
            &value,
            "set_task_type_enabled",
            now,
        )
        .await?;
        Ok(IdempotentTaskTypeMutation {
            value,
            replayed: false,
        })
    }
}

impl From<TaskTypeRow> for TaskType {
    fn from(row: TaskTypeRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            task_type_code: row.task_type_code,
            task_type_name: row.task_type_name,
            default_priority: row.default_priority,
            estimated_minutes: row.estimated_minutes,
            mergeable: row.mergeable,
            insertable: row.insertable,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

async fn load_task_type_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    task_type_code: &str,
) -> Result<Option<TaskTypeRow>, TaskTypeError> {
    sqlx::query_as::<_, TaskTypeRow>(
        r#"
        SELECT id, owner_id, task_type_code, task_type_name, default_priority,
               estimated_minutes, mergeable, insertable, enabled,
               created_at, updated_at, version
          FROM task_types
         WHERE owner_id = $1 AND task_type_code = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(task_type_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)
}

async fn finish_mutation(
    mut tx: Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    before: Option<TaskType>,
    value: &TaskType,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), TaskTypeError> {
    store_idempotency_success(
        &mut tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        "task_type",
        value.id.to_string(),
        value,
        now,
    )
    .await?;
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M-TE",
        "task_type",
        value.id.to_string(),
        Some(AuditDiff::compute(
            before.map_or_else(|| Ok(serde_json::json!({})), |item| json_value(&item))?,
            json_value(value)?,
        )),
    );
    audit.occurred_at = now;
    append_event_in_tx(&mut tx, &audit)
        .await
        .map_err(|error| TaskTypeError::Audit(format!("{error:?}")))?;
    tx.commit().await.map_err(map_database_error)
}

async fn replay_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<TaskType>, TaskTypeError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_database_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(TaskTypeError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| TaskTypeError::Serialize(error.to_string()))
}

async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), TaskTypeError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| TaskTypeError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_database_error)?;
    Ok(())
}

async fn lock_key(
    tx: &mut Transaction<'_, Postgres>,
    namespace: &str,
    owner_id: Uuid,
    key: &str,
) -> Result<(), TaskTypeError> {
    let lock_key = advisory_lock_key(namespace, owner_id, key);
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(lock_key)
        .execute(&mut **tx)
        .await
        .map_err(map_database_error)?;
    Ok(())
}

fn advisory_lock_key(namespace: &str, owner_id: Uuid, key: &str) -> i64 {
    let digest = Sha256::digest(format!("{namespace}:{owner_id}:{key}").as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

fn request_hash(value: &Value) -> Result<String, TaskTypeError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| TaskTypeError::Serialize(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn json_value<T: Serialize>(value: &T) -> Result<Value, TaskTypeError> {
    serde_json::to_value(value).map_err(|error| TaskTypeError::Serialize(error.to_string()))
}

fn map_database_error(error: sqlx::Error) -> TaskTypeError {
    TaskTypeError::Database(format!("{error:?}"))
}
