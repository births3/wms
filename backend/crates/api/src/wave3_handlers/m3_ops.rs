use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use uuid::Uuid;
use wms_domain::{
    HandleInventoryAlertRequest, InventoryAbcClassification, InventoryAbcListResponse,
    InventoryAbcQuery, InventoryAlertEvent, InventoryAlertListResponse, InventoryAlertQuery,
    InventoryRecallImpact, InventoryRelocation, OverrideInventoryAbcRequest,
    RecomputeInventoryAbcRequest, RelocateInventoryRequest,
};

use super::{
    require_any_permission, AuditWriteRequest, AuthContext, PgWave3Repository, Wave3AppState,
    Wave3HandlerError, Wave3RepositoryError,
};

pub(super) fn apply_m3_ops_routes(router: Router<Wave3AppState>) -> Router<Wave3AppState> {
    router
        .route(
            "/api/v1/inventory/relocations",
            post(relocate_inventory_handler),
        )
        .route(
            "/api/v1/inventory/alerts",
            get(list_inventory_alerts_handler),
        )
        .route(
            "/api/v1/inventory/alerts/:id/handle",
            post(handle_inventory_alert_handler),
        )
        .route(
            "/api/v1/inventory/alerts/generate-near-expiry",
            post(generate_near_expiry_alerts_handler),
        )
        .route(
            "/api/v1/inventory/abc",
            get(list_abc_handler).post(recompute_abc_handler),
        )
        .route("/api/v1/inventory/abc/override", post(override_abc_handler))
        .route(
            "/api/v1/inventory/batches/:id/shipped-customers",
            get(list_shipped_customers_handler),
        )
        .route(
            "/api/v1/inventory/status-erp-outbox/process",
            post(process_status_erp_outbox_handler),
        )
}

async fn relocate_inventory_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<RelocateInventoryRequest>,
) -> Result<Json<InventoryRelocation>, Wave3HandlerError> {
    ctx.require_permission("m3.relocation.write")?;
    let idempotency_key = super::idempotency_key_from_headers(&headers)?;
    let repository = ops_repository(&state)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "relocate_inventory",
        "M3",
        "inventory_relocation",
        req.batch_id.to_string(),
        None,
    );
    let result = repository
        .relocate_inventory_with_audit(&ctx, req, Utc::now(), &idempotency_key, Some(audit))
        .await?;
    Ok(Json(result.value))
}

async fn list_inventory_alerts_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Query(query): Query<InventoryAlertQuery>,
) -> Result<Json<InventoryAlertListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.alert.read", "m3.read"])?;
    Ok(Json(
        ops_repository(&state)?
            .list_inventory_alerts(&ctx, &query)
            .await?,
    ))
}

async fn handle_inventory_alert_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<HandleInventoryAlertRequest>,
) -> Result<Json<InventoryAlertEvent>, Wave3HandlerError> {
    ctx.require_permission("m3.alert.write")?;
    Ok(Json(
        ops_repository(&state)?
            .handle_inventory_alert(&ctx, id, req, Utc::now())
            .await?,
    ))
}

async fn generate_near_expiry_alerts_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
) -> Result<Json<serde_json::Value>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.alert.write", "m3.write"])?;
    let created = ops_repository(&state)?
        .generate_near_expiry_alerts(&ctx, Utc::now(), 180)
        .await?;
    Ok(Json(serde_json::json!({ "created": created })))
}

async fn list_abc_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Query(query): Query<InventoryAbcQuery>,
) -> Result<Json<InventoryAbcListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.abc.read", "m3.read"])?;
    Ok(Json(
        ops_repository(&state)?
            .list_abc_classifications(&ctx, &query)
            .await?,
    ))
}

async fn recompute_abc_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<RecomputeInventoryAbcRequest>,
) -> Result<Json<InventoryAbcListResponse>, Wave3HandlerError> {
    ctx.require_permission("m3.abc.write")?;
    Ok(Json(
        ops_repository(&state)?
            .recompute_abc_classifications(&ctx, req, Utc::now())
            .await?,
    ))
}

async fn override_abc_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Json(req): Json<OverrideInventoryAbcRequest>,
) -> Result<Json<InventoryAbcClassification>, Wave3HandlerError> {
    ctx.require_permission("m3.abc.write")?;
    Ok(Json(
        ops_repository(&state)?
            .override_abc_classification(&ctx, req, Utc::now())
            .await?,
    ))
}

async fn list_shipped_customers_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<InventoryRecallImpact>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.write"])?;
    let repository = ops_repository(&state)?;
    let batches = repository.list_inventory_batches(&ctx).await?;
    let batch = batches
        .into_iter()
        .find(|item| item.id == id)
        .ok_or(Wave3RepositoryError::NotFound)?;
    let shipped_customers = repository
        .list_shipped_customers_for_batch(&ctx, id)
        .await?;
    Ok(Json(InventoryRecallImpact {
        batch_id: batch.id,
        batch_no: batch.batch_no,
        product_code: batch.product_code,
        shipped_customers,
    }))
}

async fn process_status_erp_outbox_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
) -> Result<Json<serde_json::Value>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.write"])?;
    let processed = ops_repository(&state)?
        .process_status_erp_feedback_outbox(Utc::now(), 50)
        .await?;
    Ok(Json(serde_json::json!({ "processed": processed })))
}

fn ops_repository(state: &Wave3AppState) -> Result<&PgWave3Repository, Wave3HandlerError> {
    state.wave3_repository.as_deref().ok_or_else(|| {
        Wave3HandlerError::Repository(Wave3RepositoryError::Database(
            "需要 PostgreSQL repository".to_string(),
        ))
    })
}
