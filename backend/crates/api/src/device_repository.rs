//! T02：设备中台仓储层（iot_devices / location_device_bindings 读写）。

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::device_service::DeviceError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    pub id: Uuid,
    pub warehouse_id: Uuid,
    pub device_code: String,
    pub device_type: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub protocol: String,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
    pub extra_config: Value,
    pub online_status: String,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BindingRow {
    pub id: Uuid,
    pub warehouse_id: Uuid,
    pub location_id: Uuid,
    pub device_id: Uuid,
    pub binding_role: String,
    pub point_address: Option<String>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

const DEVICE_COLUMNS: &str = "id, warehouse_id, device_code, device_type, vendor, model, \
     protocol, ip_address, port, extra_config, online_status, last_heartbeat_at, enabled, \
     version, created_at, updated_at";

pub(crate) async fn insert_device(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    warehouse_id: Uuid,
    device_code: &str,
    device_type: &str,
    vendor: Option<&str>,
    model: Option<&str>,
    protocol: &str,
    ip_address: Option<&str>,
    port: Option<i32>,
    extra_config: Value,
    now: DateTime<Utc>,
) -> Result<(), DeviceError> {
    sqlx::query(
        r#"
        INSERT INTO iot_devices (
            id, warehouse_id, device_code, device_type, vendor, model, protocol,
            ip_address, port, extra_config, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
        "#,
    )
    .bind(id)
    .bind(warehouse_id)
    .bind(device_code)
    .bind(device_type)
    .bind(vendor)
    .bind(model)
    .bind(protocol)
    .bind(ip_address)
    .bind(port)
    .bind(extra_config)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| {
        if error
            .to_string()
            .contains("iot_devices_warehouse_id_device_code_key")
        {
            DeviceError::DuplicateCode
        } else {
            DeviceError::Database(error.to_string())
        }
    })?;
    Ok(())
}

pub(crate) async fn find_device_by_code(
    pool: &PgPool,
    warehouse_id: Uuid,
    device_code: &str,
) -> Result<Option<DeviceRow>, DeviceError> {
    sqlx::query_as::<_, DeviceRow>(&format!(
        "SELECT {DEVICE_COLUMNS} FROM iot_devices WHERE warehouse_id = $1 AND device_code = $2"
    ))
    .bind(warehouse_id)
    .bind(device_code)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn get_device(pool: &PgPool, id: Uuid) -> Result<Option<DeviceRow>, DeviceError> {
    sqlx::query_as::<_, DeviceRow>(&format!(
        "SELECT {DEVICE_COLUMNS} FROM iot_devices WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn list_devices(
    pool: &PgPool,
    warehouse_id: Uuid,
    device_type: Option<&str>,
    online_status: Option<&str>,
    enabled: Option<bool>,
) -> Result<Vec<DeviceRow>, DeviceError> {
    sqlx::query_as::<_, DeviceRow>(&format!(
        r#"
        SELECT {DEVICE_COLUMNS}
          FROM iot_devices
         WHERE warehouse_id = $1
           AND ($2::text IS NULL OR device_type = $2)
           AND ($3::text IS NULL OR online_status = $3)
           AND ($4::bool IS NULL OR enabled = $4)
         ORDER BY device_code
        "#
    ))
    .bind(warehouse_id)
    .bind(device_type)
    .bind(online_status)
    .bind(enabled)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn touch_heartbeat(
    pool: &PgPool,
    id: Uuid,
    now: DateTime<Utc>,
) -> Result<Option<DeviceRow>, DeviceError> {
    sqlx::query_as::<_, DeviceRow>(&format!(
        r#"
        UPDATE iot_devices
           SET last_heartbeat_at = $2,
               online_status = CASE WHEN enabled THEN 'online' ELSE 'disabled' END,
               updated_at = $2
         WHERE id = $1
         RETURNING {DEVICE_COLUMNS}
        "#
    ))
    .bind(id)
    .bind(now)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn insert_event_log(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    warehouse_id: Uuid,
    device_id: Uuid,
    event_type: &str,
    task_id: Option<Uuid>,
    location_id: Option<Uuid>,
    payload: Value,
    now: DateTime<Utc>,
) -> Result<bool, DeviceError> {
    let inserted = sqlx::query(
        r#"
        INSERT INTO iot_event_logs (
            id, warehouse_id, device_id, event_type, task_id, location_id, payload,
            occurred_at, received_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(warehouse_id)
    .bind(device_id)
    .bind(event_type)
    .bind(task_id)
    .bind(location_id)
    .bind(payload)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(inserted.rows_affected() == 1)
}

pub(crate) async fn insert_binding(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    warehouse_id: Uuid,
    location_id: Uuid,
    device_id: Uuid,
    binding_role: &str,
    point_address: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), DeviceError> {
    sqlx::query(
        r#"
        INSERT INTO location_device_bindings (
            id, warehouse_id, location_id, device_id, binding_role, point_address,
            valid_from, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7, $7)
        "#,
    )
    .bind(id)
    .bind(warehouse_id)
    .bind(location_id)
    .bind(device_id)
    .bind(binding_role)
    .bind(point_address)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| {
        if error
            .to_string()
            .contains("location_device_bindings_active_uidx")
        {
            DeviceError::BindConflict
        } else {
            DeviceError::Database(error.to_string())
        }
    })?;
    Ok(())
}

pub(crate) async fn find_active_binding(
    pool: &PgPool,
    warehouse_id: Uuid,
    location_id: Uuid,
    binding_role: &str,
) -> Result<Option<BindingRow>, DeviceError> {
    sqlx::query_as::<_, BindingRow>(
        r#"
        SELECT id, warehouse_id, location_id, device_id, binding_role, point_address,
               valid_from, valid_to
          FROM location_device_bindings
         WHERE warehouse_id = $1 AND location_id = $2 AND binding_role = $3 AND valid_to IS NULL
        "#,
    )
    .bind(warehouse_id)
    .bind(location_id)
    .bind(binding_role)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn get_binding(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<BindingRow>, DeviceError> {
    sqlx::query_as::<_, BindingRow>(
        r#"
        SELECT id, warehouse_id, location_id, device_id, binding_role, point_address,
               valid_from, valid_to
          FROM location_device_bindings
         WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn list_stale_online_devices(
    pool: &PgPool,
    heartbeat_timeout: chrono::Duration,
    now: DateTime<Utc>,
) -> Result<Vec<DeviceRow>, DeviceError> {
    sqlx::query_as::<_, DeviceRow>(&format!(
        r#"
        SELECT {DEVICE_COLUMNS}
          FROM iot_devices
         WHERE online_status = 'online'
           AND last_heartbeat_at < $1 - make_interval(secs => $2)
        "#
    ))
    .bind(now)
    .bind(heartbeat_timeout.num_seconds())
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn list_bindings_for_device(
    pool: &PgPool,
    warehouse_id: Uuid,
    device_id: Uuid,
) -> Result<Vec<BindingRow>, DeviceError> {
    sqlx::query_as::<_, BindingRow>(
        r#"
        SELECT id, warehouse_id, location_id, device_id, binding_role, point_address,
               valid_from, valid_to
          FROM location_device_bindings
         WHERE warehouse_id = $1 AND device_id = $2 AND valid_to IS NULL
         ORDER BY valid_from DESC
        "#,
    )
    .bind(warehouse_id)
    .bind(device_id)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn list_recent_events_for_device(
    pool: &PgPool,
    warehouse_id: Uuid,
    device_id: Uuid,
    limit: i64,
) -> Result<Vec<crate::device_service::DeviceRecentEvent>, DeviceError> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<Uuid>, Value, DateTime<Utc>)>(
        r#"
        SELECT id, event_type, task_id, payload, received_at
          FROM iot_event_logs
         WHERE warehouse_id = $1 AND device_id = $2
         ORDER BY received_at DESC
         LIMIT $3
        "#,
    )
    .bind(warehouse_id)
    .bind(device_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|(id, event_type, task_id, payload, received_at)| {
            crate::device_service::DeviceRecentEvent {
                id,
                event_type,
                task_id,
                payload,
                received_at,
            }
        })
        .collect())
}

pub(crate) async fn update_device_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    warehouse_id: Uuid,
    id: Uuid,
    device_code: Option<&str>,
    vendor: Option<&str>,
    model: Option<&str>,
    ip_address: Option<&str>,
    port: Option<i32>,
    extra_config: Option<&Value>,
    enabled: Option<bool>,
    expected_version: i64,
    now: DateTime<Utc>,
) -> Result<Option<DeviceRow>, DeviceError> {
    sqlx::query_as::<_, DeviceRow>(&format!(
        r#"
        UPDATE iot_devices
           SET device_code = COALESCE($2, device_code),
               vendor = COALESCE($3, vendor),
               model = COALESCE($4, model),
               ip_address = COALESCE($5, ip_address),
               port = COALESCE($6, port),
               extra_config = COALESCE($7, extra_config),
               enabled = COALESCE($8, enabled),
               version = version + 1,
               updated_at = $9
         WHERE warehouse_id = $1
           AND id = $10
           AND version = $11
         RETURNING {DEVICE_COLUMNS}
        "#
    ))
    .bind(warehouse_id)
    .bind(device_code)
    .bind(vendor)
    .bind(model)
    .bind(ip_address)
    .bind(port)
    .bind(extra_config)
    .bind(enabled)
    .bind(now)
    .bind(id)
    .bind(expected_version)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))
}

pub(crate) async fn mark_offline_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), DeviceError> {
    sqlx::query(
        r#"
        UPDATE iot_devices
           SET online_status = 'offline', updated_at = $2
         WHERE id = $1 AND online_status = 'online'
        "#,
    )
    .bind(id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(())
}

pub(crate) async fn soft_unbind_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    warehouse_id: Uuid,
    id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), DeviceError> {
    sqlx::query(
        r#"
        UPDATE location_device_bindings
           SET valid_to = $3, updated_at = $3
         WHERE warehouse_id = $1 AND id = $2
        "#,
    )
    .bind(warehouse_id)
    .bind(id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(())
}

pub(crate) async fn mark_offline(
    pool: &PgPool,
    id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), DeviceError> {
    sqlx::query(
        r#"
        UPDATE iot_devices
           SET online_status = 'offline', updated_at = $2
         WHERE id = $1 AND online_status = 'online'
        "#,
    )
    .bind(id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    Ok(())
}
