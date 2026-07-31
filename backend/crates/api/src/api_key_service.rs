use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    ApiKey, ApiKeyListResponse, ApiKeyRotationResponse, CreateApiKeyRequest, RotateApiKeyRequest,
    API_KEY_SCOPES,
};

use crate::{
    api_key_repository::{
        ApiKeyAuthPolicy, ApiKeyContext, ApiKeyListQuery, ApiKeyRepository, ApiKeyRepositoryError,
    },
    operation_context::OperationContext as AuthContext,
};

pub const API_KEY_MANAGE_PERMISSION: &str = "h1.api_keys.manage";
pub const API_KEY_DEFAULT_EXPIRY_DAYS: i64 = 180;
pub const API_KEY_DEFAULT_GRACE_DAYS: i64 = 7;

#[derive(Clone)]
pub struct ApiKeyService {
    repository: ApiKeyRepository,
}

impl ApiKeyService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: ApiKeyRepository::new(pool),
        }
    }

    pub async fn list(
        &self,
        ctx: &AuthContext,
        keyword: Option<String>,
        status: Option<String>,
    ) -> Result<ApiKeyListResponse, ApiKeyServiceError> {
        self.repository
            .list(
                ctx.owner_id,
                &ApiKeyListQuery {
                    keyword: clean(keyword),
                    status: clean(status),
                },
            )
            .await
            .map_err(Into::into)
    }

    pub async fn create(
        &self,
        ctx: &AuthContext,
        request: CreateApiKeyRequest,
        idempotency_key: &str,
    ) -> Result<ApiKey, ApiKeyServiceError> {
        let now = Utc::now();
        let (expires_at, scopes) = validate_create(&request, now)?;
        let key_id = Uuid::new_v4();
        let secret = key_material(key_id);
        self.repository
            .create(
                ctx,
                &request,
                now,
                expires_at,
                &scopes,
                idempotency_key,
                &request_hash(&request)?,
                key_id,
                &secret,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn rotate(
        &self,
        ctx: &AuthContext,
        key_id: Uuid,
        request: RotateApiKeyRequest,
        idempotency_key: &str,
    ) -> Result<ApiKeyRotationResponse, ApiKeyServiceError> {
        let now = Utc::now();
        let grace_days = request
            .grace_period_days
            .unwrap_or(API_KEY_DEFAULT_GRACE_DAYS);
        if grace_days < 0 {
            return Err(ApiKeyServiceError::InvalidGracePeriod);
        }
        let expires_at = request
            .expires_at
            .unwrap_or_else(|| now + Duration::days(API_KEY_DEFAULT_EXPIRY_DAYS));
        if expires_at <= now {
            return Err(ApiKeyServiceError::InvalidExpiry);
        }
        let new_key_id = Uuid::new_v4();
        let secret = key_material(new_key_id);
        let hash = request_hash(&(key_id, &request))?;
        self.repository
            .rotate(
                ctx,
                key_id,
                &request,
                now,
                expires_at,
                idempotency_key,
                &hash,
                new_key_id,
                &secret,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn revoke(
        &self,
        ctx: &AuthContext,
        key_id: Uuid,
        idempotency_key: &str,
    ) -> Result<ApiKey, ApiKeyServiceError> {
        let hash = request_hash(&(key_id, "revoke"))?;
        self.repository
            .revoke(ctx, key_id, Utc::now(), idempotency_key, &hash)
            .await
            .map_err(Into::into)
    }

    pub async fn authenticate(
        &self,
        raw_key: &str,
        owner_id: Uuid,
        scope: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<ApiKeyContext, ApiKeyAuthError> {
        self.repository
            .authenticate(
                raw_key,
                owner_id,
                scope,
                Utc::now(),
                ip,
                user_agent,
                ApiKeyAuthPolicy::default(),
            )
            .await
            .map_err(Into::into)
    }

    pub async fn authenticate_any_owner(
        &self,
        raw_key: &str,
        scope: &str,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<ApiKeyContext, ApiKeyAuthError> {
        self.repository
            .authenticate_any_owner(
                raw_key,
                scope,
                Utc::now(),
                ip,
                user_agent,
                ApiKeyAuthPolicy::default(),
            )
            .await
            .map_err(Into::into)
    }

    pub async fn record_request_audit(
        &self,
        context: &ApiKeyContext,
        method: &str,
        path: &str,
        status_code: u16,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), ApiKeyServiceError> {
        self.repository
            .append_request_audit(
                context,
                method,
                path,
                status_code,
                ip,
                user_agent,
                Utc::now(),
            )
            .await
            .map_err(Into::into)
    }
}

fn validate_create(
    request: &CreateApiKeyRequest,
    now: DateTime<Utc>,
) -> Result<(DateTime<Utc>, Vec<String>), ApiKeyServiceError> {
    if request.caller_name.trim().is_empty()
        || request.purpose.trim().is_empty()
        || request.responsible_user_id.is_nil()
    {
        return Err(ApiKeyServiceError::InvalidRequest);
    }
    let mut scopes = request
        .scopes
        .iter()
        .map(|scope| scope.trim().to_string())
        .collect::<Vec<_>>();
    scopes.sort();
    scopes.dedup();
    if scopes.is_empty()
        || scopes
            .iter()
            .any(|scope| !API_KEY_SCOPES.contains(&scope.as_str()))
    {
        return Err(ApiKeyServiceError::InvalidScope);
    }
    let expires_at = request
        .expires_at
        .unwrap_or_else(|| now + Duration::days(API_KEY_DEFAULT_EXPIRY_DAYS));
    if expires_at <= now {
        return Err(ApiKeyServiceError::InvalidExpiry);
    }
    Ok((expires_at, scopes))
}

fn key_material(key_id: Uuid) -> String {
    format!("wms_{}_{}", key_id.simple(), Uuid::new_v4().simple())
}

fn request_hash<T: Serialize>(value: &T) -> Result<String, ApiKeyServiceError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ApiKeyServiceError::Serialize(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiKeyAuthError {
    Invalid,
    Expired,
    TemporarilyDisabled,
    RateLimited,
    InvalidScope,
    CrossOwner,
    Database,
}

#[derive(Debug)]
pub enum ApiKeyServiceError {
    InvalidRequest,
    InvalidScope,
    InvalidExpiry,
    InvalidGracePeriod,
    Serialize(String),
    Repository(ApiKeyRepositoryError),
}

impl From<ApiKeyRepositoryError> for ApiKeyServiceError {
    fn from(value: ApiKeyRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl From<ApiKeyRepositoryError> for ApiKeyAuthError {
    fn from(value: ApiKeyRepositoryError) -> Self {
        match value {
            ApiKeyRepositoryError::Expired => Self::Expired,
            ApiKeyRepositoryError::TemporarilyDisabled => Self::TemporarilyDisabled,
            ApiKeyRepositoryError::RateLimited => Self::RateLimited,
            ApiKeyRepositoryError::InvalidScope => Self::InvalidScope,
            ApiKeyRepositoryError::CrossOwner => Self::CrossOwner,
            ApiKeyRepositoryError::Database(_) | ApiKeyRepositoryError::Audit(_) => Self::Database,
            _ => Self::Invalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_create, API_KEY_DEFAULT_EXPIRY_DAYS};
    use chrono::{Duration, Utc};
    use uuid::Uuid;
    use wms_domain::CreateApiKeyRequest;

    fn request() -> CreateApiKeyRequest {
        CreateApiKeyRequest {
            caller_name: "ERP".into(),
            purpose: "推送".into(),
            warehouse_ids: vec![],
            scopes: vec!["inbound:push".into()],
            expires_at: None,
            responsible_user_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn create_defaults_to_story_expiry_and_deduplicates_scopes() {
        let now = Utc::now();
        let mut request = request();
        request.scopes.push("inbound:push".into());
        let (expires_at, scopes) = validate_create(&request, now).expect("request should validate");
        assert_eq!(
            expires_at.date_naive(),
            (now + Duration::days(API_KEY_DEFAULT_EXPIRY_DAYS)).date_naive()
        );
        assert_eq!(scopes, vec!["inbound:push"]);
    }

    #[test]
    fn create_accepts_h8_outbound_and_return_scopes() {
        let now = Utc::now();
        let mut request = request();
        request.scopes = vec![
            "outbound:push".into(),
            "outbound:receipt".into(),
            "return:push".into(),
        ];
        let (_, scopes) = validate_create(&request, now).expect("H8 scopes should validate");
        assert_eq!(
            scopes,
            vec!["outbound:push", "outbound:receipt", "return:push"]
        );
    }
}
