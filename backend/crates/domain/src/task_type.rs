use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;

pub const TASK_TYPE_PICK: &str = "pick";
pub const TASK_TYPE_PUTAWAY: &str = "putaway";
pub const TASK_TYPE_REPLENISH: &str = "replenish";
pub const TASK_TYPE_RELOCATION: &str = "relocation";
pub const TASK_TYPE_INVENTORY_COUNT: &str = "inventory_count";
pub const TASK_TYPE_LOADING: &str = "loading";
pub const TASK_TYPE_RETURN_PUTAWAY: &str = "return_putaway";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct TaskType {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub task_type_code: String,
    pub task_type_name: String,
    pub default_priority: i32,
    pub estimated_minutes: i32,
    pub mergeable: bool,
    pub insertable: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct TaskTypeListResponse {
    pub data: Vec<TaskType>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpsertTaskTypeRequest {
    pub task_type_name: String,
    pub default_priority: i32,
    pub estimated_minutes: i32,
    pub mergeable: bool,
    pub insertable: bool,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct SetTaskTypeEnabledRequest {
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskTypeValidationError {
    CodeEmpty,
    CodeInvalid,
    NameEmpty,
    NameInvalid,
    PriorityInvalid,
    EstimatedMinutesInvalid,
}

impl fmt::Display for TaskTypeValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::CodeEmpty => "任务类型编码不能为空",
            Self::CodeInvalid => "任务类型编码只能包含字母、数字、下划线、连字符或点号",
            Self::NameEmpty => "任务类型名称不能为空",
            Self::NameInvalid => "任务类型名称非法",
            Self::PriorityInvalid => "默认优先级必须在 0 到 1000 之间",
            Self::EstimatedMinutesInvalid => "预计耗时必须在 1 到 10080 分钟之间",
        };
        f.write_str(message)
    }
}

impl UpsertTaskTypeRequest {
    pub fn validate(&self) -> Result<(), TaskTypeValidationError> {
        let name = self.task_type_name.trim();
        if name.is_empty() {
            return Err(TaskTypeValidationError::NameEmpty);
        }
        if name.len() > 128 || name.chars().any(char::is_control) {
            return Err(TaskTypeValidationError::NameInvalid);
        }
        if !(0..=1000).contains(&self.default_priority) {
            return Err(TaskTypeValidationError::PriorityInvalid);
        }
        if !(1..=10_080).contains(&self.estimated_minutes) {
            return Err(TaskTypeValidationError::EstimatedMinutesInvalid);
        }
        Ok(())
    }

    pub fn normalized(mut self) -> Self {
        self.task_type_name = self.task_type_name.trim().to_string();
        self
    }
}

pub fn normalize_task_type_code(code: &str) -> Result<String, TaskTypeValidationError> {
    let code = code.trim().to_ascii_lowercase();
    if code.is_empty() {
        return Err(TaskTypeValidationError::CodeEmpty);
    }
    if code.len() > 64
        || !code.as_bytes()[0].is_ascii_alphanumeric()
        || code
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')))
    {
        return Err(TaskTypeValidationError::CodeInvalid);
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_validates_extensible_task_type_codes() {
        assert_eq!(normalize_task_type_code("  PICKING ").unwrap(), "picking");
        assert!(normalize_task_type_code("退货上架").is_err());
        assert!(normalize_task_type_code("pick type").is_err());
    }

    #[test]
    fn rejects_invalid_task_type_configuration() {
        let request = UpsertTaskTypeRequest {
            task_type_name: " ".to_string(),
            default_priority: -1,
            estimated_minutes: 0,
            mergeable: true,
            insertable: false,
            enabled: true,
        };

        assert!(request.validate().is_err());
    }
}
