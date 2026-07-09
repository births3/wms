//! H4 企业微信通知与审批通道。

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    CreateH4ApprovalRequest, H4ApprovalCallbackRequest, H4ApprovalRecord, H4NotificationConfig,
    H4NotificationConfigListResponse, H4NotificationRecord, H4NotificationRecordListResponse,
    H4WechatSettings, H4WechatSettingsResponse, SendH4NotificationRequest,
    UpsertH4NotificationConfigRequest, UpsertH4WechatSettingsRequest,
};

use crate::{
    auth::AuthContext,
    wechat_notify_service::{H4RecordQuery, PgWechatNotifyService, WechatNotifyError},
};

pub const H4_READ_PERMISSION: &str = "h4.notify.read";
pub const H4_WRITE_PERMISSION: &str = "h4.notify.write";
pub const H4_SEND_PERMISSION: &str = "h4.notify.send";
pub const H4_APPROVAL_PERMISSION: &str = "h4.approval.write";
const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

#[derive(Clone, Debug)]
pub struct WechatNotifyAppState {
    pool: PgPool,
    service: PgWechatNotifyService,
}

#[derive(Debug)]
enum WechatNotifyHandlerError {
    Auth(crate::auth::AuthError),
    Wechat(WechatNotifyError),
    MissingIdempotencyKey,
}

impl WechatNotifyAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            pool,
            service: PgWechatNotifyService::new(),
        }
    }
}

impl From<crate::auth::AuthError> for WechatNotifyHandlerError {
    fn from(value: crate::auth::AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<WechatNotifyError> for WechatNotifyHandlerError {
    fn from(value: WechatNotifyError) -> Self {
        Self::Wechat(value)
    }
}

impl IntoResponse for WechatNotifyHandlerError {
    fn into_response(self) -> Response {
        if let WechatNotifyHandlerError::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            WechatNotifyHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H4_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            WechatNotifyHandlerError::Wechat(WechatNotifyError::EventNotFound) => (
                StatusCode::NOT_FOUND,
                "H4_EVENT_NOT_FOUND",
                "通知事件未配置或未启用",
            ),
            WechatNotifyHandlerError::Wechat(WechatNotifyError::TemplateInvalid) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H4_TEMPLATE_INVALID",
                "通知模板变量无法渲染",
            ),
            WechatNotifyHandlerError::Wechat(WechatNotifyError::NoRecipients) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H4_NO_RECIPIENTS",
                "通知接收人为空",
            ),
            WechatNotifyHandlerError::Wechat(WechatNotifyError::ApprovalNotFound) => (
                StatusCode::NOT_FOUND,
                "H4_APPROVAL_NOT_FOUND",
                "审批记录不存在",
            ),
            WechatNotifyHandlerError::Wechat(WechatNotifyError::InvalidStatus) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H4_APPROVAL_STATUS_INVALID",
                "审批结论非法",
            ),
            WechatNotifyHandlerError::Wechat(WechatNotifyError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "H4_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            WechatNotifyHandlerError::Wechat(WechatNotifyError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H4_REQUEST_INVALID",
                "H4 请求非法",
            ),
            WechatNotifyHandlerError::Wechat(
                WechatNotifyError::Audit(_)
                | WechatNotifyError::Database(_)
                | WechatNotifyError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H4_NOTIFY_INTERNAL",
                "H4 通知处理失败",
            ),
            WechatNotifyHandlerError::Auth(_) => unreachable!("auth returned above"),
        };
        (
            status,
            Json(wms_domain::ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn wechat_notify_router(state: WechatNotifyAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/wechat-notify/configs",
            get(list_configs_handler).post(upsert_config_handler),
        )
        .route(
            "/api/v1/wechat-notify/settings",
            get(get_wechat_settings_handler).post(upsert_wechat_settings_handler),
        )
        .route(
            "/api/v1/wechat-notify/send",
            post(send_notification_handler),
        )
        .route(
            "/api/v1/wechat-notify/approvals",
            post(create_approval_handler),
        )
        .route(
            "/api/v1/wechat-notify/approvals/:approval_id/callback",
            post(apply_approval_callback_handler),
        )
        .route("/api/v1/wechat-notify/records", get(list_records_handler))
        .route(
            "/api/v1/wechat-notify/records/:record_id/resend",
            post(resend_record_handler),
        )
        .with_state(state)
}

async fn list_configs_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
    Query(query): Query<H4RecordQuery>,
) -> Result<Json<H4NotificationConfigListResponse>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_configs(&state.pool, &ctx, query.event_type.as_deref())
            .await?,
    ))
}

async fn upsert_config_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertH4NotificationConfigRequest>,
) -> Result<Json<H4NotificationConfig>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .upsert_config(&state.pool, &ctx, req, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn get_wechat_settings_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
) -> Result<Json<H4WechatSettingsResponse>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_READ_PERMISSION)?;
    Ok(Json(
        state.service.get_wechat_settings(&state.pool, &ctx).await?,
    ))
}

async fn upsert_wechat_settings_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertH4WechatSettingsRequest>,
) -> Result<Json<H4WechatSettings>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .upsert_wechat_settings(&state.pool, &ctx, req, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn send_notification_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
    headers: HeaderMap,
    Json(req): Json<SendH4NotificationRequest>,
) -> Result<Json<Vec<H4NotificationRecord>>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_SEND_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .send_notification(&state.pool, &ctx, req, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn create_approval_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateH4ApprovalRequest>,
) -> Result<Json<H4ApprovalRecord>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_APPROVAL_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .create_approval(&state.pool, &ctx, req, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn apply_approval_callback_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
    Path(approval_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<H4ApprovalCallbackRequest>,
) -> Result<Json<H4ApprovalRecord>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_APPROVAL_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .apply_approval_callback(
            &state.pool,
            &ctx,
            approval_id,
            req,
            Utc::now(),
            &idempotency_key,
        )
        .await?;
    Ok(Json(result.value))
}

async fn list_records_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
    Query(query): Query<H4RecordQuery>,
) -> Result<Json<H4NotificationRecordListResponse>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_READ_PERMISSION)?;
    Ok(Json(
        state.service.list_records(&state.pool, &ctx, query).await?,
    ))
}

async fn resend_record_handler(
    ctx: AuthContext,
    State(state): State<WechatNotifyAppState>,
    Path(record_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<H4NotificationRecord>, WechatNotifyHandlerError> {
    ctx.require_permission(H4_SEND_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .resend_record(&state.pool, &ctx, record_id, Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, WechatNotifyHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or(WechatNotifyHandlerError::MissingIdempotencyKey)
}
