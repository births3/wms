//! Wave 1 H1 auth use cases.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;
use wms_domain::{
    AuthRevocationResponse, AuthSessionListResponse, AuthSessionRevokeResponse,
    AuthUserStatusRequest, CurrentUser, LoginRequest, LoginResponse, PasswordChangeRequest,
};

use crate::{
    audit::AuditDiff,
    auth::{
        build_access_claims, encode_access_token, AuthContext, AuthError, AuthRevocationStore,
        AuthRuntimePolicy, LogoutContext, ACCESS_TOKEN_TTL_SECONDS, JWT_SECRET_ENV,
    },
    auth_repository::{AuthRepository, AuthRepositoryError, LoginUser, SessionRevokeState},
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
        self.login_with_metadata(request, LoginMetadata::default())
            .await
    }

    pub async fn login_with_metadata(
        &self,
        request: LoginRequest,
        metadata: LoginMetadata,
    ) -> Result<LoginResponse, AuthServiceError> {
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
            .record_login_session(
                &current_user,
                &jti,
                now + Duration::seconds(ACCESS_TOKEN_TTL_SECONDS),
                now,
                &metadata.device_name,
                metadata.ip.as_deref(),
                metadata.user_agent.as_deref(),
            )
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

    pub async fn list_sessions(
        &self,
        ctx: &AuthContext,
        user_id: Uuid,
    ) -> Result<AuthSessionListResponse, AuthServiceError> {
        let sessions = self
            .repository
            .active_sessions(ctx.owner_id, user_id, Utc::now())
            .await?
            .into_iter()
            .map(|session| session.into_active_session(&ctx.jti))
            .collect::<Vec<_>>();
        Ok(AuthSessionListResponse {
            count: sessions.len() as u32,
            data: sessions,
        })
    }

    pub async fn logout(
        &self,
        ctx: &LogoutContext,
        policy: &AuthRuntimePolicy,
    ) -> Result<AuthRevocationResponse, AuthServiceError> {
        let store = policy.revocation_store();
        let (already_blacklisted, read_degraded) = match store
            .jti_is_blacklisted(&ctx.auth.jti)
            .await
        {
            Ok(value) => (value, false),
            Err(error) => {
                tracing::warn!(error = ?error, alert = "P1", jti = %ctx.auth.jti, "auth token blacklist read unavailable; fail-open");
                (false, true)
            }
        };
        let state = self
            .repository
            .revoke_session(
                &ctx.auth,
                ctx.auth.user_id,
                &ctx.auth.jti,
                "主动登出",
                "auth.logout",
            )
            .await?;
        if matches!(
            state,
            SessionRevokeState::NotFound | SessionRevokeState::AlreadyRevoked { .. }
        ) && !self
            .repository
            .logout_audit_exists(ctx.auth.owner_id, &ctx.auth.jti)
            .await?
        {
            self.repository
                .append_auth_event(
                    &ctx.auth,
                    "auth.logout",
                    "auth_session",
                    &ctx.auth.jti,
                    Some(AuditDiff::compute(
                        serde_json::json!({"status": "active"}),
                        serde_json::json!({"status": "revoked", "reason": "主动登出"}),
                    )),
                )
                .await?;
        }
        let degraded = blacklist_jti(store.as_ref(), &ctx.auth.jti, ctx.expires_at).await;
        Ok(AuthRevocationResponse {
            revoked_jti: ctx.auth.jti.clone(),
            revocation_degraded: read_degraded || (degraded && !already_blacklisted),
        })
    }

    pub async fn revoke_session(
        &self,
        ctx: &AuthContext,
        session_id: &str,
        policy: &AuthRuntimePolicy,
    ) -> Result<AuthRevocationResponse, AuthServiceError> {
        let state = self
            .repository
            .revoke_session(
                ctx,
                ctx.user_id,
                session_id,
                "单设备失效",
                "auth.session.revoke",
            )
            .await?;
        let expires_at = match state {
            SessionRevokeState::NotFound => return Err(AuthServiceError::SessionNotFound),
            SessionRevokeState::AlreadyRevoked { expires_at }
            | SessionRevokeState::Revoked { expires_at } => expires_at,
        };
        let degraded = blacklist_jti(
            policy.revocation_store().as_ref(),
            session_id,
            expires_at.timestamp(),
        )
        .await;
        Ok(AuthRevocationResponse {
            revoked_jti: session_id.to_string(),
            revocation_degraded: degraded,
        })
    }

    pub async fn revoke_other_sessions(
        &self,
        ctx: &AuthContext,
        policy: &AuthRuntimePolicy,
    ) -> Result<AuthSessionRevokeResponse, AuthServiceError> {
        let rows = self
            .repository
            .revoke_active_sessions(
                ctx,
                ctx.user_id,
                Some(&ctx.jti),
                "主动登出其他设备",
                "auth.sessions.revoke_others",
            )
            .await?;
        let store = policy.revocation_store();
        let mut degraded = false;
        for row in &rows {
            degraded |=
                blacklist_jti(store.as_ref(), &row.session_id, row.expires_at.timestamp()).await;
        }
        Ok(AuthSessionRevokeResponse {
            user_id: ctx.user_id,
            revoked_sessions: rows.len() as u32,
            revocation_degraded: degraded,
        })
    }

    pub async fn change_password(
        &self,
        ctx: &AuthContext,
        request: PasswordChangeRequest,
        policy: &AuthRuntimePolicy,
    ) -> Result<AuthSessionRevokeResponse, AuthServiceError> {
        let Some(current_hash) = self.repository.password_hash(ctx).await? else {
            return Err(AuthServiceError::UserNotFound);
        };
        if !verify_password(request.current_password, current_hash).await? {
            return Err(AuthServiceError::InvalidCredentials);
        }
        if !password_meets_policy(&request.new_password) {
            return Err(AuthServiceError::InvalidPassword);
        }
        let new_hash = hash_password(request.new_password).await?;
        let changed_at = Utc::now();
        if !self
            .repository
            .change_password(ctx, &new_hash, changed_at)
            .await?
        {
            return Err(AuthServiceError::UserNotFound);
        }
        self.invalidate_user_sessions(
            ctx,
            ctx.user_id,
            "修改密码",
            "auth.password.sessions_revoked",
            policy,
        )
        .await
    }

    pub async fn change_user_status(
        &self,
        ctx: &AuthContext,
        user_id: Uuid,
        request: AuthUserStatusRequest,
        policy: &AuthRuntimePolicy,
    ) -> Result<AuthSessionRevokeResponse, AuthServiceError> {
        if !matches!(request.status.as_str(), "active" | "disabled") {
            return Err(AuthServiceError::InvalidStatus);
        }
        if user_id == ctx.user_id && request.status == "disabled" {
            return Err(AuthServiceError::CannotDisableSelf);
        }
        let changed_at = Utc::now();
        if !self
            .repository
            .change_user_status(ctx, user_id, &request.status, changed_at)
            .await?
        {
            return Err(AuthServiceError::UserNotFound);
        }
        self.invalidate_user_sessions(
            ctx,
            user_id,
            if request.status == "disabled" {
                "用户停用"
            } else {
                "用户启用"
            },
            "auth.user.sessions_revoked",
            policy,
        )
        .await
    }

    pub async fn kick_user(
        &self,
        ctx: &AuthContext,
        user_id: Uuid,
        policy: &AuthRuntimePolicy,
    ) -> Result<AuthSessionRevokeResponse, AuthServiceError> {
        if !self
            .repository
            .user_belongs_to_owner(ctx.owner_id, user_id)
            .await?
        {
            return Err(AuthServiceError::UserNotFound);
        }
        self.invalidate_user_sessions(ctx, user_id, "管理员强制踢人", "auth.token_revoked", policy)
            .await
    }

    async fn invalidate_user_sessions(
        &self,
        ctx: &AuthContext,
        user_id: Uuid,
        reason: &str,
        action: &str,
        policy: &AuthRuntimePolicy,
    ) -> Result<AuthSessionRevokeResponse, AuthServiceError> {
        let rows = self
            .repository
            .revoke_active_sessions(ctx, user_id, None, reason, action)
            .await?;
        let store = policy.revocation_store();
        let mut degraded = false;
        for row in &rows {
            degraded |=
                blacklist_jti(store.as_ref(), &row.session_id, row.expires_at.timestamp()).await;
        }
        let changed_at = Utc::now().timestamp() + 1;
        if let Err(error) = store.set_permissions_changed_at(user_id, changed_at).await {
            tracing::warn!(error = ?error, alert = "P1", user_id = %user_id, "auth user-wide revocation unavailable; fail-open");
            degraded = true;
        }
        Ok(AuthSessionRevokeResponse {
            user_id,
            revoked_sessions: rows.len() as u32,
            revocation_degraded: degraded,
        })
    }
}

#[derive(Clone, Debug)]
pub struct LoginMetadata {
    pub device_name: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

impl Default for LoginMetadata {
    fn default() -> Self {
        Self {
            device_name: "unknown".to_string(),
            ip: None,
            user_agent: None,
        }
    }
}

async fn blacklist_jti(store: &dyn AuthRevocationStore, jti: &str, expires_at_unix: i64) -> bool {
    let ttl_seconds = (expires_at_unix - Utc::now().timestamp()).max(1) as u64;
    match store.blacklist_jti(jti, ttl_seconds).await {
        Ok(()) => false,
        Err(error) => {
            tracing::warn!(error = ?error, alert = "P1", jti, "auth token blacklist unavailable; fail-open");
            true
        }
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

pub(crate) async fn hash_password(password: String) -> Result<String, AuthServiceError> {
    tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST))
        .await
        .map_err(|_| AuthServiceError::PasswordHash)?
        .map_err(|_| AuthServiceError::PasswordHash)
}

pub(crate) fn password_meets_policy(password: &str) -> bool {
    password.chars().count() >= 8
        && password
            .chars()
            .any(|character| character.is_ascii_uppercase())
        && password
            .chars()
            .any(|character| character.is_ascii_lowercase())
        && password.chars().any(|character| character.is_ascii_digit())
}

#[derive(Debug)]
pub enum AuthServiceError {
    Auth(AuthError),
    InvalidCredentials,
    AccountLocked(Option<DateTime<Utc>>),
    Repository,
    PasswordHash,
    MissingSecret,
    SessionNotFound,
    UserNotFound,
    InvalidPassword,
    InvalidStatus,
    CannotDisableSelf,
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
