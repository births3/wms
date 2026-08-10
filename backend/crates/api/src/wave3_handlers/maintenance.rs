use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;
use wms_domain::{
    CreateMaintenanceRecordRequest, MaintenanceRecordListResponse, MaintenanceRecordQuery,
    MaintenanceTaskListResponse, MaintenanceTaskQuery, PageMeta,
};

/// 养护任务列表查询：过滤条件 + offset 分页（page 从 1 起，默认 1；page_size 默认 20，上限 200）。
#[derive(Debug, Deserialize)]
pub(super) struct MaintenanceTaskListQuery {
    task_id: Option<Uuid>,
    batch_id: Option<Uuid>,
    status: Option<String>,
    page: Option<u32>,
    page_size: Option<u32>,
}

/// 养护记录列表查询：过滤条件 + offset 分页（page 从 1 起，默认 1；page_size 默认 20，上限 200）。
#[derive(Debug, Deserialize)]
pub(super) struct MaintenanceRecordListQuery {
    task_id: Option<Uuid>,
    batch_id: Option<Uuid>,
    page: Option<u32>,
    page_size: Option<u32>,
}

fn list_page(page: Option<u32>) -> u32 {
    page.filter(|value| *value >= 1).unwrap_or(1)
}

fn list_page_size(page_size: Option<u32>) -> u32 {
    page_size
        .filter(|value| *value >= 1)
        .map_or(20, |value| value.min(200))
}

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
    Query(query): Query<MaintenanceTaskListQuery>,
) -> Result<Json<MaintenanceTaskListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.maintenance.write"])?;
    let repository = maintenance_repository(&state)?;
    let (data, total) = repository
        .list_maintenance_tasks(
            &ctx,
            MaintenanceTaskQuery {
                task_id: query.task_id,
                batch_id: query.batch_id,
                status: query.status,
            },
            list_page(query.page),
            list_page_size(query.page_size),
        )
        .await?;
    Ok(Json(MaintenanceTaskListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
            total: Some(total.clamp(0, u32::MAX as i64) as u32),
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
        .generate_maintenance_tasks(&ctx, Utc::now(), None)
        .await?;
    Ok(Json(serde_json::json!({ "created": created })))
}

async fn list_maintenance_records_handler(
    ctx: AuthContext,
    State(state): State<Wave3AppState>,
    Query(query): Query<MaintenanceRecordListQuery>,
) -> Result<Json<MaintenanceRecordListResponse>, Wave3HandlerError> {
    require_any_permission(&ctx, &["m3.read", "m3.maintenance.write"])?;
    let repository = maintenance_repository(&state)?;
    let (data, total) = repository
        .list_maintenance_records(
            &ctx,
            MaintenanceRecordQuery {
                task_id: query.task_id,
                batch_id: query.batch_id,
            },
            list_page(query.page),
            list_page_size(query.page_size),
        )
        .await?;
    Ok(Json(MaintenanceRecordListResponse {
        page: PageMeta {
            count: data.len() as u32,
            next_cursor: None,
            total: Some(total.clamp(0, u32::MAX as i64) as u32),
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
