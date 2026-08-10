//! US-H9-011 print device HTTP handlers: sites, printers, trays and leases.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    CreatePrintSiteRequest, CreatePrinterRequest, CreatePrinterTrayRequest,
    CreateSiteOwnerMappingRequest, DeviceLease, DeviceLeaseListResponse, ErrorResponse, PageMeta,
    PrintSite, PrintSiteListResponse, PrintSiteOwnerMapping, PrintSiteOwnerMappingListResponse,
    Printer, PrinterListResponse, PrinterTestPrint, PrinterTray, PrinterTrayListResponse,
    ReleaseDeviceLeaseRequest, TestPrintRequest, UpdatePrinterRequest, UpdatePrinterTrayRequest,
};

use crate::{
    auth::{AuthContext, AuthError},
    print_device::{PrintDeviceError, PrintDeviceService, DEVICE_LEASE_RELEASE_PERMISSION},
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const READ_PERMISSION: &str = "h9.print_device.read";
const WRITE_PERMISSION: &str = "h9.print_device.write";

/// H9 print device HTTP state.
#[derive(Clone, Debug)]
pub struct PrintDeviceAppState {
    service: PrintDeviceService,
}

impl PrintDeviceAppState {
    /// Builds the H9 print device HTTP state.
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            service: PrintDeviceService::with_postgres(pool),
        }
    }
}

#[derive(Debug)]
enum PrintDeviceHandlerError {
    Auth(AuthError),
    Device(PrintDeviceError),
    MissingIdempotencyKey,
}

impl From<AuthError> for PrintDeviceHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<PrintDeviceError> for PrintDeviceHandlerError {
    fn from(value: PrintDeviceError) -> Self {
        Self::Device(value)
    }
}

impl IntoResponse for PrintDeviceHandlerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Auth(error) => return error.into_response(),
            Self::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H9_PRINT_DEVICE_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key".to_string(),
            ),
            Self::Device(PrintDeviceError::InvalidRequest) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_PRINT_DEVICE_INVALID",
                "打印设备参数非法".to_string(),
            ),
            Self::Device(PrintDeviceError::ConfirmationRequired) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H9_DEVICE_LEASE_CONFIRM_REQUIRED",
                "人工释放租约必须携带二次确认".to_string(),
            ),
            Self::Device(PrintDeviceError::ReleasePermissionRequired) => (
                StatusCode::FORBIDDEN,
                "H9_DEVICE_LEASE_RELEASE_FORBIDDEN",
                "缺少人工释放租约专用权限".to_string(),
            ),
            Self::Device(PrintDeviceError::OwnerPermissionRequired) => (
                StatusCode::FORBIDDEN,
                "H9_PRINT_DEVICE_OWNER_FORBIDDEN",
                "缺少打印站点映射货主的管理权限".to_string(),
            ),
            Self::Device(PrintDeviceError::SiteNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_PRINT_SITE_NOT_FOUND",
                "物理打印站点不存在".to_string(),
            ),
            Self::Device(PrintDeviceError::SiteCodeConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINT_SITE_CODE_CONFLICT",
                "站点编码已存在".to_string(),
            ),
            Self::Device(PrintDeviceError::MappingNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_PRINT_SITE_MAPPING_NOT_FOUND",
                "站点货主仓映射不存在".to_string(),
            ),
            Self::Device(PrintDeviceError::MappingConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINT_SITE_MAPPING_CONFLICT",
                "同一货主+仓库已映射到该站点".to_string(),
            ),
            Self::Device(PrintDeviceError::MappingAlreadyDisabled) => (
                StatusCode::CONFLICT,
                "H9_PRINT_SITE_MAPPING_DISABLED",
                "站点货主仓映射已停用".to_string(),
            ),
            Self::Device(PrintDeviceError::PrinterNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_PRINTER_NOT_FOUND",
                "打印机不存在".to_string(),
            ),
            Self::Device(PrintDeviceError::PrinterNameConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINTER_NAME_CONFLICT",
                "同站点打印机名称已存在".to_string(),
            ),
            Self::Device(PrintDeviceError::PrinterDisabled) => (
                StatusCode::CONFLICT,
                "H9_PRINTER_DISABLED",
                "打印机已停用，不能执行测试打印".to_string(),
            ),
            Self::Device(PrintDeviceError::TrayNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_PRINTER_TRAY_NOT_FOUND",
                "纸盒不存在或不属于该打印机".to_string(),
            ),
            Self::Device(PrintDeviceError::TrayConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINTER_TRAY_CONFLICT",
                "同打印机纸盒设备标识已存在".to_string(),
            ),
            Self::Device(PrintDeviceError::TrayDisabled) => (
                StatusCode::CONFLICT,
                "H9_PRINTER_TRAY_DISABLED",
                "纸盒已停用，不能执行测试打印".to_string(),
            ),
            Self::Device(PrintDeviceError::LeaseNotFound) => (
                StatusCode::NOT_FOUND,
                "H9_DEVICE_LEASE_NOT_FOUND",
                "设备租约不存在".to_string(),
            ),
            Self::Device(PrintDeviceError::LeaseAlreadyReleased) => (
                StatusCode::CONFLICT,
                "H9_DEVICE_LEASE_ALREADY_RELEASED",
                "设备租约已释放".to_string(),
            ),
            Self::Device(PrintDeviceError::LeaseBusy(state)) => (
                StatusCode::CONFLICT,
                "H9_DEVICE_LEASE_BUSY",
                format!("租约处于 {state} 硬安全状态，必须先完成打印结果确认或对账"),
            ),
            Self::Device(PrintDeviceError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "H9_PRINT_DEVICE_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用".to_string(),
            ),
            Self::Device(PrintDeviceError::Audit(_))
            | Self::Device(PrintDeviceError::Database(_))
            | Self::Device(PrintDeviceError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H9_PRINT_DEVICE_FAILED",
                "打印设备维护失败".to_string(),
            ),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message,
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

/// Builds the H9 print device routes.
pub fn print_device_router(state: PrintDeviceAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/print-devices/sites",
            get(list_sites_handler).post(create_site_handler),
        )
        .route(
            "/api/v1/print-devices/sites/:site_id/owner-mappings",
            get(list_site_owner_mappings_handler).post(create_site_owner_mapping_handler),
        )
        .route(
            "/api/v1/print-devices/sites/:site_id/owner-mappings/:mapping_id/disable",
            post(disable_site_owner_mapping_handler),
        )
        .route(
            "/api/v1/print-devices/printers",
            get(list_printers_handler).post(create_printer_handler),
        )
        .route(
            "/api/v1/print-devices/printers/:printer_id",
            patch(update_printer_handler),
        )
        .route(
            "/api/v1/print-devices/printers/:printer_id/trays",
            get(list_printer_trays_handler).post(create_printer_tray_handler),
        )
        .route(
            "/api/v1/print-devices/printers/:printer_id/trays/:tray_id",
            patch(update_printer_tray_handler),
        )
        .route(
            "/api/v1/print-devices/printers/:printer_id/test-print",
            post(test_print_handler),
        )
        .route("/api/v1/print-devices/leases", get(list_leases_handler))
        .route(
            "/api/v1/print-devices/leases/:lease_id/release",
            post(release_lease_handler),
        )
        .with_state(state)
}

async fn list_sites_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
) -> Result<Json<PrintSiteListResponse>, PrintDeviceHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.service.list_sites(&ctx).await?))
}

async fn create_site_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    headers: HeaderMap,
    Json(request): Json<CreatePrintSiteRequest>,
) -> Result<Json<PrintSite>, PrintDeviceHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .create_site(
            &ctx,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn list_site_owner_mappings_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path(site_id): Path<Uuid>,
) -> Result<Json<PrintSiteOwnerMappingListResponse>, PrintDeviceHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state
            .service
            .list_site_owner_mappings(&ctx, site_id)
            .await?,
    ))
}

async fn create_site_owner_mapping_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path(site_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreateSiteOwnerMappingRequest>,
) -> Result<Json<PrintSiteOwnerMapping>, PrintDeviceHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .create_site_owner_mapping(
            &ctx,
            site_id,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn disable_site_owner_mapping_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path((site_id, mapping_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
) -> Result<Json<PrintSiteOwnerMapping>, PrintDeviceHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .disable_site_owner_mapping(
            &ctx,
            site_id,
            mapping_id,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn list_printers_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
) -> Result<Json<PrinterListResponse>, PrintDeviceHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(state.service.list_printers(&ctx).await?))
}

async fn create_printer_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    headers: HeaderMap,
    Json(request): Json<CreatePrinterRequest>,
) -> Result<Json<Printer>, PrintDeviceHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .create_printer(
            &ctx,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn update_printer_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path(printer_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<UpdatePrinterRequest>,
) -> Result<Json<Printer>, PrintDeviceHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .update_printer(
            &ctx,
            printer_id,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn list_printer_trays_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path(printer_id): Path<Uuid>,
) -> Result<Json<PrinterTrayListResponse>, PrintDeviceHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    Ok(Json(
        state.service.list_printer_trays(&ctx, printer_id).await?,
    ))
}

async fn create_printer_tray_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path(printer_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CreatePrinterTrayRequest>,
) -> Result<Json<PrinterTray>, PrintDeviceHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .create_printer_tray(
            &ctx,
            printer_id,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn update_printer_tray_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path((printer_id, tray_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(request): Json<UpdatePrinterTrayRequest>,
) -> Result<Json<PrinterTray>, PrintDeviceHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .update_printer_tray(
            &ctx,
            printer_id,
            tray_id,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

async fn test_print_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path(printer_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<TestPrintRequest>,
) -> Result<Json<PrinterTestPrint>, PrintDeviceHandlerError> {
    ctx.require_permission(WRITE_PERMISSION)?;
    let result = state
        .service
        .test_print(
            &ctx,
            printer_id,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

/// 设备租约列表分页查询参数（offset 分页）。
#[derive(Debug, Deserialize)]
struct LeaseListQuery {
    page: Option<u32>,
    page_size: Option<u32>,
}

impl LeaseListQuery {
    fn page(&self) -> u32 {
        self.page.filter(|p| *p >= 1).unwrap_or(1)
    }

    fn page_size(&self) -> u32 {
        self.page_size
            .filter(|s| *s >= 1)
            .map_or(20, |s| s.min(200))
    }
}

async fn list_leases_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Query(query): Query<LeaseListQuery>,
) -> Result<Json<DeviceLeaseListResponse>, PrintDeviceHandlerError> {
    ctx.require_permission(READ_PERMISSION)?;
    let (data, total) = state
        .service
        .list_leases(&ctx, query.page(), query.page_size())
        .await?;
    Ok(Json(DeviceLeaseListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
            total: Some(total.clamp(0, u32::MAX as i64) as u32),
        },
        data,
    }))
}

async fn release_lease_handler(
    ctx: AuthContext,
    State(state): State<PrintDeviceAppState>,
    Path(lease_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<ReleaseDeviceLeaseRequest>,
) -> Result<Json<DeviceLease>, PrintDeviceHandlerError> {
    ctx.require_permission(DEVICE_LEASE_RELEASE_PERMISSION)?;
    let result = state
        .service
        .release_lease(
            &ctx,
            lease_id,
            request,
            Utc::now(),
            idempotency_key_from_headers(&headers)?,
        )
        .await?;
    Ok(Json(result.value))
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<&str, PrintDeviceHandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(PrintDeviceHandlerError::MissingIdempotencyKey)
}
