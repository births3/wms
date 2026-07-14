use axum::{extract::State, http::HeaderMap, Json};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    auth::AuthContext,
    auth_service::{hash_password, password_meets_policy},
    role_management::{
        finish, idempotency_key, lock_key, replay, request_hash, CreateUserRequest, RoleError,
        RoleManagementState, RoleUserResponse, ROLE_MANAGE_PERMISSION,
    },
};

#[derive(FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    display_name: String,
}

pub(super) async fn create_user(
    ctx: AuthContext,
    State(state): State<RoleManagementState>,
    headers: HeaderMap,
    Json(mut req): Json<CreateUserRequest>,
) -> Result<Json<RoleUserResponse>, RoleError> {
    ctx.require_permission(ROLE_MANAGE_PERMISSION)?;
    req.username = req.username.trim().to_owned();
    req.display_name = req.display_name.trim().to_owned();
    req.phone = req.phone.trim().to_owned();
    req.role_ids.sort();
    req.role_ids.dedup();
    if req.username.is_empty()
        || req.display_name.is_empty()
        || req.phone.chars().count() < 7
        || !password_meets_policy(&req.password)
        || req.role_ids.is_empty()
    {
        return Err(RoleError::UserValidation);
    }

    let key = idempotency_key(&headers)?;
    let hash = request_hash(&req)?;
    let password_hash = hash_password(req.password)
        .await
        .map_err(|_| RoleError::PasswordHash)?;
    let mut tx = state.pool.begin().await?;
    lock_key(&mut tx, ctx.owner_id, &key).await?;
    if let Some(response) = replay(&mut tx, ctx.owner_id, &key, &hash).await? {
        tx.commit().await?;
        return Ok(Json(response));
    }

    let role_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM auth_roles WHERE owner_id=$1 AND id=ANY($2)")
            .bind(ctx.owner_id)
            .bind(&req.role_ids)
            .fetch_one(&mut *tx)
            .await?;
    if role_count != req.role_ids.len() as i64 {
        return Err(RoleError::RoleNotFound);
    }

    let username_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM auth_users WHERE lower(username)=lower($1))",
    )
    .bind(&req.username)
    .fetch_one(&mut *tx)
    .await?;
    if username_exists {
        return Err(RoleError::UserDuplicate);
    }

    let user_id = Uuid::new_v4();
    let user = sqlx::query_as::<_, UserRow>(
        "INSERT INTO auth_users(id,username,display_name,phone,password_hash) VALUES($1,$2,$3,$4,$5) RETURNING id,username,display_name",
    )
    .bind(user_id)
    .bind(&req.username)
    .bind(&req.display_name)
    .bind(&req.phone)
    .bind(password_hash)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| {
        if matches!(&error, sqlx::Error::Database(db) if db.is_unique_violation()) {
            RoleError::UserDuplicate
        } else {
            RoleError::Database
        }
    })?;

    sqlx::query(
        "INSERT INTO auth_user_owner_bindings(user_id,owner_id,is_primary) VALUES($1,$2,true)",
    )
    .bind(user.id)
    .bind(ctx.owner_id)
    .execute(&mut *tx)
    .await?;
    for role_id in &req.role_ids {
        sqlx::query("INSERT INTO auth_user_roles(user_id,owner_id,role_id) VALUES($1,$2,$3)")
            .bind(user.id)
            .bind(ctx.owner_id)
            .bind(role_id)
            .execute(&mut *tx)
            .await?;
    }

    let response = RoleUserResponse {
        user_id: user.id,
        username: user.username,
        display_name: user.display_name,
        role_ids: req.role_ids.clone(),
    };
    finish(
        &mut tx,
        &ctx,
        &key,
        &hash,
        "POST",
        "/api/v1/auth/users",
        user_id,
        &response,
        "auth.user.create",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(response))
}
