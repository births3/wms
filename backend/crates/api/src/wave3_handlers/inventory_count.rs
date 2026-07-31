use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use uuid::Uuid;
use wms_domain::{
    ApproveInventoryCountRequest, CreateInventoryCountRequest, InventoryCount, InventoryCountLine,
    InventoryCountListResponse, SubmitInventoryCountLineRequest,
};

use super::{
    require_any_permission, AuditWriteRequest, AuthContext, PgWave3Repository, Wave3AppState,
    Wave3HandlerError, Wave3RepositoryError,
};

pub(super) fn apply_inventory_count_routes(router: Router<Wave3AppState>) -> Router<Wave3AppState> {
    router
        .route(
            "/api/v1/inventory/counts",
            get(list_inventory_counts_handler).post(create_inventory_count_handler),
        )
        .route(
            "/api/v1/inventory/counts/:id",
            get(get_inventory_count_handler),
        )
        .route(
            "/api/v1/inventory/counts/:id/lines/:line_id/submit",
            post(submit_inventory_count_line_handler),
        )
        .route(
            "/api/v1/inventory/counts/:id/approve",
            post(approve_inventory_count_handler),
        )
}

async fn list_inventory_counts_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
) -> Result<Json<InventoryCountListResponse>, Wave3HandlerError> {
    require_any_permission(
        &ctx,
        &[
            "m3.read",
            "m3.inventory_count.write",
            "m3.inventory_count.approve",
        ],
    )?;
    let data = inventory_count_repository(&state)?
        .list_inventory_counts(&ctx)
        .await?;
    Ok(Json(InventoryCountListResponse {
        page: wms_domain::PageMeta {
            count: data.len() as u32,
            next_cursor: None,
        },
        data,
    }))
}

async fn create_inventory_count_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateInventoryCountRequest>,
) -> Result<Json<InventoryCount>, Wave3HandlerError> {
    ctx.require_permission("m3.inventory_count.write")?;
    let idempotency_key = super::idempotency_key_from_headers(&headers)?;
    let repository = inventory_count_repository(&state)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_inventory_count",
        "M3",
        "inventory_count",
        "",
        None,
    );
    let result = repository
        .create_inventory_count_with_audit(&ctx, req, Utc::now(), &idempotency_key, Some(audit))
        .await?;
    Ok(Json(result.value))
}

async fn get_inventory_count_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(count_id): Path<Uuid>,
) -> Result<Json<InventoryCount>, Wave3HandlerError> {
    require_any_permission(
        &ctx,
        &[
            "m3.read",
            "m3.inventory_count.write",
            "m3.inventory_count.approve",
        ],
    )?;
    let repository = inventory_count_repository(&state)?;
    Ok(Json(repository.get_inventory_count(&ctx, count_id).await?))
}

async fn submit_inventory_count_line_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path((count_id, line_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(req): Json<SubmitInventoryCountLineRequest>,
) -> Result<Json<InventoryCountLine>, Wave3HandlerError> {
    ctx.require_permission("m3.inventory_count.write")?;
    let idempotency_key = super::idempotency_key_from_headers(&headers)?;
    let repository = inventory_count_repository(&state)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "submit_inventory_count_line",
        "M3",
        "inventory_count_line",
        line_id.to_string(),
        None,
    );
    let result = repository
        .submit_inventory_count_line_with_audit(
            &ctx,
            count_id,
            line_id,
            req,
            Utc::now(),
            &idempotency_key,
            Some(audit),
        )
        .await?;
    Ok(Json(result.value))
}

async fn approve_inventory_count_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(count_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ApproveInventoryCountRequest>,
) -> Result<Json<InventoryCount>, Wave3HandlerError> {
    ctx.require_permission("m3.inventory_count.approve")?;
    let idempotency_key = super::idempotency_key_from_headers(&headers)?;
    let repository = inventory_count_repository(&state)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "approve_inventory_count",
        "M3",
        "inventory_count",
        count_id.to_string(),
        None,
    );
    let result = repository
        .approve_inventory_count_with_audit(
            &ctx,
            count_id,
            req,
            Utc::now(),
            &idempotency_key,
            Some(audit),
        )
        .await?;
    Ok(Json(result.value))
}

fn inventory_count_repository(
    state: &Wave3AppState,
) -> Result<&PgWave3Repository, Wave3HandlerError> {
    state.wave3_repository.as_deref().ok_or_else(|| {
        Wave3RepositoryError::Database("库存盘点接口需要 PostgreSQL repository".to_string()).into()
    })
}
