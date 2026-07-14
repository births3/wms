use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::DualPersonPolicy;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StockAdjustmentSource {
    Erp,
    Manual,
}

impl StockAdjustmentSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Erp => "erp",
            Self::Manual => "manual",
        }
    }
}

impl TryFrom<&str> for StockAdjustmentSource {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "erp" => Ok(Self::Erp),
            "manual" => Ok(Self::Manual),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StockLossReason {
    Expired,
    Damaged,
    QualityUnqualified,
    InventoryLoss,
    Destruction,
    RecallDestruction,
    Other,
}

impl StockLossReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Expired => "expired",
            Self::Damaged => "damaged",
            Self::QualityUnqualified => "quality_unqualified",
            Self::InventoryLoss => "inventory_loss",
            Self::Destruction => "destruction",
            Self::RecallDestruction => "recall_destruction",
            Self::Other => "other",
        }
    }

    pub const fn is_destruction(self) -> bool {
        matches!(self, Self::Destruction | Self::RecallDestruction)
    }
}

impl TryFrom<&str> for StockLossReason {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "expired" => Ok(Self::Expired),
            "damaged" => Ok(Self::Damaged),
            "quality_unqualified" => Ok(Self::QualityUnqualified),
            "inventory_loss" => Ok(Self::InventoryLoss),
            "destruction" => Ok(Self::Destruction),
            "recall_destruction" => Ok(Self::RecallDestruction),
            "other" => Ok(Self::Other),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StockAdjustmentStatus {
    PendingApproval,
    PendingExecution,
    InProgress,
    Completed,
    Rejected,
    Cancelled,
    ExceptionSuspended,
}

impl StockAdjustmentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PendingApproval => "pending_approval",
            Self::PendingExecution => "pending_execution",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::ExceptionSuspended => "exception_suspended",
        }
    }
}

impl TryFrom<&str> for StockAdjustmentStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending_approval" => Ok(Self::PendingApproval),
            "pending_execution" => Ok(Self::PendingExecution),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "rejected" => Ok(Self::Rejected),
            "cancelled" => Ok(Self::Cancelled),
            "exception_suspended" => Ok(Self::ExceptionSuspended),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateStockLossOrderRequest {
    pub warehouse_id: Uuid,
    pub batch_id: Uuid,
    pub quantity: i64,
    pub reason: StockLossReason,
    pub recall_id: Option<String>,
    pub source: StockAdjustmentSource,
    pub external_ref: Option<String>,
    pub requires_quality_approval: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct StockLossQualityApprovalRequest {
    pub quality_liaison_id: String,
    pub approved: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct ExecuteStockLossOrderRequest {
    pub second_operator_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct StockLossOrder {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
    pub order_no: String,
    pub batch_id: Uuid,
    pub product_code: String,
    pub batch_no: String,
    pub quantity: i64,
    pub reason: StockLossReason,
    pub recall_id: Option<String>,
    pub source: StockAdjustmentSource,
    pub external_ref: Option<String>,
    pub status: StockAdjustmentStatus,
    pub requires_quality_approval: bool,
    pub quality_liaison_id: Option<String>,
    pub policy: Option<DualPersonPolicy>,
    pub source_rule_id: Option<Uuid>,
    pub first_operator_id: Option<Uuid>,
    pub second_operator_id: Option<Uuid>,
    pub approval_record_id: Option<Uuid>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
