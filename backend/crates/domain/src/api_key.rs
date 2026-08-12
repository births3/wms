use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

pub const H8_WORKER_API_KEY_SCOPE: &str = "h8:worker";

pub const API_KEY_SCOPES: [&str; 9] = [
    "master-data:write",
    "inbound:push",
    "outbound:push",
    "outbound:receipt",
    "return:push",
    "inventory:seed",
    "order:command",
    "tms:callback",
    H8_WORKER_API_KEY_SCOPE,
];

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApiKey {
    pub key_id: Uuid,
    pub owner_id: Uuid,
    pub caller_name: String,
    pub purpose: String,
    pub warehouse_ids: Vec<Uuid>,
    pub scopes: Vec<String>,
    pub responsible_user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub status: String,
    pub grace_expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub temporarily_disabled_until: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub secret: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApiKeyListResponse {
    pub data: Vec<ApiKey>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub caller_name: String,
    pub purpose: String,
    pub warehouse_ids: Vec<Uuid>,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub responsible_user_id: Uuid,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct RotateApiKeyRequest {
    pub grace_period_days: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApiKeyRotationResponse {
    pub previous_key_id: Uuid,
    pub previous_grace_expires_at: DateTime<Utc>,
    pub new_key: ApiKey,
}
