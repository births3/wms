//! T02：设备中台服务层（注册/启停/心跳/绑定/离线扫描）。

use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_repository::{
    find_active_binding, find_device_by_code, get_binding, get_device, insert_binding,
    insert_device, insert_event_log, list_devices, list_stale_online_devices, mark_offline,
    soft_unbind, touch_heartbeat, update_device, DeviceRow,
};
use crate::h2_lifecycle::publish_event_in_tx;
use crate::idempotency;

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
    BindNotFound,
    TaskNotFound,
    TaskStateInvalid,
    TaskVoidBlocked,
    PtLightBusy,
    PtQtyDiffExceeded,
    PodMoveActive,
    EventTaskMismatch,
    LocationUnreachable,
    Database(String),
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
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
        }
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct RegisterDeviceRequest {
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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateDeviceRequest {
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

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BindDeviceRequest {
    pub location_id: Uuid,
    pub device_id: Uuid,
    pub binding_role: String,
    #[serde(default)]
    pub point_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
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

    pub async fn register(
        &self,
        ctx: &AuthContext,
        req: RegisterDeviceRequest,
        idempotency_key: &str,
    ) -> Result<DeviceResponse, DeviceError> {
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
        if find_device_by_code(&self.pool, ctx.owner_id, &req.device_code)
            .await?
            .is_some()
        {
            return Err(DeviceError::DuplicateCode);
        }
        let id = Uuid::new_v4();
        insert_device(
            &mut tx,
            id,
            ctx.owner_id,
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
            warehouse_id: ctx.owner_id,
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
        tx.commit().await.map_err(db_err)?;
        Ok(response)
    }

    pub async fn list(
        &self,
        ctx: &AuthContext,
        device_type: Option<String>,
        online_status: Option<String>,
        enabled: Option<bool>,
    ) -> Result<Vec<DeviceResponse>, DeviceError> {
        let rows = list_devices(
            &self.pool,
            ctx.owner_id,
            device_type.as_deref(),
            online_status.as_deref(),
            enabled,
        )
        .await?;
        Ok(rows.into_iter().map(DeviceResponse::from).collect())
    }

    pub async fn get(&self, _ctx: &AuthContext, id: Uuid) -> Result<DeviceResponse, DeviceError> {
        let row = get_device(&self.pool, id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        Ok(row.into())
    }

    pub async fn update(
        &self,
        _ctx: &AuthContext,
        id: Uuid,
        req: UpdateDeviceRequest,
    ) -> Result<DeviceResponse, DeviceError> {
        let existing = get_device(&self.pool, id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        update_device(
            &self.pool,
            id,
            req.vendor.as_deref(),
            req.model.as_deref(),
            req.ip_address.as_deref(),
            req.port,
            req.extra_config.as_ref(),
            req.enabled,
            Utc::now(),
        )
        .await?;
        // 停用设备视为 disabled 在线态
        if req.enabled == Some(false) && existing.online_status == "online" {
            mark_offline(&self.pool, id, Utc::now()).await?;
        }
        let row = get_device(&self.pool, id)
            .await?
            .ok_or(DeviceError::NotFound)?;
        Ok(row.into())
    }

    pub async fn heartbeat(
        &self,
        _ctx: &AuthContext,
        id: Uuid,
    ) -> Result<DeviceResponse, DeviceError> {
        let now = Utc::now();
        let row = touch_heartbeat(&self.pool, id, now)
            .await?
            .ok_or(DeviceError::NotFound)?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        insert_event_log(
            &mut tx,
            row.warehouse_id,
            row.id,
            "heartbeat",
            None,
            None,
            json!({"battery": null}),
            now,
        )
        .await?;
        tx.commit().await.map_err(db_err)?;
        Ok(row.into())
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
        if find_active_binding(&self.pool, req.location_id, &req.binding_role)
            .await?
            .is_some()
        {
            return Err(DeviceError::BindConflict);
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
        let id = Uuid::new_v4();
        insert_binding(
            &mut tx,
            id,
            ctx.owner_id,
            req.location_id,
            req.device_id,
            &req.binding_role,
            req.point_address.as_deref(),
            now,
        )
        .await?;
        let row = crate::device_repository::BindingRow {
            id,
            warehouse_id: ctx.owner_id,
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
            200,
            "device_binding",
            &id.to_string(),
            &response,
            now,
        )
        .await
        .map_err(|error| DeviceError::Database(format!("{error:?}")))?;
        tx.commit().await.map_err(db_err)?;
        Ok(response)
    }

    pub async fn unbind(
        &self,
        _ctx: &AuthContext,
        id: Uuid,
        _req: UnbindRequest,
    ) -> Result<(), DeviceError> {
        let binding = get_binding(&self.pool, id)
            .await?
            .ok_or(DeviceError::BindNotFound)?;
        if binding.valid_to.is_some() {
            return Err(DeviceError::BindNotFound);
        }
        soft_unbind(&self.pool, id, Utc::now()).await?;
        Ok(())
    }

    pub async fn run_heartbeat_scan(&self) -> Result<usize, DeviceError> {
        let now = Utc::now();
        let timeout = Duration::seconds(HEARTBEAT_TIMEOUT_SECS);
        let stale = list_stale_online_devices(&self.pool, timeout, now).await?;
        let mut count = 0usize;
        for device in &stale {
            mark_offline(&self.pool, device.id, now).await?;
            // H4 离线告警（business.device_offline）
            let mut tx = self.pool.begin().await.map_err(db_err)?;
            publish_event_in_tx(
                &mut tx,
                device.warehouse_id,
                &format!("device_offline:{}", device.id),
                "business.device_offline",
                "M1",
                "iot_device",
                &device.id.to_string(),
                json!({
                    "device_id": device.id,
                    "device_code": device.device_code,
                    "offline_seconds": HEARTBEAT_TIMEOUT_SECS
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
