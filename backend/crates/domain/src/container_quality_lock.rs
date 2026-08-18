use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::lpn_container::LPN_CONTAINER_STATUS_IN_USE;

pub const LPN_LOCK_CATEGORY_QUALIFIED: &str = "qualified";
pub const LPN_LOCK_CATEGORY_QUARANTINE: &str = "quarantine";
pub const LPN_LOCK_CATEGORY_REJECTED: &str = "rejected";

pub const LPN_LOCK_EVENT_TYPE_LOCK: &str = "lock";
pub const LPN_LOCK_EVENT_TYPE_CHANGE_REASON: &str = "change_reason";
pub const LPN_LOCK_EVENT_TYPE_RELEASE: &str = "release";

pub const PERMISSION_QUALITY_LOCK_MANAGE: &str = "m1.quality-lock.manage";

pub const VALID_LOCK_CATEGORIES: &[&str] = &[
    LPN_LOCK_CATEGORY_QUALIFIED,
    LPN_LOCK_CATEGORY_QUARANTINE,
    LPN_LOCK_CATEGORY_REJECTED,
];

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApplyContainerQualityLockRequest {
    pub lock_category: String,
    pub reason_dict_item_code: String,
    #[serde(default)]
    pub reason_desc: Option<String>,
    #[serde(default)]
    pub evidence_urls: Vec<String>,
    #[serde(default)]
    pub quality_liaison_id: Option<Uuid>,
    pub witness_id: Uuid,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ChangeContainerQualityLockReasonRequest {
    #[serde(default)]
    pub lock_category: Option<String>,
    pub reason_dict_item_code: String,
    #[serde(default)]
    pub reason_desc: Option<String>,
    #[serde(default)]
    pub evidence_urls: Vec<String>,
    pub witness_id: Uuid,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReleaseContainerQualityLockRequest {
    pub witness_id: Uuid,
    #[serde(default)]
    pub reason_desc: Option<String>,
    #[serde(default)]
    pub quality_liaison_id: Option<Uuid>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ContainerQualityLockEvent {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub container_id: Uuid,
    pub lpn_code: String,
    pub event_type: String,
    pub lock_category: Option<String>,
    pub reason_dict_item_code: Option<String>,
    pub reason_desc: Option<String>,
    pub evidence_urls: serde_json::Value,
    pub quality_liaison_id: Option<Uuid>,
    pub operated_by: Uuid,
    pub witness_id: Option<Uuid>,
    pub occurred_at: DateTime<Utc>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContainerQualityLockValidationError {
    StateInvalid,
    WitnessInvalid,
    MqlNotFinal,
    ZoneMismatch,
    LocationTypeInvalid,
    CategoryInvalid,
    ReasonRequired,
    MqlRequired,
    NotLocked,
}

pub fn is_valid_lock_category(category: &str) -> bool {
    VALID_LOCK_CATEGORIES.contains(&category)
}

pub fn validate_witness(
    operator_id: Uuid,
    witness_id: Uuid,
) -> Result<(), ContainerQualityLockValidationError> {
    if operator_id == witness_id {
        return Err(ContainerQualityLockValidationError::WitnessInvalid);
    }
    Ok(())
}

fn validate_lock_category_value(category: &str) -> Result<(), ContainerQualityLockValidationError> {
    if category != LPN_LOCK_CATEGORY_QUARANTINE && category != LPN_LOCK_CATEGORY_REJECTED {
        return Err(ContainerQualityLockValidationError::CategoryInvalid);
    }
    Ok(())
}

fn validate_reason_required(
    reason_dict_item_code: &str,
) -> Result<(), ContainerQualityLockValidationError> {
    if reason_dict_item_code.trim().is_empty() {
        return Err(ContainerQualityLockValidationError::ReasonRequired);
    }
    Ok(())
}

pub fn validate_apply_lock(
    container_status: &str,
    req: &ApplyContainerQualityLockRequest,
    operator_id: Uuid,
) -> Result<(), ContainerQualityLockValidationError> {
    if container_status != LPN_CONTAINER_STATUS_IN_USE {
        return Err(ContainerQualityLockValidationError::StateInvalid);
    }
    validate_witness(operator_id, req.witness_id)?;
    validate_lock_category_value(&req.lock_category)?;
    validate_reason_required(&req.reason_dict_item_code)?;
    if req.lock_category == LPN_LOCK_CATEGORY_REJECTED && req.quality_liaison_id.is_none() {
        return Err(ContainerQualityLockValidationError::MqlRequired);
    }
    Ok(())
}

pub fn validate_change_reason(
    current_lock_category: Option<&str>,
    req: &ChangeContainerQualityLockReasonRequest,
    operator_id: Uuid,
) -> Result<(), ContainerQualityLockValidationError> {
    let current = current_lock_category.unwrap_or(LPN_LOCK_CATEGORY_QUALIFIED);
    if current == LPN_LOCK_CATEGORY_QUALIFIED {
        return Err(ContainerQualityLockValidationError::NotLocked);
    }
    let target = req.lock_category.as_deref().unwrap_or(current);
    validate_lock_category_value(target)?;
    validate_reason_required(&req.reason_dict_item_code)?;
    // 换原因同样强制双人见证（GSP 双人作业），缺见证人或与操作人相同均拒绝。
    validate_witness(operator_id, req.witness_id)?;
    Ok(())
}

pub fn validate_release_lock(
    current_lock_category: Option<&str>,
    req: &ReleaseContainerQualityLockRequest,
    operator_id: Uuid,
    mql_status: Option<&str>,
) -> Result<(), ContainerQualityLockValidationError> {
    let cat = current_lock_category.unwrap_or(LPN_LOCK_CATEGORY_QUALIFIED);
    if cat == LPN_LOCK_CATEGORY_QUALIFIED {
        return Err(ContainerQualityLockValidationError::NotLocked);
    }
    validate_witness(operator_id, req.witness_id)?;
    if let Some(status) = mql_status {
        if status != "closed" && status != "rejected" {
            return Err(ContainerQualityLockValidationError::MqlNotFinal);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_witness_defense() {
        let user = Uuid::new_v4();
        assert_eq!(
            validate_witness(user, user),
            Err(ContainerQualityLockValidationError::WitnessInvalid)
        );
        let user2 = Uuid::new_v4();
        assert!(validate_witness(user, user2).is_ok());
    }

    #[test]
    fn test_apply_lock_precondition() {
        let op = Uuid::new_v4();
        let wit = Uuid::new_v4();
        let req = ApplyContainerQualityLockRequest {
            lock_category: "quarantine".to_string(),
            reason_dict_item_code: "temp_anomaly".to_string(),
            reason_desc: None,
            evidence_urls: vec![],
            quality_liaison_id: None,
            witness_id: wit,
            note: None,
        };
        assert_eq!(
            validate_apply_lock("idle", &req, op),
            Err(ContainerQualityLockValidationError::StateInvalid)
        );
        assert!(validate_apply_lock("in_use", &req, op).is_ok());
    }

    #[test]
    fn test_rejected_lock_requires_mql() {
        let op = Uuid::new_v4();
        let wit = Uuid::new_v4();
        let req = ApplyContainerQualityLockRequest {
            lock_category: "rejected".to_string(),
            reason_dict_item_code: "expired".to_string(),
            reason_desc: None,
            evidence_urls: vec![],
            quality_liaison_id: None,
            witness_id: wit,
            note: None,
        };
        assert_eq!(
            validate_apply_lock("in_use", &req, op),
            Err(ContainerQualityLockValidationError::MqlRequired)
        );
    }

    #[test]
    fn test_change_reason_precondition() {
        let op = Uuid::new_v4();
        let wit = Uuid::new_v4();
        let req = ChangeContainerQualityLockReasonRequest {
            lock_category: Some("quarantine".to_string()),
            reason_dict_item_code: "temp_anomaly".to_string(),
            reason_desc: None,
            evidence_urls: vec![],
            witness_id: wit,
            note: None,
        };
        // Unlocked container cannot change reason
        assert_eq!(
            validate_change_reason(Some("qualified"), &req, op),
            Err(ContainerQualityLockValidationError::NotLocked)
        );
        // Locked container with valid target passes
        assert!(validate_change_reason(Some("rejected"), &req, op).is_ok());
        // Witness equal to operator is rejected
        let same_witness = ChangeContainerQualityLockReasonRequest {
            witness_id: op,
            ..req.clone()
        };
        assert_eq!(
            validate_change_reason(Some("rejected"), &same_witness, op),
            Err(ContainerQualityLockValidationError::WitnessInvalid)
        );
        // Invalid target category is rejected
        let bad = ChangeContainerQualityLockReasonRequest {
            lock_category: Some("qualified".to_string()),
            ..req.clone()
        };
        assert_eq!(
            validate_change_reason(Some("rejected"), &bad, op),
            Err(ContainerQualityLockValidationError::CategoryInvalid)
        );
        // Empty reason is rejected
        let empty = ChangeContainerQualityLockReasonRequest {
            reason_dict_item_code: "  ".to_string(),
            ..req.clone()
        };
        assert!(validate_change_reason(Some("rejected"), &empty, op).is_err());
        // Distinct witness passes
        assert!(validate_change_reason(Some("rejected"), &req, op).is_ok());
    }

    #[test]
    fn test_release_mql_gate() {
        let op = Uuid::new_v4();
        let wit = Uuid::new_v4();
        let req = ReleaseContainerQualityLockRequest {
            witness_id: wit,
            reason_desc: None,
            quality_liaison_id: None,
            note: None,
        };
        // Unlocked container cannot be released
        assert_eq!(
            validate_release_lock(Some("qualified"), &req, op, None),
            Err(ContainerQualityLockValidationError::NotLocked)
        );
        // Pending approval M-QL blocks release
        assert_eq!(
            validate_release_lock(Some("rejected"), &req, op, Some("pending_approval")),
            Err(ContainerQualityLockValidationError::MqlNotFinal)
        );
        // Approved M-QL blocks release
        assert_eq!(
            validate_release_lock(Some("rejected"), &req, op, Some("approved")),
            Err(ContainerQualityLockValidationError::MqlNotFinal)
        );
        // Closed M-QL allows release
        assert!(validate_release_lock(Some("rejected"), &req, op, Some("closed")).is_ok());
        // Rejected M-QL allows release (fallback)
        assert!(validate_release_lock(Some("rejected"), &req, op, Some("rejected")).is_ok());
    }
}
