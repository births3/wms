//! Phase 2 补货领域不变量（US-M3-012 / ADR-0048）。无 IO。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;
use crate::quantity::Quantity;

pub const LOCATION_TYPE_STORAGE: &str = "storage";
pub const LOCATION_TYPE_CASE_PICK: &str = "case_pick";
pub const LOCATION_TYPE_PIECE_PICK: &str = "piece_pick";

pub const REPLENISH_STATUS_PENDING: &str = "pending";
pub const REPLENISH_STATUS_IN_PROGRESS: &str = "in_progress";
pub const REPLENISH_STATUS_SUSPENDED: &str = "suspended";
pub const REPLENISH_STATUS_DONE: &str = "done";
pub const REPLENISH_STATUS_CANCELLED: &str = "cancelled";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplenishRouteError {
    InvalidCombination {
        source_type: String,
        target_type: String,
    },
}

/// 补货动线：仅 storage→case_pick / storage→piece_pick / case_pick→piece_pick。
pub fn validate_replenish_route(
    source_type: &str,
    target_type: &str,
) -> Result<(), ReplenishRouteError> {
    let legal = matches!(
        (source_type, target_type),
        (LOCATION_TYPE_STORAGE, LOCATION_TYPE_CASE_PICK)
            | (LOCATION_TYPE_STORAGE, LOCATION_TYPE_PIECE_PICK)
            | (LOCATION_TYPE_CASE_PICK, LOCATION_TYPE_PIECE_PICK)
    );
    if legal {
        Ok(())
    } else {
        Err(ReplenishRouteError::InvalidCombination {
            source_type: source_type.to_string(),
            target_type: target_type.to_string(),
        })
    }
}

pub struct AvailableQtyInput {
    pub qty_on_hand: Quantity,
    pub qty_allocated: Quantity,
    pub qty_frozen: Quantity,
    pub qty_replenish_in_transit: Quantity,
    pub qty_replenish_out_transit: Quantity,
}

/// 拣选位某商品可用量 = on_hand − allocated − frozen + in_transit
pub fn pick_available_qty(input: &AvailableQtyInput) -> Quantity {
    input.qty_on_hand - input.qty_allocated - input.qty_frozen + input.qty_replenish_in_transit
}

/// 来源批次可下架量 = on_hand − allocated − frozen − out_transit
pub fn source_available_qty(input: &AvailableQtyInput) -> Quantity {
    input.qty_on_hand - input.qty_allocated - input.qty_frozen - input.qty_replenish_out_transit
}

/// 任务量 = floor(min(目标量, 来源可下架) / pack) * pack；不足 1 包装为 0。
pub fn task_qty(target_need: Quantity, source_available: Quantity, pack_ratio: i64) -> Quantity {
    let raw = if target_need < source_available {
        target_need
    } else {
        source_available
    };
    if raw <= Quantity::ZERO {
        return Quantity::ZERO;
    }
    let ratio = if pack_ratio > 0 {
        Quantity::from(pack_ratio)
    } else {
        Quantity::from(1)
    };
    if raw < ratio {
        return Quantity::ZERO;
    }
    (raw / ratio).trunc() * ratio
}

pub fn can_cancel(picked_qty: Quantity, done_qty: Quantity) -> bool {
    picked_qty == Quantity::ZERO && done_qty == Quantity::ZERO
}

pub fn can_pick(status: &str) -> bool {
    status == REPLENISH_STATUS_IN_PROGRESS
}

pub fn can_confirm(status: &str, picked_qty: Quantity) -> bool {
    if picked_qty <= Quantity::ZERO {
        return false;
    }
    status == REPLENISH_STATUS_IN_PROGRESS || status == REPLENISH_STATUS_SUSPENDED
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertReplenishmentStrategyRequest {
    pub strategy_code: String,
    pub strategy_name: String,
    pub scope_type: String,
    pub scope_ref: Uuid,
    pub source_type: String,
    pub target_type: String,
    #[schema(value_type = String, format = "decimal")]
    pub min_safety_threshold: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub max_replenish_target: Quantity,
    pub trigger_modes: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplenishmentStrategy {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub strategy_code: String,
    pub strategy_name: String,
    pub scope_type: String,
    pub scope_ref: Uuid,
    pub location_type: String,
    pub source_type: String,
    pub target_type: String,
    #[schema(value_type = String, format = "decimal")]
    pub min_safety_threshold: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub max_replenish_target: Quantity,
    pub trigger_modes: Vec<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplenishmentStrategyListResponse {
    pub data: Vec<ReplenishmentStrategy>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BindReplenishmentLocationsRequest {
    pub location_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BindReplenishmentLocationsResponse {
    pub strategy_id: Uuid,
    pub location_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplenishmentPreviewItem {
    pub location_id: Uuid,
    pub location_code: String,
    pub product_id: Option<Uuid>,
    #[schema(value_type = String, format = "decimal")]
    pub available_qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub min_safety_threshold: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub max_replenish_target: Quantity,
    pub would_trigger: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplenishmentPreviewResponse {
    pub data: Vec<ReplenishmentPreviewItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertReplenishmentLocationGroupRequest {
    pub group_code: String,
    pub group_name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub location_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplenishmentLocationGroup {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub group_code: String,
    pub group_name: String,
    pub enabled: bool,
    pub location_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplenishmentLocationGroupListResponse {
    pub data: Vec<ReplenishmentLocationGroup>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateReplenishmentTaskRequest {
    pub source_location_id: Uuid,
    pub source_batch_id: Uuid,
    pub target_location_id: Uuid,
    #[schema(value_type = String, format = "decimal")]
    pub qty: Quantity,
    #[serde(default)]
    pub source_lpn_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReplenishmentTask {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub task_no: String,
    pub trigger_mode: String,
    pub priority: String,
    pub strategy_id: Option<Uuid>,
    pub source_location_id: Uuid,
    pub source_batch_id: Uuid,
    pub source_lpn_id: Option<Uuid>,
    pub target_location_id: Uuid,
    pub product_id: Uuid,
    pub batch_no: String,
    #[schema(value_type = String, format = "decimal")]
    pub qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub picked_qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub done_qty: Quantity,
    pub status: String,
    pub operator_id: Option<Uuid>,
    pub created_by: String,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ClaimReplenishmentTaskRequest {
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PickReplenishmentTaskRequest {
    pub version: i64,
    pub scanned_location_code: String,
    #[serde(default)]
    pub scanned_lpn_code: Option<String>,
    #[schema(value_type = String, format = "decimal")]
    pub qty: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ConfirmReplenishmentTaskRequest {
    pub version: i64,
    pub scanned_location_code: String,
    #[schema(value_type = String, format = "decimal")]
    pub qty: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CancelReplenishmentTaskRequest {
    pub version: i64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReassignReplenishmentTaskRequest {
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReturnReplenishmentTaskRequest {
    pub version: i64,
    pub return_reason: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(n: i64) -> Quantity {
        Quantity::from(n)
    }

    #[test]
    fn piece_pick_to_storage_is_invalid() {
        let err = validate_replenish_route(LOCATION_TYPE_PIECE_PICK, LOCATION_TYPE_STORAGE)
            .expect_err("piece_pick→storage must be rejected");
        assert_eq!(
            err,
            ReplenishRouteError::InvalidCombination {
                source_type: LOCATION_TYPE_PIECE_PICK.to_string(),
                target_type: LOCATION_TYPE_STORAGE.to_string(),
            }
        );
    }

    #[test]
    fn legal_routes_pass() {
        assert!(validate_replenish_route(LOCATION_TYPE_STORAGE, LOCATION_TYPE_CASE_PICK).is_ok());
        assert!(validate_replenish_route(LOCATION_TYPE_STORAGE, LOCATION_TYPE_PIECE_PICK).is_ok());
        assert!(
            validate_replenish_route(LOCATION_TYPE_CASE_PICK, LOCATION_TYPE_PIECE_PICK).is_ok()
        );
    }

    #[test]
    fn pick_available_qty_includes_in_transit() {
        let qty = pick_available_qty(&AvailableQtyInput {
            qty_on_hand: q(10),
            qty_allocated: q(3),
            qty_frozen: q(1),
            qty_replenish_in_transit: q(4),
            qty_replenish_out_transit: q(99),
        });
        assert_eq!(qty, q(10));
    }

    #[test]
    fn source_available_qty_subtracts_out_transit() {
        let qty = source_available_qty(&AvailableQtyInput {
            qty_on_hand: q(30),
            qty_allocated: q(5),
            qty_frozen: q(2),
            qty_replenish_in_transit: q(99),
            qty_replenish_out_transit: q(8),
        });
        assert_eq!(qty, q(15));
    }

    #[test]
    fn task_qty_floors_to_pack_and_drops_short_remainder() {
        assert_eq!(task_qty(q(18), q(30), 1), q(18));
        assert_eq!(task_qty(q(5), q(30), 12), q(0));
        assert_eq!(task_qty(q(25), q(20), 6), q(18));
    }

    #[test]
    fn cancel_blocked_when_picked_qty_positive() {
        assert!(!can_cancel(q(4), q(0)));
        assert!(can_cancel(q(0), q(0)));
        assert!(!can_cancel(q(0), q(1)));
    }

    #[test]
    fn pick_only_in_progress() {
        assert!(can_pick(REPLENISH_STATUS_IN_PROGRESS));
        assert!(!can_pick(REPLENISH_STATUS_PENDING));
        assert!(!can_pick(REPLENISH_STATUS_SUSPENDED));
    }

    #[test]
    fn confirm_requires_picked_qty_and_allows_suspended_with_pick() {
        assert!(!can_confirm(REPLENISH_STATUS_IN_PROGRESS, q(0)));
        assert!(can_confirm(REPLENISH_STATUS_IN_PROGRESS, q(4)));
        assert!(can_confirm(REPLENISH_STATUS_SUSPENDED, q(4)));
        assert!(!can_confirm(REPLENISH_STATUS_SUSPENDED, q(0)));
        assert!(!can_confirm(REPLENISH_STATUS_PENDING, q(4)));
    }
}
