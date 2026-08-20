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

pub(crate) const TASK_COLUMNS: &str = "id, owner_id, task_no, task_type, device_id, location_id, \
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
    .map_err(|error| match &error {
        sqlx::Error::Database(database)
            if database.constraint() == Some("wcs_tasks_active_ptl_light_on_device_uq") =>
        {
            DeviceError::PtLightBusy
        }
        sqlx::Error::Database(database)
            if database.constraint() == Some("wcs_tasks_active_pod_move_code_uq") =>
        {
            DeviceError::PodMoveActive
        }
        _ => DeviceError::Database(error.to_string()),
    })?;
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

/// 状态推进参数（收敛 13 个位置参数）。
pub(crate) struct TaskTransition<'a> {
    pub owner_id: Uuid,
    pub id: Uuid,
    pub from_statuses: &'a [&'a str],
    pub to: &'a str,
    pub retry_count: Option<i32>,
    pub error_code: Option<&'a str>,
    pub error_message: Option<&'a str>,
    pub ack_payload: Option<Value>,
    pub sent_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub expected_version: i64,
    pub now: DateTime<Utc>,
}

/// 乐观锁状态推进：仅允许从 from_statuses 迁移，version 不匹配则失败。
pub(crate) async fn transition(
    pool: &PgPool,
    t: TaskTransition<'_>,
) -> Result<Option<WcsTaskRow>, DeviceError> {
    let TaskTransition {
        owner_id,
        id,
        from_statuses,
        to,
        retry_count,
        error_code,
        error_message,
        ack_payload,
        sent_at,
        finished_at,
        expected_version,
        now,
    } = t;
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

pub(crate) async fn find_active_task_by_device_location(
    pool: &PgPool,
    owner_id: Uuid,
    device_id: Uuid,
    location_id: Option<Uuid>,
    task_type: &str,
    pod_code: Option<&str>,
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
           AND ($5::text IS NULL OR payload->>'pod_code' = $5)
         ORDER BY created_at
         LIMIT 1
        "#
    ))
    .bind(device_id)
    .bind(location_id)
    .bind(task_type)
    .bind(owner_id)
    .bind(pod_code)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

/// I4：同一货架（payload.pod_code）同一时刻最多一个未终态 pod_move，不限设备。
pub(crate) async fn find_active_pod_move(
    pool: &PgPool,
    owner_id: Uuid,
    pod_code: &str,
) -> Result<Option<WcsTaskRow>, DeviceError> {
    sqlx::query_as::<_, WcsTaskRow>(&format!(
        r#"
        SELECT {TASK_COLUMNS}
          FROM wcs_tasks
         WHERE owner_id = $1
           AND task_type = 'pod_move'
           AND status IN ('pending', 'sent', 'executing', 'timeout')
           AND payload->>'pod_code' = $2
         ORDER BY created_at
         LIMIT 1
        "#
    ))
    .bind(owner_id)
    .bind(pod_code)
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
           AND NOT EXISTS (
                SELECT 1 FROM wcs_tasks
                 WHERE wcs_tasks.owner_id = iot_event_logs.warehouse_id
                   AND wcs_tasks.device_id = iot_event_logs.device_id
                   AND wcs_tasks.task_type = 'ptl_light_on'
                   AND wcs_tasks.status = 'succeeded'
                   AND wcs_tasks.created_at >= iot_event_logs.received_at
                   AND wcs_tasks.created_at <= iot_event_logs.received_at + make_interval(secs => $2)
           )
        "#,
    )
    .bind(now)
    .bind(window_secs)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(rows)
}

/// AGV 格口不可达标记：按货架编码置位/清除（pod_move executing 置位，终态清除）。
/// 格口不可达校验（I5）：库位存在且不可达标记非空 → 阻断。
pub(crate) async fn location_is_unreachable(
    pool: &PgPool,
    owner_id: Uuid,
    location_id: Uuid,
) -> Result<bool, DeviceError> {
    let unreachable: Option<bool> = sqlx::query_scalar(
        r#"
        SELECT (agv_unreachable_at IS NOT NULL)
          FROM warehouse_locations
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(location_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(unreachable.unwrap_or(false))
}

pub(crate) async fn set_pod_unreachable_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    pod_code: &str,
    ts: DateTime<Utc>,
) -> Result<u64, DeviceError> {
    let affected = sqlx::query(
        r#"
        UPDATE warehouse_locations
           SET agv_unreachable_at = $3, updated_at = $3
         WHERE owner_id = $1 AND agv_pod_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(pod_code)
    .bind(ts)
    .execute(&mut **tx)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?
    .rows_affected();
    Ok(affected)
}

pub(crate) async fn clear_pod_unreachable_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    pod_code: &str,
    ts: DateTime<Utc>,
) -> Result<u64, DeviceError> {
    let affected = sqlx::query(
        r#"
        UPDATE warehouse_locations
           SET agv_unreachable_at = NULL, updated_at = $3
         WHERE owner_id = $1 AND agv_pod_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(pod_code)
    .bind(ts)
    .execute(&mut **tx)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?
    .rows_affected();
    Ok(affected)
}

/// 窗口内未认领的 ptl_press（task_id 为空且未超窗），供任务到达后认领。
pub(crate) async fn list_pending_press_in_window(
    pool: &PgPool,
    device_id: Uuid,
    location_id: Option<Uuid>,
    window_secs: i64,
    now: DateTime<Utc>,
) -> Result<Vec<(Uuid, Option<Uuid>, Value)>, DeviceError> {
    sqlx::query_as::<_, (Uuid, Option<Uuid>, Value)>(
        r#"
        SELECT id, location_id, payload
          FROM iot_event_logs
         WHERE event_type = 'ptl_press'
           AND task_id IS NULL
           AND device_id = $1
           AND ($2::uuid IS NULL OR location_id = $2)
           AND received_at >= $3 - make_interval(secs => $4)
         ORDER BY received_at
        "#,
    )
    .bind(device_id)
    .bind(location_id)
    .bind(now)
    .bind(window_secs)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn list_affected_location_ids(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<Vec<Uuid>, DeviceError> {
    sqlx::query_scalar(
        r#"
        SELECT id FROM warehouse_locations
         WHERE owner_id = $1 AND agv_unreachable_at IS NOT NULL
         ORDER BY location_code
        "#,
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

/// 同事务状态推进（账务与状态必须同一事务，I7）。
pub(crate) async fn transition_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    t: TaskTransition<'_>,
) -> Result<Option<WcsTaskRow>, DeviceError> {
    let TaskTransition {
        owner_id,
        id,
        from_statuses,
        to,
        retry_count,
        error_code,
        error_message,
        ack_payload,
        sent_at,
        finished_at,
        expected_version,
        now,
    } = t;
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
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(result)
}

pub(crate) async fn list_events(
    pool: &PgPool,
    warehouse_id: Uuid,
    device_id: Option<Uuid>,
    event_type: Option<&str>,
    limit: i64,
) -> Result<
    Vec<(
        Uuid,
        Uuid,
        String,
        Option<Uuid>,
        Value,
        chrono::DateTime<chrono::Utc>,
    )>,
    DeviceError,
> {
    sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            Option<Uuid>,
            Value,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, device_id, event_type, task_id, payload, received_at
          FROM iot_event_logs
         WHERE warehouse_id = $1
           AND ($2::uuid IS NULL OR device_id = $2)
           AND ($3::text IS NULL OR event_type = $3)
         ORDER BY received_at DESC
         LIMIT $4
        "#,
    )
    .bind(warehouse_id)
    .bind(device_id)
    .bind(event_type)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

/// 设备大盘汇总：设备状态计数与异常任务计数。
pub(crate) async fn device_dashboard_summary(
    pool: &PgPool,
    warehouse_id: Uuid,
    owner_id: Uuid,
) -> Result<(i64, i64, i64, i64, i64, i64), DeviceError> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64)>(
        r#"
        SELECT
            (SELECT count(*) FROM iot_devices WHERE warehouse_id = $1) AS total_devices,
            (SELECT count(*) FROM iot_devices WHERE warehouse_id = $1 AND online_status = 'online') AS online_devices,
            (SELECT count(*) FROM iot_devices WHERE warehouse_id = $1 AND online_status = 'offline') AS offline_devices,
            (SELECT count(*) FROM wcs_tasks WHERE owner_id = $2 AND status = 'failed') AS failed_tasks,
            (SELECT count(*) FROM wcs_tasks WHERE owner_id = $2 AND status = 'timeout') AS timeout_tasks,
            (SELECT count(*) FROM wcs_tasks WHERE owner_id = $2 AND status IN ('pending', 'sent', 'executing')) AS pending_tasks
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(row)
}
