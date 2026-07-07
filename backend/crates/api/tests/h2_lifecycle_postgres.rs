use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::{append_event, AuditWriteRequest},
    h2_lifecycle::{
        pending_event_deliveries, plan_business_archive_job, publish_event,
        record_delivery_failure, run_audit_archive_cycle, seed_default_business_retention_policies,
        upsert_event_subscription, DeliveryStatus,
    },
};

#[sqlx::test(migrations = "../../migrations")]
async fn audit_archive_cycle_is_idempotent_and_audited(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 1, 1, 0, 0)
        .single()
        .expect("fixed UTC archive timestamp should be valid");
    for partition_start in ["2020-01-01", "2025-01-01"] {
        sqlx::query("SELECT create_audit_partition($1)")
            .bind(
                chrono::NaiveDate::parse_from_str(partition_start, "%Y-%m-%d")
                    .expect("fixed partition date should parse"),
            )
            .execute(&pool)
            .await
            .expect("partition should create");
    }

    let first = run_audit_archive_cycle(&pool, owner_id, now.date_naive(), now, "archive-cycle-1")
        .await
        .expect("archive run should complete");
    let second = run_audit_archive_cycle(&pool, owner_id, now.date_naive(), now, "archive-cycle-1")
        .await
        .expect("archive run should replay");

    assert_eq!(first.id, second.id);
    assert!(first.partitions_seen >= 2);
    assert!(first.partitions_archived >= 2);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'audit.archive.run'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should query");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn event_bus_delivers_by_pattern_and_dead_letters_after_retries(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 1, 2, 0, 0)
        .single()
        .expect("fixed UTC event timestamp should be valid");
    upsert_event_subscription(
        &pool,
        owner_id,
        "mvr-rule-engine",
        "business.inventory.*",
        true,
        now,
    )
    .await
    .expect("subscription should upsert");

    let event = publish_event(
        &pool,
        owner_id,
        "event-1",
        "business.inventory.status_change",
        "M3",
        "inventory_batch",
        "BATCH-1",
        serde_json::json!({"status": "qualified"}),
        now,
    )
    .await
    .expect("event should publish");
    let replay = publish_event(
        &pool,
        owner_id,
        "event-1",
        "business.inventory.status_change",
        "M3",
        "inventory_batch",
        "BATCH-1",
        serde_json::json!({"status": "qualified"}),
        now,
    )
    .await
    .expect("event should replay by idempotency");

    assert_eq!(event.id, replay.id);
    assert_eq!(event.delivery_count, 1);

    let delivery = pending_event_deliveries(&pool, owner_id, 10)
        .await
        .expect("pending deliveries should list")
        .into_iter()
        .find(|delivery| delivery.event_id == event.id)
        .expect("business event delivery should exist");
    let delivery = record_delivery_failure(&pool, owner_id, delivery.id, "timeout", now)
        .await
        .expect("first failure should keep pending");
    assert_eq!(delivery.status, DeliveryStatus::Pending);
    let delivery = record_delivery_failure(&pool, owner_id, delivery.id, "timeout", now)
        .await
        .expect("second failure should keep pending");
    assert_eq!(delivery.status, DeliveryStatus::Pending);
    let delivery = record_delivery_failure(&pool, owner_id, delivery.id, "timeout", now)
        .await
        .expect("third failure should dead letter");

    assert_eq!(delivery.status, DeliveryStatus::DeadLetter);
    assert_eq!(delivery.attempt_count, 3);
}

#[sqlx::test(migrations = "../../migrations")]
async fn audit_write_publishes_audit_event_to_subscribers(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 1, 3, 0, 0)
        .single()
        .expect("fixed UTC audit timestamp should be valid");
    upsert_event_subscription(
        &pool,
        owner_id,
        "audit-reader",
        "audit.audit_test_item.*",
        true,
        now,
    )
    .await
    .expect("subscription should upsert");

    append_event(
        &pool,
        &AuditWriteRequest {
            occurred_at: now,
            actor_id: Uuid::new_v4(),
            actor_name: "alice".to_string(),
            owner_id,
            jti: "jti-h2".to_string(),
            action: "create".to_string(),
            module: "H2".to_string(),
            resource_type: "audit_test_item".to_string(),
            resource_id: "ITEM-1".to_string(),
            diff: None,
            request_id: None,
            ip: None,
            user_agent: None,
        },
    )
    .await
    .expect("audit event should append");

    let outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM event_bus_event WHERE owner_id = $1 AND event_type = 'audit.audit_test_item.create'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("outbox count should query");
    let delivery_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
          FROM event_bus_delivery d
          JOIN event_bus_event e ON e.id = d.event_id
         WHERE e.owner_id = $1
           AND e.event_type = 'audit.audit_test_item.create'
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("delivery count should query");

    assert_eq!(outbox_count, 1);
    assert_eq!(delivery_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn business_retention_policy_plans_archive_without_delete(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 1, 4, 0, 0)
        .single()
        .expect("fixed UTC retention timestamp should be valid");
    let policies = seed_default_business_retention_policies(&pool, owner_id, now)
        .await
        .expect("default policies should seed");
    assert!(policies
        .iter()
        .any(|policy| policy.policy_code == "special_drug_ledger"
            && policy.retention_years == Some(30)));

    let job = plan_business_archive_job(
        &pool,
        owner_id,
        "special_drug_ledger",
        "special_drug_flow_ledger",
        now.date_naive(),
        now,
        "business-archive-1",
    )
    .await
    .expect("archive job should plan");
    let replay = plan_business_archive_job(
        &pool,
        owner_id,
        "special_drug_ledger",
        "special_drug_flow_ledger",
        now.date_naive(),
        now,
        "business-archive-1",
    )
    .await
    .expect("archive job should replay");
    let skipped = plan_business_archive_job(
        &pool,
        owner_id,
        "master_data",
        "products",
        now.date_naive(),
        now,
        "business-archive-2",
    )
    .await
    .expect("master data should skip");

    assert_eq!(job.id, replay.id);
    assert_eq!(job.target_layer, "archive");
    assert_eq!(job.status, "planned");
    assert!(!job.delete_allowed);
    assert_eq!(skipped.status, "skipped");
    assert_eq!(skipped.target_layer, "skip");
}
