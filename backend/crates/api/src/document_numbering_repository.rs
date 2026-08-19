//! US-CG-001 / US-CG-002 no-gap document numbering service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::PageMeta;

mod persistence;
mod service;
mod support;

pub const DEFAULT_DOCUMENT_NUMBER_ALLOCATION_LIMIT: u32 = 50;
pub const MAX_DOCUMENT_NUMBER_ALLOCATION_LIMIT: u32 = 100;

#[derive(Clone, Debug)]
pub struct PgDocumentNumberingService;

#[derive(Clone, Debug)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentNumberingError {
    DocumentTypeInvalid,
    RuleNotFound,
    InvalidRule,
    InvalidEffectiveWindow,
    TemplateInvalid,
    SequenceOverflow,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl From<crate::idempotency::IdempotencyError> for DocumentNumberingError {
    fn from(error: crate::idempotency::IdempotencyError) -> Self {
        match error {
            crate::idempotency::IdempotencyError::Conflict => Self::IdempotencyConflict,
            crate::idempotency::IdempotencyError::Database(error) => {
                Self::Database(error.to_string())
            }
            crate::idempotency::IdempotencyError::Serialize(error) => Self::Serialize(error),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpsertDocumentNumberRuleRequest {
    pub document_type: String,
    pub rule_name: String,
    pub template: String,
    pub reset_policy: String,
    pub sequence_width: i32,
    pub enabled: bool,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct SetDocumentNumberRuleEnabledRequest {
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DocumentNumberRule {
    pub id: Uuid,
    pub owner_id: Option<Uuid>,
    pub document_type: String,
    pub rule_code: String,
    pub rule_name: String,
    pub template: String,
    pub reset_policy: String,
    pub sequence_width: i32,
    pub sequence_mode: String,
    pub enabled: bool,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DocumentNumberRuleListResponse {
    pub data: Vec<DocumentNumberRule>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GenerateDocumentNumberRequest {
    pub document_type: String,
    pub idempotency_key: String,
    pub source_module: String,
    pub source_document_id: Option<Uuid>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentNumberAllocationQuery {
    pub document_type: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: u32,
}

#[derive(Clone, Debug, FromRow)]
struct RuleRow {
    id: Uuid,
    document_type: String,
    template: String,
    reset_policy: String,
    sequence_width: i32,
}

#[derive(Clone, Debug, FromRow)]
struct DocumentNumberRuleRow {
    id: Uuid,
    owner_id: Option<Uuid>,
    document_type: String,
    rule_code: String,
    rule_name: String,
    template: String,
    reset_policy: String,
    sequence_width: i32,
    sequence_mode: String,
    enabled: bool,
    effective_from: Option<DateTime<Utc>>,
    effective_to: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

#[derive(Clone, Debug, FromRow)]
struct AllocationRow {
    id: Uuid,
    owner_id: Uuid,
    rule_id: Uuid,
    document_type: String,
    generated_no: String,
    sequence_value: i64,
    counter_key: String,
    source_module: String,
    source_document_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
struct AllocationWithHashRow {
    id: Uuid,
    owner_id: Uuid,
    rule_id: Uuid,
    document_type: String,
    generated_no: String,
    sequence_value: i64,
    counter_key: String,
    source_module: String,
    source_document_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    request_hash: String,
}
