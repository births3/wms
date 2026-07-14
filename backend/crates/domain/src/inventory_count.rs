use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub const INVENTORY_COUNT_TYPE_CYCLE: &str = "cycle";
pub const INVENTORY_COUNT_TYPE_FULL: &str = "full";
pub const INVENTORY_COUNT_TYPE_BLIND: &str = "blind";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateInventoryCountRequest {
    pub count_type: String,
    pub warehouse_id: Option<Uuid>,
    pub zone_id: Option<Uuid>,
    pub product_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SubmitInventoryCountLineRequest {
    pub physical_qty: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApproveInventoryCountRequest {
    pub approval_source: String,
    pub approval_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryCountLine {
    pub id: Uuid,
    pub count_id: Uuid,
    pub owner_id: Uuid,
    pub inventory_batch_id: Uuid,
    pub location_id: Uuid,
    pub location_code: String,
    pub product_code: String,
    pub batch_no: String,
    pub book_qty: i64,
    pub physical_qty: Option<i64>,
    pub variance_qty: Option<i64>,
    pub variance_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryCount {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub count_type: String,
    pub warehouse_id: Option<Uuid>,
    pub zone_id: Option<Uuid>,
    pub product_code: Option<String>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approval_source: Option<String>,
    pub approval_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lines: Vec<InventoryCountLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryCountValidationError {
    InvalidCountType,
    InvalidPhysicalQuantity,
    MissingApprovalSource,
    MissingApprovalId,
}

pub fn validate_count_type(value: &str) -> Result<(), InventoryCountValidationError> {
    if matches!(
        value,
        INVENTORY_COUNT_TYPE_CYCLE | INVENTORY_COUNT_TYPE_FULL | INVENTORY_COUNT_TYPE_BLIND
    ) {
        Ok(())
    } else {
        Err(InventoryCountValidationError::InvalidCountType)
    }
}

pub fn validate_physical_quantity(value: i64) -> Result<(), InventoryCountValidationError> {
    if value >= 0 {
        Ok(())
    } else {
        Err(InventoryCountValidationError::InvalidPhysicalQuantity)
    }
}

pub fn validate_approval(
    request: &ApproveInventoryCountRequest,
) -> Result<(), InventoryCountValidationError> {
    if request.approval_source.trim().is_empty() {
        return Err(InventoryCountValidationError::MissingApprovalSource);
    }
    if request.approval_id.trim().is_empty() {
        return Err(InventoryCountValidationError::MissingApprovalId);
    }
    Ok(())
}

pub fn calculate_variance(book_qty: i64, physical_qty: i64) -> (i64, &'static str) {
    let variance = physical_qty - book_qty;
    let kind = match variance.cmp(&0) {
        std::cmp::Ordering::Greater => "gain",
        std::cmp::Ordering::Less => "loss",
        std::cmp::Ordering::Equal => "none",
    };
    (variance, kind)
}
