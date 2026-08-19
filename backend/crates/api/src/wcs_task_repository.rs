//! T03：指令任务仓储层（wcs_tasks 六态推进 / 事件匹配）。

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::device_service::DeviceError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WcsTaskRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub task_no: String,
    pub task_type: String,
    pub device_id: Uuid,
    pub location_id: Option<Uuid>,
    pub business_ref_type: Option<String>,
    pub business_ref_no: Option<String>,
    pub payload: Value,
    pub status: String,
    pub ack_payload: Value,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub idempotency_key: String,
    pub sent_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub version: i64,
    pub updated_at: DateTime<Utc>,
}

const TASK_COLUMNS: &str = "id, owner_id, task_no, task_type, device_id, location_id, \
     business_ref_type, business_ref_no, payload, status, ack_payload, error_code, \
     error_message, retry_count, max_retries, idempotency_key, sent_at, finished_at, \
     created_by, version, updated_at";

pub(crate) async fn insert_task(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    owner_id: Uuid,
    task_no: &str,
    task_type: &str,
    device_id: Uuid,
    location_id: Option<Uuid>,
    business_ref_type: Option<&str>,
    business_ref_no: Option<&str>,
    payload: Value,
    idempotency_key: &str,
    created_by: &str,
    now: DateTime<Utc>,
) -> Result<(), DeviceError> {
    sqlx::query(
        r#"
        INSERT INTO wcs_tasks (
            id, owner_id, task_no, task_type, device_id, location_id,
            business_ref_type, business_ref_no, payload, status, ack_payload,
            idempotency_key, created_by, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', '{}', $10, $11, $12, $12)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(task_no)
    .bind(task_type)
    .bind(device_id)
    .bind(location_id)
    .bind(business_ref_type)
    .bind(business_ref_no)
    .bind(payload)
    .bind(idempotency_key)
    .bind(created_by)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(())
}

pub(crate) async fn find_task_by_idempotency(
    pool: &PgPool,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<WcsTaskRow>, DeviceError> {
    sqlx::query_as::<_, WcsTaskRow>(&format!(
        "SELECT {TASK_COLUMNS} FROM wcs_tasks WHERE owner_id = $1 AND idempotency_key = $2"
    ))
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn get_task(
    pool: &PgPool,
    owner_id: Uuid,
    id: Uuid,
) -> Result<Option<WcsTaskRow>, DeviceError> {
    sqlx::query_as::<_, WcsTaskRow>(&format!(
        "SELECT {TASK_COLUMNS} FROM wcs_tasks WHERE owner_id = $1 AND id = $2"
    ))
    .bind(owner_id)
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn list_tasks(
    pool: &PgPool,
    owner_id: Uuid,
    status: Option<&str>,
    task_type: Option<&str>,
) -> Result<Vec<WcsTaskRow>, DeviceError> {
    sqlx::query_as::<_, WcsTaskRow>(&format!(
        r#"
        SELECT {TASK_COLUMNS}
          FROM wcs_tasks
         WHERE owner_id = $1
           AND ($2::text IS NULL OR status = $2)
           AND ($3::text IS NULL OR task_type = $3)
         ORDER BY created_at DESC
        "#
    ))
    .bind(owner_id)
    .bind(status)
    .bind(task_type)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

/// 乐观锁状态推进：仅允许从 from_statuses 迁移，version 不匹配则失败。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn transition(
    pool: &PgPool,
    owner_id: Uuid,
    id: Uuid,
    from_statuses: &[&str],
    to: &str,
    retry_count: Option<i32>,
    error_code: Option<&str>,
    error_message: Option<&str>,
    ack_payload: Option<Value>,
    sent_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    expected_version: i64,
    now: DateTime<Utc>,
) -> Result<Option<WcsTaskRow>, DeviceError> {
    let result = sqlx::query_as::<_, WcsTaskRow>(&format!(
        r#"
        UPDATE wcs_tasks
           SET status = $3,
               retry_count = COALESCE($4, retry_count),
               error_code = COALESCE($5, error_code),
               error_message = COALESCE($6, error_message),
               ack_payload = COALESCE($7::jsonb, ack_payload),
               sent_at = COALESCE($8, sent_at),
               finished_at = COALESCE($9, finished_at),
               version = version + 1,
               updated_at = $10
         WHERE owner_id = $12
           AND id = $1
           AND version = $11
           AND status = ANY($2)
         RETURNING {TASK_COLUMNS}
        "#
    ))
    .bind(id)
    .bind(from_statuses)
    .bind(to)
    .bind(retry_count)
    .bind(error_code)
    .bind(error_message)
    .bind(ack_payload)
    .bind(sent_at)
    .bind(finished_at)
    .bind(now)
    .bind(expected_version)
    .bind(owner_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(result)
}

/// 按设备+库位找未终态指定类型任务（PTL 亮灯互斥 / 事件认领）。
pub(crate) async fn find_active_task_by_device_location(
    pool: &PgPool,
    owner_id: Uuid,
    device_id: Uuid,
    location_id: Option<Uuid>,
    task_type: &str,
) -> Result<Option<WcsTaskRow>, DeviceError> {
    sqlx::query_as::<_, WcsTaskRow>(&format!(
        r#"
        SELECT {TASK_COLUMNS}
          FROM wcs_tasks
         WHERE owner_id = $4
           AND device_id = $1
           AND task_type = $3
           AND status IN ('pending', 'sent', 'executing', 'timeout')
           AND ($2::uuid IS NULL OR location_id = $2)
         ORDER BY created_at
         LIMIT 1
        "#
    ))
    .bind(device_id)
    .bind(location_id)
    .bind(task_type)
    .bind(owner_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

/// 孤儿事件：无 task_id 的 ptl_press 超过窗口未认领。
pub(crate) async fn list_orphan_press_events(
    pool: &PgPool,
    window_secs: i64,
    now: DateTime<Utc>,
) -> Result<Vec<(Uuid, Uuid, Option<Uuid>)>, DeviceError> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>)>(
        r#"
        SELECT id, device_id, location_id
          FROM iot_event_logs
         WHERE event_type = 'ptl_press'
           AND task_id IS NULL
           AND received_at < $1 - make_interval(secs => $2)
        "#,
    )
    .bind(now)
    .bind(window_secs)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(rows)
}

pub(crate) async fn link_event_to_task(
    pool: &PgPool,
    event_id: Uuid,
    task_id: Uuid,
) -> Result<(), DeviceError> {
    sqlx::query(
        r#"
        UPDATE iot_event_logs
           SET task_id = $2
         WHERE id = $1 AND task_id IS NULL
        "#,
    )
    .bind(event_id)
    .bind(task_id)
    .execute(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(())
}
