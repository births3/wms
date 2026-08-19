#[allow(unused_imports)]
use wms_domain::{
    ChangeDrugInspectionPlatformStatusRequest, DrugInspectionPlatform,
    DrugInspectionPlatformListResponse, ErrorResponse, UpsertDrugInspectionPlatformRequest,
};

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/platforms",
    tag = "drug-inspection",
    params(("status" = Option<String>, Query, description = "connected / testing / disabled")),
    responses(
        (status = 200, description = "药检平台配置列表", body = DrugInspectionPlatformListResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 422, description = "状态筛选非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_drug_inspection_platforms() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/platforms",
    tag = "drug-inspection",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = UpsertDrugInspectionPlatformRequest,
    responses(
        (status = 200, description = "新增或更新药检平台配置", body = DrugInspectionPlatform),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "平台配置非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn upsert_drug_inspection_platform() {}

#[utoipa::path(
    patch,
    path = "/api/v1/drug-inspection/platforms/{platform_id}/status",
    tag = "drug-inspection",
    params(
        ("platform_id" = uuid::Uuid, Path, description = "药检平台配置 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键"),
    ),
    request_body = ChangeDrugInspectionPlatformStatusRequest,
    responses(
        (status = 200, description = "变更药检平台状态", body = DrugInspectionPlatform),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 404, description = "平台不存在", body = ErrorResponse),
        (status = 409, description = "幂等冲突", body = ErrorResponse),
        (status = 422, description = "平台状态非法", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn change_drug_inspection_platform_status() {}
