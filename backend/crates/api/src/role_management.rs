//! US-H1-002 role and permission management vertical slice.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::PageMeta;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::{AuthContext, AuthRevocationStore},
    idempotency,
};

mod errors;
mod user_management;

pub use errors::RoleError;

pub use crate::role_management_models::{
    BatchAssignRolesRequest, BatchAssignRolesResponse, CreateRoleRequest, CreateUserRequest,
    DeleteRoleResponse, PermissionListResponse, PermissionResponse, ReplaceRolePermissionsRequest,
    RoleListResponse, RoleResponse, RoleUserListResponse, RoleUserResponse, UpdateRoleRequest,
};

pub const ROLE_MANAGE_PERMISSION: &str = "h1.roles.manage";

impl From<crate::idempotency::IdempotencyError> for RoleError {
    fn from(error: crate::idempotency::IdempotencyError) -> Self {
        match error {
            crate::idempotency::IdempotencyError::Conflict => Self::IdempotencyConflict,
            crate::idempotency::IdempotencyError::Database(_) => Self::Database,
            crate::idempotency::IdempotencyError::Serialize(_) => Self::Serialize,
        }
    }
}

#[derive(Clone)]
pub struct RoleManagementState {
    pool: PgPool,
    revocations: Arc<dyn AuthRevocationStore>,
}

impl RoleManagementState {
    pub fn new(pool: PgPool, revocations: Arc<dyn AuthRevocationStore>) -> Self {
        Self { pool, revocations }
    }
}

pub fn role_management_router(state: RoleManagementState) -> Router {
    Router::new()
        .route("/api/v1/auth/roles", get(list_roles).post(create_role))
        .route(
            "/api/v1/auth/roles/:role_id",
            put(update_role).delete(delete_role),
        )
        .route(
            "/api/v1/auth/roles/:role_id/permissions",
            put(replace_role_permissions),
        )
        .route("/api/v1/auth/user-roles/batch", put(batch_assign_roles))
        .route("/api/v1/auth/permissions", get(list_permissions))
        .route(
            "/api/v1/auth/users",
            get(list_role_users).post(user_management::create_user),
        )
        .with_state(state)
}

#[derive(FromRow)]
struct RoleRow {
    id: Uuid,
    role_code: String,
    role_name: String,
    data_scope: String,
    parent_role_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

async fn list_roles(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
) -> Result<Json<RoleListResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    let rows = sqlx::query_as::<_, RoleRow>(
        "SELECT id,role_code,role_name,data_scope,parent_role_id,created_at FROM auth_roles WHERE owner_id=$1 ORDER BY role_code",
    )
    .bind(ctx.owner_id)
    .fetch_all(&state.pool)
    .await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(role_response(&state.pool, row).await?);
    }
    Ok(Json(RoleListResponse { items }))
}

async fn list_permissions(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
) -> Result<Json<PermissionListResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    let items = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT id,permission_code,permission_name FROM auth_permissions ORDER BY permission_code",
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(
        |(id, permission_code, permission_name)| PermissionResponse {
            id,
            permission_code,
            permission_name,
        },
    )
    .collect();
    Ok(Json(PermissionListResponse { items }))
}

/// 角色用户列表分页查询参数（offset 分页）。
#[derive(Clone, Debug, serde::Deserialize)]
struct RoleUserListQuery {
    /// 页码，从 1 开始；缺省为 1。
    page: Option<u32>,
    /// 每页条数；缺省为 20，上限 200。
    page_size: Option<u32>,
}

impl RoleUserListQuery {
    fn page(&self) -> u32 {
        self.page.filter(|p| *p >= 1).unwrap_or(1)
    }

    fn page_size(&self) -> u32 {
        self.page_size
            .filter(|s| *s >= 1)
            .map_or(20, |s| s.min(200))
    }
}

async fn list_role_users(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
    Query(query): Query<RoleUserListQuery>,
) -> Result<Json<RoleUserListResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    let page = query.page().max(1);
    let page_size = query.page_size().clamp(1, 200);
    let offset = ((page - 1) as i64) * (page_size as i64);
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
          FROM (
               SELECT user_row.id
                 FROM auth_user_owner_bindings binding
                 JOIN auth_users user_row ON user_row.id = binding.user_id
                WHERE binding.owner_id = $1
                  AND binding.is_active
                GROUP BY user_row.id
          ) bound_users
        "#,
    )
    .bind(ctx.owner_id)
    .fetch_one(&state.pool)
    .await?;
    let items: Vec<RoleUserResponse> = sqlx::query_as::<_, (Uuid, String, String, Vec<Uuid>)>(
        r#"
        SELECT user_row.id, user_row.username, user_row.display_name,
               COALESCE(
                   array_agg(user_role.role_id) FILTER (WHERE user_role.role_id IS NOT NULL),
                   ARRAY[]::uuid[]
               ) AS role_ids
          FROM auth_user_owner_bindings binding
          JOIN auth_users user_row ON user_row.id = binding.user_id
          LEFT JOIN auth_user_roles user_role
            ON user_role.user_id = binding.user_id
           AND user_role.owner_id = binding.owner_id
         WHERE binding.owner_id = $1
           AND binding.is_active
         GROUP BY user_row.id, user_row.username, user_row.display_name
         ORDER BY user_row.username, user_row.id
         LIMIT $2 OFFSET $3
        "#,
    )
    .bind(ctx.owner_id)
    .bind(page_size as i64)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(
        |(user_id, username, display_name, role_ids)| RoleUserResponse {
            user_id,
            username,
            display_name,
            role_ids,
        },
    )
    .collect();
    Ok(Json(RoleUserListResponse {
        page: PageMeta {
            next_cursor: None,
            count: items.len() as u32,
            total: Some(total.clamp(0, u32::MAX as i64) as u32),
        },
        data: items,
    }))
}

async fn create_role(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
    headers: HeaderMap,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Json<RoleResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    validate_role(&req.role_code, &req.role_name, &req.data_scope)?;
    let key = idempotency_key(&headers)?;
    let mut tx = state.pool.begin().await?;
    lock_key(&mut tx, ctx.owner_id, &key).await?;
    let hash = request_hash(&req)?;
    if let Some(response) = replay(
        &mut tx,
        ctx.owner_id,
        &key,
        &hash,
        "POST",
        "/api/v1/auth/roles",
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(response));
    }
    ensure_parent(&mut tx, ctx.owner_id, req.parent_role_id, None).await?;
    let id = Uuid::new_v4();
    let row = sqlx::query_as::<_, RoleRow>(
        "INSERT INTO auth_roles(id,owner_id,role_code,role_name,data_scope,parent_role_id) VALUES($1,$2,$3,$4,$5,$6) RETURNING id,role_code,role_name,data_scope,parent_role_id,created_at",
    )
    .bind(id).bind(ctx.owner_id).bind(req.role_code.trim()).bind(req.role_name.trim())
    .bind(req.data_scope.trim()).bind(req.parent_role_id)
    .fetch_one(&mut *tx).await.map_err(map_write_error)?;
    let response = role_response_tx(&mut tx, row).await?;
    finish(
        &mut tx,
        &ctx,
        &key,
        &hash,
        "POST",
        "/api/v1/auth/roles",
        id,
        &response,
        "auth.role.create",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(response))
}

async fn update_role(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
    Path(role_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    validate_role("unchanged", &req.role_name, &req.data_scope)?;
    let key = idempotency_key(&headers)?;
    let hash = request_hash(&req)?;
    let mut tx = state.pool.begin().await?;
    lock_key(&mut tx, ctx.owner_id, &key).await?;
    if let Some(response) = replay(
        &mut tx,
        ctx.owner_id,
        &key,
        &hash,
        "PUT",
        &format!("/api/v1/auth/roles/{role_id}"),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(response));
    }
    ensure_role_owner(&mut tx, ctx.owner_id, role_id).await?;
    ensure_parent(&mut tx, ctx.owner_id, req.parent_role_id, Some(role_id)).await?;
    let row=sqlx::query_as::<_,RoleRow>("UPDATE auth_roles SET role_name=$3,data_scope=$4,parent_role_id=$5,updated_at=now() WHERE id=$1 AND owner_id=$2 RETURNING id,role_code,role_name,data_scope,parent_role_id,created_at")
        .bind(role_id).bind(ctx.owner_id).bind(req.role_name.trim()).bind(req.data_scope.trim()).bind(req.parent_role_id).fetch_one(&mut *tx).await?;
    let response = role_response_tx(&mut tx, row).await?;
    let users = affected_users(&mut tx, ctx.owner_id, role_id).await?;
    mark_users_changed(&mut tx, &users).await?;
    finish(
        &mut tx,
        &ctx,
        &key,
        &hash,
        "PUT",
        &format!("/api/v1/auth/roles/{role_id}"),
        role_id,
        &response,
        "auth.role.update",
    )
    .await?;
    revoke_users(&state, &users).await?;
    tx.commit().await?;
    Ok(Json(response))
}

async fn delete_role(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
    Path(role_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<DeleteRoleResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    let key = idempotency_key(&headers)?;
    let hash = request_hash(&(role_id, "delete"))?;
    let mut tx = state.pool.begin().await?;
    lock_key(&mut tx, ctx.owner_id, &key).await?;
    if let Some(response) = replay(
        &mut tx,
        ctx.owner_id,
        &key,
        &hash,
        "DELETE",
        &format!("/api/v1/auth/roles/{role_id}"),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(response));
    }
    ensure_role_owner(&mut tx, ctx.owner_id, role_id).await?;
    let in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM auth_user_roles WHERE owner_id=$1 AND role_id=$2 UNION ALL SELECT 1 FROM auth_roles WHERE owner_id=$1 AND parent_role_id=$2)",
    )
    .bind(ctx.owner_id)
    .bind(role_id)
    .fetch_one(&mut *tx)
    .await?;
    if in_use {
        return Err(RoleError::RoleInUse);
    }
    sqlx::query("DELETE FROM auth_roles WHERE owner_id=$1 AND id=$2")
        .bind(ctx.owner_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    let response = DeleteRoleResponse { id: role_id };
    finish(
        &mut tx,
        &ctx,
        &key,
        &hash,
        "DELETE",
        &format!("/api/v1/auth/roles/{role_id}"),
        role_id,
        &response,
        "auth.role.delete",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(response))
}

async fn replace_role_permissions(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
    Path(role_id): Path<Uuid>,
    headers: HeaderMap,
    Json(mut req): Json<ReplaceRolePermissionsRequest>,
) -> Result<Json<RoleResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    req.permission_codes.sort();
    req.permission_codes.dedup();
    let key = idempotency_key(&headers)?;
    let hash = request_hash(&req)?;
    let mut tx = state.pool.begin().await?;
    lock_key(&mut tx, ctx.owner_id, &key).await?;
    if let Some(response) = replay(
        &mut tx,
        ctx.owner_id,
        &key,
        &hash,
        "PUT",
        &format!("/api/v1/auth/roles/{role_id}/permissions"),
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(response));
    }
    ensure_role_owner(&mut tx, ctx.owner_id, role_id).await?;
    let permission_ids: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id,permission_code FROM auth_permissions WHERE permission_code=ANY($1)",
    )
    .bind(&req.permission_codes)
    .fetch_all(&mut *tx)
    .await?;
    if permission_ids.len() != req.permission_codes.len() {
        return Err(RoleError::UnknownPermission);
    }
    let parent_role_id: Option<Uuid> =
        sqlx::query_scalar("SELECT parent_role_id FROM auth_roles WHERE id=$1 AND owner_id=$2")
            .bind(role_id)
            .bind(ctx.owner_id)
            .fetch_one(&mut *tx)
            .await?;
    let parent_permissions = match parent_role_id {
        Some(parent_id) => effective_permission_ids(&mut tx, parent_id).await?,
        None => Vec::new(),
    };
    sqlx::query("DELETE FROM auth_role_permissions WHERE role_id=$1")
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM auth_role_permission_exclusions WHERE role_id=$1")
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    for (permission_id, _) in &permission_ids {
        if !parent_permissions.contains(permission_id) {
            sqlx::query("INSERT INTO auth_role_permissions(role_id,permission_id) VALUES($1,$2)")
                .bind(role_id)
                .bind(permission_id)
                .execute(&mut *tx)
                .await?;
        }
    }
    for permission_id in parent_permissions {
        if !permission_ids.iter().any(|(id, _)| *id == permission_id) {
            sqlx::query(
                "INSERT INTO auth_role_permission_exclusions(role_id,permission_id) VALUES($1,$2)",
            )
            .bind(role_id)
            .bind(permission_id)
            .execute(&mut *tx)
            .await?;
        }
    }
    let users = affected_users(&mut tx, ctx.owner_id, role_id).await?;
    mark_users_changed(&mut tx, &users).await?;
    let row = fetch_role_tx(&mut tx, ctx.owner_id, role_id).await?;
    let response = role_response_tx(&mut tx, row).await?;
    finish(
        &mut tx,
        &ctx,
        &key,
        &hash,
        "PUT",
        &format!("/api/v1/auth/roles/{role_id}/permissions"),
        role_id,
        &response,
        "auth.role.permissions.replace",
    )
    .await?;
    revoke_users(&state, &users).await?;
    tx.commit().await?;
    Ok(Json(response))
}

async fn batch_assign_roles(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
    headers: HeaderMap,
    Json(mut req): Json<BatchAssignRolesRequest>,
) -> Result<Json<BatchAssignRolesResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    req.user_ids.sort();
    req.user_ids.dedup();
    req.role_ids.sort();
    req.role_ids.dedup();
    if req.user_ids.is_empty() {
        return Err(RoleError::Validation);
    }
    let key = idempotency_key(&headers)?;
    let hash = request_hash(&req)?;
    let mut tx = state.pool.begin().await?;
    lock_key(&mut tx, ctx.owner_id, &key).await?;
    if let Some(response) = replay(
        &mut tx,
        ctx.owner_id,
        &key,
        &hash,
        "PUT",
        "/api/v1/auth/user-roles/batch",
    )
    .await?
    {
        tx.commit().await?;
        return Ok(Json(response));
    }
    let users:i64=sqlx::query_scalar("SELECT count(*) FROM auth_user_owner_bindings WHERE owner_id=$1 AND user_id=ANY($2) AND is_active=true").bind(ctx.owner_id).bind(&req.user_ids).fetch_one(&mut *tx).await?;
    if users != req.user_ids.len() as i64 {
        return Err(RoleError::CrossOwner);
    }
    let roles: i64 =
        sqlx::query_scalar("SELECT count(*) FROM auth_roles WHERE owner_id=$1 AND id=ANY($2)")
            .bind(ctx.owner_id)
            .bind(&req.role_ids)
            .fetch_one(&mut *tx)
            .await?;
    if roles != req.role_ids.len() as i64 {
        return Err(RoleError::CrossOwner);
    }
    sqlx::query("DELETE FROM auth_user_roles WHERE owner_id=$1 AND user_id=ANY($2)")
        .bind(ctx.owner_id)
        .bind(&req.user_ids)
        .execute(&mut *tx)
        .await?;
    for user_id in &req.user_ids {
        for role_id in &req.role_ids {
            sqlx::query("INSERT INTO auth_user_roles(user_id,owner_id,role_id) VALUES($1,$2,$3)")
                .bind(user_id)
                .bind(ctx.owner_id)
                .bind(role_id)
                .execute(&mut *tx)
                .await?;
        }
    }
    mark_users_changed(&mut tx, &req.user_ids).await?;
    let response = BatchAssignRolesResponse {
        user_ids: req.user_ids.clone(),
        role_ids: req.role_ids.clone(),
    };
    finish(
        &mut tx,
        &ctx,
        &key,
        &hash,
        "PUT",
        "/api/v1/auth/user-roles/batch",
        ctx.owner_id,
        &response,
        "auth.user_roles.batch_replace",
    )
    .await?;
    revoke_users(&state, &req.user_ids).await?;
    tx.commit().await?;
    Ok(Json(response))
}

async fn revoke_users(state: &RoleManagementState, users: &[Uuid]) -> Result<(), RoleError> {
    let changed_at = Utc::now().timestamp() + 1;
    for user in users {
        state
            .revocations
            .set_permissions_changed_at(*user, changed_at)
            .await
            .map_err(|_| RoleError::Revocation)?;
    }
    Ok(())
}

async fn mark_users_changed(
    tx: &mut Transaction<'_, Postgres>,
    users: &[Uuid],
) -> Result<(), RoleError> {
    if !users.is_empty() {
        sqlx::query(
            "UPDATE auth_users SET permissions_changed_at=now(),updated_at=now() WHERE id=ANY($1)",
        )
        .bind(users)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn affected_users(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    role_id: Uuid,
) -> Result<Vec<Uuid>, RoleError> {
    sqlx::query_scalar(
        "WITH RECURSIVE affected AS (SELECT id FROM auth_roles WHERE id=$2 AND owner_id=$1 UNION ALL SELECT r.id FROM auth_roles r JOIN affected a ON r.parent_role_id=a.id WHERE r.owner_id=$1) SELECT DISTINCT ur.user_id FROM affected a JOIN auth_user_roles ur ON ur.role_id=a.id AND ur.owner_id=$1",
    )
    .bind(owner_id)
    .bind(role_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(Into::into)
}

async fn ensure_role_owner(
    tx: &mut Transaction<'_, Postgres>,
    owner: Uuid,
    role: Uuid,
) -> Result<(), RoleError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_roles WHERE id=$1 AND owner_id=$2)")
            .bind(role)
            .bind(owner)
            .fetch_one(&mut **tx)
            .await?;
    if exists {
        Ok(())
    } else {
        Err(RoleError::CrossOwner)
    }
}

async fn ensure_parent(
    tx: &mut Transaction<'_, Postgres>,
    owner: Uuid,
    parent: Option<Uuid>,
    role: Option<Uuid>,
) -> Result<(), RoleError> {
    let Some(parent) = parent else { return Ok(()) };
    ensure_role_owner(tx, owner, parent).await?;
    if role == Some(parent) {
        return Err(RoleError::Validation);
    }
    if let Some(role) = role {
        let cyclic:bool=sqlx::query_scalar("WITH RECURSIVE ancestors AS (SELECT parent_role_id id FROM auth_roles WHERE id=$1 UNION ALL SELECT r.parent_role_id FROM auth_roles r JOIN ancestors a ON r.id=a.id WHERE r.parent_role_id IS NOT NULL) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=$2)").bind(parent).bind(role).fetch_one(&mut **tx).await?;
        if cyclic {
            return Err(RoleError::Validation);
        }
    }
    Ok(())
}

fn validate_role(code: &str, name: &str, scope: &str) -> Result<(), RoleError> {
    if code.trim().is_empty()
        || name.trim().is_empty()
        || !matches!(scope.trim(), "self" | "warehouse" | "owner" | "all")
    {
        Err(RoleError::Validation)
    } else {
        Ok(())
    }
}

async fn fetch_role_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner: Uuid,
    id: Uuid,
) -> Result<RoleRow, RoleError> {
    sqlx::query_as("SELECT id,role_code,role_name,data_scope,parent_role_id,created_at FROM auth_roles WHERE owner_id=$1 AND id=$2").bind(owner).bind(id).fetch_one(&mut **tx).await.map_err(Into::into)
}

async fn role_response(pool: &PgPool, row: RoleRow) -> Result<RoleResponse, RoleError> {
    let mut tx = pool.begin().await?;
    let response = role_response_tx(&mut tx, row).await?;
    tx.commit().await?;
    Ok(response)
}

async fn role_response_tx(
    tx: &mut Transaction<'_, Postgres>,
    row: RoleRow,
) -> Result<RoleResponse, RoleError> {
    let permission_ids = effective_permission_ids(tx, row.id).await?;
    let permission_codes = sqlx::query_scalar(
        "SELECT permission_code FROM auth_permissions WHERE id=ANY($1) ORDER BY permission_code",
    )
    .bind(&permission_ids)
    .fetch_all(&mut **tx)
    .await?;
    Ok(RoleResponse {
        id: row.id,
        role_code: row.role_code,
        role_name: row.role_name,
        data_scope: row.data_scope,
        parent_role_id: row.parent_role_id,
        permission_codes,
        created_at: row.created_at,
    })
}

async fn effective_permission_ids(
    tx: &mut Transaction<'_, Postgres>,
    role_id: Uuid,
) -> Result<Vec<Uuid>, RoleError> {
    sqlx::query_scalar(
        r#"
        WITH RECURSIVE hierarchy AS (
            SELECT id, parent_role_id, 0 AS depth
              FROM auth_roles
             WHERE id = $1
            UNION ALL
            SELECT parent.id, parent.parent_role_id, child.depth + 1
              FROM auth_roles parent
              JOIN hierarchy child ON child.parent_role_id = parent.id
        ), decisions AS (
            SELECT grant_row.permission_id, hierarchy.depth, TRUE AS allowed
              FROM hierarchy
              JOIN auth_role_permissions grant_row ON grant_row.role_id = hierarchy.id
            UNION ALL
            SELECT exclusion.permission_id, hierarchy.depth, FALSE AS allowed
              FROM hierarchy
              JOIN auth_role_permission_exclusions exclusion ON exclusion.role_id = hierarchy.id
        ), nearest AS (
            SELECT DISTINCT ON (permission_id) permission_id, allowed
              FROM decisions
             ORDER BY permission_id, depth, allowed
        )
        SELECT permission_id
          FROM nearest
         WHERE allowed
         ORDER BY permission_id
        "#,
    )
    .bind(role_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(Into::into)
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, RoleError> {
    headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .ok_or(RoleError::MissingIdempotency)
}
fn request_hash<T: Serialize>(req: &T) -> Result<String, RoleError> {
    idempotency::request_hash(req).map_err(Into::into)
}
async fn lock_key(
    tx: &mut Transaction<'_, Postgres>,
    owner: Uuid,
    key: &str,
) -> Result<(), RoleError> {
    idempotency::lock_key(tx, "role-management", owner, key)
        .await
        .map_err(Into::into)
}
async fn replay<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner: Uuid,
    key: &str,
    hash: &str,
    method: &str,
    path: &str,
) -> Result<Option<T>, RoleError> {
    idempotency::replay(tx, owner, key, hash, method, path, Utc::now())
        .await
        .map_err(Into::into)
}

#[allow(clippy::too_many_arguments)]
async fn finish<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    key: &str,
    hash: &str,
    method: &str,
    path: &str,
    resource_id: Uuid,
    response: &T,
    action: &str,
) -> Result<(), RoleError> {
    let resource_type = if action.contains("user_roles") {
        "auth_user_roles"
    } else if action.starts_with("auth.user.") {
        "auth_user"
    } else {
        "auth_role"
    };
    idempotency::store_success(
        tx,
        ctx.owner_id,
        key,
        hash,
        method,
        path,
        resource_type,
        &resource_id.to_string(),
        response,
        Utc::now(),
    )
    .await
    .map_err(RoleError::from)?;
    let body = serde_json::to_value(response).map_err(|_| RoleError::Serialize)?;
    append_event_in_tx(
        tx,
        &AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "H1",
            resource_type,
            resource_id.to_string(),
            Some(AuditDiff::compute(serde_json::json!({}), body)),
        ),
    )
    .await
    .map_err(|_| RoleError::Audit)?;
    Ok(())
}

fn map_write_error(error: sqlx::Error) -> RoleError {
    if matches!(&error,sqlx::Error::Database(db) if db.is_unique_violation()) {
        RoleError::DuplicateRole
    } else {
        RoleError::Database
    }
}
