//! H6 state machine registry and transition validation API.

use axum::{
    extract::{rejection::QueryRejection, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use wms_domain::{
    ErrorResponse, PageMeta, StateMachineDefinition, StateMachineDefinitionListResponse,
    StateMachineState, StateMachineTransition, StateTransitionValidationResponse,
};

use crate::auth::{AuthContext, AuthError};

const READ_PERMISSION: &str = "h6.state_machine.read";

#[derive(Debug, Deserialize)]
struct ValidateTransitionQuery {
    from_state: String,
    to_state: String,
    event_code: Option<String>,
}

#[derive(Debug)]
enum StateMachineError {
    Auth(AuthError),
    InvalidQuery,
    NotFound(String),
}

impl From<AuthError> for StateMachineError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl IntoResponse for StateMachineError {
    fn into_response(self) -> Response {
        if let StateMachineError::Auth(error) = self {
            return error.into_response();
        }

        let (status, code, message, machine_code) = match self {
            StateMachineError::InvalidQuery => (
                StatusCode::BAD_REQUEST,
                "H6_INVALID_TRANSITION_QUERY",
                "缺少或无法解析状态转换查询参数",
                String::new(),
            ),
            StateMachineError::NotFound(machine_code) => (
                StatusCode::NOT_FOUND,
                "H6_STATE_MACHINE_NOT_FOUND",
                "状态机定义不存在",
                machine_code,
            ),
            StateMachineError::Auth(_) => unreachable!("auth error returned above"),
        };

        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({ "machine_code": machine_code }),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

pub fn state_machine_router() -> Router {
    Router::new()
        .route("/api/v1/state-machines", get(list_state_machines_handler))
        .route(
            "/api/v1/state-machines/:machine_code",
            get(get_state_machine_handler),
        )
        .route(
            "/api/v1/state-machines/:machine_code/transition-validation",
            get(validate_state_transition_handler),
        )
}

async fn list_state_machines_handler(
    ctx: AuthContext,
) -> Result<Json<StateMachineDefinitionListResponse>, StateMachineError> {
    ctx.require_permission(READ_PERMISSION)?;
    let data = state_machine_definitions();
    Ok(Json(StateMachineDefinitionListResponse {
        page: PageMeta {
            next_cursor: None,
            count: data.len() as u32,
        },
        data,
    }))
}

async fn get_state_machine_handler(
    ctx: AuthContext,
    Path(machine_code): Path<String>,
) -> Result<Json<StateMachineDefinition>, StateMachineError> {
    ctx.require_permission(READ_PERMISSION)?;
    find_definition(&machine_code)
        .ok_or(StateMachineError::NotFound(machine_code))
        .map(Json)
}

async fn validate_state_transition_handler(
    ctx: AuthContext,
    Path(machine_code): Path<String>,
    query: Result<Query<ValidateTransitionQuery>, QueryRejection>,
) -> Result<Json<StateTransitionValidationResponse>, StateMachineError> {
    ctx.require_permission(READ_PERMISSION)?;
    let Query(query) = query.map_err(|_| StateMachineError::InvalidQuery)?;
    let definition =
        find_definition(&machine_code).ok_or(StateMachineError::NotFound(machine_code))?;
    Ok(Json(validate_transition(&definition, query)))
}

fn find_definition(machine_code: &str) -> Option<StateMachineDefinition> {
    state_machine_definitions()
        .into_iter()
        .find(|definition| definition.machine_code == machine_code)
}

fn validate_transition(
    definition: &StateMachineDefinition,
    query: ValidateTransitionQuery,
) -> StateTransitionValidationResponse {
    let has_from = definition
        .states
        .iter()
        .any(|state| state.code == query.from_state);
    let has_to = definition
        .states
        .iter()
        .any(|state| state.code == query.to_state);
    let matched = definition.transitions.iter().any(|transition| {
        transition.from_state == query.from_state
            && transition.to_state == query.to_state
            && query
                .event_code
                .as_ref()
                .map_or(true, |event_code| transition.event_code == *event_code)
    });

    let reason = if !has_from {
        Some("from_state 不在状态机定义中".to_string())
    } else if !has_to {
        Some("to_state 不在状态机定义中".to_string())
    } else if !matched {
        Some("非法状态转换".to_string())
    } else {
        None
    };

    StateTransitionValidationResponse {
        machine_code: definition.machine_code.clone(),
        from_state: query.from_state,
        to_state: query.to_state,
        event_code: query.event_code,
        allowed: reason.is_none(),
        reason,
    }
}

fn state_machine_definitions() -> Vec<StateMachineDefinition> {
    vec![
        asn_definition(),
        outbound_order_definition(),
        task_definition(),
    ]
}

fn asn_definition() -> StateMachineDefinition {
    StateMachineDefinition {
        machine_code: "asn".to_string(),
        machine_name: "M2 入库 ASN".to_string(),
        business_module: "M2".to_string(),
        version: "2026-07-10".to_string(),
        states: vec![
            state("pending_validation", "待校验", true, false),
            state("validation_error", "校验异常", false, false),
            state("pending_receipt", "待收货", false, false),
            state("receiving", "收货中", false, false),
            state("inspecting", "验收中", false, false),
            state("archive_replenishing", "档案补录中", false, false),
            state("putaway", "上架中", false, false),
            state("completed", "已完成", false, false),
            state("closed", "已关闭", false, true),
            state("cancelled", "已作废", false, true),
            state("closed_rejected", "已关闭_拒收", false, true),
            state("closed_shortage", "已关闭_短少", false, true),
        ],
        transitions: vec![
            transition(
                "pending_validation",
                "pending_receipt",
                "validation_passed",
                "校验通过",
            ),
            transition(
                "pending_validation",
                "validation_error",
                "validation_failed",
                "校验不通过",
            ),
            transition(
                "validation_error",
                "pending_validation",
                "manual_fix",
                "人工修改",
            ),
            transition(
                "pending_receipt",
                "receiving",
                "start_receiving",
                "开始收货",
            ),
            transition(
                "pending_receipt",
                "cancelled",
                "cancel_approved",
                "审批作废",
            ),
            transition("receiving", "inspecting", "receipt_submitted", "提交收货"),
            transition("receiving", "closed_rejected", "reject_all", "整单拒收"),
            transition(
                "inspecting",
                "putaway",
                "dual_sign_completed",
                "双人签字完成",
            ),
            transition(
                "inspecting",
                "closed_shortage",
                "force_close_shortage",
                "短少关闭",
            ),
            transition(
                "inspecting",
                "archive_replenishing",
                "archive_replenishment_required",
                "触发档案补录",
            ),
            transition(
                "archive_replenishing",
                "inspecting",
                "archive_synced",
                "ERP 同步完成",
            ),
            transition("putaway", "completed", "putaway_completed", "上架完成"),
            transition(
                "completed",
                "closed",
                "erp_feedback_succeeded",
                "ERP 反馈成功",
            ),
        ],
    }
}

fn outbound_order_definition() -> StateMachineDefinition {
    StateMachineDefinition {
        machine_code: "outbound_order".to_string(),
        machine_name: "M4 出库订单".to_string(),
        business_module: "M4".to_string(),
        version: "2026-07-10".to_string(),
        states: vec![
            state("pending_validation", "待校验", true, false),
            state("validation_exception", "校验异常", false, false),
            state("confirmed", "已确认", false, false),
            state("in_wave", "已入波次", false, false),
            state("inventory_locked", "库存锁定", false, false),
            state("reviewed", "已复核", false, false),
            state("shipped", "已发货", false, false),
            state("signed", "已签收", false, true),
            state("cancelled", "已作废", false, true),
            state("cancelled_rollback", "已作废_回退", false, true),
        ],
        transitions: vec![
            transition(
                "pending_validation",
                "confirmed",
                "validation_passed",
                "校验通过",
            ),
            transition(
                "pending_validation",
                "validation_exception",
                "validation_failed",
                "校验不通过",
            ),
            transition(
                "validation_exception",
                "pending_validation",
                "manual_fix",
                "人工修改",
            ),
            transition("confirmed", "in_wave", "wave_assigned", "进入波次"),
            transition("confirmed", "cancelled", "cancel_approved", "审批作废"),
            transition("in_wave", "inventory_locked", "start_picking", "开始拣选"),
            transition(
                "in_wave",
                "cancelled_rollback",
                "force_cancel_before_picking",
                "强制作废",
            ),
            transition(
                "inventory_locked",
                "reviewed",
                "review_completed",
                "复核完成",
            ),
            transition("reviewed", "shipped", "handover_confirmed", "发货交接"),
            transition("shipped", "signed", "customer_signed", "客户签收"),
        ],
    }
}

fn task_definition() -> StateMachineDefinition {
    StateMachineDefinition {
        machine_code: "warehouse_task".to_string(),
        machine_name: "M-TE 仓库作业任务".to_string(),
        business_module: "M-TE".to_string(),
        version: "2026-07-10".to_string(),
        states: vec![
            state("pending_release", "待释放", true, false),
            state("pending_assignment", "待分配", false, false),
            state("assigned", "已分配", false, false),
            state("dispatched", "已下发", false, false),
            state("in_progress", "执行中", false, false),
            state("exception", "异常", false, false),
            state("completed", "已完成", false, true),
            state("cancelled", "已取消", false, true),
        ],
        transitions: vec![
            transition(
                "pending_release",
                "pending_assignment",
                "release",
                "释放任务",
            ),
            transition(
                "pending_release",
                "cancelled",
                "source_cancelled",
                "业务源作废",
            ),
            transition("pending_assignment", "assigned", "assign", "任务分配"),
            transition("assigned", "pending_assignment", "reassign", "主管重分配"),
            transition("assigned", "dispatched", "pda_received", "PDA 接收"),
            transition("dispatched", "pending_assignment", "recall", "主管召回"),
            transition("dispatched", "in_progress", "start", "开始执行"),
            transition("in_progress", "completed", "complete", "操作完成"),
            transition("in_progress", "exception", "raise_exception", "业务异常"),
            transition("exception", "completed", "resolve_complete", "处置完成"),
            transition("exception", "cancelled", "resolve_cancel", "主管放弃"),
        ],
    }
}

fn state(code: &str, label: &str, is_initial: bool, is_terminal: bool) -> StateMachineState {
    StateMachineState {
        code: code.to_string(),
        label: label.to_string(),
        is_initial,
        is_terminal,
    }
}

fn transition(
    from_state: &str,
    to_state: &str,
    event_code: &str,
    label: &str,
) -> StateMachineTransition {
    StateMachineTransition {
        from_state: from_state.to_string(),
        to_state: to_state.to_string(),
        event_code: event_code.to_string(),
        label: label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::FromRequestParts;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;

    fn ctx(permissions: &[&str]) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            actor_name: "h6-tester".to_string(),
            permissions: permissions
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn h6_registry_contains_core_machine_definitions() {
        let definitions = state_machine_definitions();
        let machine_codes = definitions
            .iter()
            .map(|definition| definition.machine_code.as_str())
            .collect::<Vec<_>>();

        assert!(machine_codes.contains(&"asn"));
        assert!(machine_codes.contains(&"outbound_order"));
        assert!(machine_codes.contains(&"warehouse_task"));
        for definition in definitions {
            let state_codes = definition
                .states
                .iter()
                .map(|state| state.code.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                definition
                    .states
                    .iter()
                    .filter(|state| state.is_initial)
                    .count(),
                1,
                "state machine should have exactly one initial state: {}",
                definition.machine_code
            );
            assert!(
                definition.states.iter().any(|state| state.is_terminal),
                "state machine should have terminal state: {}",
                definition.machine_code
            );
            assert!(
                definition.transitions.iter().all(|transition| {
                    state_codes.contains(&transition.from_state.as_str())
                        && state_codes.contains(&transition.to_state.as_str())
                }),
                "state machine transitions should reference known states: {}",
                definition.machine_code
            );
        }

        let outbound =
            find_definition("outbound_order").expect("outbound order state machine should exist");
        let outbound_states = outbound
            .states
            .iter()
            .map(|state| state.code.as_str())
            .collect::<Vec<_>>();
        assert!(outbound_states.contains(&"validation_exception"));
        assert!(outbound_states.contains(&"in_wave"));
        assert!(!outbound_states.contains(&"validation_error"));
        assert!(!outbound_states.contains(&"waved"));
    }

    #[test]
    fn h6_transition_validation_allows_only_registered_edges() {
        let definition = find_definition("asn").expect("asn state machine should exist");
        let allowed = validate_transition(
            &definition,
            ValidateTransitionQuery {
                from_state: "pending_validation".to_string(),
                to_state: "pending_receipt".to_string(),
                event_code: Some("validation_passed".to_string()),
            },
        );
        assert!(allowed.allowed);

        let rejected = validate_transition(
            &definition,
            ValidateTransitionQuery {
                from_state: "pending_validation".to_string(),
                to_state: "completed".to_string(),
                event_code: None,
            },
        );
        assert!(!rejected.allowed);
        assert_eq!(rejected.reason.as_deref(), Some("非法状态转换"));
    }

    #[tokio::test]
    async fn h6_router_rejects_missing_authentication() {
        let response = state_machine_router()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/state-machines")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn h6_handlers_enforce_permission_and_unknown_machine() {
        let denied = list_state_machines_handler(ctx(&[]))
            .await
            .expect_err("missing permission should fail")
            .into_response();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let missing =
            get_state_machine_handler(ctx(&[READ_PERMISSION]), Path("missing".to_string()))
                .await
                .expect_err("unknown state machine should fail")
                .into_response();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn h6_transition_handler_returns_illegal_transition_as_data() {
        let response = validate_state_transition_handler(
            ctx(&[READ_PERMISSION]),
            Path("asn".to_string()),
            Ok(Query(ValidateTransitionQuery {
                from_state: "pending_validation".to_string(),
                to_state: "completed".to_string(),
                event_code: None,
            })),
        )
        .await
        .expect("known machine should return validation result")
        .0;

        assert!(!response.allowed);
        assert_eq!(response.reason.as_deref(), Some("非法状态转换"));
    }

    #[tokio::test]
    async fn h6_transition_handler_returns_json_error_for_missing_query_fields() {
        let (mut parts, _) = Request::builder()
            .uri("/api/v1/state-machines/asn/transition-validation?from_state=receiving")
            .body(Body::empty())
            .expect("request should build")
            .into_parts();
        let rejection = Query::<ValidateTransitionQuery>::from_request_parts(&mut parts, &())
            .await
            .expect_err("missing to_state should reject query extraction");

        let response = validate_state_transition_handler(
            ctx(&[READ_PERMISSION]),
            Path("asn".to_string()),
            Err(rejection),
        )
        .await
        .expect_err("malformed query should return domain error")
        .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
    }

    #[tokio::test]
    async fn h6_handlers_expose_all_core_definitions_for_api_consumers() {
        let response = list_state_machines_handler(ctx(&[READ_PERMISSION]))
            .await
            .expect("authorized consumer should list state machines")
            .0;

        for machine_code in ["asn", "outbound_order", "warehouse_task"] {
            assert!(response
                .data
                .iter()
                .any(|definition| definition.machine_code == machine_code));
            let detail =
                get_state_machine_handler(ctx(&[READ_PERMISSION]), Path(machine_code.to_string()))
                    .await
                    .expect("authorized consumer should read state machine detail")
                    .0;
            assert_eq!(detail.machine_code, machine_code);
        }
    }

    #[test]
    fn h6_read_permission_is_seeded_for_system_admin() {
        let migration =
            include_str!("../../../migrations/202607100001_h6_state_machine_permission.sql");

        assert!(migration.contains("h6.state_machine.read"));
        assert!(migration.contains("system_admin"));
    }
}
