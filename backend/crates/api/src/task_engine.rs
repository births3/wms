use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    normalize_task_type_code, CreateWarehouseTaskRequest, TaskGroup, TaskListQuery,
    TaskTransitionAction, TransitionWarehouseTaskRequest, UpsertTaskGroupRequest, WarehouseTask,
    TASK_STATUS_ASSIGNED, TASK_STATUS_CANCELLED, TASK_STATUS_COMPLETED, TASK_STATUS_DISPATCHED,
    TASK_STATUS_EXCEPTION, TASK_STATUS_IN_PROGRESS, TASK_STATUS_PENDING_ASSIGNMENT,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

#[derive(Clone, Debug)]
pub struct PgTaskEngineRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentTaskMutation<T> {
    pub value: T,
    pub replayed: bool,
}

pub(crate) fn default_task_group_code(warehouse_id: Uuid) -> String {
    format!("default-{}", warehouse_id.simple())
}

#[derive(Clone, Debug, PartialEq)]
pub enum TaskEngineError {
    Validation(String),
    TaskTypeNotFound,
    TaskGroupNotFound,
    WarehouseNotFound,
    ZoneNotFound,
    UserNotFound,
    WorkerNotQualified,
    NoAvailableWorker,
    TaskNotFound,
    NotAssignee,
    InvalidTransition,
    QuantityDifferenceRequiresException,
    SourceTaskConflict,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, FromRow)]
struct TaskGroupRow {
    id: Uuid,
    owner_id: Uuid,
    task_group_code: String,
    task_group_name: String,
    warehouse_id: Uuid,
    zone_ids: Vec<Uuid>,
    task_type_codes: Vec<String>,
    member_user_ids: Vec<Uuid>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

#[derive(Clone, Debug, FromRow)]
struct WarehouseTaskRow {
    id: Uuid,
    owner_id: Uuid,
    task_no: String,
    task_type_code: String,
    source_module: String,
    source_doc_type: String,
    source_doc_id: Option<Uuid>,
    source_doc_no: String,
    source_line_no: Option<i32>,
    source_task_key: String,
    warehouse_id: Uuid,
    task_group_code: String,
    product_id: Option<Uuid>,
    product_code: String,
    batch_id: Option<Uuid>,
    batch_no: Option<String>,
    planned_qty: i64,
    actual_qty: Option<i64>,
    source_location_id: Option<Uuid>,
    source_location_code: Option<String>,
    target_location_id: Option<Uuid>,
    target_location_code: Option<String>,
    priority: i32,
    estimated_minutes: i32,
    assignee_user_id: Option<Uuid>,
    status: String,
    exception_code: Option<String>,
    exception_note: Option<String>,
    assigned_at: Option<DateTime<Utc>>,
    dispatched_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

include!("task_engine/repository.rs");
include!("task_engine/transitions.rs");
include!("task_engine/persistence.rs");
include!("task_engine/mappers.rs");
