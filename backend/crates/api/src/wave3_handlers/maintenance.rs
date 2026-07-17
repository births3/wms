use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use wms_domain::{
    CreateMaintenanceRecordRequest, MaintenanceRecordListResponse, MaintenanceRecordQuery,
    MaintenanceTaskListResponse, MaintenanceTaskQuery, PageMeta,
};

use super::{
    require_any_permission, AuditWriteRequest, AuthContext, PgWave3Repository, Wave3AppState,
    Wave3HandlerError, Wave3RepositoryError,
};

pub(super) fn apply_maintenance_routes(router: Router<Wave3AppState>) -> Router<Wave3AppState> {
    router
        .route(
            "/api/v1/inventory/maintenance/tasks",
            get(list_maintenance_tasks_handler),
        )
        .route(
            "/api/v1/inventory/maintenance/tasks/generate",
            post(generate_maintenance_tasks_handler),
        )
        .route(
            "/api/v1/inventory/maintenance/records",
            get(list_maintenance_records_handler),
        )
        .route(
            "/api/v1/inventory/maintenance/records",
            post(create_maintenance_record_handler),
        )
}

async fn list_maintenance_tasks_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Query(query): Query<MaintenanceTaskQuery>,
) -> Result<Json<MaintenanceTaskListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.maintenance.write"])?;
    let repository = maintenance_repository(&state)?;
    let data = repository.list_maintenance_tasks(&ctx, query).await?;
    Ok(Json(MaintenanceTaskListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
        },
        data,
    }))
}

async fn generate_maintenance_tasks_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
) -> Result<Json<serde_json::Value>, Wave3HandlerError> {
    ctx.require_permission("m3.maintenance.write")?;
    let created = maintenance_repository(&state)?
        .generate_maintenance_tasks(&ctx, Utc::now(), 180)
        .await?;
    Ok(Json(serde_json::json!({ "created": created })))
}

async fn list_maintenance_records_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Query(query): Query<MaintenanceRecordQuery>,
) -> Result<Json<MaintenanceRecordListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.maintenance.write"])?;
    let repository = maintenance_repository(&state)?;
    let data = repository.list_maintenance_records(&ctx, query).await?;
    Ok(Json(MaintenanceRecordListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
        },
        data,
    }))
}

async fn create_maintenance_record_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateMaintenanceRecordRequest>,
) -> Result<Json<wms_domain::MaintenanceRecord>, Wave3HandlerError> {
    ctx.require_permission("m3.maintenance.write")?;
    let idempotency_key = super::idempotency_key_from_headers(&headers)?;
    let repository = maintenance_repository(&state)?;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create_maintenance_record",
        "M3",
        "inventory_maintenance_record",
        req.task_id.to_string(),
        None,
    );
    let result = repository
        .create_maintenance_record_with_audit(&ctx, req, Utc::now(), &idempotency_key, Some(audit))
        .await?;
    Ok(Json(result.value))
}

fn maintenance_repository(state: &Wave3AppState) -> Result<&PgWave3Repository, Wave3HandlerError> {
    state.wave3_repository.as_deref().ok_or_else(|| {
        Wave3RepositoryError::Database("养护接口需要 PostgreSQL repository".to_string()).into()
    })
}
