use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::free_form_json_schema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertQualityLiaisonTypeRequest {
    pub type_code: String,
    pub type_name: String,
    pub approval_template_id: String,
    pub approver_user_id: Uuid,
    pub timeout_seconds: i32,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct QualityLiaisonTypeConfig {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub type_code: String,
    pub type_name: String,
    pub approval_template_id: String,
    pub approver_user_id: Uuid,
    pub timeout_seconds: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateQualityLiaisonRequest {
    pub type_code: String,
    pub related_document_type: String,
    pub related_document_no: String,
    pub problem_description: String,
    pub disposition_suggestion: String,
    pub trigger_source: String,
    #[schema(schema_with = free_form_json_schema)]
    pub business_payload: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct QualityLiaisonApprovalCallbackRequest {
    pub conclusion: String,
    pub opinion: String,
    pub external_approval_id: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct QualityLiaisonOrder {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub liaison_no: String,
    pub type_code: String,
    pub related_document_type: String,
    pub related_document_no: String,
    pub problem_description: String,
    pub disposition_suggestion: String,
    pub trigger_source: String,
    #[schema(schema_with = free_form_json_schema)]
    pub business_payload: serde_json::Value,
    pub status: String,
    pub approval_record_id: Option<Uuid>,
    pub approved_by: Option<Uuid>,
    pub approval_opinion: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}
