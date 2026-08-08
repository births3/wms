use chrono::{Duration, Utc};
use h8_erp_worker::{
    error::WorkerError,
    mssql::MarkStatus,
    runner::{failure_decision, FailureDecision},
};

#[test]
fn invalid_payload_is_dead_without_retry() {
    let decision = failure_decision(
        &WorkerError::new("INVALID_DATA", "digest mismatch"),
        0,
        5,
        Utc::now(),
    );
    assert_eq!(
        decision,
        FailureDecision {
            status: MarkStatus::Dead,
            retry_count: 0
        }
    );
}

#[test]
fn rejected_required_mapping_is_dead_without_retry() {
    assert_eq!(
        failure_decision(
            &WorkerError::new("H8_WORKER_HTTP_REJECTED", "422"),
            0,
            5,
            Utc::now(),
        ),
        FailureDecision {
            status: MarkStatus::Dead,
            retry_count: 0,
        }
    );
}

#[test]
fn transient_failure_retries_then_dies_at_limit() {
    let inserted = Utc::now();
    assert_eq!(
        failure_decision(
            &WorkerError::new("H8_WORKER_HTTP_RETRYABLE", "503"),
            3,
            5,
            inserted
        ),
        FailureDecision {
            status: MarkStatus::Retry,
            retry_count: 4
        }
    );
    assert_eq!(
        failure_decision(
            &WorkerError::new("H8_WORKER_HTTP_RETRYABLE", "503"),
            4,
            5,
            inserted
        ),
        FailureDecision {
            status: MarkStatus::Dead,
            retry_count: 5
        }
    );
}

#[test]
fn order_not_ready_has_independent_30_minute_timeout() {
    let error = WorkerError::new("ORDER_NOT_READY", "not created");
    assert_eq!(
        failure_decision(&error, 0, 5, Utc::now() - Duration::minutes(29)),
        FailureDecision {
            status: MarkStatus::Retry,
            retry_count: 0
        }
    );
    assert_eq!(
        failure_decision(&error, 0, 5, Utc::now() - Duration::minutes(31)),
        FailureDecision {
            status: MarkStatus::Dead,
            retry_count: 0
        }
    );
}
