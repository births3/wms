//! T02：设备中台 HTTP 层（注册/启停/心跳/绑定/解绑）。

use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::device_platform_error::{
    idempotency_key, require_bind_manage, require_manage, require_monitor,
    DevicePlatformHandlerError,
};
use crate::device_service::{
    BindDeviceRequest, DeviceBindingResponse, DeviceError, DeviceResponse, DeviceService,
    RegisterDeviceRequest, UnbindRequest, UpdateDeviceRequest,
};

#[derive(Clone)]
pub struct DeviceAppState {
    pub service: DeviceService,
    pool: PgPool,
}

impl DeviceAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            service: DeviceService::new(pool.clone()),
            pool,
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
    pub warehouse_id: Uuid,
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
    request: Result<Json<RegisterDeviceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DeviceResponse>), DevicePlatformHandlerError> {
    let Json(req) = request?;
    require_manage(&ctx)?;
    ensure_warehouse_owner(&state.pool, &ctx, req.warehouse_id).await?;
    let key = idempotency_key(&headers)?;
    Ok((
        StatusCode::CREATED,
        Json(state.service.register(&ctx, req, &key).await?),
    ))
}

async fn list_devices_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    request: Result<Query<DeviceListQuery>, QueryRejection>,
) -> Result<Json<Vec<DeviceResponse>>, DevicePlatformHandlerError> {
    let Query(query) = request?;
    require_monitor(&ctx)?;
    ensure_warehouse_owner(&state.pool, &ctx, query.warehouse_id).await?;
    Ok(Json(
        state
            .service
            .list(
                &ctx,
                query.warehouse_id,
                query.device_type,
                query.online_status,
                query.enabled,
            )
            .await?,
    ))
}

async fn get_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    request: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<DeviceResponse>, DevicePlatformHandlerError> {
    let Path(id) = request?;
    require_monitor(&ctx)?;
    ensure_device_owner(&state.pool, &ctx, id).await?;
    Ok(Json(state.service.get(&ctx, id).await?))
}

async fn update_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    request: Result<Json<UpdateDeviceRequest>, JsonRejection>,
) -> Result<Json<DeviceResponse>, DevicePlatformHandlerError> {
    let Path(id) = path?;
    let Json(req) = request?;
    require_manage(&ctx)?;
    ensure_device_owner(&state.pool, &ctx, id).await?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.update(&ctx, id, req, &key).await?))
}

async fn heartbeat_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    request: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> Result<Json<DeviceResponse>, DevicePlatformHandlerError> {
    let Path(id) = request?;
    require_manage(&ctx)?;
    ensure_device_owner(&state.pool, &ctx, id).await?;
    let key = idempotency_key(&headers)?;
    Ok(Json(state.service.heartbeat(&ctx, id, &key).await?))
}

async fn bind_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    headers: HeaderMap,
    request: Result<Json<BindDeviceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DeviceBindingResponse>), DevicePlatformHandlerError> {
    let Json(req) = request?;
    require_bind_manage(&ctx)?;
    ensure_device_owner(&state.pool, &ctx, req.device_id).await?;
    let key = idempotency_key(&headers)?;
    Ok((
        StatusCode::CREATED,
        Json(state.service.bind(&ctx, req, &key).await?),
    ))
}

async fn unbind_device_handler(
    ctx: AuthContext,
    State(state): State<DeviceAppState>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    request: Result<Json<UnbindRequest>, JsonRejection>,
) -> Result<StatusCode, DevicePlatformHandlerError> {
    let Path(id) = path?;
    let Json(req) = request?;
    require_bind_manage(&ctx)?;
    ensure_binding_owner(&state.pool, &ctx, id).await?;
    let key = idempotency_key(&headers)?;
    state.service.unbind(&ctx, id, req, &key).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn ensure_warehouse_owner(
    pool: &PgPool,
    ctx: &AuthContext,
    warehouse_id: Uuid,
) -> Result<(), DevicePlatformHandlerError> {
    let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM warehouses WHERE id = $1")
        .bind(warehouse_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| DeviceError::Database(error.to_string()))?;
    if owner_id.is_some_and(|owner_id| owner_id != ctx.owner_id) {
        return Err(DeviceError::WarehouseForbidden.into());
    }
    Ok(())
}

async fn ensure_device_owner(
    pool: &PgPool,
    ctx: &AuthContext,
    device_id: Uuid,
) -> Result<(), DevicePlatformHandlerError> {
    let owner_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT warehouse.owner_id
          FROM iot_devices device
          JOIN warehouses warehouse ON warehouse.id = device.warehouse_id
         WHERE device.id = $1
        "#,
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    if owner_id.is_some_and(|owner_id| owner_id != ctx.owner_id) {
        return Err(DeviceError::WarehouseForbidden.into());
    }
    Ok(())
}

async fn ensure_binding_owner(
    pool: &PgPool,
    ctx: &AuthContext,
    binding_id: Uuid,
) -> Result<(), DevicePlatformHandlerError> {
    let owner_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT warehouse.owner_id
          FROM location_device_bindings binding
          JOIN warehouses warehouse ON warehouse.id = binding.warehouse_id
         WHERE binding.id = $1
        "#,
    )
    .bind(binding_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| DeviceError::Database(error.to_string()))?;
    if owner_id.is_some_and(|owner_id| owner_id != ctx.owner_id) {
        return Err(DeviceError::WarehouseForbidden.into());
    }
    Ok(())
}
