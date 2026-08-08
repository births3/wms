use h8_erp_worker::{
    outbound::OutboxRow,
    outbound_runner::{effective_message_type, lifecycle_body, RouteBinding},
};
use serde_json::json;
use uuid::Uuid;

fn source(event_type: &str) -> OutboxRow {
    OutboxRow {
        table: "receiving_putaway_erp_feedback_outbox",
        id: Uuid::nil(),
        owner_id: Uuid::nil(),
        event_type: event_type.to_owned(),
        payload: json!({"correlation_id":"corr-1"}),
        external_ref: "order-1".to_owned(),
        attempt_count: 1,
        max_attempts: 5,
        created_at: "2026-08-05T00:00:00.000Z".to_owned(),
    }
}

#[test]
fn order_status_uses_its_own_catalog_message_type() {
    assert_eq!(
        effective_message_type("putaway_complete", &source("order_status")),
        "order_status"
    );
    assert_eq!(
        effective_message_type("putaway_complete", &source("inbound_putaway_completed")),
        "putaway_complete"
    );
}

#[test]
fn lifecycle_binding_is_stable_across_stages() {
    let row = source("order_status");
    let body = lifecycle_body(
        &row,
        "order_status",
        "receive",
        "ok",
        None,
        &RouteBinding {
            connector_id: "334c3ff7-1018-40c6-b1f4-c19b2d2c88e5".to_owned(),
            connector_code: "connector-1".to_owned(),
            config_version: 7,
        },
    );
    assert_eq!(
        body["idempotency_key"],
        format!("out:{}:{}", row.table, row.id)
    );
    assert_eq!(body["correlation_id"], "corr-1");
    assert_eq!(body["channel"], "interface_table");
    assert_eq!(body["config_version"], 7);
    assert!(body.get("payload").is_some());
}
