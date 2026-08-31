use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::container_quality_lock::LPN_LOCK_CATEGORY_QUALIFIED;

pub const LPN_CONTAINER_TYPE_PALLET: &str = "pallet";
pub const LPN_CONTAINER_TYPE_TOTE: &str = "tote";
pub const LPN_CONTAINER_TYPE_OUTBOUND_BOX: &str = "outbound_box";
pub const LPN_CONTAINER_TYPE_INSULATED_BOX: &str = "insulated_box";
pub const LPN_CONTAINER_TYPE_BLIND_LABEL: &str = "blind_label";

pub const LPN_CONTAINER_STATUS_IDLE: &str = "idle";
pub const LPN_CONTAINER_STATUS_IN_USE: &str = "in_use";
pub const LPN_CONTAINER_STATUS_IN_TRANSIT: &str = "in_transit";
pub const LPN_CONTAINER_STATUS_RECYCLING: &str = "recycling";
pub const LPN_CONTAINER_STATUS_SHIPPED: &str = "shipped";
pub const LPN_CONTAINER_STATUS_DISABLED: &str = "disabled";

const VALID_TYPES: &[&str] = &[
    LPN_CONTAINER_TYPE_PALLET,
    LPN_CONTAINER_TYPE_TOTE,
    LPN_CONTAINER_TYPE_OUTBOUND_BOX,
    LPN_CONTAINER_TYPE_INSULATED_BOX,
    LPN_CONTAINER_TYPE_BLIND_LABEL,
];

const VALID_STATUSES: &[&str] = &[
    LPN_CONTAINER_STATUS_IDLE,
    LPN_CONTAINER_STATUS_IN_USE,
    LPN_CONTAINER_STATUS_IN_TRANSIT,
    LPN_CONTAINER_STATUS_RECYCLING,
    LPN_CONTAINER_STATUS_SHIPPED,
    LPN_CONTAINER_STATUS_DISABLED,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LpnContainerValidationError {
    CodeEmpty,
    CodeTooLong,
    TypeInvalid,
    StatusInvalid,
    BatchCountInvalid,
}

pub const LPN_CODE_MAX_LEN: usize = 64;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LpnContainer {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub lpn_code: String,
    pub container_type: String,
    pub capacity_cm3: Option<i64>,
    pub status: String,
    pub location_id: Option<Uuid>,
    pub current_lock_category: Option<String>,
    pub current_lock_reason_item_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LpnContainerListResponse {
    pub data: Vec<LpnContainer>,
}

/// 周转箱状态预检轻量响应。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ToteStatusResponse {
    pub tote_code: String,
    /// AVAILABLE 可用 | IN_USE 被占用 | SEALED 已封箱
    pub status: String,
    pub current_order_id: Option<Uuid>,
    pub loaded_sku_count: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UnlockSkippedBatch {
    pub batch_id: Uuid,
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReleaseContainerQualityLockResponse {
    pub container: LpnContainer,
    pub skipped_batches: Vec<UnlockSkippedBatch>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateLpnContainerRequest {
    pub container_type: String,
    pub capacity_cm3: Option<i64>,
}

pub const LPN_BATCH_CREATE_MAX_COUNT: i32 = 100;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchCreateLpnContainerRequest {
    pub container_type: String,
    pub capacity_cm3: Option<i64>,
    pub count: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LpnContainerTypePolicy {
    pub owner_id: Uuid,
    pub container_type: String,
    pub allow_mix_batch: bool,
    pub allow_mix_sku: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertLpnContainerTypePolicyRequest {
    pub container_type: String,
    pub allow_mix_batch: bool,
    pub allow_mix_sku: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateLpnContainerRequest {
    pub status: Option<String>,
    pub location_id: Option<Uuid>,
    pub capacity_cm3: Option<i64>,
}

pub fn is_valid_lpn_container_type(value: &str) -> bool {
    VALID_TYPES.contains(&value)
}

pub fn is_valid_lpn_container_status(value: &str) -> bool {
    VALID_STATUSES.contains(&value)
}

pub fn lpn_status_allows_putaway(status: &str) -> bool {
    matches!(
        status,
        LPN_CONTAINER_STATUS_IDLE | LPN_CONTAINER_STATUS_IN_USE
    )
}

pub fn lpn_status_allows_soft_delete(status: &str) -> bool {
    matches!(
        status,
        LPN_CONTAINER_STATUS_IDLE | LPN_CONTAINER_STATUS_DISABLED
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LpnPutawayBindDecision {
    Allow,
    NotUsable,
    LocationConflict,
}

pub fn decide_lpn_putaway_bind(
    status: &str,
    current_location_id: Option<Uuid>,
    target_location_id: Uuid,
) -> LpnPutawayBindDecision {
    if !lpn_status_allows_putaway(status) {
        return LpnPutawayBindDecision::NotUsable;
    }
    if status == LPN_CONTAINER_STATUS_IN_USE {
        if let Some(current) = current_location_id {
            if current != target_location_id {
                return LpnPutawayBindDecision::LocationConflict;
            }
        }
    }
    LpnPutawayBindDecision::Allow
}

pub fn lpn_numbering_document_type(container_type: &str) -> String {
    format!("lpn_{}", container_type.trim())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LpnMixDenied {
    Sku,
    Batch,
}

impl CreateLpnContainerRequest {
    pub fn validate(&self) -> Result<(), LpnContainerValidationError> {
        if !is_valid_lpn_container_type(self.container_type.trim()) {
            return Err(LpnContainerValidationError::TypeInvalid);
        }
        Ok(())
    }

    pub fn into_new_container(
        self,
        id: Uuid,
        owner_id: Uuid,
        lpn_code: String,
        now: DateTime<Utc>,
    ) -> Result<LpnContainer, LpnContainerValidationError> {
        self.validate()?;
        let code = lpn_code.trim();
        if code.is_empty() {
            return Err(LpnContainerValidationError::CodeEmpty);
        }
        if code.len() > LPN_CODE_MAX_LEN {
            return Err(LpnContainerValidationError::CodeTooLong);
        }
        Ok(LpnContainer {
            id,
            owner_id,
            lpn_code: code.to_string(),
            container_type: self.container_type.trim().to_string(),
            capacity_cm3: self.capacity_cm3,
            status: LPN_CONTAINER_STATUS_IDLE.to_string(),
            location_id: None,
            current_lock_category: Some(LPN_LOCK_CATEGORY_QUALIFIED.to_string()),
            current_lock_reason_item_code: None,
            created_at: now,
            updated_at: now,
        })
    }
}

impl BatchCreateLpnContainerRequest {
    pub fn validate(&self) -> Result<(), LpnContainerValidationError> {
        if !is_valid_lpn_container_type(self.container_type.trim()) {
            return Err(LpnContainerValidationError::TypeInvalid);
        }
        if self.count < 1 || self.count > LPN_BATCH_CREATE_MAX_COUNT {
            return Err(LpnContainerValidationError::BatchCountInvalid);
        }
        Ok(())
    }

    pub fn item_request(&self) -> CreateLpnContainerRequest {
        CreateLpnContainerRequest {
            container_type: self.container_type.trim().to_string(),
            capacity_cm3: self.capacity_cm3,
        }
    }
}

pub fn decide_lpn_mix(
    allow_mix_sku: bool,
    allow_mix_batch: bool,
    existing: &[(String, String)],
    incoming_product: &str,
    incoming_batch: &str,
) -> Result<(), LpnMixDenied> {
    if existing.is_empty() {
        return Ok(());
    }
    if !allow_mix_sku
        && existing
            .iter()
            .any(|(product, _)| product != incoming_product)
    {
        return Err(LpnMixDenied::Sku);
    }
    if !allow_mix_batch && existing.iter().any(|(_, batch)| batch != incoming_batch) {
        return Err(LpnMixDenied::Batch);
    }
    Ok(())
}

pub fn lpn_inventory_identity_allows(
    existing_container_lpn: Option<&str>,
    incoming_lpn: Option<&str>,
) -> bool {
    existing_container_lpn == incoming_lpn
}

impl UpdateLpnContainerRequest {
    pub fn validate(&self) -> Result<(), LpnContainerValidationError> {
        if let Some(status) = self.status.as_deref() {
            if !is_valid_lpn_container_status(status.trim()) {
                return Err(LpnContainerValidationError::StatusInvalid);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request(container_type: &str) -> CreateLpnContainerRequest {
        CreateLpnContainerRequest {
            container_type: container_type.to_string(),
            capacity_cm3: Some(1_000),
        }
    }

    #[test]
    fn invalid_container_type_fails() {
        assert_eq!(
            sample_request("nest").validate(),
            Err(LpnContainerValidationError::TypeInvalid)
        );
    }

    #[test]
    fn batch_create_rejects_invalid_type_or_count() {
        let valid = BatchCreateLpnContainerRequest {
            container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
            capacity_cm3: Some(1_000),
            count: 3,
        };
        assert_eq!(valid.validate(), Ok(()));
        assert_eq!(
            BatchCreateLpnContainerRequest {
                container_type: "nest".to_string(),
                capacity_cm3: None,
                count: 3,
            }
            .validate(),
            Err(LpnContainerValidationError::TypeInvalid)
        );
        assert_eq!(
            BatchCreateLpnContainerRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                capacity_cm3: None,
                count: 0,
            }
            .validate(),
            Err(LpnContainerValidationError::BatchCountInvalid)
        );
        assert_eq!(
            BatchCreateLpnContainerRequest {
                container_type: LPN_CONTAINER_TYPE_PALLET.to_string(),
                capacity_cm3: None,
                count: LPN_BATCH_CREATE_MAX_COUNT + 1,
            }
            .validate(),
            Err(LpnContainerValidationError::BatchCountInvalid)
        );
    }

    #[test]
    fn valid_create_defaults_idle_and_trims_generated_code() {
        let now = Utc::now();
        let id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let container = sample_request(LPN_CONTAINER_TYPE_PALLET)
            .into_new_container(id, owner_id, "  LPN-OK  ".to_string(), now)
            .expect("valid create");
        assert_eq!(container.lpn_code, "LPN-OK");
        assert_eq!(container.status, LPN_CONTAINER_STATUS_IDLE);
        assert_eq!(
            lpn_numbering_document_type(LPN_CONTAINER_TYPE_PALLET),
            "lpn_pallet"
        );
    }

    #[test]
    fn generated_code_over_64_fails() {
        assert_eq!(
            sample_request(LPN_CONTAINER_TYPE_PALLET)
                .into_new_container(Uuid::new_v4(), Uuid::new_v4(), "A".repeat(65), Utc::now())
                .err(),
            Some(LpnContainerValidationError::CodeTooLong)
        );
    }

    #[test]
    fn mix_policy_default_denies_other_sku_or_batch() {
        let existing = vec![("P1".to_string(), "B1".to_string())];
        assert_eq!(
            decide_lpn_mix(false, false, &existing, "P2", "B1"),
            Err(LpnMixDenied::Sku)
        );
        assert_eq!(
            decide_lpn_mix(false, false, &existing, "P1", "B2"),
            Err(LpnMixDenied::Batch)
        );
        assert_eq!(decide_lpn_mix(false, false, &existing, "P1", "B1"), Ok(()));
        assert_eq!(decide_lpn_mix(true, true, &existing, "P2", "B2"), Ok(()));
        assert_eq!(
            decide_lpn_mix(true, false, &existing, "P2", "B2"),
            Err(LpnMixDenied::Batch)
        );
        assert_eq!(decide_lpn_mix(true, false, &existing, "P2", "B1"), Ok(()));
        assert_eq!(decide_lpn_mix(false, true, &existing, "P1", "B2"), Ok(()));
    }

    #[test]
    fn inventory_identity_rejects_other_lpn_or_loose_merge() {
        assert!(lpn_inventory_identity_allows(Some("A"), Some("A")));
        assert!(lpn_inventory_identity_allows(None, None));
        assert!(!lpn_inventory_identity_allows(Some("A"), Some("B")));
        assert!(!lpn_inventory_identity_allows(Some("A"), None));
        assert!(!lpn_inventory_identity_allows(None, Some("A")));
    }

    #[test]
    fn lpn_status_allows_putaway_only_idle_and_in_use() {
        assert!(lpn_status_allows_putaway(LPN_CONTAINER_STATUS_IDLE));
        assert!(lpn_status_allows_putaway(LPN_CONTAINER_STATUS_IN_USE));
        assert!(!lpn_status_allows_putaway(LPN_CONTAINER_STATUS_SHIPPED));
        assert!(!lpn_status_allows_putaway(LPN_CONTAINER_STATUS_IN_TRANSIT));
        assert!(!lpn_status_allows_putaway(LPN_CONTAINER_STATUS_RECYCLING));
        assert!(!lpn_status_allows_putaway(LPN_CONTAINER_STATUS_DISABLED));
    }

    #[test]
    fn only_idle_or_already_disabled_allows_soft_delete() {
        assert!(lpn_status_allows_soft_delete(LPN_CONTAINER_STATUS_IDLE));
        assert!(lpn_status_allows_soft_delete(LPN_CONTAINER_STATUS_DISABLED));
        assert!(!lpn_status_allows_soft_delete(LPN_CONTAINER_STATUS_IN_USE));
        assert!(!lpn_status_allows_soft_delete(
            LPN_CONTAINER_STATUS_IN_TRANSIT
        ));
        assert!(!lpn_status_allows_soft_delete(LPN_CONTAINER_STATUS_SHIPPED));
        assert!(!lpn_status_allows_soft_delete(
            LPN_CONTAINER_STATUS_RECYCLING
        ));
    }

    #[test]
    fn in_use_lpn_rejects_cross_location_putaway() {
        let loc_a = Uuid::new_v4();
        let loc_b = Uuid::new_v4();
        assert_eq!(
            decide_lpn_putaway_bind(LPN_CONTAINER_STATUS_IN_USE, Some(loc_a), loc_a),
            LpnPutawayBindDecision::Allow
        );
        assert_eq!(
            decide_lpn_putaway_bind(LPN_CONTAINER_STATUS_IN_USE, None, loc_a),
            LpnPutawayBindDecision::Allow
        );
        assert_eq!(
            decide_lpn_putaway_bind(LPN_CONTAINER_STATUS_IN_USE, Some(loc_a), loc_b),
            LpnPutawayBindDecision::LocationConflict
        );
        assert_eq!(
            decide_lpn_putaway_bind(LPN_CONTAINER_STATUS_SHIPPED, None, loc_a),
            LpnPutawayBindDecision::NotUsable
        );
        assert_eq!(
            decide_lpn_putaway_bind(LPN_CONTAINER_STATUS_IDLE, Some(loc_a), loc_b),
            LpnPutawayBindDecision::Allow
        );
    }
}
