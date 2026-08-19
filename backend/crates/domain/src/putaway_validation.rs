use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::quantity::Quantity;

pub const M2_PUTAWAY_ZONE_CATEGORY_DENIED: &str = "M2_PUTAWAY_ZONE_CATEGORY_DENIED";
pub const M2_PUTAWAY_TEMPERATURE_MISMATCH: &str = "M2_PUTAWAY_TEMPERATURE_MISMATCH";
pub const M2_PUTAWAY_QUALITY_LOCKED: &str = "M2_PUTAWAY_QUALITY_LOCKED";
pub const M2_PUTAWAY_SPECIAL_DUAL_REQUIRED: &str = "M2_PUTAWAY_SPECIAL_DUAL_REQUIRED";
pub const M2_PUTAWAY_PACK_GRANULARITY_INVALID: &str = "M2_PUTAWAY_PACK_GRANULARITY_INVALID";
pub const M2_PUTAWAY_EXTERNAL_FRAGRANT_CONFLICT: &str = "M2_PUTAWAY_EXTERNAL_FRAGRANT_CONFLICT";
pub const M2_PUTAWAY_CAPACITY_EXCEEDED: &str = "M2_PUTAWAY_CAPACITY_EXCEEDED";

pub const PUTAWAY_DIMENSION_CATEGORY_ZONE: &str = "category_zone";
pub const PUTAWAY_DIMENSION_TEMPERATURE_ZONE: &str = "temperature_zone";
pub const PUTAWAY_DIMENSION_QUALITY_LOCK: &str = "quality_lock";
pub const PUTAWAY_DIMENSION_SPECIAL_DUAL: &str = "special_dual";
pub const PUTAWAY_DIMENSION_PACK_GRANULARITY: &str = "pack_granularity";
pub const PUTAWAY_DIMENSION_EXTERNAL_FRAGRANT: &str = "external_fragrant_conflict";
pub const PUTAWAY_DIMENSION_CAPACITY: &str = "capacity";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayLocationValidationRequest {
    pub target_location_id: Uuid,
    #[serde(default)]
    pub target_location_code: Option<String>,
    #[serde(default)]
    pub product_id: Option<Uuid>,
    #[serde(default)]
    pub product_code: Option<String>,
    #[serde(default)]
    pub container_code: Option<String>,
    #[serde(default)]
    pub is_container: Option<bool>,
    #[serde(default)]
    pub batch_status: Option<String>,
    #[serde(default)]
    pub witness_id: Option<Uuid>,
    #[serde(default)]
    #[schema(value_type = String, format = "decimal")]
    pub qty: Option<Quantity>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayLocationValidationResponse {
    pub valid: bool,
    pub message: String,
    #[serde(default)]
    pub location_id: Option<Uuid>,
    #[serde(default)]
    pub location_code: Option<String>,
    #[serde(default)]
    pub zone_code: Option<String>,
    #[serde(default)]
    pub temperature_zone: Option<String>,
    #[serde(default)]
    pub quality_color: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PutawayRejectionLog {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub operated_by: Uuid,
    pub container_code: Option<String>,
    pub product_id: Option<Uuid>,
    pub target_location_id: Uuid,
    pub rejection_dimension: String,
    pub error_code: String,
    pub reason: String,
    pub occurred_at: DateTime<Utc>,
}

pub fn parse_temperature_range(temp_zone: &str) -> Option<(f64, f64)> {
    match temp_zone {
        "normal_10_30" | "normal" => Some((10.0, 30.0)),
        "cool_le_20" | "cool" => Some((10.0, 20.0)),
        "cold_2_8" | "cold" => Some((2.0, 8.0)),
        "freeze_le_minus_20" | "frozen" => Some((-f64::INFINITY, -20.0)),
        "ultra_cold_minus_80" => Some((-f64::INFINITY, -80.0)),
        _ => None,
    }
}

pub fn is_temperature_zone_subset(zone_temp: &str, product_temp: &str) -> bool {
    let (z_min, z_max) = match parse_temperature_range(zone_temp) {
        Some(range) => range,
        None => return false,
    };
    let (p_min, p_max) = match parse_temperature_range(product_temp) {
        Some(range) => range,
        None => return false,
    };
    z_min >= p_min && z_max <= p_max
}

pub fn validate_category_zone(
    allowed_categories: &serde_json::Value,
    product_category: &str,
) -> bool {
    let arr = match allowed_categories.as_array() {
        Some(a) => a,
        None => return true,
    };
    if arr.is_empty() {
        return true;
    }
    arr.iter()
        .any(|v| v.as_str().map_or(false, |c| c == product_category))
}

pub fn zone_treats_as_qualified(zone_quality_color: &str) -> bool {
    matches!(
        zone_quality_color.trim(),
        "" | "qualified_green" | "qualified"
    )
}

pub fn validate_quality_match(zone_quality_color: &str, lock_or_batch_status: &str) -> bool {
    match lock_or_batch_status {
        "qualified" => zone_treats_as_qualified(zone_quality_color),
        "quarantine" | "quarantined" => zone_quality_color == "quarantine_yellow",
        "rejected" | "unqualified" => zone_quality_color == "unqualified_red",
        _ => zone_treats_as_qualified(zone_quality_color),
    }
}

pub fn validate_special_drug_dual(
    is_special: bool,
    operator_id: Uuid,
    witness_id: Option<Uuid>,
) -> bool {
    if !is_special {
        return true;
    }
    match witness_id {
        Some(witness) => witness != operator_id,
        None => false,
    }
}

pub fn lock_category_is_active(lock_category: Option<&str>) -> bool {
    matches!(lock_category, Some("quarantine") | Some("rejected"))
}

pub fn validate_pack_granularity(
    location_type: &str,
    allows_container: bool,
    is_container: bool,
    lock_category: Option<&str>,
) -> bool {
    if location_type == "storage" || allows_container {
        return is_container;
    }
    // 箱拣/零拣：已加锁容器禁止；未加锁容器上架后自动解绑，散货直接允许。
    if is_container && lock_category_is_active(lock_category) {
        return false;
    }
    true
}

pub fn validate_external_fragrant(
    is_external_product: bool,
    is_external_zone: bool,
    is_fragrant_product: bool,
    is_fragrant_zone: bool,
) -> bool {
    is_external_product == is_external_zone && is_fragrant_product == is_fragrant_zone
}

/// M-VR 双人策略：Single 不强制见证人；其余策略沿用双人核验。
pub fn special_dual_passes(
    is_special: bool,
    policy_is_single: bool,
    operator_id: Uuid,
    witness_id: Option<Uuid>,
) -> bool {
    if !is_special || policy_is_single {
        return true;
    }
    validate_special_drug_dual(true, operator_id, witness_id)
}

/// 推荐位可离线判定的 6 维：① 品类 ② 温区 ③ 质量 ⑤ 包装粒度 ⑥ 外用/串味。
/// ④ 特药双人依赖操作人/见证人，在上架事务执行，不作为库位候选过滤。
pub fn recommend_candidate_passes_6d(
    zone_temp: &str,
    product_temp: &str,
    zone_quality: &str,
    batch_or_lock_status: &str,
    allowed_categories: &serde_json::Value,
    product_category: &str,
    product_external: bool,
    zone_external: bool,
    product_fragrant: bool,
    zone_fragrant: bool,
    location_type: &str,
    allows_container: bool,
    is_container: bool,
    lock_category: Option<&str>,
) -> bool {
    let quality_status = match lock_category {
        Some(category @ ("quarantine" | "rejected")) => category,
        _ => batch_or_lock_status,
    };
    is_temperature_zone_subset(zone_temp, product_temp)
        && validate_quality_match(zone_quality, quality_status)
        && validate_category_zone(allowed_categories, product_category)
        && validate_pack_granularity(location_type, allows_container, is_container, lock_category)
        && validate_external_fragrant(
            product_external,
            zone_external,
            product_fragrant,
            zone_fragrant,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_subset_uses_containment_not_equality() {
        assert!(is_temperature_zone_subset("cold_2_8", "cold_2_8"));
        assert!(is_temperature_zone_subset("cold_2_8", "cool_le_20") == false);
        assert!(is_temperature_zone_subset("cool_le_20", "normal_10_30"));
        assert!(!is_temperature_zone_subset("normal_10_30", "cold_2_8"));
        assert!(parse_temperature_range("frozen").is_some());
    }

    #[test]
    fn qualified_quality_match_treats_unset_zone_as_qualified() {
        assert!(validate_quality_match("", "qualified"));
        assert!(validate_quality_match("qualified_green", "qualified"));
        assert!(!validate_quality_match("quarantine_yellow", "qualified"));
        assert!(validate_quality_match("quarantine_yellow", "quarantine"));
        assert!(validate_quality_match("unqualified_red", "rejected"));
    }

    #[test]
    fn pack_granularity_allows_unlocked_container_on_pick_and_rejects_locked() {
        assert!(validate_pack_granularity(
            "storage",
            true,
            true,
            Some("qualified"),
        ));
        assert!(!validate_pack_granularity("storage", true, false, None));
        assert!(validate_pack_granularity(
            "case_pick",
            false,
            true,
            Some("qualified"),
        ));
        assert!(validate_pack_granularity("case_pick", false, false, None));
        assert!(!validate_pack_granularity(
            "piece_pick",
            false,
            true,
            Some("rejected"),
        ));
        assert!(!validate_pack_granularity(
            "case_pick",
            false,
            true,
            Some("quarantine"),
        ));
    }

    #[test]
    fn special_dual_requires_distinct_witness() {
        let operator = Uuid::new_v4();
        assert!(validate_special_drug_dual(false, operator, None));
        assert!(!validate_special_drug_dual(true, operator, None));
        assert!(!validate_special_drug_dual(true, operator, Some(operator)));
        assert!(validate_special_drug_dual(
            true,
            operator,
            Some(Uuid::new_v4())
        ));
    }

    #[test]
    fn special_dual_mvr_single_skips_witness() {
        let operator = Uuid::new_v4();
        assert!(special_dual_passes(true, true, operator, None));
        assert!(!special_dual_passes(true, false, operator, None));
        assert!(special_dual_passes(
            true,
            false,
            operator,
            Some(Uuid::new_v4())
        ));
        assert!(special_dual_passes(false, false, operator, None));
    }

    #[test]
    fn recommend_candidate_runs_temperature_quality_category_and_fragrant() {
        let empty = serde_json::json!([]);
        assert!(recommend_candidate_passes_6d(
            "cold_2_8",
            "cold_2_8",
            "qualified_green",
            "qualified",
            &empty,
            "drug",
            false,
            false,
            false,
            false,
            "storage",
            true,
            true,
            None,
        ));
        assert!(!recommend_candidate_passes_6d(
            "normal_10_30",
            "cold_2_8",
            "qualified_green",
            "qualified",
            &empty,
            "drug",
            false,
            false,
            false,
            false,
            "storage",
            true,
            true,
            None,
        ));
        assert!(!recommend_candidate_passes_6d(
            "cold_2_8",
            "cold_2_8",
            "qualified_green",
            "quarantine",
            &empty,
            "drug",
            false,
            false,
            false,
            false,
            "storage",
            true,
            true,
            None,
        ));
        assert!(!recommend_candidate_passes_6d(
            "cold_2_8",
            "cold_2_8",
            "qualified_green",
            "qualified",
            &serde_json::json!(["device"]),
            "drug",
            false,
            false,
            false,
            false,
            "storage",
            true,
            true,
            None,
        ));
        assert!(!recommend_candidate_passes_6d(
            "cold_2_8",
            "cold_2_8",
            "qualified_green",
            "qualified",
            &empty,
            "drug",
            true,
            false,
            false,
            false,
            "storage",
            true,
            true,
            None,
        ));
        assert!(!recommend_candidate_passes_6d(
            "cold_2_8",
            "cold_2_8",
            "qualified_green",
            "qualified",
            &empty,
            "drug",
            false,
            false,
            false,
            false,
            "storage",
            true,
            false,
            None,
        ));
    }

    #[test]
    fn recommend_quality_prefers_container_lock_over_query_status() {
        let empty = serde_json::json!([]);
        assert!(
            !recommend_candidate_passes_6d(
                "normal_10_30",
                "normal_10_30",
                "qualified_green",
                "qualified",
                &empty,
                "drug",
                false,
                false,
                false,
                false,
                "storage",
                true,
                true,
                Some("quarantine"),
            ),
            "隔离锁容器不得推荐合格区"
        );
        assert!(recommend_candidate_passes_6d(
            "normal_10_30",
            "normal_10_30",
            "quarantine_yellow",
            "qualified",
            &empty,
            "drug",
            false,
            false,
            false,
            false,
            "storage",
            true,
            true,
            Some("quarantine"),
        ));
    }
}
