use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// H9 授权人工截单命令。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct ManualDeliveryNoteCutoffRequest {
    pub warehouse_id: Uuid,
    pub delivery_address_id: Uuid,
    pub order_ids: Vec<Uuid>,
    pub reason: String,
}

/// H9 已冻结的随货同行单归集结果。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DeliveryNoteGroup {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
    pub customer_id: Uuid,
    pub delivery_address_id: Uuid,
    pub route_code: String,
    pub delivery_note_no: String,
    pub cutoff_mode: String,
    pub cutoff_reason: Option<String>,
    pub cutoff_plan_id: Option<Uuid>,
    pub scheduled_cutoff_at: Option<DateTime<Utc>>,
    pub cutoff_at: DateTime<Utc>,
    pub order_ids: Vec<Uuid>,
}

/// H9 待截单出库订单。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DeliveryNoteCandidate {
    pub outbound_order_id: Uuid,
    pub wms_order_no: String,
    pub erp_order_no: Option<String>,
    pub warehouse_id: Uuid,
    pub warehouse_code: String,
    pub warehouse_name: String,
    pub customer_id: Uuid,
    pub customer_code: String,
    pub customer_name: String,
    pub delivery_address_id: Uuid,
    pub delivery_address: String,
    pub route_code: String,
    pub created_at: DateTime<Utc>,
}

/// H9 待截单订单列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DeliveryNoteCandidateListResponse {
    pub data: Vec<DeliveryNoteCandidate>,
}

/// H9 随货同行单归集结果列表项。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DeliveryNoteGroupListItem {
    pub id: Uuid,
    pub delivery_note_no: String,
    pub warehouse_id: Uuid,
    pub warehouse_code: String,
    pub warehouse_name: String,
    pub customer_id: Uuid,
    pub customer_code: String,
    pub customer_name: String,
    pub delivery_address_id: Uuid,
    pub delivery_address: String,
    pub route_code: String,
    pub cutoff_mode: String,
    pub cutoff_reason: Option<String>,
    pub cutoff_plan_id: Option<Uuid>,
    pub scheduled_cutoff_at: Option<DateTime<Utc>>,
    pub cutoff_at: DateTime<Utc>,
    pub order_ids: Vec<Uuid>,
    pub order_nos: Vec<String>,
}

/// H9 随货同行单归集结果列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DeliveryNoteGroupListResponse {
    pub data: Vec<DeliveryNoteGroupListItem>,
}

/// H9 送货地址线路绑定发布命令。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct PublishRouteBindingRequest {
    pub warehouse_id: Uuid,
    pub customer_id: Uuid,
    pub delivery_address_id: Uuid,
    pub route_code: String,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
}

/// H9 已发布的送货地址线路绑定。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct RouteBinding {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
    pub warehouse_code: String,
    pub warehouse_name: String,
    pub customer_id: Uuid,
    pub customer_code: String,
    pub customer_name: String,
    pub delivery_address_id: Uuid,
    pub delivery_address: String,
    pub route_code: String,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// H9 线路绑定列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct RouteBindingListResponse {
    pub data: Vec<RouteBinding>,
}

/// 截单计划匹配层级；枚举顺序不代表运行优先级。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CutoffPlanScope {
    Customer,
    Route,
    OwnerWarehouse,
}

/// 截单计划的一条结构化周计划。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct WeeklyCutoffSlot {
    /// ISO weekday: Monday=1, Sunday=7.
    pub weekday: u8,
    /// Local time in HH:MM.
    pub cutoff_time: String,
}

/// 截单计划的例外日期；空时间表示当天不截单。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CutoffDateException {
    pub date: NaiveDate,
    pub cutoff_time: Option<String>,
}

/// 创建 H9 截单计划草稿。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CreateCutoffPlanRequest {
    pub name: String,
    pub warehouse_id: Uuid,
    pub scope: CutoffPlanScope,
    pub customer_id: Option<Uuid>,
    pub route_code: Option<String>,
    pub utc_offset_minutes: i16,
    pub weekly_schedule: Vec<WeeklyCutoffSlot>,
    pub exceptions: Vec<CutoffDateException>,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
}

/// H9 截单计划。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CutoffPlan {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub warehouse_id: Uuid,
    pub scope: CutoffPlanScope,
    pub customer_id: Option<Uuid>,
    pub route_code: Option<String>,
    pub utc_offset_minutes: i16,
    pub weekly_schedule: Vec<WeeklyCutoffSlot>,
    pub exceptions: Vec<CutoffDateException>,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// H9 截单计划列表。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct CutoffPlanListResponse {
    pub data: Vec<CutoffPlan>,
}

/// 人工截单的纯业务校验失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManualDeliveryNoteCutoffValidationError {
    EmptyOrderSelection,
    DuplicateOrder,
    ReasonRequired,
    ReasonTooLong,
}

/// 线路绑定的纯业务校验失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RouteBindingValidationError {
    IdentifierRequired,
    RouteCodeRequired,
    RouteCodeTooLong,
    InvalidEffectivePeriod,
}

/// 截单计划的纯业务校验失败。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CutoffPlanValidationError {
    NameRequired,
    NameTooLong,
    IdentifierRequired,
    ScopeMismatch,
    InvalidUtcOffset,
    WeeklyScheduleRequired,
    InvalidWeekday,
    DuplicateWeekday,
    InvalidCutoffTime,
    DuplicateExceptionDate,
    InvalidEffectivePeriod,
}

/// 校验人工截单命令中不依赖数据库的业务约束。
pub fn validate_manual_delivery_note_cutoff(
    request: &ManualDeliveryNoteCutoffRequest,
) -> Result<(), ManualDeliveryNoteCutoffValidationError> {
    if request.order_ids.is_empty() {
        return Err(ManualDeliveryNoteCutoffValidationError::EmptyOrderSelection);
    }
    if request.order_ids.iter().collect::<HashSet<_>>().len() != request.order_ids.len() {
        return Err(ManualDeliveryNoteCutoffValidationError::DuplicateOrder);
    }
    let reason = request.reason.trim();
    if reason.is_empty() {
        return Err(ManualDeliveryNoteCutoffValidationError::ReasonRequired);
    }
    if reason.chars().count() > 500 {
        return Err(ManualDeliveryNoteCutoffValidationError::ReasonTooLong);
    }
    Ok(())
}

/// 校验线路绑定中不依赖数据库的边界。
pub fn validate_route_binding(
    request: &PublishRouteBindingRequest,
) -> Result<(), RouteBindingValidationError> {
    if request.warehouse_id.is_nil()
        || request.customer_id.is_nil()
        || request.delivery_address_id.is_nil()
    {
        return Err(RouteBindingValidationError::IdentifierRequired);
    }
    let route_code = request.route_code.trim();
    if route_code.is_empty() {
        return Err(RouteBindingValidationError::RouteCodeRequired);
    }
    if route_code.chars().count() > 64 {
        return Err(RouteBindingValidationError::RouteCodeTooLong);
    }
    if request
        .effective_to
        .is_some_and(|effective_to| effective_to <= request.effective_from)
    {
        return Err(RouteBindingValidationError::InvalidEffectivePeriod);
    }
    Ok(())
}

/// 校验截单计划草稿中不依赖数据库的边界。
pub fn validate_cutoff_plan(
    request: &CreateCutoffPlanRequest,
) -> Result<(), CutoffPlanValidationError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(CutoffPlanValidationError::NameRequired);
    }
    if name.chars().count() > 100 {
        return Err(CutoffPlanValidationError::NameTooLong);
    }
    if request.warehouse_id.is_nil() {
        return Err(CutoffPlanValidationError::IdentifierRequired);
    }
    let scope_valid = match request.scope {
        CutoffPlanScope::Customer => {
            request.customer_id.is_some_and(|id| !id.is_nil()) && request.route_code.is_none()
        }
        CutoffPlanScope::Route => {
            request.customer_id.is_none()
                && request
                    .route_code
                    .as_deref()
                    .is_some_and(|code| !code.trim().is_empty() && code.chars().count() <= 64)
        }
        CutoffPlanScope::OwnerWarehouse => {
            request.customer_id.is_none() && request.route_code.is_none()
        }
    };
    if !scope_valid {
        return Err(CutoffPlanValidationError::ScopeMismatch);
    }
    if !(-720..=840).contains(&request.utc_offset_minutes) {
        return Err(CutoffPlanValidationError::InvalidUtcOffset);
    }
    if request.weekly_schedule.is_empty() {
        return Err(CutoffPlanValidationError::WeeklyScheduleRequired);
    }
    let mut weekdays = HashSet::new();
    for slot in &request.weekly_schedule {
        if !(1..=7).contains(&slot.weekday) {
            return Err(CutoffPlanValidationError::InvalidWeekday);
        }
        if !weekdays.insert(slot.weekday) {
            return Err(CutoffPlanValidationError::DuplicateWeekday);
        }
        validate_cutoff_time(&slot.cutoff_time)?;
    }
    let mut exception_dates = HashSet::new();
    for exception in &request.exceptions {
        if !exception_dates.insert(exception.date) {
            return Err(CutoffPlanValidationError::DuplicateExceptionDate);
        }
        if let Some(cutoff_time) = &exception.cutoff_time {
            validate_cutoff_time(cutoff_time)?;
        }
    }
    if request
        .effective_to
        .is_some_and(|effective_to| effective_to <= request.effective_from)
    {
        return Err(CutoffPlanValidationError::InvalidEffectivePeriod);
    }
    Ok(())
}

fn validate_cutoff_time(value: &str) -> Result<(), CutoffPlanValidationError> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map(|_| ())
        .map_err(|_| CutoffPlanValidationError::InvalidCutoffTime)
}

#[cfg(test)]
mod tests {
    use super::{
        validate_cutoff_plan, validate_manual_delivery_note_cutoff, validate_route_binding,
        CreateCutoffPlanRequest, CutoffDateException, CutoffPlanScope, CutoffPlanValidationError,
        ManualDeliveryNoteCutoffRequest, ManualDeliveryNoteCutoffValidationError,
        PublishRouteBindingRequest, RouteBindingValidationError, WeeklyCutoffSlot,
    };
    use chrono::{NaiveDate, TimeZone, Utc};
    use uuid::Uuid;

    fn request(order_ids: Vec<Uuid>, reason: &str) -> ManualDeliveryNoteCutoffRequest {
        ManualDeliveryNoteCutoffRequest {
            warehouse_id: Uuid::new_v4(),
            delivery_address_id: Uuid::new_v4(),
            order_ids,
            reason: reason.to_string(),
        }
    }

    #[test]
    fn manual_cutoff_rejects_empty_duplicate_orders_and_blank_reason() {
        assert_eq!(
            validate_manual_delivery_note_cutoff(&request(vec![], "补单截单")),
            Err(ManualDeliveryNoteCutoffValidationError::EmptyOrderSelection)
        );
        let order_id = Uuid::new_v4();
        assert_eq!(
            validate_manual_delivery_note_cutoff(&request(vec![order_id, order_id], "补单截单")),
            Err(ManualDeliveryNoteCutoffValidationError::DuplicateOrder)
        );
        assert_eq!(
            validate_manual_delivery_note_cutoff(&request(vec![order_id], "  ")),
            Err(ManualDeliveryNoteCutoffValidationError::ReasonRequired)
        );
    }

    #[test]
    fn route_binding_rejects_blank_route_and_reversed_period() {
        let mut request = PublishRouteBindingRequest {
            warehouse_id: Uuid::new_v4(),
            customer_id: Uuid::new_v4(),
            delivery_address_id: Uuid::new_v4(),
            route_code: " ".to_string(),
            effective_from: Utc
                .with_ymd_and_hms(2026, 7, 26, 0, 0, 0)
                .single()
                .expect("valid effective time"),
            effective_to: None,
        };
        assert_eq!(
            validate_route_binding(&request),
            Err(RouteBindingValidationError::RouteCodeRequired)
        );
        request.route_code = "LINE-A".to_string();
        request.effective_to = Some(request.effective_from);
        assert_eq!(
            validate_route_binding(&request),
            Err(RouteBindingValidationError::InvalidEffectivePeriod)
        );
    }

    #[test]
    fn cutoff_plan_rejects_scope_mismatch_and_duplicate_weekday() {
        let mut request = CreateCutoffPlanRequest {
            name: "客户截单".to_string(),
            warehouse_id: Uuid::new_v4(),
            scope: CutoffPlanScope::Customer,
            customer_id: None,
            route_code: Some("LINE-A".to_string()),
            utc_offset_minutes: 480,
            weekly_schedule: vec![WeeklyCutoffSlot {
                weekday: 1,
                cutoff_time: "09:00".to_string(),
            }],
            exceptions: vec![CutoffDateException {
                date: NaiveDate::from_ymd_opt(2026, 7, 27).expect("valid date"),
                cutoff_time: None,
            }],
            effective_from: Utc
                .with_ymd_and_hms(2026, 7, 1, 0, 0, 0)
                .single()
                .expect("valid effective time"),
            effective_to: None,
        };
        assert_eq!(
            validate_cutoff_plan(&request),
            Err(CutoffPlanValidationError::ScopeMismatch)
        );
        request.customer_id = Some(Uuid::new_v4());
        request.route_code = None;
        request
            .weekly_schedule
            .push(request.weekly_schedule[0].clone());
        assert_eq!(
            validate_cutoff_plan(&request),
            Err(CutoffPlanValidationError::DuplicateWeekday)
        );
    }
}
