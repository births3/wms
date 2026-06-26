//! Wave 1 H1 auth use cases.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;
use wms_domain::{CurrentUser, LoginRequest, LoginResponse};

use crate::{
    auth::{
        build_access_claims, encode_access_token, AuthContext, AuthError, ACCESS_TOKEN_TTL_SECONDS,
        JWT_SECRET_ENV,
    },
    auth_repository::{AuthRepository, AuthRepositoryError, LoginUser},
};

#[derive(Clone)]
pub struct AuthService {
    repository: AuthRepository,
}

impl AuthService {
    pub fn new(repository: AuthRepository) -> Self {
        Self { repository }
    }

    pub async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AuthServiceError> {
        let now = Utc::now();
        let Some(user) = self
            .repository
            .find_login_user(&request.owner_code, &request.username)
            .await?
        else {
            return Err(AuthServiceError::InvalidCredentials);
        };

        ensure_login_allowed(&user, now)?;

        if !verify_password(request.password, user.password_hash.clone()).await? {
            self.repository
                .record_failed_login(user.user_id, now)
                .await?;
            return Err(AuthServiceError::InvalidCredentials);
        }

        self.repository
            .reset_login_failures(user.user_id, now)
            .await?;
        let current_user = self
            .repository
            .current_user(user.user_id, user.owner_id)
            .await?
            .ok_or(AuthServiceError::InvalidCredentials)?;
        let jwt_secret = std::env::var(JWT_SECRET_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or(AuthServiceError::MissingSecret)?;
        let jti = Uuid::new_v4().to_string();
        let claims = build_access_claims(
            current_user.user_id,
            current_user.owner_id,
            current_user.display_name.clone(),
            current_user.permissions.clone(),
            jti.clone(),
            now,
        );
        let access_token = encode_access_token(&claims, &jwt_secret)?;
        self.repository
            .append_login_success_audit(&current_user, &jti, now)
            .await?;

        Ok(LoginResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_at: now + Duration::seconds(ACCESS_TOKEN_TTL_SECONDS),
            user: current_user,
        })
    }

    pub async fn current_user(&self, ctx: &AuthContext) -> Result<CurrentUser, AuthServiceError> {
        self.repository
            .current_user(ctx.user_id, ctx.owner_id)
            .await?
            .ok_or(AuthServiceError::Auth(AuthError::InvalidToken))
    }
}

fn ensure_login_allowed(user: &LoginUser, now: DateTime<Utc>) -> Result<(), AuthServiceError> {
    match user.status.as_str() {
        "active" => Ok(()),
        "locked"
            if user
                .locked_until
                .is_some_and(|locked_until| locked_until <= now) =>
        {
            Ok(())
        }
        "locked" => Err(AuthServiceError::AccountLocked(user.locked_until)),
        _ => Err(AuthServiceError::InvalidCredentials),
    }
}

async fn verify_password(
    password: String,
    password_hash: String,
) -> Result<bool, AuthServiceError> {
    tokio::task::spawn_blocking(move || bcrypt::verify(password, &password_hash))
        .await
        .map_err(|_| AuthServiceError::PasswordHash)?
        .map_err(|_| AuthServiceError::PasswordHash)
}

#[derive(Debug)]
pub enum AuthServiceError {
    Auth(AuthError),
    InvalidCredentials,
    AccountLocked(Option<DateTime<Utc>>),
    Repository,
    PasswordHash,
    MissingSecret,
}

impl From<AuthError> for AuthServiceError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<AuthRepositoryError> for AuthServiceError {
    fn from(_value: AuthRepositoryError) -> Self {
        Self::Repository
    }
}
