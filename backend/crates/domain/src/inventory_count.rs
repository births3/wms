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
    /// 超阈值差异需要更高审批源（如「盘点-高级」）。
    ElevatedApprovalRequired,
}

/// 默认阈值：单行 |差异| / 账面 > 10%（账面为 0 且有差异视为超阈值）。
pub const INVENTORY_COUNT_VARIANCE_RATIO_BPS: i64 = 1000; // 10% = 1000 基点

pub fn line_exceeds_variance_threshold(book_qty: i64, variance_qty: i64) -> bool {
    let abs_var = variance_qty.abs();
    if abs_var == 0 {
        return false;
    }
    if book_qty <= 0 {
        return true;
    }
    abs_var.saturating_mul(10_000) > book_qty.saturating_mul(INVENTORY_COUNT_VARIANCE_RATIO_BPS)
}

pub fn count_requires_elevated_approval<'a, I>(lines: I) -> bool
where
    I: IntoIterator<Item = (i64, i64)>,
{
    lines
        .into_iter()
        .any(|(book_qty, variance_qty)| line_exceeds_variance_threshold(book_qty, variance_qty))
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

/// 普通盘点审批源为「盘点」；超阈值必须「盘点-高级」。
pub fn validate_approval_for_variance(
    request: &ApproveInventoryCountRequest,
    requires_elevated: bool,
) -> Result<(), InventoryCountValidationError> {
    validate_approval(request)?;
    let source = request.approval_source.trim();
    if requires_elevated {
        if source != "盘点-高级" {
            return Err(InventoryCountValidationError::ElevatedApprovalRequired);
        }
    } else if source != "盘点" && source != "盘点-高级" {
        return Err(InventoryCountValidationError::MissingApprovalSource);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variance_threshold_detects_over_ten_percent() {
        assert!(!line_exceeds_variance_threshold(100, 10));
        assert!(line_exceeds_variance_threshold(100, 11));
        assert!(line_exceeds_variance_threshold(0, 1));
        assert!(!line_exceeds_variance_threshold(0, 0));
    }

    #[test]
    fn elevated_approval_source_required_when_threshold_hit() {
        let req = ApproveInventoryCountRequest {
            approval_source: "盘点".to_string(),
            approval_id: "c1".to_string(),
        };
        assert_eq!(
            validate_approval_for_variance(&req, true),
            Err(InventoryCountValidationError::ElevatedApprovalRequired)
        );
        let elevated = ApproveInventoryCountRequest {
            approval_source: "盘点-高级".to_string(),
            approval_id: "c1".to_string(),
        };
        assert!(validate_approval_for_variance(&elevated, true).is_ok());
    }
}
