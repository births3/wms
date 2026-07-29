use crate::{
    audit,
    auth::PortalAuth,
    models::{CreateUserRequest, PortalUserSummary, UpdateUserRequest},
    PortalError, PortalState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use sqlx::Row;
use uuid::Uuid;

pub async fn list_users(
    State(state): State<PortalState>,
    auth: PortalAuth,
) -> Result<Json<Vec<PortalUserSummary>>, PortalError> {
    require_admin(&auth)?;
    let rows = sqlx::query(
        "SELECT u.id, u.customer_id, u.username, u.display_name, u.role, u.status,
                u.can_view_report_history,
                COALESCE(
                    array_agg(ua.address_id ORDER BY ua.address_id)
                        FILTER (WHERE ua.address_id IS NOT NULL),
                    ARRAY[]::uuid[]
                ) AS address_ids
         FROM portal_users u
         LEFT JOIN portal_user_addresses ua ON ua.user_id = u.id
         WHERE u.customer_id = $1
         GROUP BY u.id
         ORDER BY u.username",
    )
    .bind(auth.customer_id)
    .fetch_all(&state.pool)
    .await?;
    let users = rows
        .into_iter()
        .map(user_summary_from_row)
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    Ok(Json(users))
}

pub async fn create_user(
    State(state): State<PortalState>,
    auth: PortalAuth,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<PortalUserSummary>, PortalError> {
    require_admin(&auth)?;
    validate_request(&request)?;
    if request.role == "customer_user" && request.address_ids.is_empty() {
        return Err(PortalError::Validation(
            "普通客户账号至少绑定一个客户地址".to_string(),
        ));
    }
    let distinct_addresses = request
        .address_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let valid_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM portal_customer_addresses
         WHERE customer_id = $1 AND id = ANY($2)",
    )
    .bind(auth.customer_id)
    .bind(request.address_ids.as_slice())
    .fetch_one(&state.pool)
    .await?;
    if valid_count != distinct_addresses.len() as i64 {
        return Err(PortalError::Validation(
            "客户地址不存在或不属于当前客户".to_string(),
        ));
    }
    let password = request.password;
    let password_hash =
        tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
            .await
            .map_err(|error| PortalError::Internal(error.to_string()))?
            .map_err(|error| PortalError::Internal(error.to_string()))?;
    let user_id = Uuid::new_v4();
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO portal_users (
            id, customer_id, username, display_name, password_hash, role,
            status, can_view_report_history
         )
         VALUES ($1, $2, $3, $4, $5, $6, 'active', $7)",
    )
    .bind(user_id)
    .bind(auth.customer_id)
    .bind(request.username.trim())
    .bind(request.display_name.trim())
    .bind(password_hash)
    .bind(&request.role)
    .bind(request.can_view_report_history)
    .execute(&mut *transaction)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|database| database.is_unique_violation())
        {
            PortalError::Conflict("用户名已存在".to_string())
        } else {
            PortalError::Database(error)
        }
    })?;
    for address_id in distinct_addresses {
        sqlx::query(
            "INSERT INTO portal_user_addresses (user_id, address_id)
             VALUES ($1, $2)",
        )
        .bind(user_id)
        .bind(address_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    audit(
        &state.pool,
        Some(auth.user_id),
        Some(auth.customer_id),
        "create",
        "portal_user",
        &user_id.to_string(),
        serde_json::json!({
            "role": request.role,
            "address_count": request.address_ids.len(),
            "can_view_report_history": request.can_view_report_history
        }),
    )
    .await?;
    Ok(Json(PortalUserSummary {
        id: user_id,
        customer_id: auth.customer_id,
        username: request.username.trim().to_string(),
        display_name: request.display_name.trim().to_string(),
        role: request.role,
        status: "active".to_string(),
        can_view_report_history: request.can_view_report_history,
        address_ids: request.address_ids,
    }))
}

pub async fn update_user(
    State(state): State<PortalState>,
    auth: PortalAuth,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<PortalUserSummary>, PortalError> {
    require_admin(&auth)?;
    validate_update_request(&request)?;
    if user_id == auth.user_id && request.status != "active" {
        return Err(PortalError::Validation(
            "不能停用当前登录的客户管理员账号".to_string(),
        ));
    }
    validate_addresses(
        &state,
        auth.customer_id,
        &request.role,
        &request.address_ids,
    )
    .await?;
    let distinct_addresses = request
        .address_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let mut transaction = state.pool.begin().await?;
    let username = sqlx::query_scalar::<_, String>(
        "UPDATE portal_users
         SET display_name = $3, role = $4, status = $5,
             can_view_report_history = $6, updated_at = now()
         WHERE id = $1 AND customer_id = $2
         RETURNING username",
    )
    .bind(user_id)
    .bind(auth.customer_id)
    .bind(request.display_name.trim())
    .bind(&request.role)
    .bind(&request.status)
    .bind(request.can_view_report_history)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(PortalError::NotFound)?;
    sqlx::query("DELETE FROM portal_user_addresses WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    for address_id in &distinct_addresses {
        sqlx::query("INSERT INTO portal_user_addresses (user_id, address_id) VALUES ($1, $2)")
            .bind(user_id)
            .bind(address_id)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    audit(
        &state.pool,
        Some(auth.user_id),
        Some(auth.customer_id),
        "update",
        "portal_user",
        &user_id.to_string(),
        serde_json::json!({
            "role": request.role,
            "status": request.status,
            "address_count": distinct_addresses.len(),
            "can_view_report_history": request.can_view_report_history
        }),
    )
    .await?;
    Ok(Json(PortalUserSummary {
        id: user_id,
        customer_id: auth.customer_id,
        username,
        display_name: request.display_name.trim().to_string(),
        role: request.role,
        status: request.status,
        can_view_report_history: request.can_view_report_history,
        address_ids: distinct_addresses.into_iter().collect(),
    }))
}

fn require_admin(auth: &PortalAuth) -> Result<(), PortalError> {
    if auth.is_customer_admin() {
        Ok(())
    } else {
        Err(PortalError::Forbidden)
    }
}

fn validate_request(request: &CreateUserRequest) -> Result<(), PortalError> {
    if request.username.trim().len() < 3 {
        return Err(PortalError::Validation("用户名至少 3 个字符".to_string()));
    }
    if request.display_name.trim().is_empty() {
        return Err(PortalError::Validation("显示名称不能为空".to_string()));
    }
    if request.password.len() < 12
        || !request
            .password
            .chars()
            .any(|character| character.is_ascii_uppercase())
        || !request
            .password
            .chars()
            .any(|character| character.is_ascii_lowercase())
        || !request
            .password
            .chars()
            .any(|character| character.is_ascii_digit())
    {
        return Err(PortalError::Validation(
            "密码至少 12 位且包含大小写字母和数字".to_string(),
        ));
    }
    if !matches!(request.role.as_str(), "customer_admin" | "customer_user") {
        return Err(PortalError::Validation("不支持的客户角色".to_string()));
    }
    Ok(())
}

fn validate_update_request(request: &UpdateUserRequest) -> Result<(), PortalError> {
    if request.display_name.trim().is_empty() {
        return Err(PortalError::Validation("显示名称不能为空".to_string()));
    }
    if !matches!(request.role.as_str(), "customer_admin" | "customer_user") {
        return Err(PortalError::Validation("不支持的客户角色".to_string()));
    }
    if !matches!(request.status.as_str(), "active" | "disabled") {
        return Err(PortalError::Validation(
            "账号状态只允许 active 或 disabled".to_string(),
        ));
    }
    Ok(())
}

async fn validate_addresses(
    state: &PortalState,
    customer_id: Uuid,
    role: &str,
    address_ids: &[Uuid],
) -> Result<(), PortalError> {
    if role == "customer_user" && address_ids.is_empty() {
        return Err(PortalError::Validation(
            "普通客户账号至少绑定一个客户地址".to_string(),
        ));
    }
    let distinct = address_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let valid_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM portal_customer_addresses
         WHERE customer_id = $1 AND id = ANY($2)",
    )
    .bind(customer_id)
    .bind(address_ids)
    .fetch_one(&state.pool)
    .await?;
    if valid_count != distinct.len() as i64 {
        return Err(PortalError::Validation(
            "客户地址不存在或不属于当前客户".to_string(),
        ));
    }
    Ok(())
}

fn user_summary_from_row(row: sqlx::postgres::PgRow) -> Result<PortalUserSummary, sqlx::Error> {
    Ok(PortalUserSummary {
        id: row.try_get("id")?,
        customer_id: row.try_get("customer_id")?,
        username: row.try_get("username")?,
        display_name: row.try_get("display_name")?,
        role: row.try_get("role")?,
        status: row.try_get("status")?,
        can_view_report_history: row.try_get("can_view_report_history")?,
        address_ids: row.try_get("address_ids")?,
    })
}
