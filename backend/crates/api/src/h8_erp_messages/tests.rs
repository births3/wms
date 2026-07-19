//! H8 消息内存仓储与重放规则测试。

use chrono::Utc;
use uuid::Uuid;
use wms_domain::H8ErpMessage;

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
