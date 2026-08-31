use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{common::PageMeta, Quantity};

pub const INVENTORY_COUNT_TYPE_CYCLE: &str = "cycle";
pub const INVENTORY_COUNT_TYPE_FULL: &str = "full";
pub const INVENTORY_COUNT_TYPE_BLIND: &str = "blind";
pub const INVENTORY_COUNT_TYPE_SPOT: &str = "spot";

pub const VARIANCE_TYPE_MATCH: &str = "MATCH";
pub const VARIANCE_TYPE_SURPLUS: &str = "SURPLUS";
pub const VARIANCE_TYPE_SHORTAGE: &str = "SHORTAGE";

pub const COUNT_STATUS_APPROVED: &str = "approved";
pub const COUNT_STATUS_PENDING_APPROVAL: &str = "pending_approval";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateInventoryCountRequest {
    pub count_type: String,
    pub warehouse_id: Option<Uuid>,
    pub zone_id: Option<Uuid>,
    pub product_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SubmitInventoryCountLineRequest {
    #[schema(value_type = String, format = "decimal")]
    pub physical_qty: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApproveInventoryCountRequest {
    pub approval_source: String,
    pub approval_id: String,
}

/// 现场快速动盘请求。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct QuickSpotCountRequest {
    pub location_code: String,
    pub product_code: String,
    pub batch_no: String,
    #[schema(value_type = String, format = "decimal")]
    pub physical_qty: Quantity,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub operated_at: Option<DateTime<Utc>>,
}

/// 现场快速动盘响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct QuickSpotCountResponse {
    pub count_id: Uuid,
    #[schema(value_type = String, format = "decimal")]
    pub book_qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub physical_qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub variance_qty: Quantity,
    /// 差异类型：MATCH 账实相符 | SHORTAGE 盘亏 | SURPLUS 盘盈
    pub variance_type: String,
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
    #[schema(value_type = String, format = "decimal")]
    pub book_qty: Quantity,
    #[schema(value_type = String, format = "decimal", nullable = true)]
    pub physical_qty: Option<Quantity>,
    #[schema(value_type = String, format = "decimal", nullable = true)]
    pub variance_qty: Option<Quantity>,
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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InventoryCountListResponse {
    pub data: Vec<InventoryCount>,
    pub page: PageMeta,
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

pub fn line_exceeds_variance_threshold(book_qty: Quantity, variance_qty: Quantity) -> bool {
    let abs_var = variance_qty.abs();
    if abs_var.is_zero() {
        return false;
    }
    if book_qty <= Quantity::ZERO {
        return true;
    }
    abs_var * Quantity::from(10_000) > book_qty * Quantity::from(INVENTORY_COUNT_VARIANCE_RATIO_BPS)
}

pub fn count_requires_elevated_approval<I>(lines: I) -> bool
where
    I: IntoIterator<Item = (Quantity, Quantity)>,
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

pub fn validate_physical_quantity(value: Quantity) -> Result<(), InventoryCountValidationError> {
    if value >= Quantity::ZERO {
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

pub fn calculate_variance(book_qty: Quantity, physical_qty: Quantity) -> (Quantity, &'static str) {
    let variance = physical_qty - book_qty;
    let kind = match variance.cmp(&Quantity::ZERO) {
        std::cmp::Ordering::Greater => "gain",
        std::cmp::Ordering::Less => "loss",
        std::cmp::Ordering::Equal => "none",
    };
    (variance, kind)
}

pub fn variance_kind_to_classification(kind: &str) -> &'static str {
    match kind {
        "gain" | VARIANCE_TYPE_SURPLUS => VARIANCE_TYPE_SURPLUS,
        "loss" | VARIANCE_TYPE_SHORTAGE => VARIANCE_TYPE_SHORTAGE,
        _ => VARIANCE_TYPE_MATCH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_variance_computes_correct_variance_and_kind() {
        assert_eq!(calculate_variance(10.into(), 15.into()), (5.into(), "gain"));
        assert_eq!(
            calculate_variance(15.into(), 10.into()),
            ((-5).into(), "loss")
        );
        assert_eq!(calculate_variance(10.into(), 10.into()), (0.into(), "none"));
    }

    #[test]
    fn variance_kind_to_classification_maps_properly() {
        assert_eq!(
            variance_kind_to_classification("gain"),
            VARIANCE_TYPE_SURPLUS
        );
        assert_eq!(
            variance_kind_to_classification("loss"),
            VARIANCE_TYPE_SHORTAGE
        );
        assert_eq!(variance_kind_to_classification("none"), VARIANCE_TYPE_MATCH);
    }

    #[test]
    fn variance_threshold_detects_over_ten_percent() {
        assert!(!line_exceeds_variance_threshold(100.into(), 10.into()));
        assert!(line_exceeds_variance_threshold(100.into(), 11.into()));
        assert!(line_exceeds_variance_threshold(0.into(), 1.into()));
        assert!(!line_exceeds_variance_threshold(0.into(), 0.into()));
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
