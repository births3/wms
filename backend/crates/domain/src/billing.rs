use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingAccount {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateBillingAccountRequest {
    pub account_code: String,
    pub account_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingContract {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub account_id: Uuid,
    pub contract_no: String,
    pub valid_from: String,
    pub valid_to: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateBillingContractRequest {
    pub account_id: Uuid,
    pub contract_no: String,
    pub valid_from: String,
    pub valid_to: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingRule {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub contract_id: Uuid,
    pub charge_item: String,
    pub unit: String,
    pub unit_price_cents: i64,
    pub billing_cycle: String,
    pub effective_from: String,
    pub effective_to: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateBillingRuleRequest {
    pub contract_id: Uuid,
    pub charge_item: String,
    pub unit: String,
    pub unit_price_cents: i64,
    pub billing_cycle: String,
    pub effective_from: String,
    pub effective_to: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingChargeCalculation {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub contract_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub charge_item: String,
    pub quantity: i64,
    pub amount_cents: i64,
    pub source_refs: Vec<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CalculateBillingChargesRequest {
    pub contract_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub charge_item: String,
    pub quantity: i64,
    pub source_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BillingStatement {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub contract_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub status: String,
    pub total_amount_cents: i64,
    pub charge_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GenerateBillingStatementRequest {
    pub contract_id: Uuid,
    pub period_start: String,
    pub period_end: String,
    pub charge_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ConfirmBillingStatementRequest {
    pub confirmation_note: Option<String>,
}
