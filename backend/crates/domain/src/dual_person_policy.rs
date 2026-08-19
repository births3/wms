use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::common::PageMeta;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DualPersonPolicy {
    Single,
    DualScan,
    DualScanWithApproval,
}

impl DualPersonPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::DualScan => "dual_scan",
            Self::DualScanWithApproval => "dual_scan_with_approval",
        }
    }
}

impl TryFrom<&str> for DualPersonPolicy {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "single" => Ok(Self::Single),
            "dual_scan" => Ok(Self::DualScan),
            "dual_scan_with_approval" => Ok(Self::DualScanWithApproval),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ResolveDualPersonPolicyQuery {
    pub product_id: Uuid,
    pub process: String,
    pub node: String,
    pub owner_id: Uuid,
    pub warehouse_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DualPersonPolicyResponse {
    pub policy: DualPersonPolicy,
    pub source_rule_id: Option<Uuid>,
    pub process: String,
    pub node: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DualPersonPolicyScope {
    Global,
    Owner,
    Warehouse,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpsertDualPersonPolicyRuleRequest {
    pub special_drug_category: String,
    pub process: String,
    pub node: String,
    pub policy: DualPersonPolicy,
    pub scope: DualPersonPolicyScope,
    pub warehouse_id: Option<Uuid>,
    pub priority: i32,
    pub enabled: bool,
    pub confirmed_by_user_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DualPersonPolicyRule {
    pub id: Uuid,
    pub special_drug_category: String,
    pub process: String,
    pub node: String,
    pub owner_id: Option<Uuid>,
    pub warehouse_id: Option<Uuid>,
    pub policy: DualPersonPolicy,
    pub priority: i32,
    pub enabled: bool,
    pub confirmed_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Default, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct DualPersonPolicyRuleListQuery {
    pub warehouse_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DualPersonPolicyRuleListResponse {
    pub data: Vec<DualPersonPolicyRule>,
    pub page: PageMeta,
}
