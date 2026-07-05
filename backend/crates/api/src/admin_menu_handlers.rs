//! HTTP handlers for US-H1-007 admin menu management.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    AdminMenuNode, AdminMenuTreeResponse, AdminMenuVersion, BatchEnableAdminMenuRequest,
    CreateAdminMenuNodeRequest, ErrorResponse, PageMeta, PublishAdminMenuRequest,
    RollbackAdminMenuRequest, UpdateAdminMenuNodeRequest,
};

use crate::{
    admin_menu::{
        AdminMenuError, PgAdminMenuService, ADMIN_MENU_PUBLISH_PERMISSION,
        ADMIN_MENU_WRITE_PERMISSION,
    },
    auth::{AuthContext, AuthError},
};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

#[derive(Clone, Debug)]
pub struct AdminMenuAppState {
    pool: PgPool,
    service: PgAdminMenuService,
}

#[derive(Debug)]
enum AdminMenuHandlerError {
    Auth(AuthError),
    AdminMenu(AdminMenuError),
    MissingIdempotencyKey,
}

impl AdminMenuAppState {
    pub fn with_postgres(pool: PgPool) -> Self {
        Self {
            pool,
            service: PgAdminMenuService::new(),
        }
    }
}

impl From<AuthError> for AdminMenuHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<AdminMenuError> for AdminMenuHandlerError {
    fn from(value: AdminMenuError) -> Self {
        Self::AdminMenu(value)
    }
}

impl IntoResponse for AdminMenuHandlerError {
    fn into_response(self) -> Response {
        if let AdminMenuHandlerError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message) = match self {
            AdminMenuHandlerError::MissingIdempotencyKey => (
                StatusCode::BAD_REQUEST,
                "H1_MENU_IDEMPOTENCY_REQUIRED",
                "缺少 Idempotency-Key",
            ),
            AdminMenuHandlerError::AdminMenu(AdminMenuError::NodeNotFound) => (
                StatusCode::NOT_FOUND,
                "H1_MENU_NODE_NOT_FOUND",
                "菜单节点不存在",
            ),
            AdminMenuHandlerError::AdminMenu(AdminMenuError::VersionNotFound) => (
                StatusCode::NOT_FOUND,
                "H1_MENU_VERSION_NOT_FOUND",
                "菜单版本不存在",
            ),
            AdminMenuHandlerError::AdminMenu(AdminMenuError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "H1_MENU_IDEMPOTENCY_CONFLICT",
                "幂等键已被不同请求使用",
            ),
            AdminMenuHandlerError::AdminMenu(AdminMenuError::InvalidTree) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_MENU_INVALID_TREE",
                "菜单层级或父子关系非法",
            ),
            AdminMenuHandlerError::AdminMenu(AdminMenuError::UnknownView) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_MENU_UNKNOWN_VIEW",
                "菜单绑定了未注册页面",
            ),
            AdminMenuHandlerError::AdminMenu(AdminMenuError::InvalidIcon) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_MENU_INVALID_ICON",
                "菜单图标不在白名单",
            ),
            AdminMenuHandlerError::AdminMenu(AdminMenuError::InvalidPermission) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H1_MENU_INVALID_PERMISSION",
                "菜单权限点非法",
            ),
            AdminMenuHandlerError::AdminMenu(
                AdminMenuError::Audit(_)
                | AdminMenuError::Database(_)
                | AdminMenuError::Serialize(_),
            ) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H1_MENU_INTERNAL",
                "菜单管理处理失败",
            ),
            AdminMenuHandlerError::Auth(_) => unreachable!("auth error returned above"),
        };

        (
            status,
            Json(ErrorResponse {
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

pub fn admin_menu_router(state: AdminMenuAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/admin/menus/published",
            get(list_published_menu_handler),
        )
        .route("/api/v1/admin/menus/draft", get(list_draft_menu_handler))
        .route(
            "/api/v1/admin/menus/draft/nodes",
            post(create_menu_node_handler),
        )
        .route(
            "/api/v1/admin/menus/draft/nodes/:id",
            patch(update_menu_node_handler),
        )
        .route(
            "/api/v1/admin/menus/draft/batch-enable",
            post(batch_enable_menu_nodes_handler),
        )
        .route("/api/v1/admin/menus/publish", post(publish_menu_handler))
        .route("/api/v1/admin/menus/rollback", post(rollback_menu_handler))
        .with_state(state)
}

async fn list_published_menu_handler(
    ctx: AuthContext,
    State(state): State<AdminMenuAppState>,
) -> Result<Json<AdminMenuTreeResponse>, AdminMenuHandlerError> {
    let (tree, version_no) = state.service.list_published_tree(&state.pool, &ctx).await?;
    Ok(Json(menu_tree_response(tree, version_no)))
}

async fn list_draft_menu_handler(
    ctx: AuthContext,
    State(state): State<AdminMenuAppState>,
) -> Result<Json<AdminMenuTreeResponse>, AdminMenuHandlerError> {
    require_any_permission(
        &ctx,
        &[ADMIN_MENU_WRITE_PERMISSION, ADMIN_MENU_PUBLISH_PERMISSION],
    )?;
    let tree = state.service.list_draft_tree(&state.pool).await?;
    Ok(Json(menu_tree_response(tree, None)))
}

async fn create_menu_node_handler(
    ctx: AuthContext,
    State(state): State<AdminMenuAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateAdminMenuNodeRequest>,
) -> Result<Json<AdminMenuNode>, AdminMenuHandlerError> {
    ctx.require_permission(ADMIN_MENU_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .create_node(&state.pool, &ctx, req, chrono::Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn update_menu_node_handler(
    ctx: AuthContext,
    State(state): State<AdminMenuAppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateAdminMenuNodeRequest>,
) -> Result<Json<AdminMenuNode>, AdminMenuHandlerError> {
    ctx.require_permission(ADMIN_MENU_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .update_node(
            &state.pool,
            &ctx,
            id,
            req,
            chrono::Utc::now(),
            &idempotency_key,
        )
        .await?;
    Ok(Json(result.value))
}

async fn batch_enable_menu_nodes_handler(
    ctx: AuthContext,
    State(state): State<AdminMenuAppState>,
    headers: HeaderMap,
    Json(req): Json<BatchEnableAdminMenuRequest>,
) -> Result<Json<Vec<AdminMenuNode>>, AdminMenuHandlerError> {
    ctx.require_permission(ADMIN_MENU_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .batch_enable(&state.pool, &ctx, req, chrono::Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn publish_menu_handler(
    ctx: AuthContext,
    State(state): State<AdminMenuAppState>,
    headers: HeaderMap,
    Json(req): Json<PublishAdminMenuRequest>,
) -> Result<Json<AdminMenuVersion>, AdminMenuHandlerError> {
    ctx.require_permission(ADMIN_MENU_PUBLISH_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .publish(&state.pool, &ctx, req, chrono::Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

async fn rollback_menu_handler(
    ctx: AuthContext,
    State(state): State<AdminMenuAppState>,
    headers: HeaderMap,
    Json(req): Json<RollbackAdminMenuRequest>,
) -> Result<Json<AdminMenuVersion>, AdminMenuHandlerError> {
    ctx.require_permission(ADMIN_MENU_PUBLISH_PERMISSION)?;
    let idempotency_key = idempotency_key_from_headers(&headers)?;
    let result = state
        .service
        .rollback(&state.pool, &ctx, req, chrono::Utc::now(), &idempotency_key)
        .await?;
    Ok(Json(result.value))
}

fn menu_tree_response(tree: Vec<AdminMenuNode>, version_no: Option<i64>) -> AdminMenuTreeResponse {
    let count = tree.len() as u32;
    AdminMenuTreeResponse {
        data: tree,
        version_no,
        page: PageMeta {
            next_cursor: None,
            count,
        },
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, AdminMenuHandlerError> {
    let Some(value) = headers.get(IDEMPOTENCY_KEY_HEADER) else {
        return Err(AdminMenuHandlerError::MissingIdempotencyKey);
    };
    let value = value
        .to_str()
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(AdminMenuHandlerError::MissingIdempotencyKey)?;
    Ok(value.to_string())
}

fn require_any_permission(ctx: &AuthContext, permissions: &[&str]) -> Result<(), AuthError> {
    if permissions
        .iter()
        .any(|permission| ctx.has_permission(permission))
    {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(permissions.join("|")))
    }
}
