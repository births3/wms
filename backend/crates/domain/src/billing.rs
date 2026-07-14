use chrono::{DateTime, NaiveDate, Utc};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BillingRuleValidationError {
    InvalidChargeItem,
    InvalidUnit,
    InvalidBillingCycle,
    InvalidRate,
    InvalidEffectiveWindow,
}

pub fn validate_billing_rule_request(
    request: &CreateBillingRuleRequest,
) -> Result<(), BillingRuleValidationError> {
    if !matches!(
        request.charge_item.as_str(),
        "storage"
            | "inbound_operation"
            | "outbound_operation"
            | "consumable"
            | "handling"
            | "loading_unloading"
            | "packing_operation"
    ) {
        return Err(BillingRuleValidationError::InvalidChargeItem);
    }
    if !matches!(
        request.unit.as_str(),
        "square_meter_day"
            | "square_meter"
            | "pallet_day"
            | "pallet"
            | "pallet_position"
            | "order"
            | "line"
            | "piece"
            | "box"
            | "job"
            | "hour"
    ) {
        return Err(BillingRuleValidationError::InvalidUnit);
    }
    if !matches!(
        request.billing_cycle.as_str(),
        "daily" | "weekly" | "monthly" | "quarterly" | "one_off"
    ) {
        return Err(BillingRuleValidationError::InvalidBillingCycle);
    }
    if request.unit_price_cents < 0 {
        return Err(BillingRuleValidationError::InvalidRate);
    }
    let effective_from = NaiveDate::parse_from_str(&request.effective_from, "%Y-%m-%d")
        .map_err(|_| BillingRuleValidationError::InvalidEffectiveWindow)?;
    let effective_to = NaiveDate::parse_from_str(&request.effective_to, "%Y-%m-%d")
        .map_err(|_| BillingRuleValidationError::InvalidEffectiveWindow)?;
    if effective_to < effective_from {
        return Err(BillingRuleValidationError::InvalidEffectiveWindow);
    }
    Ok(())
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
