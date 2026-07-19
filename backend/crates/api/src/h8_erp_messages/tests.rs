//! H8 消息内存仓储与重放规则测试。

use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
use wms_domain::{H8ErpMessage, H8ErpMessageAttempt};

use super::repository::MemoryH8ErpMessageRepository;
use super::state::H8ErpMessageAppState;

fn sample_message(owner: Uuid, status: &str) -> H8ErpMessage {
    let now = Utc::now();
    H8ErpMessage {
        id: Uuid::new_v4(),
        owner_id: owner,
        warehouse_id: None,
        connector_id: None,
        connector_code: Some("demo".into()),
        config_version: Some(1),
        direction: "inbound".into(),
        message_type: "asn".into(),
        channel: "rest".into(),
        external_ref: "ERP-ASN-1".into(),
        wms_resource_id: None,
        idempotency_key: "idem-1".into(),
        correlation_id: "corr-1".into(),
        sync_status: status.into(),
        retry_count: 2,
        next_retry_at: None,
        last_error_summary: Some("mapping failed".into()),
        payload_digest: "digest".into(),
        claimed_by: None,
        lease_expires_at: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
        acked_at: None,
    }
}

#[tokio::test]
async fn list_filters_by_status_and_stats() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let dead = sample_message(owner, "dead");
    let ok = sample_message(owner, "succeeded");
    state.repository.upsert_for_test(&dead).await.unwrap();
    state.repository.upsert_for_test(&ok).await.unwrap();
    let listed = state
        .repository
        .list(owner, None, None, Some("dead"), None, None)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].sync_status, "dead");
    let stats = state.repository.stats(owner).await.unwrap();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.dead, 1);
    assert_eq!(stats.succeeded, 1);
}

#[tokio::test]
async fn replay_failed_adds_attempt_and_sets_processing() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "failed");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let replayed = state
        .repository
        .replay(owner, id, "manual fix", "admin", Utc::now())
        .await
        .unwrap();
    assert_eq!(replayed.sync_status, "processing");
    let attempts = state.repository.list_attempts(owner, id).await.unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].result, "replayed");
}

#[tokio::test]
async fn replay_succeeded_rejected() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "succeeded");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let err = state
        .repository
        .replay(owner, id, "nope", "admin", Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        super::error::H8ErpMessageRepoError::Domain(wms_domain::H8MessageError::ReplayNotAllowed)
    ));
}

#[tokio::test]
async fn claim_pending_sets_lease_and_attempt() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "pending");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let claimed = state
        .repository
        .claim(owner, id, "worker-1", 60, Utc::now())
        .await
        .unwrap();
    assert_eq!(claimed.sync_status, "processing");
    assert_eq!(claimed.claimed_by.as_deref(), Some("worker-1"));
    assert!(claimed.lease_expires_at.is_some());
    let attempts = state.repository.list_attempts(owner, id).await.unwrap();
    assert_eq!(attempts[0].result, "claimed");
}

#[tokio::test]
async fn claim_under_active_lease_conflicts() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let mut msg = sample_message(owner, "processing");
    msg.lease_expires_at = Some(Utc::now() + chrono::Duration::minutes(5));
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let err = state
        .repository
        .claim(owner, id, "worker-2", 60, Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        super::error::H8ErpMessageRepoError::Domain(wms_domain::H8MessageError::LeaseConflict)
    ));
}

#[tokio::test]
async fn purge_requires_retention_policy() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let err = state
        .repository
        .purge_terminal(owner, None, Utc::now())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        super::error::H8ErpMessageRepoError::Domain(wms_domain::H8MessageError::FieldRequired(
            "retention_days"
        ))
    ));
}

#[tokio::test]
async fn purge_terminal_only_when_retention_set() {
    let memory = Arc::new(MemoryH8ErpMessageRepository::default());
    memory.set_retention_for_test(Uuid::nil(), 7);
    let state = H8ErpMessageAppState {
        repository: memory.clone(),
    };
    let owner = Uuid::nil();
    let mut old = sample_message(owner, "succeeded");
    old.updated_at = Utc::now() - chrono::Duration::days(30);
    let keep = sample_message(owner, "failed");
    state.repository.upsert_for_test(&old).await.unwrap();
    state.repository.upsert_for_test(&keep).await.unwrap();
    let (deleted, days) = state
        .repository
        .purge_terminal(owner, Some(7), Utc::now())
        .await
        .unwrap();
    assert_eq!(days, 7);
    assert_eq!(deleted, 1);
    assert!(state.repository.get(owner, keep.id).await.is_ok());
}

#[tokio::test]
async fn stats_include_p95_from_attempts() {
    let state = H8ErpMessageAppState::with_memory();
    let owner = Uuid::nil();
    let msg = sample_message(owner, "succeeded");
    let id = msg.id;
    state.repository.upsert_for_test(&msg).await.unwrap();
    let started = Utc::now() - chrono::Duration::milliseconds(200);
    let finished = Utc::now();
    state
        .repository
        .append_attempt_for_test(&H8ErpMessageAttempt {
            id: Uuid::new_v4(),
            message_id: id,
            attempt_no: 1,
            channel: "rest".into(),
            started_at: started,
            finished_at: Some(finished),
            result: "succeeded".into(),
            error_summary: None,
            actor: "worker".into(),
        })
        .await
        .unwrap();
    let stats = state.repository.stats(owner).await.unwrap();
    assert!(stats.p95_latency_ms >= 100);
}
