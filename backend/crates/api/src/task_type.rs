use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    normalize_task_type_code, SetTaskTypeEnabledRequest, TaskReleaseStrategy, TaskType,
    TaskTypeValidationError, UpsertTaskTypeRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    idempotency::{self, IdempotencyError},
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
    release_strategy: String,
    release_interval_minutes: Option<i32>,
    release_batch_size: Option<i32>,
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
                   estimated_minutes, mergeable, insertable, release_strategy,
                   release_interval_minutes, release_batch_size, enabled,
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
        if let Some(value) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PUT",
            &format!("/api/v1/task-engine/task-types/{task_type_code}"),
            now,
        )
        .await?
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
                       release_strategy = $6,
                       release_interval_minutes = $7,
                       release_batch_size = $8,
                       enabled = $9,
                       updated_at = $10,
                       version = version + 1
                 WHERE id = $11 AND owner_id = $12
                 RETURNING id, owner_id, task_type_code, task_type_name, default_priority,
                           estimated_minutes, mergeable, insertable, release_strategy,
                           release_interval_minutes, release_batch_size, enabled,
                           created_at, updated_at, version
                "#,
            )
            .bind(&request.task_type_name)
            .bind(request.default_priority)
            .bind(request.estimated_minutes)
            .bind(request.mergeable)
            .bind(request.insertable)
            .bind(release_strategy_name(request.release_strategy))
            .bind(request.release_interval_minutes)
            .bind(request.release_batch_size)
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
                    estimated_minutes, mergeable, insertable, release_strategy,
                    release_interval_minutes, release_batch_size, enabled, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13)
                RETURNING id, owner_id, task_type_code, task_type_name, default_priority,
                          estimated_minutes, mergeable, insertable, release_strategy,
                          release_interval_minutes, release_batch_size, enabled,
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
            .bind(release_strategy_name(request.release_strategy))
            .bind(request.release_interval_minutes)
            .bind(request.release_batch_size)
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
        if let Some(value) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/task-engine/task-types/{task_type_code}/enabled"),
            now,
        )
        .await?
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
                       estimated_minutes, mergeable, insertable, release_strategy,
                       release_interval_minutes, release_batch_size, enabled,
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
            release_strategy: release_strategy(&row.release_strategy),
            release_interval_minutes: row.release_interval_minutes,
            release_batch_size: row.release_batch_size,
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
               estimated_minutes, mergeable, insertable, release_strategy,
               release_interval_minutes, release_batch_size, enabled,
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

fn release_strategy_name(value: TaskReleaseStrategy) -> &'static str {
    match value {
        TaskReleaseStrategy::Immediate => "immediate",
        TaskReleaseStrategy::Scheduled => "scheduled",
        TaskReleaseStrategy::Conditional => "conditional",
        TaskReleaseStrategy::Capacity => "capacity",
    }
}

fn release_strategy(value: &str) -> TaskReleaseStrategy {
    match value {
        "scheduled" => TaskReleaseStrategy::Scheduled,
        "conditional" => TaskReleaseStrategy::Conditional,
        "capacity" => TaskReleaseStrategy::Capacity,
        _ => TaskReleaseStrategy::Immediate,
    }
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
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<TaskType>, TaskTypeError> {
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
    idempotency::store_success(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        &resource_id,
        response,
        now,
    )
    .await
    .map_err(Into::into)
}

async fn lock_key(
    tx: &mut Transaction<'_, Postgres>,
    namespace: &str,
    owner_id: Uuid,
    key: &str,
) -> Result<(), TaskTypeError> {
    idempotency::lock_key(tx, namespace, owner_id, key)
        .await
        .map_err(Into::into)
}

fn request_hash(value: &Value) -> Result<String, TaskTypeError> {
    idempotency::request_hash(value).map_err(Into::into)
}

fn json_value<T: Serialize>(value: &T) -> Result<Value, TaskTypeError> {
    serde_json::to_value(value).map_err(|error| TaskTypeError::Serialize(error.to_string()))
}

fn map_database_error(error: sqlx::Error) -> TaskTypeError {
    TaskTypeError::Database(format!("{error:?}"))
}

impl From<IdempotencyError> for TaskTypeError {
    fn from(error: IdempotencyError) -> Self {
        match error {
            IdempotencyError::Conflict => Self::IdempotencyConflict,
            IdempotencyError::Database(error) => Self::Database(format!("{error:?}")),
            IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
}
