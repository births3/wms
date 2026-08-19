//! Shared, immutable M4 outbound order transition rules.
//!
//! H6 exposes these rules for read-only validation. M4 repositories use the
//! same table inside their PostgreSQL transactions before persisting a state.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct OutboundState {
    pub(crate) code: &'static str,
    pub(crate) label: &'static str,
    pub(crate) is_initial: bool,
    pub(crate) is_terminal: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct OutboundTransition {
    pub(crate) from_state: &'static str,
    pub(crate) to_state: &'static str,
    pub(crate) event_code: &'static str,
    pub(crate) label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutboundTransitionError {
    UnknownState,
    IllegalTransition,
}

pub(crate) const OUTBOUND_STATES: &[OutboundState] = &[
    OutboundState {
        code: "pending_validation",
        label: "待校验",
        is_initial: true,
        is_terminal: false,
    },
    OutboundState {
        code: "validation_exception",
        label: "校验异常",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "confirmed",
        label: "已确认",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "void_requested",
        label: "作废申请中",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "in_wave",
        label: "已入波次",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "inventory_locked",
        label: "库存锁定",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "picked",
        label: "已拣货",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "picked_short",
        label: "已拣货_短拣",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "reviewed",
        label: "已复核",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "reviewed_short",
        label: "已复核_短拣",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "shipped",
        label: "已发货",
        is_initial: false,
        is_terminal: false,
    },
    OutboundState {
        code: "signed",
        label: "已签收",
        is_initial: false,
        is_terminal: true,
    },
    OutboundState {
        code: "cancelled",
        label: "已作废",
        is_initial: false,
        is_terminal: true,
    },
    OutboundState {
        code: "cancelled_rollback",
        label: "已作废_回退",
        is_initial: false,
        is_terminal: true,
    },
];

pub(crate) const OUTBOUND_TRANSITIONS: &[OutboundTransition] = &[
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
    transition(
        "pending_validation",
        "void_requested",
        "void_requested",
        "作废申请",
    ),
    transition(
        "validation_exception",
        "void_requested",
        "void_requested",
        "作废申请",
    ),
    transition("confirmed", "void_requested", "void_requested", "作废申请"),
    transition("void_requested", "cancelled", "cancel_approved", "审批作废"),
    transition("in_wave", "inventory_locked", "start_picking", "开始拣选"),
    transition(
        "in_wave",
        "cancelled_rollback",
        "force_cancel_before_picking",
        "强制作废",
    ),
    transition("inventory_locked", "picked", "pick_completed", "拣货完成"),
    transition(
        "inventory_locked",
        "picked_short",
        "short_pick_recorded",
        "短拣记录",
    ),
    transition("in_wave", "picked", "pick_completed", "拣货完成"),
    transition("in_wave", "picked_short", "short_pick_recorded", "短拣记录"),
    transition("picked", "picked", "pick_completed", "拣货完成"),
    transition("picked", "picked_short", "short_pick_recorded", "短拣记录"),
    transition(
        "picked_short",
        "picked_short",
        "short_pick_replenished",
        "补拣记录",
    ),
    transition(
        "picked_short",
        "picked",
        "short_pick_replenished",
        "补拣完成",
    ),
    transition(
        "reviewed_short",
        "picked_short",
        "short_pick_replenished",
        "补拣记录",
    ),
    transition(
        "reviewed_short",
        "picked",
        "short_pick_replenished",
        "补拣完成",
    ),
    transition("picked", "reviewed", "review_completed", "复核完成"),
    transition(
        "picked",
        "reviewed_short",
        "review_completed",
        "复核完成_短拣",
    ),
    transition("picked_short", "reviewed", "review_completed", "复核完成"),
    transition(
        "picked_short",
        "reviewed_short",
        "review_completed",
        "复核完成_短拣",
    ),
    transition("reviewed", "shipped", "handover_confirmed", "发货交接"),
    transition("shipped", "signed", "customer_signed", "客户签收"),
];

pub(crate) fn validate_outbound_transition(
    from_state: &str,
    to_state: &str,
    event_code: &str,
) -> Result<(), OutboundTransitionError> {
    let known_from = OUTBOUND_STATES.iter().any(|state| state.code == from_state);
    let known_to = OUTBOUND_STATES.iter().any(|state| state.code == to_state);
    if !known_from || !known_to {
        return Err(OutboundTransitionError::UnknownState);
    }
    if OUTBOUND_TRANSITIONS.iter().any(|transition| {
        transition.from_state == from_state
            && transition.to_state == to_state
            && transition.event_code == event_code
    }) {
        Ok(())
    } else {
        Err(OutboundTransitionError::IllegalTransition)
    }
}

pub(crate) fn pick_transition_event(from_state: &str, to_state: &str) -> &'static str {
    match (from_state, to_state) {
        ("picked_short" | "reviewed_short", "picked" | "picked_short") => "short_pick_replenished",
        (_, "picked_short") => "short_pick_recorded",
        _ => "pick_completed",
    }
}

const fn transition(
    from_state: &'static str,
    to_state: &'static str,
    event_code: &'static str,
    label: &'static str,
) -> OutboundTransition {
    OutboundTransition {
        from_state,
        to_state,
        event_code,
        label,
    }
}

#[cfg(test)]
mod tests {
    use crate::outbound::{
        OUTBOUND_STATUS_CONFIRMED, OUTBOUND_STATUS_IN_WAVE, OUTBOUND_STATUS_PENDING_VALIDATION,
        OUTBOUND_STATUS_REVIEWED, OUTBOUND_STATUS_REVIEWED_SHORT, OUTBOUND_STATUS_SHIPPED,
        OUTBOUND_STATUS_VALIDATION_EXCEPTION, OUTBOUND_STATUS_VOID_REQUESTED,
    };

    use super::{pick_transition_event, validate_outbound_transition, OutboundTransitionError};

    #[test]
    fn validates_the_short_pick_branch_and_rejects_skips() {
        assert_eq!(
            pick_transition_event("in_wave", "picked_short"),
            "short_pick_recorded"
        );
        assert_eq!(
            pick_transition_event("reviewed_short", "picked"),
            "short_pick_replenished"
        );
        assert_eq!(
            validate_outbound_transition("picked_short", "reviewed_short", "review_completed"),
            Ok(())
        );
        assert_eq!(
            validate_outbound_transition("in_wave", "shipped", "handover_confirmed"),
            Err(OutboundTransitionError::IllegalTransition)
        );
        assert_eq!(
            validate_outbound_transition("missing", "picked", "pick_completed"),
            Err(OutboundTransitionError::UnknownState)
        );
    }

    #[test]
    fn h6_registry_covers_every_persisted_m4_order_status() {
        let persisted_statuses = [
            OUTBOUND_STATUS_PENDING_VALIDATION,
            OUTBOUND_STATUS_VALIDATION_EXCEPTION,
            OUTBOUND_STATUS_CONFIRMED,
            OUTBOUND_STATUS_VOID_REQUESTED,
            OUTBOUND_STATUS_IN_WAVE,
            "picked",
            "picked_short",
            OUTBOUND_STATUS_REVIEWED,
            OUTBOUND_STATUS_REVIEWED_SHORT,
            OUTBOUND_STATUS_SHIPPED,
        ];

        for status in persisted_statuses {
            assert!(
                super::OUTBOUND_STATES
                    .iter()
                    .any(|state| state.code == status),
                "H6 outbound definition is missing persisted M4 status {status}"
            );
        }
    }
}
