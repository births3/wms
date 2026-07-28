#[allow(unused_imports)]
use super::{
    CreatePrintSiteRequest, CreatePrinterRequest, CreatePrinterTrayRequest,
    CreateSiteOwnerMappingRequest, DeviceLease, DeviceLeaseListResponse, ErrorResponse, PrintSite,
    PrintSiteListResponse, PrintSiteOwnerMapping, PrintSiteOwnerMappingListResponse, Printer,
    PrinterListResponse, PrinterTestPrint, PrinterTray, PrinterTrayListResponse,
    ReleaseDeviceLeaseRequest, TestPrintRequest, UpdatePrinterRequest, UpdatePrinterTrayRequest,
};

#[utoipa::path(
    get,
    path = "/api/v1/print-devices/sites",
    responses(
        (status = 200, description = "物理打印站点列表", body = PrintSiteListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn list_print_sites() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-devices/sites",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreatePrintSiteRequest,
    responses(
        (status = 200, description = "站点创建成功或幂等重放", body = PrintSite),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备维护权限", body = ErrorResponse),
        (status = 409, description = "站点编码已存在", body = ErrorResponse),
        (status = 422, description = "站点参数非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn create_print_site() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-devices/sites/{site_id}/owner-mappings",
    params(("site_id" = uuid::Uuid, Path, description = "物理打印站点 ID")),
    responses(
        (status = 200, description = "站点货主仓映射列表（含软删行）", body = PrintSiteOwnerMappingListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备读取权限", body = ErrorResponse),
        (status = 404, description = "站点不存在", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn list_print_site_owner_mappings() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-devices/sites/{site_id}/owner-mappings",
    params(
        ("site_id" = uuid::Uuid, Path, description = "物理打印站点 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = CreateSiteOwnerMappingRequest,
    responses(
        (status = 200, description = "映射创建成功或幂等重放", body = PrintSiteOwnerMapping),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备维护权限", body = ErrorResponse),
        (status = 404, description = "站点不存在", body = ErrorResponse),
        (status = 409, description = "同货主+仓库映射已存在", body = ErrorResponse),
        (status = 422, description = "映射参数非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn create_print_site_owner_mapping() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-devices/sites/{site_id}/owner-mappings/{mapping_id}/disable",
    params(
        ("site_id" = uuid::Uuid, Path, description = "物理打印站点 ID"),
        ("mapping_id" = uuid::Uuid, Path, description = "映射 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, description = "映射软删成功或幂等重放", body = PrintSiteOwnerMapping),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备维护权限", body = ErrorResponse),
        (status = 404, description = "映射不存在", body = ErrorResponse),
        (status = 409, description = "映射已停用", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn disable_print_site_owner_mapping() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-devices/printers",
    responses(
        (status = 200, description = "打印机列表（含生效释放模式）", body = PrinterListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn list_printers() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-devices/printers",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreatePrinterRequest,
    responses(
        (status = 200, description = "打印机创建成功或幂等重放；打印机归属唯一站点", body = Printer),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备维护权限", body = ErrorResponse),
        (status = 404, description = "站点不存在", body = ErrorResponse),
        (status = 409, description = "同站点打印机名称已存在", body = ErrorResponse),
        (status = 422, description = "打印机参数非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn create_printer() {}

#[utoipa::path(
    patch,
    path = "/api/v1/print-devices/printers/{printer_id}",
    params(
        ("printer_id" = uuid::Uuid, Path, description = "打印机 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = UpdatePrinterRequest,
    responses(
        (status = 200, description = "打印机状态或释放模式覆盖更新成功；运行中的租约保持快照", body = Printer),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备维护权限", body = ErrorResponse),
        (status = 404, description = "打印机不存在", body = ErrorResponse),
        (status = 422, description = "参数非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn update_printer() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-devices/printers/{printer_id}/trays",
    params(("printer_id" = uuid::Uuid, Path, description = "打印机 ID")),
    responses(
        (status = 200, description = "纸盒列表（纸张能力/启用状态/设备标识）", body = PrinterTrayListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备读取权限", body = ErrorResponse),
        (status = 404, description = "打印机不存在", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn list_printer_trays() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-devices/printers/{printer_id}/trays",
    params(
        ("printer_id" = uuid::Uuid, Path, description = "打印机 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = CreatePrinterTrayRequest,
    responses(
        (status = 200, description = "纸盒创建成功或幂等重放", body = PrinterTray),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备维护权限", body = ErrorResponse),
        (status = 404, description = "打印机不存在", body = ErrorResponse),
        (status = 409, description = "同打印机纸盒设备标识已存在", body = ErrorResponse),
        (status = 422, description = "纸盒参数非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn create_printer_tray() {}

#[utoipa::path(
    patch,
    path = "/api/v1/print-devices/printers/{printer_id}/trays/{tray_id}",
    params(
        ("printer_id" = uuid::Uuid, Path, description = "打印机 ID"),
        ("tray_id" = uuid::Uuid, Path, description = "纸盒 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = UpdatePrinterTrayRequest,
    responses(
        (status = 200, description = "纸盒能力或启用状态更新成功", body = PrinterTray),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备维护权限", body = ErrorResponse),
        (status = 404, description = "纸盒不存在或不属于该打印机", body = ErrorResponse),
        (status = 422, description = "参数非法", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn update_printer_tray() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-devices/printers/{printer_id}/test-print",
    params(
        ("printer_id" = uuid::Uuid, Path, description = "打印机 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = TestPrintRequest,
    responses(
        (status = 200, description = "测试指令受控下发并落表；真实硬件回执由 Print Agent/S4 验收登记", body = PrinterTestPrint),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备维护权限", body = ErrorResponse),
        (status = 404, description = "打印机或纸盒不存在", body = ErrorResponse),
        (status = 409, description = "打印机或纸盒已停用", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn test_print_printer() {}

#[utoipa::path(
    get,
    path = "/api/v1/print-devices/leases",
    responses(
        (status = 200, description = "设备租约列表（含释放模式快照与安全状态）", body = DeviceLeaseListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无打印设备读取权限", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn list_device_leases() {}

#[utoipa::path(
    post,
    path = "/api/v1/print-devices/leases/{lease_id}/release",
    params(
        ("lease_id" = uuid::Uuid, Path, description = "设备租约 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = ReleaseDeviceLeaseRequest,
    responses(
        (status = 200, description = "人工释放成功或幂等重放；需专用权限+原因+二次确认", body = DeviceLease),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "缺少 h9.device_lease.release 专用权限", body = ErrorResponse),
        (status = 404, description = "租约不存在", body = ErrorResponse),
        (status = 409, description = "租约已释放，或处于 printing/result_unknown/reconciling 硬安全状态", body = ErrorResponse),
        (status = 422, description = "缺少原因或二次确认", body = ErrorResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "print-devices"
)]
#[allow(dead_code)]
pub(crate) fn release_device_lease() {}
