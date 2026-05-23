//! api — utoipa 注解 + OpenAPI 文档定义
//!
//! 不引入 Axum 等 HTTP 框架（spike 阶段仅验证契约生成链路）；
//! 实际 handler 在 Wave 1 W1.A 落地。

use utoipa::OpenApi;
use wms_domain::{
    Audit, ColdChainPoint, ErrorResponse, InventoryStatus, Item, PaginatedItems,
};

/// GET /api/v1/items/{id} — 单条商品查询
#[utoipa::path(
    get,
    path = "/api/v1/items/{id}",
    tag = "items",
    params(
        ("id" = uuid::Uuid, Path, description = "商品 UUID"),
    ),
    responses(
        (status = 200, description = "成功", body = Item),
        (status = 404, description = "未找到", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn get_item() {}

/// GET /api/v1/items — 分页列表
#[utoipa::path(
    get,
    path = "/api/v1/items",
    tag = "items",
    params(
        ("page" = Option<u32>, Query, description = "页码（默认 1）"),
        ("page_size" = Option<u32>, Query, description = "每页大小（默认 20，最大 100）"),
    ),
    responses(
        (status = 200, description = "成功", body = PaginatedItems),
    ),
)]
#[allow(dead_code)]
fn list_items() {}

/// GET /api/v1/audit/events/{id} — 审计事件详情
#[utoipa::path(
    get,
    path = "/api/v1/audit/events/{id}",
    tag = "audit",
    params(
        ("id" = uuid::Uuid, Path, description = "审计事件 UUID"),
    ),
    responses(
        (status = 200, description = "成功", body = Audit),
        (status = 404, description = "未找到", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn get_audit_event() {}

/// GET /api/v1/cold-chain/points — 冷链温度时序
#[utoipa::path(
    get,
    path = "/api/v1/cold-chain/points",
    tag = "cold-chain",
    params(
        ("from" = String, Query, description = "起始时间 RFC3339"),
        ("to" = String, Query, description = "结束时间 RFC3339"),
    ),
    responses(
        (status = 200, description = "成功", body = Vec<ColdChainPoint>),
    ),
)]
#[allow(dead_code)]
fn list_cold_chain_points() {}

/// POST /api/v1/items/{id}/isolate — 隔离（演示 tagged union 写入）
#[utoipa::path(
    post,
    path = "/api/v1/items/{id}/isolate",
    tag = "items",
    params(
        ("id" = uuid::Uuid, Path, description = "商品 UUID"),
    ),
    request_body = InventoryStatus,
    responses(
        (status = 200, description = "成功", body = Item),
        (status = 400, description = "参数错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
fn isolate_item() {}

/// API 文档根（导出 openapi.json 时用此 struct）
#[derive(OpenApi)]
#[openapi(
    info(
        title = "WMS API",
        version = "0.0.1-spike-003",
        description = "SPIKE-003 utoipa → OpenAPI → openapi-typescript 全链路验证",
    ),
    paths(
        get_item,
        list_items,
        get_audit_event,
        list_cold_chain_points,
        isolate_item,
    ),
    components(schemas(
        Item,
        InventoryStatus,
        Audit,
        PaginatedItems,
        ColdChainPoint,
        ErrorResponse,
    )),
    tags(
        (name = "items", description = "商品档案与库存"),
        (name = "audit", description = "审计追踪"),
        (name = "cold-chain", description = "冷链温度"),
    ),
)]
pub struct ApiDoc;
