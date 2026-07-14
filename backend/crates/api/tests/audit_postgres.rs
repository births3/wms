use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::audit::{
    append_event, export_events, list_events, seal_audit_chain, AuditDiff, AuditEventQuery,
    AuditWriteRequest,
};

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

#[sqlx::test(migrations = "../../migrations")]
async fn list_events_filters_action_resource_owner_and_paginates_with_ip(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let base = Utc::now();
    for (owner, action, resource_id, offset, ip) in [
        (owner_id, "create", "ORDER-1", 0_i64, "192.0.2.1"),
        (owner_id, "create", "ORDER-1", 1, "192.0.2.2"),
        (owner_id, "update", "ORDER-1", 2, "192.0.2.3"),
        (owner_id, "create", "ORDER-2", 3, "192.0.2.4"),
        (other_owner_id, "create", "ORDER-1", 4, "192.0.2.5"),
    ] {
        let mut request = audit_request(resource_id, action, None);
        request.owner_id = owner;
        request.occurred_at = base + chrono::Duration::milliseconds(offset);
        request.ip = Some(ip.to_string());
        append_event(&pool, &request)
            .await
            .expect("audit event should insert");
    }

    let mut query = AuditEventQuery {
        owner_id,
        resource_type: Some("audit_test_item".to_string()),
        action: Some("create".to_string()),
        resource_id: Some("ORDER-1".to_string()),
        product_code: None,
        batch_no: None,
        actor_id: None,
        from: None,
        to: None,
        cursor: None,
        limit: 1,
    };
    let first = list_events(&pool, &query)
        .await
        .expect("filtered audit query should succeed");
    assert_eq!(first.events.len(), 1);
    assert_eq!(first.events[0].owner_id, owner_id);
    assert_eq!(first.events[0].ip.as_deref(), Some("192.0.2.2"));
    query.cursor = first.next_cursor;
    let second = list_events(&pool, &query)
        .await
        .expect("filtered audit query should paginate");
    assert_eq!(second.events.len(), 1);
    assert_eq!(second.events[0].resource_id, "ORDER-1");
    assert_eq!(second.events[0].ip.as_deref(), Some("192.0.2.1"));
    assert!(second.next_cursor.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_events_filters_product_and_batch_from_diff_values(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let mut matching = audit_request(
        "ORDER-M3-1",
        "update",
        Some(AuditDiff::compute(
            serde_json::json!({"product_code": "P-001", "batch_no": "B-001"}),
            serde_json::json!({"product_code": "P-001", "batch_no": "B-002"}),
        )),
    );
    matching.owner_id = owner_id;
    matching.occurred_at = Utc::now();
    append_event(&pool, &matching)
        .await
        .expect("matching audit event should insert");

    let mut other = audit_request(
        "ORDER-M3-2",
        "update",
        Some(AuditDiff::compute(
            serde_json::json!({"product_code": "P-002", "batch_no": "B-003"}),
            serde_json::json!({"product_code": "P-002", "batch_no": "B-004"}),
        )),
    );
    other.owner_id = owner_id;
    other.occurred_at = matching.occurred_at + chrono::Duration::milliseconds(1);
    append_event(&pool, &other)
        .await
        .expect("other audit event should insert");

    let query = AuditEventQuery {
        owner_id,
        resource_type: None,
        action: Some("update".to_string()),
        resource_id: None,
        product_code: Some("P-001".to_string()),
        batch_no: Some("B-002".to_string()),
        actor_id: None,
        from: None,
        to: None,
        cursor: None,
        limit: 100,
    };
    let result = list_events(&pool, &query)
        .await
        .expect("product and batch filters should succeed");
    assert_eq!(result.events.len(), 1);
    assert_eq!(result.events[0].resource_id, "ORDER-M3-1");
}

#[sqlx::test(migrations = "../../migrations")]
async fn export_events_walks_all_matching_pages_without_cross_owner_rows(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let base = Utc::now();
    for index in 0..205_i64 {
        let mut request = audit_request(&format!("ORDER-{index}"), "create", None);
        request.owner_id = owner_id;
        request.occurred_at = base + chrono::Duration::milliseconds(index);
        append_event(&pool, &request)
            .await
            .expect("audit event should insert");
    }
    let mut other = audit_request("OTHER-ORDER", "create", None);
    other.owner_id = other_owner_id;
    other.occurred_at = base + chrono::Duration::milliseconds(206);
    append_event(&pool, &other)
        .await
        .expect("other owner audit event should insert");

    let query = AuditEventQuery {
        owner_id,
        resource_type: Some("audit_test_item".to_string()),
        action: Some("create".to_string()),
        resource_id: None,
        product_code: None,
        batch_no: None,
        actor_id: None,
        from: None,
        to: None,
        cursor: None,
        limit: 1,
    };
    let events = export_events(&pool, &query)
        .await
        .expect("export should walk all matching pages");
    assert_eq!(events.len(), 205);
    assert!(events.iter().all(|event| event.owner_id == owner_id));
    assert_eq!(
        events.first().map(|event| event.resource_id.as_str()),
        Some("ORDER-204")
    );
    assert_eq!(
        events.last().map(|event| event.resource_id.as_str()),
        Some("ORDER-0")
    );
}
