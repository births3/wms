use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::Quantity;

use crate::PageMeta;

pub const TASK_STATUS_PENDING_RELEASE: &str = "pending_release";
pub const TASK_STATUS_PENDING_ASSIGNMENT: &str = "pending_assignment";
pub const TASK_STATUS_ASSIGNED: &str = "assigned";
pub const TASK_STATUS_DISPATCHED: &str = "dispatched";
pub const TASK_STATUS_IN_PROGRESS: &str = "in_progress";
pub const TASK_STATUS_COMPLETED: &str = "completed";
pub const TASK_STATUS_EXCEPTION: &str = "exception";
pub const TASK_STATUS_CANCELLED: &str = "cancelled";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct TaskGroup {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub task_group_code: String,
    pub task_group_name: String,
    pub warehouse_id: Uuid,
    pub zone_ids: Vec<Uuid>,
    pub task_type_codes: Vec<String>,
    pub member_user_ids: Vec<Uuid>,
    pub member_qualifications: Vec<TaskGroupMemberQualification>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct TaskGroupMemberQualification {
    pub user_id: Uuid,
    pub valid_until: Option<DateTime<Utc>>,
    pub max_active_tasks: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpsertTaskGroupRequest {
    pub task_group_name: String,
    pub warehouse_id: Uuid,
    #[serde(default)]
    pub zone_ids: Vec<Uuid>,
    pub task_type_codes: Vec<String>,
    #[serde(default)]
    pub member_user_ids: Vec<Uuid>,
    #[serde(default)]
    pub member_qualifications: Vec<TaskGroupMemberQualification>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TaskGroupListResponse {
    pub data: Vec<TaskGroup>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct TaskWorker {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TaskWorkerListResponse {
    pub data: Vec<TaskWorker>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct TaskPriorityRule {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub urgent_order_bonus: i32,
    pub waiting_minutes_per_point: i32,
    pub cold_chain_bonus: i32,
    pub manual_expedite_bonus: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpsertTaskPriorityRuleRequest {
    pub urgent_order_bonus: i32,
    pub waiting_minutes_per_point: i32,
    pub cold_chain_bonus: i32,
    pub manual_expedite_bonus: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct WarehouseTask {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub task_no: String,
    pub task_type_code: String,
    pub source_module: String,
    pub source_doc_type: String,
    pub source_doc_id: Option<Uuid>,
    pub source_doc_no: String,
    pub source_line_no: Option<i32>,
    pub source_task_key: String,
    pub warehouse_id: Uuid,
    pub task_group_code: String,
    pub product_id: Option<Uuid>,
    pub product_code: String,
    pub batch_id: Option<Uuid>,
    pub batch_no: Option<String>,
    pub planned_qty: Quantity,
    pub actual_qty: Option<Quantity>,
    pub source_location_id: Option<Uuid>,
    pub source_location_code: Option<String>,
    pub target_location_id: Option<Uuid>,
    pub target_location_code: Option<String>,
    pub priority: i32,
    pub urgent_order: bool,
    pub cold_chain: bool,
    pub manually_expedited: bool,
    pub estimated_minutes: i32,
    pub predecessor_task_id: Option<Uuid>,
    pub release_due_at: Option<DateTime<Utc>>,
    pub released_at: Option<DateTime<Utc>>,
    pub assignee_user_id: Option<Uuid>,
    pub status: String,
    pub exception_code: Option<String>,
    pub exception_note: Option<String>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub dispatched_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CreateWarehouseTaskRequest {
    pub task_type_code: String,
    pub source_module: String,
    pub source_doc_type: String,
    pub source_doc_id: Option<Uuid>,
    pub source_doc_no: String,
    pub source_line_no: Option<i32>,
    pub source_task_key: String,
    pub warehouse_id: Uuid,
    pub task_group_code: String,
    pub product_id: Option<Uuid>,
    pub product_code: String,
    pub batch_id: Option<Uuid>,
    pub batch_no: Option<String>,
    pub planned_qty: Quantity,
    pub source_location_id: Option<Uuid>,
    pub source_location_code: Option<String>,
    pub target_location_id: Option<Uuid>,
    pub target_location_code: Option<String>,
    pub priority: Option<i32>,
    #[serde(default)]
    pub urgent_order: bool,
    #[serde(default)]
    pub predecessor_task_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskTransitionAction {
    Release,
    Assign,
    Dispatch,
    Reassign,
    Recall,
    Start,
    Complete,
    ReportException,
    ResolveComplete,
    Cancel,
    Expedite,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct TransitionWarehouseTaskRequest {
    pub action: TaskTransitionAction,
    pub assignee_user_id: Option<Uuid>,
    pub actual_qty: Option<Quantity>,
    pub exception_code: Option<String>,
    pub exception_note: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, IntoParams, PartialEq, Serialize, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct TaskListQuery {
    #[serde(default)]
    pub mine_only: bool,
    pub status: Option<String>,
    pub task_type_code: Option<String>,
    pub warehouse_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct WarehouseTaskListResponse {
    pub data: Vec<WarehouseTask>,
    pub page: PageMeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_actions_use_stable_api_names() {
        assert_eq!(
            serde_json::to_string(&TaskTransitionAction::ReportException).unwrap(),
            "\"report_exception\""
        );
    }
}
