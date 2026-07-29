use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension,
};
use chrono::Utc;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    reconciliation::{
        ErpInventorySnapshotItem, PgReconciliationRepository, ReconciliationDisposition,
        ReconciliationError, ReconciliationInventoryAllocation, RunReconciliationRequest,
    },
    reconciliation_handlers::{reconciliation_router, ReconciliationAppState},
    stock_adjustment::PgStockAdjustmentRepository,
};

#[path = "reconciliation_postgres/keyset.rs"]
mod keyset;
#[path = "reconciliation_postgres/support.rs"]
mod support;

use support::{seed_active_claim, seed_batch};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "rc-test".into(),
        permissions: vec!["rc.reconciliation.execute".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn reconciliation_http_enforces_permissions_and_rule_contract(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'RC API 货主')",
    )
    .bind(owner_id)
    .bind(format!("RC-API-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .unwrap();
    let denied = reconciliation_router(ReconciliationAppState::with_postgres(pool.clone()))
        .layer(Extension(ctx(owner_id)));
    let response = denied
        .oneshot(
            Request::builder()
                .uri("/api/v1/reconciliation/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let mut executor = ctx(owner_id);
    executor.permissions = vec!["rc.reconciliation.execute".into()];
    let app = reconciliation_router(ReconciliationAppState::with_postgres(pool.clone()))
        .layer(Extension(executor));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/reconciliation/rule")
                .header("content-type", "application/json")
                .header("idempotency-key", "rc-rule-1")
                .body(Body::from(r#"{"interval_hours":6,"enabled":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let rule: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rule["interval_hours"], 6);
    let method: String = sqlx::query_scalar(
        "SELECT method FROM idempotency_request
          WHERE owner_id = $1 AND idempotency_key = 'rc-rule-1'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(method, "PUT");

    let due_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reconciliation/claims")
                .header("content-type", "application/json")
                .header("idempotency-key", "rc-claim-must-be-service-only")
                .body(Body::from(
                    r#"{"worker_id":"warehouse-manager","lease_seconds":120}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(due_response.status(), StatusCode::FORBIDDEN);

    let run_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reconciliation/runs")
                .header("content-type", "application/json")
                .header("idempotency-key", "rc-run-must-be-service-only")
                .body(Body::from(
                    r#"{"claim_id":"10000000-0000-0000-0000-000000000001","claim_token":"20000000-0000-0000-0000-000000000002","window_key":"service-only","snapshot_at":"2026-07-23T18:00:00Z","items":[]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(run_response.status(), StatusCode::FORBIDDEN);

    let mut ingestor = ctx(owner_id);
    ingestor.permissions = vec!["rc.reconciliation.ingest".into()];
    let ingest_app = reconciliation_router(ReconciliationAppState::with_postgres(pool.clone()))
        .layer(Extension(ingestor));
    let due_response = ingest_app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reconciliation/claims")
                .header("content-type", "application/json")
                .header("idempotency-key", "rc-service-claim-none")
                .body(Body::from(
                    r#"{"worker_id":"service-worker","lease_seconds":120}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(due_response.status(), StatusCode::OK);
    let run_response = ingest_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reconciliation/runs")
                .header("content-type", "application/json")
                .header("idempotency-key", "rc-service-ingest-1")
                .body(Body::from(
                    r#"{"window_key":"service-only","snapshot_at":"2026-07-23T18:00:00Z","items":[]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(run_response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn reconciliation_compares_persists_notifies_audits_and_replays(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'RC 测试货主')",
    )
    .bind(owner_id)
    .bind(format!("RC-{}", &owner_id.simple().to_string()[..8]))
    .execute(&pool)
    .await
    .unwrap();
    let actor = ctx(owner_id);
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status)
         VALUES ($1, $2, 'RC 测试主管', 'test-hash', 'active')",
    )
    .bind(actor.user_id)
    .bind(format!(
        "rc-user-{}",
        &actor.user_id.simple().to_string()[..8]
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id)
         VALUES ($1, $2)",
    )
    .bind(actor.user_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();
    let custodian_role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND role_code = 'custodian'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO auth_user_roles (user_id, owner_id, role_id)
         VALUES ($1, $2, $3)",
    )
    .bind(actor.user_id)
    .bind(owner_id)
    .bind(custodian_role_id)
    .execute(&pool)
    .await
    .unwrap();
    seed_batch(&pool, owner_id, "P1", "B1", 10).await;
    seed_batch(&pool, owner_id, "P1", "B1", 5).await;
    let (p3_batch_id, _) = seed_batch(&pool, owner_id, "P3", "B3", 7).await;
    let (p3_second_batch_id, _) = seed_batch(&pool, owner_id, "P3", "B3", 2).await;
    let (p2_batch_id, _) = seed_batch(&pool, owner_id, "P2", "B2", 0).await;
    let (p4_batch_id, _) = seed_batch(&pool, owner_id, "P4", "B4", 2).await;

    let repository = PgReconciliationRepository::new(pool.clone());
    let (claim_id, claim_token) = seed_active_claim(&pool, owner_id, "2026-07-23T18").await;
    let request = RunReconciliationRequest {
        claim_id,
        claim_token,
        window_key: "2026-07-23T18".into(),
        snapshot_at: Utc::now(),
        items: vec![
            ErpInventorySnapshotItem {
                product_code: "P1".into(),
                batch_no: "B1".into(),
                qty_on_hand: 12,
            },
            ErpInventorySnapshotItem {
                product_code: "P2".into(),
                batch_no: "B2".into(),
                qty_on_hand: 4,
            },
            ErpInventorySnapshotItem {
                product_code: "P4".into(),
                batch_no: "B4".into(),
                qty_on_hand: 1,
            },
        ],
    };
    let first = repository
        .run(&actor, request.clone(), Utc::now(), "rc-run-1")
        .await
        .unwrap();
    assert!(!first.replayed);
    assert_eq!(first.value.matched_count, 0);
    assert_eq!(first.value.wms_more_count, 3);
    assert_eq!(first.value.erp_more_count, 1);
    assert_eq!(first.value.items.len(), 4);
    assert!(first
        .value
        .items
        .iter()
        .any(|item| item.product_code == "P1"
            && item.wms_qty == 15
            && item.erp_qty == 12
            && item.difference_qty == 3
            && item.difference_type == "wms_more"));

    let (runs, items, notifications, audits, status_changes): (i64, i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT
             (SELECT COUNT(*) FROM reconciliation_runs WHERE owner_id = $1),
             (SELECT COUNT(*) FROM reconciliation_items WHERE owner_id = $1),
             (SELECT COUNT(*) FROM h4_notification_records
               WHERE owner_id = $1 AND event_type = 'rc.reconciliation.difference'),
             (SELECT COUNT(*) FROM audit_event
               WHERE owner_id = $1 AND action = 'run_reconciliation'),
             (SELECT COUNT(*) FROM inventory_status_changes
               WHERE owner_id = $1 AND approval_source = 'reconciliation')",
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        (runs, items, notifications, audits, status_changes),
        (1, 4, 1, 1, 0)
    );

    let replay = repository
        .run(&actor, request, Utc::now(), "rc-run-1")
        .await
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.value.id, first.value.id);
    let (same_claim_id, same_claim_token) =
        seed_active_claim(&pool, owner_id, "2026-07-23T18").await;
    let same_window = repository
        .run(
            &actor,
            RunReconciliationRequest {
                claim_id: same_claim_id,
                claim_token: same_claim_token,
                window_key: "2026-07-23T18".into(),
                snapshot_at: first.value.snapshot_at,
                items: vec![
                    ErpInventorySnapshotItem {
                        product_code: "P4".into(),
                        batch_no: "B4".into(),
                        qty_on_hand: 1,
                    },
                    ErpInventorySnapshotItem {
                        product_code: "P2".into(),
                        batch_no: "B2".into(),
                        qty_on_hand: 4,
                    },
                    ErpInventorySnapshotItem {
                        product_code: "P1".into(),
                        batch_no: "B1".into(),
                        qty_on_hand: 12,
                    },
                ],
            },
            Utc::now(),
            "rc-run-same-window",
        )
        .await
        .unwrap();
    assert!(same_window.replayed);
    assert_eq!(same_window.value.id, first.value.id);
    let (conflict_claim_id, conflict_claim_token) =
        seed_active_claim(&pool, owner_id, "2026-07-23T18").await;
    let conflict = repository
        .run(
            &actor,
            RunReconciliationRequest {
                claim_id: conflict_claim_id,
                claim_token: conflict_claim_token,
                window_key: "2026-07-23T18".into(),
                snapshot_at: first.value.snapshot_at,
                items: vec![ErpInventorySnapshotItem {
                    product_code: "P1".into(),
                    batch_no: "B1".into(),
                    qty_on_hand: 13,
                }],
            },
            Utc::now(),
            "rc-run-conflicting-window",
        )
        .await
        .unwrap_err();
    assert_eq!(conflict, ReconciliationError::IdempotencyConflict);

    let p1 = first
        .value
        .items
        .iter()
        .find(|item| item.product_code == "P1")
        .unwrap();
    let isolated = repository
        .set_isolation(&actor, &[p1.id, p1.id], true, Utc::now(), "rc-isolate-1")
        .await
        .unwrap();
    assert_eq!(isolated.value, 2);
    let isolated_resource_id: String = sqlx::query_scalar(
        "SELECT resource_id
           FROM audit_event
          WHERE owner_id = $1 AND action = 'isolate_reconciliation_items'
          ORDER BY occurred_at DESC
          LIMIT 1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(isolated_resource_id, p1.id.to_string());
    let isolated_replay = repository
        .set_isolation(&actor, &[p1.id], true, Utc::now(), "rc-isolate-1")
        .await
        .unwrap();
    assert!(isolated_replay.replayed);

    let second_window = "2026-07-24T00";
    let (second_claim_id, second_claim_token) =
        seed_active_claim(&pool, owner_id, second_window).await;
    let second = repository
        .run(
            &actor,
            RunReconciliationRequest {
                claim_id: second_claim_id,
                claim_token: second_claim_token,
                window_key: second_window.into(),
                snapshot_at: Utc::now(),
                items: vec![
                    ErpInventorySnapshotItem {
                        product_code: "P1".into(),
                        batch_no: "B1".into(),
                        qty_on_hand: 12,
                    },
                    ErpInventorySnapshotItem {
                        product_code: "P2".into(),
                        batch_no: "B2".into(),
                        qty_on_hand: 4,
                    },
                    ErpInventorySnapshotItem {
                        product_code: "P4".into(),
                        batch_no: "B4".into(),
                        qty_on_hand: 1,
                    },
                ],
            },
            Utc::now(),
            "rc-run-cross-window-lock",
        )
        .await
        .unwrap();
    let second_p1 = second
        .value
        .items
        .iter()
        .find(|item| item.product_code == "P1")
        .unwrap();
    assert_eq!(
        repository
            .set_isolation(
                &actor,
                &[second_p1.id],
                true,
                Utc::now(),
                "rc-isolate-cross-window",
            )
            .await
            .unwrap()
            .value,
        2
    );
    assert_eq!(
        repository
            .set_isolation(
                &actor,
                &[p1.id],
                false,
                Utc::now(),
                "rc-release-before-resolve",
            )
            .await
            .unwrap_err(),
        ReconciliationError::InvalidRequest
    );
    let (still_quarantined, second_active_locks): (i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM inventory_batches
              WHERE owner_id=$1 AND product_code='P1' AND batch_no='B1'
                AND quality_status='quarantined'),
            (SELECT COUNT(*) FROM reconciliation_item_locks
              WHERE owner_id=$1 AND item_id=$2 AND released_at IS NULL)",
    )
    .bind(owner_id)
    .bind(second_p1.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((still_quarantined, second_active_locks), (2, 2));
    assert_eq!(
        repository
            .set_isolation(
                &actor,
                &[second_p1.id],
                false,
                Utc::now(),
                "rc-release-cross-window",
            )
            .await
            .unwrap_err(),
        ReconciliationError::InvalidRequest
    );
    assert_eq!(
        repository
            .set_isolation(
                &actor,
                &[p1.id],
                true,
                Utc::now(),
                "rc-reisolate-before-resolve",
            )
            .await
            .unwrap()
            .value,
        0
    );

    let resolved = repository
        .resolve(
            &actor,
            p1.id,
            ReconciliationDisposition::WmsTruth,
            vec![],
            Utc::now(),
            "rc-resolve-1",
        )
        .await
        .unwrap();
    assert_eq!(resolved.value.resolution_status, "erp_feedback_pending");
    let (outbox_count, quarantined_count): (i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COUNT(*) FROM reconciliation_erp_feedback_outbox
           WHERE owner_id = $1 AND payload->>'reconciliation_item_id' = $2),
         (SELECT COUNT(*) FROM inventory_batches
           WHERE owner_id = $1 AND product_code = 'P1' AND batch_no = 'B1'
             AND quality_status = 'quarantined')",
    )
    .bind(owner_id)
    .bind(p1.id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((outbox_count, quarantined_count), (1, 2));
    assert_eq!(
        repository
            .set_isolation(&actor, &[p1.id], true, Utc::now(), "rc-isolate-resolved",)
            .await
            .unwrap_err(),
        ReconciliationError::InvalidRequest
    );

    let p4 = first
        .value
        .items
        .iter()
        .find(|item| item.product_code == "P4")
        .unwrap();
    sqlx::query(
        "UPDATE inventory_batches
            SET quality_status='quarantined'
          WHERE owner_id=$1 AND id=$2",
    )
    .bind(owner_id)
    .bind(p4_batch_id)
    .execute(&pool)
    .await
    .unwrap();
    let external_quarantine = repository
        .set_isolation(
            &actor,
            &[p4.id],
            true,
            Utc::now(),
            "rc-do-not-adopt-external-quarantine",
        )
        .await
        .unwrap();
    assert_eq!(external_quarantine.value, 0);
    let external_lock_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reconciliation_item_locks
          WHERE owner_id=$1 AND item_id=$2 AND released_at IS NULL",
    )
    .bind(owner_id)
    .bind(p4.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(external_lock_count, 0);
    sqlx::query(
        "UPDATE inventory_batches
            SET quality_status='qualified'
          WHERE owner_id=$1 AND id=$2",
    )
    .bind(owner_id)
    .bind(p4_batch_id)
    .execute(&pool)
    .await
    .unwrap();
    repository
        .set_isolation(&actor, &[p4.id], true, Utc::now(), "rc-isolate-known")
        .await
        .unwrap();
    let known = repository
        .resolve(
            &actor,
            p4.id,
            ReconciliationDisposition::KnownDifference,
            vec![],
            Utc::now(),
            "rc-resolve-known",
        )
        .await
        .unwrap();
    assert_eq!(known.value.resolution_status, "known_difference");
    let (qualified, active_locks): (i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COUNT(*) FROM inventory_batches
           WHERE owner_id = $1 AND product_code = 'P4' AND batch_no = 'B4'
             AND quality_status = 'qualified'),
         (SELECT COUNT(*) FROM reconciliation_item_locks
           WHERE owner_id = $1 AND item_id = $2 AND released_at IS NULL)",
    )
    .bind(owner_id)
    .bind(p4.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((qualified, active_locks), (1, 0));

    let p3 = first
        .value
        .items
        .iter()
        .find(|item| item.product_code == "P3")
        .unwrap();
    assert_eq!(
        repository
            .resolve(
                &actor,
                p3.id,
                ReconciliationDisposition::ErpTruth,
                vec![],
                Utc::now(),
                "rc-resolve-loss-missing-target",
            )
            .await
            .unwrap_err(),
        ReconciliationError::InvalidRequest
    );
    repository
        .set_isolation(&actor, &[p3.id], true, Utc::now(), "rc-isolate-loss")
        .await
        .unwrap();
    let loss = repository
        .resolve(
            &actor,
            p3.id,
            ReconciliationDisposition::ErpTruth,
            vec![
                ReconciliationInventoryAllocation {
                    inventory_batch_id: p3_batch_id,
                    quantity: 7,
                },
                ReconciliationInventoryAllocation {
                    inventory_batch_id: p3_second_batch_id,
                    quantity: 2,
                },
            ],
            Utc::now(),
            "rc-resolve-loss-1",
        )
        .await
        .unwrap();
    assert_eq!(loss.value.resolution_status, "adjustment_pending");

    let p2 = first
        .value
        .items
        .iter()
        .find(|item| item.product_code == "P2")
        .unwrap();
    repository
        .set_isolation(&actor, &[p2.id], true, Utc::now(), "rc-isolate-surplus")
        .await
        .unwrap();
    let surplus = repository
        .resolve(
            &actor,
            p2.id,
            ReconciliationDisposition::ErpTruth,
            vec![ReconciliationInventoryAllocation {
                inventory_batch_id: p2_batch_id,
                quantity: 4,
            }],
            Utc::now(),
            "rc-resolve-surplus-1",
        )
        .await
        .unwrap();
    assert_eq!(surplus.value.resolution_status, "adjustment_pending");
    let (loss_count, surplus_count, pending_approval_count): (i64, i64, i64) = sqlx::query_as(
        "SELECT
             COUNT(*) FILTER (WHERE adjustment_type = 'loss'),
             COUNT(*) FILTER (WHERE adjustment_type = 'surplus'),
             COUNT(*) FILTER (WHERE status = 'pending_approval')
             FROM stock_adjustment_orders
             WHERE owner_id = $1 AND external_ref LIKE $2",
    )
    .bind(owner_id)
    .bind("reconciliation:%")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (loss_count, surplus_count, pending_approval_count),
        (2, 1, 3)
    );
    assert_eq!(loss.value.stock_adjustment_order_ids.len(), 2);
    assert_eq!(surplus.value.stock_adjustment_order_ids.len(), 1);

    let adjustments = PgStockAdjustmentRepository::new(pool.clone());
    for (index, order_id) in loss.value.stock_adjustment_order_ids.iter().enumerate() {
        adjustments
            .record_quality_approval(
                &actor,
                *order_id,
                &format!("QL-RC-LOSS-{index}"),
                true,
                Utc::now(),
                &format!("rc-loss-approval-{index}"),
            )
            .await
            .unwrap();
        adjustments
            .start_loss_order(
                &actor,
                *order_id,
                Utc::now(),
                &format!("rc-loss-start-{index}"),
            )
            .await
            .unwrap();
        adjustments
            .execute_loss_order(
                &actor,
                *order_id,
                None,
                Utc::now(),
                &format!("rc-loss-execute-{index}"),
            )
            .await
            .unwrap();
        let progress: (String, i64) = sqlx::query_as(
            "SELECT item.resolution_status,
                    (SELECT COUNT(*) FROM reconciliation_item_locks
                      WHERE owner_id=$1 AND item_id=item.id AND released_at IS NULL)
               FROM reconciliation_items item
              WHERE item.owner_id=$1 AND item.id=$2",
        )
        .bind(owner_id)
        .bind(p3.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            progress,
            if index == 0 {
                ("adjustment_pending".to_string(), 2)
            } else {
                ("resolved".to_string(), 0)
            }
        );
    }

    adjustments
        .record_surplus_quality_approval(
            &actor,
            surplus.value.stock_adjustment_order_ids[0],
            "QL-RC-SURPLUS-REJECT",
            false,
            Utc::now(),
            "rc-surplus-reject",
        )
        .await
        .unwrap();
    let rejected_progress: (String, i64, String) = sqlx::query_as(
        "SELECT item.resolution_status,
                (SELECT COUNT(*) FROM reconciliation_item_locks
                  WHERE owner_id=$1 AND item_id=item.id AND released_at IS NULL),
                (SELECT quality_status FROM inventory_batches
                  WHERE owner_id=$1 AND id=$3)
           FROM reconciliation_items item
          WHERE item.owner_id=$1 AND item.id=$2",
    )
    .bind(owner_id)
    .bind(p2.id)
    .bind(p2_batch_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        rejected_progress,
        ("exception".to_string(), 1, "quarantined".to_string())
    );

    let released = repository
        .set_isolation(&actor, &[p1.id], false, Utc::now(), "rc-release-1")
        .await
        .unwrap_err();
    assert_eq!(released, ReconciliationError::InvalidRequest);
    let qualified_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_batches
          WHERE owner_id = $1 AND product_code = 'P1' AND batch_no = 'B1'
            AND quality_status = 'qualified'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(qualified_count, 0);
}
