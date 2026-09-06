//! T02：设备中台服务层（注册/启停/心跳/绑定/离线扫描）。

use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::{append_event_in_tx, AuditDiff, AuditWriteRequest};
use crate::device_repository::{
    find_active_binding, find_device_by_code, get_binding, get_device, insert_binding,
    insert_device, insert_event_log, list_devices, list_stale_online_devices,
    location_belongs_to_warehouse, mark_offline_in_tx, touch_heartbeat_in_tx, DeviceRow,
};
use crate::h2_lifecycle::publish_event_in_tx;
use crate::idempotency;
use crate::operation_context::OperationContext as AuthContext;

const DEVICE_TYPES: [&str; 5] = ["agv", "ptl_light", "dws", "rfid_antenna", "stacker"];
const BIND_ROLES: [&str; 2] = ["ptl_light", "rfid_antenna"];
const HEARTBEAT_TIMEOUT_SECS: i64 = 90;

#[derive(Debug)]
pub enum DeviceError {
    DuplicateCode,
    TypeInvalid,
    NotFound,
    Disabled,
    Offline,
    BindConflict,
    BindDeviceMismatch,
    BindLocationMismatch,
    BindNotFound,
    TaskNotFound,
    TaskStateInvalid,
    TaskVoidBlocked,
    PtLightBusy,
    PtQtyDiffExceeded,
    PodMoveActive,
    EventTaskMismatch,
    LocationUnreachable,
    NumberingUnavailable,
    VersionConflict,
    IdempotencyConflict,
    WarehouseForbidden,
    Database(String),
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DeviceResponse {
    pub id: Uuid,
    pub warehouse_id: Uuid,
    pub device_code: String,
    pub device_type: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub protocol: String,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
    pub extra_config: serde_json::Value,
    pub online_status: String,
    pub last_heartbeat_at: Option<chrono::DateTime<chrono::Utc>>,
    pub enabled: bool,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<DeviceBindingResponse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_events: Vec<DeviceRecentEvent>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DeviceRecentEvent {
    pub id: Uuid,
    pub event_type: String,
    pub task_id: Option<Uuid>,
    pub payload: serde_json::Value,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

impl From<DeviceRow> for DeviceResponse {
    fn from(row: DeviceRow) -> Self {
        DeviceResponse {
            id: row.id,
            warehouse_id: row.warehouse_id,
            device_code: row.device_code,
            device_type: row.device_type,
            vendor: row.vendor,
            model: row.model,
            protocol: row.protocol,
            ip_address: row.ip_address,
            port: row.port,
            extra_config: row.extra_config,
            online_status: row.online_status,
            last_heartbeat_at: row.last_heartbeat_at,
            enabled: row.enabled,
            version: row.version,
            bindings: Vec::new(),
            recent_events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DeviceBindingResponse {
    pub id: Uuid,
    pub location_id: Uuid,
    pub device_id: Uuid,
    pub binding_role: String,
    pub point_address: Option<String>,
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_to: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<crate::device_repository::BindingRow> for DeviceBindingResponse {
    fn from(row: crate::device_repository::BindingRow) -> Self {
        DeviceBindingResponse {
            id: row.id,
            location_id: row.location_id,
            device_id: row.device_id,
            binding_role: row.binding_role,
            point_address: row.point_address,
            valid_from: row.valid_from,
            valid_to: row.valid_to,
        }
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct RegisterDeviceRequest {
    pub warehouse_id: Uuid,
    pub device_code: String,
    pub device_type: String,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub protocol: String,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub port: Option<i32>,
    #[serde(default)]
    pub extra_config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateDeviceRequest {
    pub expected_version: i64,
    #[serde(default)]
    pub device_code: Option<String>,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub port: Option<i32>,
    #[serde(default)]
    pub extra_config: Option<serde_json::Value>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct BindDeviceRequest {
    pub location_id: Uuid,
    pub device_id: Uuid,
    pub binding_role: String,
    #[serde(default)]
    pub point_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct UnbindRequest {
    pub reason: String,
}

#[derive(Clone)]
pub struct DeviceService {
    pool: PgPool,
}

impl DeviceService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 心跳超时阈值（秒）：规格 §10.5 默认 90。
    pub fn heartbeat_timeout_secs() -> i64 {
        HEARTBEAT_TIMEOUT_SECS
    }

    pub async fn register(
        &self,
        ctx: &AuthContext,
        req: RegisterDeviceRequest,
        idempotency_key: &str,
    ) -> Result<DeviceResponse, DeviceError> {
        ensure_warehouse_access(ctx, req.warehouse_id)?;
        if !DEVICE_TYPES.contains(&req.device_type.as_str()) {
            return Err(DeviceError::TypeInvalid);
        }
        let now = Utc::now();
        let hash = idempotency::request_hash(&req)
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(&mut tx, "iot_device", ctx.owner_id, idempotency_key)
            .await
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/iot-devices",
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?
        {
            return Ok(replay);
        }
        if find_device_by_code(&self.pool, req.warehouse_id, &req.device_code)
            .await?
            .is_some()
        {
            return Err(DeviceError::DuplicateCode);
        }
        let id = Uuid::new_v4();
        insert_device(
            &mut tx,
            id,
            req.warehouse_id,
            &req.device_code,
            &req.device_type,
            req.vendor.as_deref(),
            req.model.as_deref(),
            &req.protocol,
            req.ip_address.as_deref(),
            req.port,
            if req.extra_config.is_null() {
                json!({})
            } else {
                req.extra_config.clone()
            },
            now,
        )
        .await?;
        let response = DeviceResponse {
            id,
            warehouse_id: req.warehouse_id,
            device_code: req.device_code.clone(),
            device_type: req.device_type.clone(),
            vendor: req.vendor.clone(),
            model: req.model.clone(),
            protocol: req.protocol.clone(),
            ip_address: req.ip_address.clone(),
            port: req.port,
            extra_config: if req.extra_config.is_null() {
                json!({})
            } else {
                req.extra_config.clone()
            },
            online_status: "offline".into(),
            last_heartbeat_at: None,
            enabled: true,
            version: 1,
            bindings: Vec::new(),
            recent_events: Vec::new(),
        };
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/iot-devices",
            200,
            "iot_device",
            &id.to_string(),
            &response,
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "register_device",
                "M1",
                "iot_device",
                id.to_string(),
                None,
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(response)
    }

    pub async fn list(
        &self,
        ctx: &AuthContext,
        warehouse_id: Uuid,
        device_type: Option<String>,
        online_status: Option<String>,
        enabled: Option<bool>,
    ) -> Result<Vec<DeviceResponse>, DeviceError> {
        ensure_warehouse_access(ctx, warehouse_id)?;
        let rows = list_devices(
            &self.pool,
            warehouse_id,
            device_type.as_deref(),
            online_status.as_deref(),
            enabled,
        )
        .await?;
        Ok(rows.into_iter().map(DeviceResponse::from).collect())
    }

    pub async fn get(&self, ctx: &AuthContext, id: Uuid) -> Result<DeviceResponse, DeviceError> {
        let row = get_device(&self.pool, id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        ensure_warehouse_access(ctx, row.warehouse_id)?;
        let warehouse_id = row.warehouse_id;
        let mut response = DeviceResponse::from(row);
        response.bindings =
            crate::device_repository::list_bindings_for_device(&self.pool, warehouse_id, id)
                .await?
                .into_iter()
                .map(DeviceBindingResponse::from)
                .collect();
        response.recent_events = crate::device_repository::list_recent_events_for_device(
            &self.pool,
            warehouse_id,
            id,
            20,
        )
        .await?;
        Ok(response)
    }

    pub async fn update(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateDeviceRequest,
        idempotency_key: &str,
    ) -> Result<DeviceResponse, DeviceError> {
        let now = Utc::now();
        let path = format!("/api/v1/iot-devices/{id}");
        let hash = idempotency::request_hash(&json!({"id": id, "request": &req}))
            .map_err(idempotency_err)?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(&mut tx, "iot_device_update", ctx.owner_id, idempotency_key)
            .await
            .map_err(idempotency_err)?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "PATCH",
            &path,
            now,
        )
        .await
        .map_err(idempotency_err)?
        {
            return Ok(replay);
        }
        let existing = get_device(&self.pool, id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        ensure_warehouse_access(ctx, existing.warehouse_id)?;
        if let Some(device_code) = req.device_code.as_deref() {
            if find_device_by_code(&self.pool, existing.warehouse_id, device_code)
                .await?
                .is_some_and(|device| device.id != id)
            {
                return Err(DeviceError::DuplicateCode);
            }
        }
        let row = crate::device_repository::update_device_in_tx(
            &mut tx,
            existing.warehouse_id,
            id,
            req.device_code.as_deref(),
            req.vendor.as_deref(),
            req.model.as_deref(),
            req.ip_address.as_deref(),
            req.port,
            req.extra_config.as_ref(),
            req.enabled,
            req.expected_version,
            now,
        )
        .await?
        .ok_or(DeviceError::VersionConflict)?;
        if req.enabled == Some(false) && existing.online_status == "online" {
            crate::device_repository::mark_offline_in_tx(&mut tx, id, now).await?;
        }
        let response = DeviceResponse::from(row);
        idempotency::store_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "PATCH",
            &path,
            "iot_device",
            &id.to_string(),
            &response,
            now,
        )
        .await
        .map_err(idempotency_err)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "update_device",
                "M1",
                "iot_device",
                id.to_string(),
                None,
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(response)
    }

    pub async fn heartbeat(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        idempotency_key: &str,
    ) -> Result<DeviceResponse, DeviceError> {
        let now = Utc::now();
        let path = format!("/api/v1/iot-devices/{id}/heartbeat");
        let hash = idempotency::request_hash(&json!({"id": id})).map_err(idempotency_err)?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(
            &mut tx,
            "iot_device_heartbeat",
            ctx.owner_id,
            idempotency_key,
        )
        .await
        .map_err(idempotency_err)?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            now,
        )
        .await
        .map_err(idempotency_err)?
        {
            return Ok(replay);
        }
        let row = touch_heartbeat_in_tx(&mut tx, id, now)
            .await?
            .ok_or(DeviceError::NotFound)?;
        ensure_warehouse_access(ctx, row.warehouse_id)?;
        insert_event_log(
            &mut tx,
            Uuid::new_v4(),
            row.warehouse_id,
            row.id,
            "heartbeat",
            None,
            None,
            json!({"battery": null}),
            now,
        )
        .await?;
        let response = DeviceResponse::from(row);
        idempotency::store_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            "iot_device",
            &id.to_string(),
            &response,
            now,
        )
        .await
        .map_err(idempotency_err)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "heartbeat_device",
                "M1",
                "iot_device",
                id.to_string(),
                None,
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(response)
    }

    pub async fn bind(
        &self,
        ctx: &AuthContext,
        req: BindDeviceRequest,
        idempotency_key: &str,
    ) -> Result<DeviceBindingResponse, DeviceError> {
        if !BIND_ROLES.contains(&req.binding_role.as_str()) {
            return Err(DeviceError::TypeInvalid);
        }
        let device = get_device(&self.pool, req.device_id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        ensure_warehouse_access(ctx, device.warehouse_id)?;
        let device_type_matches = match req.binding_role.as_str() {
            "ptl_light" => device.device_type == "ptl_light",
            "rfid_antenna" => device.device_type == "rfid_antenna",
            _ => false,
        };
        if !device_type_matches {
            return Err(DeviceError::BindDeviceMismatch);
        }
        if !device.enabled {
            return Err(DeviceError::Disabled);
        }
        if device.online_status == "offline" {
            return Err(DeviceError::Offline);
        }
        if !location_belongs_to_warehouse(&self.pool, req.location_id, device.warehouse_id).await? {
            return Err(DeviceError::BindLocationMismatch);
        }
        let now = Utc::now();
        let hash = idempotency::request_hash(&req)
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(&mut tx, "device_binding", ctx.owner_id, idempotency_key)
            .await
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        if let Some(replay) = idempotency::replay(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/location-device-bindings",
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?
        {
            return Ok(replay);
        }
        if find_active_binding(
            &self.pool,
            device.warehouse_id,
            req.location_id,
            &req.binding_role,
        )
        .await?
        .is_some()
        {
            return Err(DeviceError::BindConflict);
        }
        let id = Uuid::new_v4();
        insert_binding(
            &mut tx,
            id,
            device.warehouse_id,
            req.location_id,
            req.device_id,
            &req.binding_role,
            req.point_address.as_deref(),
            now,
        )
        .await?;
        let row = crate::device_repository::BindingRow {
            id,
            warehouse_id: device.warehouse_id,
            location_id: req.location_id,
            device_id: req.device_id,
            binding_role: req.binding_role.clone(),
            point_address: req.point_address.clone(),
            valid_from: now,
            valid_to: None,
        };
        let response = DeviceBindingResponse::from(row);
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/location-device-bindings",
            201,
            "device_binding",
            &id.to_string(),
            &response,
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "bind_device_location",
                "M1",
                "location_device_binding",
                id.to_string(),
                None,
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(response)
    }

    pub async fn unbind(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: UnbindRequest,
        idempotency_key: &str,
    ) -> Result<(), DeviceError> {
        let now = Utc::now();
        let path = format!("/api/v1/location-device-bindings/{id}/unbind");
        let hash = idempotency::request_hash(&json!({"id": id, "request": &req}))
            .map_err(idempotency_err)?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        idempotency::lock_key(
            &mut tx,
            "device_binding_unbind",
            ctx.owner_id,
            idempotency_key,
        )
        .await
        .map_err(idempotency_err)?;
        if idempotency::replay::<serde_json::Value>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            now,
        )
        .await
        .map_err(idempotency_err)?
        .is_some()
        {
            return Ok(());
        }
        let binding = get_binding(&self.pool, id)
            .await?
            .ok_or(DeviceError::BindNotFound)?;
        ensure_warehouse_access(ctx, binding.warehouse_id)?;
        if binding.valid_to.is_some() {
            return Err(DeviceError::BindNotFound);
        }
        crate::device_repository::soft_unbind_in_tx(&mut tx, binding.warehouse_id, id, now).await?;
        idempotency::store_success_with_status(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &path,
            204,
            "location_device_binding",
            &id.to_string(),
            &json!({}),
            now,
        )
        .await
        .map_err(idempotency_err)?;
        append_event_in_tx(
            &mut tx,
            &AuditWriteRequest::from_auth_context(
                ctx,
                "unbind_device_location",
                "M1",
                "location_device_binding",
                id.to_string(),
                Some(AuditDiff {
                    before: json!({"valid_to": null}),
                    after: json!({"valid_to": now, "reason": req.reason}),
                    changed_keys: vec!["valid_to".into(), "reason".into()],
                }),
            ),
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn run_heartbeat_scan(&self) -> Result<usize, DeviceError> {
        self.run_heartbeat_scan_with_timeout(HEARTBEAT_TIMEOUT_SECS)
            .await
    }

    pub async fn run_heartbeat_scan_with_timeout(
        &self,
        timeout_secs: i64,
    ) -> Result<usize, DeviceError> {
        let now = Utc::now();
        let timeout = Duration::seconds(timeout_secs);
        let stale = list_stale_online_devices(&self.pool, timeout, now).await?;
        let mut count = 0usize;
        for device in &stale {
            let mut tx = self.pool.begin().await.map_err(db_err)?;
            mark_offline_in_tx(&mut tx, device.id, now).await?;
            let owner_id: Uuid =
                sqlx::query_scalar("SELECT owner_id FROM warehouses WHERE id = $1")
                    .bind(device.warehouse_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(|| {
                        DeviceError::Database(format!(
                            "warehouse {} missing while publishing device offline event",
                            device.warehouse_id
                        ))
                    })?;
            // H4 离线告警（business.device_offline）与离线状态同事务。
            publish_event_in_tx(
                &mut tx,
                owner_id,
                &format!("device_offline:{}", device.id),
                "business.device_offline",
                "M1",
                "iot_device",
                &device.id.to_string(),
                json!({
                    "device_id": device.id,
                    "device_code": device.device_code,
                    "offline_seconds": timeout_secs
                }),
                now,
            )
            .await
            .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
            tx.commit().await.map_err(db_err)?;
            count += 1;
        }
        Ok(count)
    }
}

fn db_err(error: sqlx::Error) -> DeviceError {
    DeviceError::Database(error.to_string())
}

pub(crate) fn ensure_warehouse_access(
    ctx: &AuthContext,
    warehouse_id: Uuid,
) -> Result<(), DeviceError> {
    if ctx
        .warehouse_scope
        .is_some_and(|allowed| allowed != warehouse_id)
    {
        Err(DeviceError::WarehouseForbidden)
    } else {
        Ok(())
    }
}

fn idempotency_err(error: idempotency::IdempotencyError) -> DeviceError {
    match error {
        idempotency::IdempotencyError::Conflict => DeviceError::IdempotencyConflict,
        other => DeviceError::Database(format!("{other:?}")),
    }
}
