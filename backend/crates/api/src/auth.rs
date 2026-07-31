//! Wave 1 H1 AuthContext runtime contract.
//!
//! ADR-0024 fixes the Wave 1 boundary: JWT claims carry `owner_id`,
//! handlers receive `AuthContext`, and PostgreSQL RLS is deferred.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use wms_domain::ErrorResponse;

pub use crate::operation_context::OperationContext;

pub const ACCESS_TOKEN_TTL_SECONDS: i64 = 60 * 60;
pub const JWT_SECRET_ENV: &str = "WMS_JWT_SECRET";
pub const REDIS_PERMISSIONS_CHANGED_AT_TTL_SECONDS: u64 = ACCESS_TOKEN_TTL_SECONDS as u64 * 2;
pub const AUTH_BLACKLIST_KEY_PREFIX: &str = "auth:blacklist:jti:";
pub const AUTH_PERMISSIONS_CHANGED_AT_KEY_PREFIX: &str = "user:";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claims {
    pub sub: Uuid,
    pub owner_id: Uuid,
    pub user_name: String,
    pub permissions: Vec<String>,
    pub jti: String,
    pub iat: i64,
    pub exp: i64,
}

/// 运行时鉴权上下文的兼容名称；值对象定义在 `operation_context`。
pub type AuthContext = OperationContext;

/// 登出专用上下文：验签但不检查撤销状态，保证重复登出仍然幂等。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogoutContext {
    pub auth: AuthContext,
    pub expires_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthError {
    MissingAuthorization,
    InvalidAuthorization,
    InvalidToken,
    MissingSecret,
    TokenRevoked,
    PermissionDenied(String),
    PermissionsRevoked,
    CrossOwnerAccess,
    MissingRuntimePolicy,
    RevocationStoreUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthRevocationStoreError {
    Unavailable(String),
}

impl From<redis::RedisError> for AuthRevocationStoreError {
    fn from(value: redis::RedisError) -> Self {
        Self::Unavailable(value.to_string())
    }
}

#[axum::async_trait]
pub trait AuthRevocationStore: Send + Sync {
    async fn jti_is_blacklisted(&self, jti: &str) -> Result<bool, AuthRevocationStoreError>;

    async fn permissions_changed_at(
        &self,
        user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError>;

    async fn blacklist_jti(
        &self,
        jti: &str,
        ttl_seconds: u64,
    ) -> Result<(), AuthRevocationStoreError>;

    async fn set_permissions_changed_at(
        &self,
        user_id: Uuid,
        changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError>;
}

#[derive(Clone)]
pub struct AuthRuntimePolicy {
    revocation_store: Arc<dyn AuthRevocationStore>,
    fail_open_on_store_error: bool,
}

impl AuthRuntimePolicy {
    pub fn new(revocation_store: Arc<dyn AuthRevocationStore>) -> Self {
        Self {
            revocation_store,
            fail_open_on_store_error: true,
        }
    }

    pub fn strict(revocation_store: Arc<dyn AuthRevocationStore>) -> Self {
        Self {
            revocation_store,
            fail_open_on_store_error: false,
        }
    }

    pub fn revocation_store(&self) -> Arc<dyn AuthRevocationStore> {
        Arc::clone(&self.revocation_store)
    }

    pub async fn validate_claims(&self, claims: &Claims) -> Result<(), AuthError> {
        match self
            .revocation_store
            .permissions_changed_at(claims.sub)
            .await
        {
            Ok(Some(changed_at)) if changed_at > claims.iat => {
                return Err(AuthError::PermissionsRevoked);
            }
            Ok(_) => {}
            Err(error) if self.fail_open_on_store_error => {
                tracing::warn!(error = ?error, alert = "P1", "auth revocation store unavailable; fail-open");
                return Ok(());
            }
            Err(_) => return Err(AuthError::RevocationStoreUnavailable),
        }

        match self.revocation_store.jti_is_blacklisted(&claims.jti).await {
            Ok(true) => Err(AuthError::TokenRevoked),
            Ok(false) => Ok(()),
            Err(error) if self.fail_open_on_store_error => {
                tracing::warn!(error = ?error, alert = "P1", "auth revocation store unavailable; fail-open");
                Ok(())
            }
            Err(_) => Err(AuthError::RevocationStoreUnavailable),
        }
    }
}

#[derive(Clone)]
pub struct RedisAuthRevocationStore {
    connection: redis::aio::MultiplexedConnection,
}

impl RedisAuthRevocationStore {
    pub fn new(connection: redis::aio::MultiplexedConnection) -> Self {
        Self { connection }
    }

    pub async fn from_url(redis_url: &str) -> Result<Self, AuthRevocationStoreError> {
        let client = redis::Client::open(redis_url)
            .map_err(|error| AuthRevocationStoreError::Unavailable(error.to_string()))?;
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(AuthRevocationStoreError::from)?;
        Ok(Self::new(connection))
    }

    pub fn multiplexed_connection(&self) -> redis::aio::MultiplexedConnection {
        self.connection.clone()
    }
}

#[axum::async_trait]
impl AuthRevocationStore for RedisAuthRevocationStore {
    async fn jti_is_blacklisted(&self, jti: &str) -> Result<bool, AuthRevocationStoreError> {
        let mut connection = self.connection.clone();
        let key = blacklist_key(jti);
        connection
            .exists(key)
            .await
            .map_err(AuthRevocationStoreError::from)
    }

    async fn permissions_changed_at(
        &self,
        user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        let mut connection = self.connection.clone();
        let key = permissions_changed_at_key(user_id);
        connection
            .get(key)
            .await
            .map_err(AuthRevocationStoreError::from)
    }

    async fn blacklist_jti(
        &self,
        jti: &str,
        ttl_seconds: u64,
    ) -> Result<(), AuthRevocationStoreError> {
        let mut connection = self.connection.clone();
        let key = blacklist_key(jti);
        let _: () = connection
            .set_ex(key, "1", ttl_seconds)
            .await
            .map_err(AuthRevocationStoreError::from)?;
        Ok(())
    }

    async fn set_permissions_changed_at(
        &self,
        user_id: Uuid,
        changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        let mut connection = self.connection.clone();
        let key = permissions_changed_at_key(user_id);
        let _: () = connection
            .set_ex(
                key,
                changed_at_unix,
                REDIS_PERMISSIONS_CHANGED_AT_TTL_SECONDS,
            )
            .await
            .map_err(AuthRevocationStoreError::from)?;
        Ok(())
    }
}

pub fn blacklist_key(jti: &str) -> String {
    format!("{AUTH_BLACKLIST_KEY_PREFIX}{jti}")
}

pub fn permissions_changed_at_key(user_id: Uuid) -> String {
    format!("{AUTH_PERMISSIONS_CHANGED_AT_KEY_PREFIX}{user_id}:permissions_changed_at")
}

pub fn auth_runtime_layer(policy: AuthRuntimePolicy) -> Extension<AuthRuntimePolicy> {
    Extension(policy)
}

impl AuthContext {
    pub fn from_claims(claims: Claims) -> Self {
        Self {
            user_id: claims.sub,
            owner_id: claims.owner_id,
            actor_name: claims.user_name,
            permissions: claims.permissions,
            jti: claims.jti,
            warehouse_scope: None,
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions
            .iter()
            .any(|candidate| candidate == permission)
    }

    pub fn require_permission(&self, permission: &str) -> Result<(), AuthError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(AuthError::PermissionDenied(permission.to_string()))
        }
    }

    pub fn require_owner(&self, owner_id: Uuid) -> Result<(), AuthError> {
        if self.owner_id == owner_id {
            Ok(())
        } else {
            Err(AuthError::CrossOwnerAccess)
        }
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(context) = parts.extensions.get::<AuthContext>().cloned() {
            return Ok(context);
        }
        let token = bearer_token(parts)?;
        let secret = std::env::var(JWT_SECRET_ENV).map_err(|_| AuthError::MissingSecret)?;
        let token_data = decode_claims(token, &secret)?;
        let policy = parts
            .extensions
            .get::<AuthRuntimePolicy>()
            .cloned()
            .ok_or(AuthError::MissingRuntimePolicy)?;
        policy.validate_claims(&token_data.claims).await?;
        Ok(AuthContext::from_claims(token_data.claims))
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for LogoutContext
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts)?;
        let secret = std::env::var(JWT_SECRET_ENV).map_err(|_| AuthError::MissingSecret)?;
        let claims = decode_claims_for_logout(token, &secret)?.claims;
        let expires_at = claims.exp;
        Ok(Self {
            auth: AuthContext::from_claims(claims),
            expires_at,
        })
    }
}

fn bearer_token(parts: &Parts) -> Result<&str, AuthError> {
    let value = parts
        .headers
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingAuthorization)?
        .to_str()
        .map_err(|_| AuthError::InvalidAuthorization)?;
    value
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or(AuthError::InvalidAuthorization)
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AuthError::MissingAuthorization => (
                StatusCode::UNAUTHORIZED,
                "AUTH-001",
                "缺少 Authorization 头",
            ),
            AuthError::InvalidAuthorization => (
                StatusCode::UNAUTHORIZED,
                "AUTH-002",
                "Authorization 格式错误",
            ),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "AUTH-003", "token 无效或已过期"),
            AuthError::TokenRevoked => (StatusCode::UNAUTHORIZED, "AUTH-004", "token 已撤销"),
            AuthError::MissingSecret => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "AUTH-SECRET-MISSING",
                "JWT 密钥未配置",
            ),
            AuthError::PermissionDenied(_) => (StatusCode::FORBIDDEN, "AUTH-005", "权限不足"),
            AuthError::PermissionsRevoked => (
                StatusCode::UNAUTHORIZED,
                "AUTH-009",
                "permissions 已失效，请重新登录",
            ),
            AuthError::CrossOwnerAccess => (StatusCode::FORBIDDEN, "AUTH-006", "跨货主越权"),
            AuthError::MissingRuntimePolicy => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "AUTH-RUNTIME-POLICY-MISSING",
                "鉴权运行策略未注入",
            ),
            AuthError::RevocationStoreUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "AUTH-REVOCATION-STORE-UNAVAILABLE",
                "权限失效检查暂不可用",
            ),
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

pub fn build_access_claims(
    user_id: Uuid,
    owner_id: Uuid,
    user_name: impl Into<String>,
    permissions: Vec<String>,
    jti: impl Into<String>,
    issued_at: DateTime<Utc>,
) -> Claims {
    Claims {
        sub: user_id,
        owner_id,
        user_name: user_name.into(),
        permissions,
        jti: jti.into(),
        iat: issued_at.timestamp(),
        exp: (issued_at + Duration::seconds(ACCESS_TOKEN_TTL_SECONDS)).timestamp(),
    }
}

pub fn encode_access_token(claims: &Claims, secret: &str) -> Result<String, AuthError> {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AuthError::InvalidToken)
}

pub fn decode_auth_context(token: &str, secret: &str) -> Result<AuthContext, AuthError> {
    let token_data = decode_claims(token, secret)?;
    Ok(AuthContext::from_claims(token_data.claims))
}

pub async fn decode_auth_context_with_policy(
    token: &str,
    secret: &str,
    policy: &AuthRuntimePolicy,
) -> Result<AuthContext, AuthError> {
    let token_data = decode_claims(token, secret)?;
    policy.validate_claims(&token_data.claims).await?;
    Ok(AuthContext::from_claims(token_data.claims))
}

fn decode_claims(token: &str, secret: &str) -> Result<TokenData<Claims>, AuthError> {
    let validation = Validation::new(Algorithm::HS256);
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AuthError::InvalidToken)
}

fn decode_claims_for_logout(token: &str, secret: &str) -> Result<TokenData<Claims>, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AuthError::InvalidToken)
}

#[cfg(test)]
mod tests {
    use super::{
        auth_runtime_layer, build_access_claims, decode_auth_context,
        decode_auth_context_with_policy, decode_claims_for_logout, encode_access_token,
        AuthContext, AuthError, AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy,
        ACCESS_TOKEN_TTL_SECONDS, JWT_SECRET_ENV,
    };
    use axum::{
        extract::FromRequestParts,
        http::{header::AUTHORIZATION, Request},
        routing::get,
        Router,
    };
    use chrono::{Duration, TimeZone, Utc};
    use std::{
        collections::{HashMap, HashSet},
        sync::{Arc, Mutex},
    };
    use uuid::Uuid;

    #[derive(Default)]
    struct InMemoryAuthRevocationStore {
        blacklisted_jtis: Mutex<HashSet<String>>,
        permissions_changed_at: Mutex<HashMap<Uuid, i64>>,
    }

    #[axum::async_trait]
    impl AuthRevocationStore for InMemoryAuthRevocationStore {
        async fn jti_is_blacklisted(&self, jti: &str) -> Result<bool, AuthRevocationStoreError> {
            let blacklisted_jtis = self.blacklisted_jtis.lock().expect("mutex should lock");
            Ok(blacklisted_jtis.contains(jti))
        }

        async fn permissions_changed_at(
            &self,
            user_id: Uuid,
        ) -> Result<Option<i64>, AuthRevocationStoreError> {
            let permissions_changed_at = self
                .permissions_changed_at
                .lock()
                .expect("mutex should lock");
            Ok(permissions_changed_at.get(&user_id).copied())
        }

        async fn blacklist_jti(
            &self,
            jti: &str,
            _ttl_seconds: u64,
        ) -> Result<(), AuthRevocationStoreError> {
            let mut blacklisted_jtis = self.blacklisted_jtis.lock().expect("mutex should lock");
            blacklisted_jtis.insert(jti.to_string());
            Ok(())
        }

        async fn set_permissions_changed_at(
            &self,
            user_id: Uuid,
            changed_at_unix: i64,
        ) -> Result<(), AuthRevocationStoreError> {
            let mut permissions_changed_at = self
                .permissions_changed_at
                .lock()
                .expect("mutex should lock");
            permissions_changed_at.insert(user_id, changed_at_unix);
            Ok(())
        }
    }

    struct UnavailableAuthRevocationStore;

    #[axum::async_trait]
    impl AuthRevocationStore for UnavailableAuthRevocationStore {
        async fn jti_is_blacklisted(&self, _jti: &str) -> Result<bool, AuthRevocationStoreError> {
            Err(AuthRevocationStoreError::Unavailable(
                "redis unavailable".to_string(),
            ))
        }

        async fn permissions_changed_at(
            &self,
            _user_id: Uuid,
        ) -> Result<Option<i64>, AuthRevocationStoreError> {
            Err(AuthRevocationStoreError::Unavailable(
                "redis unavailable".to_string(),
            ))
        }

        async fn blacklist_jti(
            &self,
            _jti: &str,
            _ttl_seconds: u64,
        ) -> Result<(), AuthRevocationStoreError> {
            Err(AuthRevocationStoreError::Unavailable(
                "redis unavailable".to_string(),
            ))
        }

        async fn set_permissions_changed_at(
            &self,
            _user_id: Uuid,
            _changed_at_unix: i64,
        ) -> Result<(), AuthRevocationStoreError> {
            Err(AuthRevocationStoreError::Unavailable(
                "redis unavailable".to_string(),
            ))
        }
    }

    async fn demo_items_handler(_ctx: AuthContext) -> &'static str {
        "ok"
    }

    #[test]
    fn auth_context_extractor_is_demo_items_handler_compatible() {
        let policy = AuthRuntimePolicy::new(Arc::new(InMemoryAuthRevocationStore::default()));
        let _router: Router = Router::new()
            .route("/api/v1/demo/items", get(demo_items_handler))
            .layer(auth_runtime_layer(policy));
    }

    #[test]
    fn access_token_uses_adr_0024_one_hour_ttl() {
        let issued_at = Utc
            .with_ymd_and_hms(2026, 6, 2, 8, 0, 0)
            .single()
            .expect("valid test timestamp");
        let claims = build_access_claims(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "alice",
            vec!["audit:read".to_string()],
            "jti-1",
            issued_at,
        );

        assert_eq!(claims.exp - claims.iat, ACCESS_TOKEN_TTL_SECONDS);
    }

    #[test]
    fn jwt_claims_decode_to_owner_scoped_auth_context() {
        let user_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let issued_at = Utc::now();
        let claims = build_access_claims(
            user_id,
            owner_id,
            "alice",
            vec!["audit:read".to_string()],
            "jti-1",
            issued_at,
        );
        let token = encode_access_token(&claims, "test-secret").expect("token should encode");

        let ctx = decode_auth_context(&token, "test-secret").expect("token should decode");

        assert_eq!(ctx.user_id, user_id);
        assert_eq!(ctx.owner_id, owner_id);
        assert_eq!(ctx.actor_name, "alice");
        assert!(ctx.has_permission("audit:read"));
        assert_eq!(ctx.require_owner(owner_id), Ok(()));
    }

    #[test]
    fn logout_decoder_accepts_expired_signed_token_for_idempotence() {
        let claims = build_access_claims(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "alice",
            Vec::new(),
            "expired-jti",
            Utc::now() - Duration::hours(2),
        );
        let token = encode_access_token(&claims, "test-secret").expect("token should encode");

        assert!(decode_auth_context(&token, "test-secret").is_err());
        assert_eq!(
            decode_claims_for_logout(&token, "test-secret")
                .expect("logout should decode expired signed token")
                .claims
                .jti,
            claims.jti
        );
    }

    #[test]
    fn auth_context_rejects_cross_owner_access() {
        let ctx = AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "alice".to_string(),
            permissions: vec!["inventory:read".to_string()],
            jti: "jti-1".to_string(),
            warehouse_scope: None,
        };

        assert!(ctx.require_owner(Uuid::new_v4()).is_err());
        assert!(ctx.require_permission("inventory:write").is_err());
    }

    #[tokio::test]
    async fn auth_runtime_rejects_blacklisted_jti_as_auth_004() {
        let issued_at = Utc::now();
        let claims = build_access_claims(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "alice",
            vec!["audit:read".to_string()],
            "jti-revoked",
            issued_at,
        );
        let token = encode_access_token(&claims, "test-secret").expect("token should encode");
        let store = Arc::new(InMemoryAuthRevocationStore::default());
        store
            .blacklist_jti(&claims.jti, ACCESS_TOKEN_TTL_SECONDS as u64)
            .await
            .expect("test store should write blacklist");
        let policy = AuthRuntimePolicy::new(store);

        let err = decode_auth_context_with_policy(&token, "test-secret", &policy)
            .await
            .expect_err("blacklisted jti should be rejected");

        assert_eq!(err, AuthError::TokenRevoked);
    }

    #[tokio::test]
    async fn auth_runtime_rejects_permissions_changed_after_iat_as_auth_009() {
        let user_id = Uuid::new_v4();
        let issued_at = Utc::now();
        let claims = build_access_claims(
            user_id,
            Uuid::new_v4(),
            "alice",
            vec!["audit:read".to_string()],
            "jti-1",
            issued_at,
        );
        let token = encode_access_token(&claims, "test-secret").expect("token should encode");
        let store = Arc::new(InMemoryAuthRevocationStore::default());
        store
            .set_permissions_changed_at(user_id, claims.iat + 1)
            .await
            .expect("test store should write permissions_changed_at");
        let policy = AuthRuntimePolicy::new(store);

        let err = decode_auth_context_with_policy(&token, "test-secret", &policy)
            .await
            .expect_err("stale permissions should be rejected");

        assert_eq!(err, AuthError::PermissionsRevoked);
    }

    #[tokio::test]
    async fn auth_runtime_degrades_open_when_redis_revocation_store_is_unavailable() {
        let issued_at = Utc::now();
        let claims = build_access_claims(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "alice",
            vec!["audit:read".to_string()],
            "jti-1",
            issued_at,
        );
        let token = encode_access_token(&claims, "test-secret").expect("token should encode");
        let policy = AuthRuntimePolicy::new(Arc::new(UnavailableAuthRevocationStore));

        let ctx = decode_auth_context_with_policy(&token, "test-secret", &policy)
            .await
            .expect("ADR-0024 fail-open degradation should accept valid JWT");

        assert_eq!(ctx.user_id, claims.sub);
    }

    #[tokio::test]
    async fn auth_context_extractor_requires_auth_runtime_policy() {
        std::env::set_var(JWT_SECRET_ENV, "test-secret");
        let claims = build_access_claims(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "alice",
            vec!["audit:read".to_string()],
            "jti-1",
            Utc::now(),
        );
        let token = encode_access_token(&claims, "test-secret").expect("token should encode");
        let (mut parts, _) = Request::builder()
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(())
            .expect("request should build")
            .into_parts();

        let err = AuthContext::from_request_parts(&mut parts, &())
            .await
            .expect_err("extractor must not skip missing runtime policy");

        assert_eq!(err, AuthError::MissingRuntimePolicy);
    }

    #[tokio::test]
    async fn auth_context_extractor_uses_auth_runtime_policy_extension() {
        std::env::set_var(JWT_SECRET_ENV, "test-secret");
        let claims = build_access_claims(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "alice",
            vec!["audit:read".to_string()],
            "jti-revoked",
            Utc::now(),
        );
        let token = encode_access_token(&claims, "test-secret").expect("token should encode");
        let store = Arc::new(InMemoryAuthRevocationStore::default());
        store
            .blacklist_jti(&claims.jti, ACCESS_TOKEN_TTL_SECONDS as u64)
            .await
            .expect("test store should write blacklist");
        let policy = AuthRuntimePolicy::new(store);
        let (mut parts, _) = Request::builder()
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(())
            .expect("request should build")
            .into_parts();
        parts.extensions.insert(policy);

        let err = AuthContext::from_request_parts(&mut parts, &())
            .await
            .expect_err("extractor should reject blacklisted jti via extension policy");

        assert_eq!(err, AuthError::TokenRevoked);
    }
}
