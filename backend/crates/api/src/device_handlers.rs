//! T02：设备中台 HTTP 层（注册/列表/详情/维护/心跳/绑定/解绑）。

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_platform_error::{
    idempotency_key, require_bind_manage, require_manage, require_monitor,
    DevicePlatformHandlerError,
};
use crate::device_service::{
    BindDeviceRequest, DeviceBindingResponse, DeviceResponse, DeviceService, RegisterDeviceRequest,
    UnbindRequest, UpdateDeviceRequest,
};

#[derive(Clone)]
pub struct DeviceAppState {
    pub service: DeviceService,
}

impl DeviceAppState {
    pub fn with_postgres(pool: sqlx::PgPool) -> Self {
        Self {
            service: DeviceService::new(pool),
        }
    }
}

pub fn device_router(state: DeviceAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/iot-devices",
            get(list_devices_handler).post(register_device_handler),
        )
        .route(
            "/api/v1/iot-devices/:id",
            get(get_device_handler).patch(update_device_handler),
        )
        .route("/api/v1/iot-devices/:id/heartbeat", post(heartbeat_handler))
        .route(
            "/api/v1/location-device-bindings",
            post(bind_device_handler),
        )
        .route(
            "/api/v1/location-device-bindings/:id/unbind",
            post(unbind_device_handler),
        )
        .with_state(state)
}

#[derive(Deserialize)]
pub struct DeviceListQuery {
    #[serde(default)]
    pub device_type: Option<String>,
    #[serde(default)]
    pub online_status: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

async fn register_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<(StatusCode, Json<DeviceResponse>), DevicePlatformHandlerError> {
    require_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok((
        StatusCode::CREATED,
        Json(state.service.register(&ctx, req, &key).await?),
    ))
}

async fn list_devices_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Query(query): Query<DeviceListQuery>,
) -> Result<Json<Vec<DeviceResponse>>, DevicePlatformHandlerError> {
    require_monitor(&ctx)?;
    Ok(Json(
        state
            .service
            .list(&ctx, query.device_type, query.online_status, query.enabled)
            .await?,
    ))
}

async fn get_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeviceResponse>, DevicePlatformHandlerError> {
    require_monitor(&ctx)?;
    Ok(Json(state.service.get(&ctx, id).await?))
}

async fn update_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDeviceRequest>,
) -> Result<Json<DeviceResponse>, DevicePlatformHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.update(&ctx, id, req).await?))
}

async fn heartbeat_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeviceResponse>, DevicePlatformHandlerError> {
    require_manage(&ctx)?;
    Ok(Json(state.service.heartbeat(&ctx, id).await?))
}

async fn bind_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    headers: HeaderMap,
    Json(req): Json<BindDeviceRequest>,
) -> Result<(StatusCode, Json<DeviceBindingResponse>), DevicePlatformHandlerError> {
    require_bind_manage(&ctx)?;
    let key = idempotency_key(&headers)?;
    Ok((
        StatusCode::CREATED,
        Json(state.service.bind(&ctx, req, &key).await?),
    ))
}

async fn unbind_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UnbindRequest>,
) -> Result<StatusCode, DevicePlatformHandlerError> {
    require_bind_manage(&ctx)?;
    state.service.unbind(&ctx, id, req).await?;
    Ok(StatusCode::NO_CONTENT)
}
