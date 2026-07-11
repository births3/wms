use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{
    CreateBillingAccountRequest, CreateBillingContractRequest, CreateBillingRuleRequest,
    CreateReceivingOrderRequest, PutawayRequest, ReceiveReceivingOrderRequest, ReceivingOrderLine,
    RejectReceivingOrderRequest, UpdateReceivingOrderRequest,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "postgres-test".to_string(),
        permissions: vec!["m2.write".to_string(), "m3.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

fn update_audit(ctx: &AuthContext, id: Uuid) -> AuditWriteRequest {
    AuditWriteRequest::from_auth_context(
        ctx,
        "update",
        "M2",
        "receiving_order",
        id.to_string(),
        None,
    )
}

fn receiving_order_req(receipt_no: &str) -> CreateReceivingOrderRequest {
    receiving_order_req_with(receipt_no, None, Uuid::new_v4())
}

fn receiving_order_req_with(
    receipt_no: &str,
    supplier_id: Option<Uuid>,
    warehouse_id: Uuid,
) -> CreateReceivingOrderRequest {
    CreateReceivingOrderRequest {
        receipt_no: receipt_no.to_string(),
        document_type: "purchase_inbound".to_string(),
        supplier_id,
        warehouse_id,
        external_ref: Some(format!("ERP-{receipt_no}")),
        expected_arrival_at: None,
        lines: vec![ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-001".to_string(),
            expected_qty: 10,
            batch_no: Some("B202606".to_string()),
            production_date: Some("2026-01-01".to_string()),
            expiry_date: Some("2028-01-01".to_string()),
        }],
    }
}

async fn seed_active_supplier_and_warehouse(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let supplier_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, $3, 'Active Supplier', $4, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("SUP-{}", &supplier_id.to_string()[..8]))
    .bind(format!("USCC-{}", &supplier_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed supplier");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'Main WH', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    (supplier_id, warehouse_id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn receiving_order_persists_document_type(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 9, 0, 0)
        .single()
        .expect("valid time");
    let mut req = receiving_order_req("SR-PG-001");
    req.document_type = "sales_return".to_string();

    let order = repo
        .create_receiving_order(&ctx, req, now)
        .await
        .expect("create sales return receiving order");
    assert_eq!(order.document_type, "sales_return");

    let stored: String = sqlx::query_scalar(
        "SELECT document_type FROM receiving_orders WHERE id = $1 AND owner_id = $2",
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("stored document type");
    assert_eq!(stored, "sales_return");
}

#[sqlx::test(migrations = "../../migrations")]
async fn receiving_order_update_is_draft_only_and_audits_before_after(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc::now();
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;
    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-UPDATE-001", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    let audit = update_audit(&ctx, order.id);

    let updated = repo
        .update_receiving_order(
            &ctx,
            order.id,
            UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: Some(Some("ERP-UPDATED".to_string())),
                expected_arrival_at: None,
                lines: None,
            },
            now,
            "update-external-ref",
            audit,
        )
        .await
        .expect("draft order update should succeed");
    assert_eq!(updated.external_ref.as_deref(), Some("ERP-UPDATED"));

    let replayed = repo
        .update_receiving_order(
            &ctx,
            order.id,
            UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: Some(Some("ERP-UPDATED".to_string())),
                expected_arrival_at: None,
                lines: None,
            },
            now,
            "update-external-ref",
            update_audit(&ctx, order.id),
        )
        .await
        .expect("same idempotency key should replay");
    assert_eq!(replayed.id, updated.id);
    assert_eq!(replayed.external_ref, updated.external_ref);
    let version: i64 = sqlx::query_scalar("SELECT version FROM receiving_orders WHERE id = $1")
        .bind(order.id)
        .fetch_one(&pool)
        .await
        .expect("receiving order version");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(order.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("update audit count");
    assert_eq!(version, 2);
    assert_eq!(audit_count, 1);

    let diff: serde_json::Value =
        sqlx::query_scalar("SELECT diff FROM audit_event WHERE owner_id = $1 AND resource_id = $2")
            .bind(owner_id)
            .bind(order.id.to_string())
            .fetch_one(&pool)
            .await
            .expect("update audit should be persisted");
    assert_eq!(diff["before"]["external_ref"], "ERP-ASN-PG-UPDATE-001");
    assert_eq!(diff["after"]["external_ref"], "ERP-UPDATED");
    assert_eq!(diff["changed_keys"], serde_json::json!(["external_ref"]));

    let cleared = repo
        .update_receiving_order(
            &ctx,
            order.id,
            UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: Some(None),
                expected_arrival_at: None,
                lines: None,
            },
            now,
            "update-clear-external-ref",
            update_audit(&ctx, order.id),
        )
        .await
        .expect("nullable field should be clearable");
    assert_eq!(cleared.external_ref, None);

    repo.release_receiving_order(&ctx, order.id, now)
        .await
        .expect("release order");
    let after_release = repo
        .update_receiving_order(
            &ctx,
            order.id,
            UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: Some(Some("TOO-LATE".to_string())),
                expected_arrival_at: None,
                lines: None,
            },
            now,
            "update-after-release",
            update_audit(&ctx, order.id),
        )
        .await;
    assert!(matches!(
        after_release,
        Err(Wave3RepositoryError::InvalidStatus { expected, .. }) if expected == "draft"
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn receiving_order_update_rejects_cross_owner_and_missing_master_data(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let foreign_owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc::now();
    let order = repo
        .create_receiving_order(&ctx, receiving_order_req("ASN-PG-OWNER-001"), now)
        .await
        .expect("create receiving order");
    let foreign_warehouse_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type) VALUES ($1, $2, 'FOREIGN', 'Foreign', 'normal')",
    )
    .bind(foreign_warehouse_id)
    .bind(foreign_owner_id)
    .execute(&pool)
    .await
    .expect("seed foreign warehouse");

    let cross_owner = repo
        .update_receiving_order(
            &ctx,
            order.id,
            UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: Some(foreign_warehouse_id),
                external_ref: None,
                expected_arrival_at: None,
                lines: None,
            },
            now,
            "update-foreign-warehouse",
            update_audit(&ctx, order.id),
        )
        .await;
    assert!(matches!(cross_owner, Err(Wave3RepositoryError::NotFound)));

    let disabled_supplier_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, 'DISABLED', 'Disabled', 'USCC-DISABLED', 'disabled')",
    )
    .bind(disabled_supplier_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed disabled supplier");
    let disabled_supplier = repo
        .update_receiving_order(
            &ctx,
            order.id,
            UpdateReceivingOrderRequest {
                supplier_id: Some(disabled_supplier_id),
                warehouse_id: None,
                external_ref: None,
                expected_arrival_at: None,
                lines: None,
            },
            now,
            "update-disabled-supplier",
            update_audit(&ctx, order.id),
        )
        .await;
    assert!(matches!(
        disabled_supplier,
        Err(Wave3RepositoryError::NotFound)
    ));

    let missing_product = repo
        .update_receiving_order(
            &ctx,
            order.id,
            UpdateReceivingOrderRequest {
                supplier_id: None,
                warehouse_id: None,
                external_ref: None,
                expected_arrival_at: None,
                lines: Some(vec![ReceivingOrderLine {
                    line_no: 1,
                    product_id: Some(Uuid::new_v4()),
                    product_code: "P-MISSING".to_string(),
                    expected_qty: 10,
                    batch_no: None,
                    production_date: None,
                    expiry_date: None,
                }]),
            },
            now,
            "update-missing-product",
            update_audit(&ctx, order.id),
        )
        .await;
    assert!(matches!(
        missing_product,
        Err(Wave3RepositoryError::NotFound)
    ));

    let persisted = repo
        .get_receiving_order(&ctx, order.id)
        .await
        .expect("failed updates must roll back");
    assert_eq!(persisted.warehouse_id, order.warehouse_id);
    assert_eq!(persisted.lines[0].product_id, None);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND resource_id = $2",
    )
    .bind(owner_id)
    .bind(order.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("count audit events");
    assert_eq!(audit_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn receiving_receipt_is_single_closure_and_idempotent(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
        .single()
        .expect("valid time");
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;

    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-001", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    repo.release_receiving_order(&ctx, order.id, now)
        .await
        .expect("release receiving order");

    let req = ReceiveReceivingOrderRequest {
        actual_qty: 8,
        shortage_qty: 2,
        rejected_qty: 0,
        arrival_temperature_celsius: Some(4.8),
        exception_note: None,
    };
    let first = repo
        .receive_receiving_order(&ctx, order.id, req.clone(), now, "idem-receive-1")
        .await
        .expect("first receive should insert");
    let replay = repo
        .receive_receiving_order(&ctx, order.id, req, now, "idem-receive-1")
        .await
        .expect("same idempotency key should replay first result");
    assert_eq!(first.id, replay.id);

    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("simulate invalid status rollback to verify unique receipt constraint");
    let duplicate = repo
        .receive_receiving_order(
            &ctx,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 8,
                shortage_qty: 2,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: None,
            },
            now,
            "idem-receive-2",
        )
        .await
        .expect_err("a receiving order can only have one receipt closure");
    assert!(matches!(duplicate, Wave3RepositoryError::DuplicateReceipt));

    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM receiving_order_receipts WHERE receiving_order_id = $1",
    )
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("count receipts");
    assert_eq!(receipt_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn receiving_order_reject_closes_order_and_replays_idempotently(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 15, 0)
        .single()
        .expect("valid time");
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;

    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-REJECT-001", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    repo.release_receiving_order(&ctx, order.id, now)
        .await
        .expect("release receiving order");

    let req = RejectReceivingOrderRequest {
        reason: "外包装严重破损，整单拒收".to_string(),
    };
    let first = repo
        .reject_receiving_order(&ctx, order.id, req.clone(), now, "idem-reject-1")
        .await
        .expect("first reject should insert");
    let replay = repo
        .reject_receiving_order(&ctx, order.id, req, now, "idem-reject-1")
        .await
        .expect("same idempotency key should replay first reject");
    assert_eq!(first.id, replay.id);

    let closed: (i64, i64, i64, Option<String>, String, i64) = sqlx::query_as(
        r#"
        SELECT
            receipt.actual_qty,
            receipt.shortage_qty,
            receipt.rejected_qty,
            receipt.exception_note,
            orders.status,
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2 AND idempotency_key = 'idem-reject-1')
          FROM receiving_order_receipts receipt
          JOIN receiving_orders orders ON orders.id = receipt.receiving_order_id
         WHERE receipt.receiving_order_id = $1
        "#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("closed reject row");
    assert_eq!(
        closed,
        (
            0,
            0,
            10,
            Some("外包装严重破损，整单拒收".to_string()),
            "closed_rejected".to_string(),
            1,
        )
    );

    let receiving_order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-REJECT-002", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving status receiving order");
    repo.release_receiving_order(&ctx, receiving_order.id, now)
        .await
        .expect("release receiving status receiving order");
    sqlx::query("UPDATE receiving_orders SET status = 'receiving' WHERE id = $1 AND owner_id = $2")
        .bind(receiving_order.id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("mark order receiving");
    let receiving_reject = repo
        .reject_receiving_order(
            &ctx,
            receiving_order.id,
            RejectReceivingOrderRequest {
                reason: "收货中发现货损，整单拒收".to_string(),
            },
            now,
            "idem-reject-receiving",
        )
        .await
        .expect("receiving status order can be rejected");
    assert_eq!(receiving_reject.rejected_qty, 10);

    let draft = repo
        .create_receiving_order(&ctx, receiving_order_req("ASN-PG-REJECT-003"), now)
        .await
        .expect("create draft receiving order");
    let invalid = repo
        .reject_receiving_order(
            &ctx,
            draft.id,
            RejectReceivingOrderRequest {
                reason: "未放行不能拒收".to_string(),
            },
            now,
            "idem-reject-draft",
        )
        .await
        .expect_err("non released order cannot be rejected");
    assert!(matches!(
        invalid,
        Wave3RepositoryError::InvalidStatus { .. }
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_same_idempotency_key_replays_first_receipt(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 30, 0)
        .single()
        .expect("valid time");
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;

    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-RACE-001", Some(supplier_id), warehouse_id),
            now,
        )
        .await
        .expect("create receiving order");
    repo.release_receiving_order(&ctx, order.id, now)
        .await
        .expect("release receiving order");

    let req = ReceiveReceivingOrderRequest {
        actual_qty: 8,
        shortage_qty: 2,
        rejected_qty: 0,
        arrival_temperature_celsius: Some(4.8),
        exception_note: None,
    };
    let (left, right) = tokio::join!(
        repo.receive_receiving_order(&ctx, order.id, req.clone(), now, "idem-receive-race"),
        repo.receive_receiving_order(&ctx, order.id, req, now, "idem-receive-race"),
    );
    let left = left.expect("left request should succeed");
    let right = right.expect("right request should replay");

    assert_eq!(left.id, right.id);
    let counts: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM receiving_order_receipts WHERE receiving_order_id = $1),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2 AND idempotency_key = 'idem-receive-race')
        "#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_commits_receiving_inventory_and_movement_in_one_transaction(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 11, 0, 0)
        .single()
        .expect("valid time");

    let order = repo
        .create_receiving_order(&ctx, receiving_order_req("ASN-PG-002"), now)
        .await
        .expect("create receiving order");
    sqlx::query("UPDATE receiving_orders SET status = 'putaway' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare putaway state");

    let req = PutawayRequest {
        batch_no: "B202606".to_string(),
        product_code: "P-001".to_string(),
        qty: 10,
        location_id: Uuid::new_v4(),
        location_code: "A-01-01".to_string(),
        quality_status: STATUS_QUALIFIED.to_string(),
    };
    let first = repo
        .putaway_receiving_order_and_inventory_with_audit(
            &ctx,
            order.id,
            req.clone(),
            now,
            "idem-putaway-1",
            Some(wms_api::audit::AuditWriteRequest::from_auth_context(
                &ctx,
                "putaway",
                "M2",
                "receiving_order",
                order.id.to_string(),
                None,
            )),
        )
        .await
        .expect("putaway should commit");
    let replay = repo
        .putaway_receiving_order_and_inventory_with_audit(
            &ctx,
            order.id,
            req,
            now,
            "idem-putaway-1",
            Some(wms_api::audit::AuditWriteRequest::from_auth_context(
                &ctx,
                "putaway",
                "M2",
                "receiving_order",
                order.id.to_string(),
                None,
            )),
        )
        .await
        .expect("same idempotency key should replay");

    assert_eq!(first.value.putaway.id, replay.value.putaway.id);
    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.inventory_batch.qty_on_hand, 10);
    assert_eq!(first.value.inventory_movement.qty_delta, 10);

    let counts: (i64, i64, i64, String, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM receiving_putaways WHERE receiving_order_id = $1),
            (SELECT COUNT(*) FROM inventory_batches WHERE owner_id = $2),
            (SELECT COUNT(*) FROM inventory_movements WHERE owner_id = $2),
            (SELECT status FROM receiving_orders WHERE id = $1),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'putaway')
        "#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, 1, 1, "completed".to_string(), 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_rule_effective_window_rejects_overlap(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 12, 0, 0)
        .single()
        .expect("valid time");

    let account = repo
        .create_billing_account(
            &ctx,
            CreateBillingAccountRequest {
                account_code: "OWNER-A-BILL".to_string(),
                account_name: "Owner A Billing".to_string(),
            },
            now,
        )
        .await
        .expect("account");
    let contract = repo
        .create_billing_contract(
            &ctx,
            CreateBillingContractRequest {
                account_id: account.id,
                contract_no: "CONTRACT-PG-001".to_string(),
                valid_from: "2026-06-01".to_string(),
                valid_to: "2027-05-31".to_string(),
            },
            now,
        )
        .await
        .expect("contract");

    repo.create_billing_rule(
        &ctx,
        CreateBillingRuleRequest {
            contract_id: contract.id,
            charge_item: "storage".to_string(),
            unit: "pallet_day".to_string(),
            unit_price_cents: 100,
            billing_cycle: "monthly".to_string(),
            effective_from: "2026-06-01".to_string(),
            effective_to: "2026-06-30".to_string(),
        },
        now,
    )
    .await
    .expect("first rule");

    let overlap = repo
        .create_billing_rule(
            &ctx,
            CreateBillingRuleRequest {
                contract_id: contract.id,
                charge_item: "storage".to_string(),
                unit: "pallet_day".to_string(),
                unit_price_cents: 110,
                billing_cycle: "monthly".to_string(),
                effective_from: "2026-06-15".to_string(),
                effective_to: "2026-07-15".to_string(),
            },
            now,
        )
        .await
        .expect_err("overlapping effective windows should be rejected");
    assert!(matches!(overlap, Wave3RepositoryError::BillingRuleConflict));
}
