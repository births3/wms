use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::audit::{append_event, seal_audit_chain, AuditDiff, AuditWriteRequest};

fn audit_request(resource_id: &str, action: &str, diff: Option<AuditDiff>) -> AuditWriteRequest {
    AuditWriteRequest {
        occurred_at: Utc::now(),
        actor_id: Uuid::new_v4(),
        actor_name: "alice".to_string(),
        owner_id: Uuid::new_v4(),
        jti: format!("jti-{resource_id}-{action}"),
        action: action.to_string(),
        module: "H2".to_string(),
        resource_type: "audit_test_item".to_string(),
        resource_id: resource_id.to_string(),
        diff,
        request_id: Some(Uuid::new_v4()),
        ip: Some("127.0.0.1".to_string()),
        user_agent: Some("wms-api-test".to_string()),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn append_event_and_seal_chain_use_real_postgres(pool: PgPool) {
    let first_req = audit_request("ITEM-1", "create", None);
    let first = append_event(&pool, &first_req)
        .await
        .expect("first audit event should insert");

    let diff = AuditDiff::compute(
        serde_json::json!({"name": "item-a"}),
        serde_json::json!({"name": "item-b"}),
    );
    let mut second_req = audit_request("ITEM-1", "update", Some(diff));
    second_req.occurred_at = first_req.occurred_at + chrono::Duration::milliseconds(1);
    second_req.actor_id = first_req.actor_id;
    second_req.actor_name = first_req.actor_name.clone();
    second_req.owner_id = first_req.owner_id;
    let second = append_event(&pool, &second_req)
        .await
        .expect("second audit event should insert");

    assert_eq!(first.prev_hash, None);
    assert_eq!(second.prev_hash.as_deref(), Some(first.self_hash.as_str()));
    assert_eq!(first.owner_id, first_req.owner_id);
    assert_eq!(
        second
            .diff
            .as_ref()
            .expect("diff should exist")
            .changed_keys,
        vec!["name".to_string()]
    );

    let seal_date = first_req.occurred_at.date_naive();
    let seal = seal_audit_chain(&pool, seal_date, Utc::now())
        .await
        .expect("valid hash chain should seal");

    assert_eq!(seal.seal_date, seal_date);
    assert_eq!(seal.last_id, second.id);
    assert_eq!(seal.last_self_hash, second.self_hash);

    let duplicate = seal_audit_chain(&pool, seal_date, Utc::now()).await;

    assert!(
        duplicate.is_err(),
        "audit_chain_seal must be insert-only and reject duplicate seals"
    );
}
