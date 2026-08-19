use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::{free_form_json_schema, PageMeta};

pub const SYSTEM_DICTIONARY_DOCUMENT_TYPE: &str = "document_type";
pub const DOCUMENT_TYPE_PURCHASE_INBOUND: &str = "purchase_inbound";
pub const DOCUMENT_TYPE_SALES_RETURN: &str = "sales_return";
pub const DOCUMENT_TYPE_PURCHASE_RETURN_OUTBOUND: &str = "purchase_return_outbound";
pub const DOCUMENT_TYPE_SALES_OUTBOUND: &str = "sales_outbound";
pub const SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE: &str = "print_template_type";
pub const PRINT_TEMPLATE_TYPE_ASN: &str = "asn";
pub const PRINT_TEMPLATE_TYPE_ACCEPTANCE_RECORD: &str = "acceptance_record";
pub const PRINT_TEMPLATE_TYPE_DELIVERY_NOTE: &str = "delivery_note";
pub const PRINT_TEMPLATE_TYPE_LOCATION_LABEL: &str = "location_label";
pub const PRINT_TEMPLATE_TYPE_LPN_LABEL: &str = "lpn_label";
pub const PRINT_TEMPLATE_TYPE_PRODUCT_LABEL: &str = "product_label";
pub const SYSTEM_DICTIONARY_CONTAINER_QUARANTINE_REASON: &str = "container_quarantine_reason";
pub const CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY: &str = "temp_anomaly";
pub const CONTAINER_QUARANTINE_REASON_DAMAGED_PENDING_INSPECT: &str = "damaged_pending_inspect";
pub const CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING: &str = "sales_return_pending";
pub const CONTAINER_QUARANTINE_REASON_ROUTINE_SAMPLING: &str = "routine_sampling";
pub const SYSTEM_DICTIONARY_CONTAINER_REJECTED_REASON: &str = "container_rejected_reason";
pub const CONTAINER_REJECTED_REASON_EXPIRED: &str = "expired";
pub const CONTAINER_REJECTED_REASON_DAMAGED_LEAKAGE: &str = "damaged_leakage";
pub const CONTAINER_REJECTED_REASON_INSPECTION_FAILED: &str = "inspection_failed";
pub const CONTAINER_REJECTED_REASON_REGULATORY_RECALL: &str = "regulatory_recall";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DocumentNumberAllocation {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub rule_id: Uuid,
    pub document_type: String,
    pub generated_no: String,
    pub sequence_value: i64,
    pub counter_key: String,
    pub source_module: String,
    pub source_document_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DocumentNumberAllocationListResponse {
    pub data: Vec<DocumentNumberAllocation>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateMachineState {
    pub code: String,
    pub label: String,
    pub is_initial: bool,
    pub is_terminal: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateMachineTransition {
    pub from_state: String,
    pub to_state: String,
    pub event_code: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateMachineDefinition {
    pub machine_code: String,
    pub machine_name: String,
    pub business_module: String,
    pub version: String,
    pub states: Vec<StateMachineState>,
    pub transitions: Vec<StateMachineTransition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateMachineDefinitionListResponse {
    pub data: Vec<StateMachineDefinition>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StateTransitionValidationResponse {
    pub machine_code: String,
    pub from_state: String,
    pub to_state: String,
    pub event_code: Option<String>,
    pub allowed: bool,
    pub reason: Option<String>,
}

/// 系统字典分类。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryCategory {
    pub dict_code: String,
    pub dict_name: String,
    pub enabled: bool,
    pub control_level: String,
    #[schema(schema_with = free_form_json_schema)]
    pub param_schema: serde_json::Value,
    pub scope_mode: String,
    #[schema(schema_with = free_form_json_schema)]
    pub override_policy: serde_json::Value,
    pub sort_order: i32,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 系统字典项。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryItem {
    pub id: Uuid,
    pub dict_code: String,
    pub item_code: String,
    pub item_name: String,
    pub enabled: bool,
    pub owner_id: Option<Uuid>,
    pub sort_order: i32,
    #[schema(schema_with = free_form_json_schema)]
    pub params: serde_json::Value,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub source: String,
    pub disabled_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryItemListResponse {
    pub data: Vec<SystemDictionaryItem>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryImpactReference {
    pub module_code: String,
    pub business_object: String,
    pub reference_count: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SystemDictionaryImpactPreview {
    pub dict_code: String,
    pub item_code: String,
    pub owner_id: Uuid,
    pub effective_at: DateTime<Utc>,
    pub total_references: i64,
    pub references: Vec<SystemDictionaryImpactReference>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertSystemDictionaryItemRequest {
    pub owner_id: Option<Uuid>,
    pub item_name: String,
    pub enabled: bool,
    pub sort_order: i32,
    #[schema(schema_with = free_form_json_schema)]
    pub params: serde_json::Value,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DisableSystemDictionaryItemRequest {
    pub owner_id: Option<Uuid>,
    pub disabled_reason: Option<String>,
}
