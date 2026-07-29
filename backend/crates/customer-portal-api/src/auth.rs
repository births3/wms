use crate::{audit, models::LoginResponse, models::PortalUserSummary, PortalError, PortalState};
use axum::{
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

pub const ACCESS_TOKEN_TTL_MINUTES: i64 = 60;
const MAX_FAILED_LOGINS: i32 = 5;
const LOCK_MINUTES: i64 = 15;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortalClaims {
    pub sub: Uuid,
    pub customer_id: Uuid,
    pub username: String,
    pub role: String,
    pub can_view_report_history: bool,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Clone, Debug)]
pub struct PortalAuth {
    pub user_id: Uuid,
    pub customer_id: Uuid,
    pub username: String,
    pub role: String,
    pub can_view_report_history: bool,
}

impl PortalAuth {
    pub fn is_customer_admin(&self) -> bool {
        self.role == "customer_admin"
    }
}

#[axum::async_trait]
impl FromRequestParts<PortalState> for PortalAuth {
    type Rejection = PortalError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &PortalState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(PortalError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(PortalError::Unauthorized)?;
        let claims = decode::<PortalClaims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|_| PortalError::Unauthorized)?
        .claims;
        let active = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1 FROM portal_users
                WHERE id = $1 AND customer_id = $2 AND status = 'active'
            )",
        )
        .bind(claims.sub)
        .bind(claims.customer_id)
        .fetch_one(&state.pool)
        .await?;
        if !active {
            return Err(PortalError::Unauthorized);
        }
        Ok(Self {
            user_id: claims.sub,
            customer_id: claims.customer_id,
            username: claims.username,
            role: claims.role,
            can_view_report_history: claims.can_view_report_history,
        })
    }
}

pub async fn login(
    State(state): State<PortalState>,
    Json(request): Json<crate::models::LoginRequest>,
) -> Result<Json<LoginResponse>, PortalError> {
    let username = request.username.trim();
    if username.is_empty() || request.password.is_empty() {
        return Err(PortalError::Unauthorized);
    }
    let row = sqlx::query(
        "SELECT id, customer_id, username, display_name, password_hash, role, status,
                can_view_report_history, failed_login_count, locked_until
         FROM portal_users
         WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(PortalError::Unauthorized)?;
    let user_id: Uuid = row.try_get("id")?;
    let customer_id: Uuid = row.try_get("customer_id")?;
    let status: String = row.try_get("status")?;
    let locked_until: Option<chrono::DateTime<Utc>> = row.try_get("locked_until")?;
    if status != "active" || locked_until.is_some_and(|until| until > Utc::now()) {
        audit(
            &state.pool,
            Some(user_id),
            Some(customer_id),
            "login_rejected",
            "session",
            &user_id.to_string(),
            serde_json::json!({ "reason": "disabled_or_locked" }),
        )
        .await?;
        return Err(PortalError::Unauthorized);
    }
    let password_hash: String = row.try_get("password_hash")?;
    let password = request.password;
    let verified = tokio::task::spawn_blocking(move || bcrypt::verify(password, &password_hash))
        .await
        .map_err(|error| PortalError::Internal(error.to_string()))?
        .map_err(|error| PortalError::Internal(error.to_string()))?;
    if !verified {
        let failures: i32 = row.try_get::<i32, _>("failed_login_count")? + 1;
        let lock =
            (failures >= MAX_FAILED_LOGINS).then(|| Utc::now() + Duration::minutes(LOCK_MINUTES));
        sqlx::query(
            "UPDATE portal_users
             SET failed_login_count = $2, locked_until = $3, updated_at = now()
             WHERE id = $1",
        )
        .bind(user_id)
        .bind(failures)
        .bind(lock)
        .execute(&state.pool)
        .await?;
        audit(
            &state.pool,
            Some(user_id),
            Some(customer_id),
            "login_rejected",
            "session",
            &user_id.to_string(),
            serde_json::json!({ "reason": "invalid_credentials" }),
        )
        .await?;
        return Err(PortalError::Unauthorized);
    }
    sqlx::query(
        "UPDATE portal_users
         SET failed_login_count = 0, locked_until = NULL, updated_at = now()
         WHERE id = $1",
    )
    .bind(user_id)
    .execute(&state.pool)
    .await?;

    let now = Utc::now();
    let expires_at = now + Duration::minutes(ACCESS_TOKEN_TTL_MINUTES);
    let user = PortalUserSummary {
        id: user_id,
        customer_id,
        username: row.try_get("username")?,
        display_name: row.try_get("display_name")?,
        role: row.try_get("role")?,
        status,
        can_view_report_history: row.try_get("can_view_report_history")?,
        address_ids: sqlx::query_scalar::<_, Uuid>(
            "SELECT address_id FROM portal_user_addresses WHERE user_id = $1 ORDER BY address_id",
        )
        .bind(user_id)
        .fetch_all(&state.pool)
        .await?,
    };
    let claims = PortalClaims {
        sub: user.id,
        customer_id: user.customer_id,
        username: user.username.clone(),
        role: user.role.clone(),
        can_view_report_history: user.can_view_report_history,
        iat: now.timestamp(),
        exp: expires_at.timestamp(),
    };
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|error| PortalError::Internal(error.to_string()))?;
    audit(
        &state.pool,
        Some(user_id),
        Some(customer_id),
        "login",
        "session",
        &user_id.to_string(),
        serde_json::json!({}),
    )
    .await?;
    Ok(Json(LoginResponse {
        access_token,
        expires_at,
        user,
    }))
}
