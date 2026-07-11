use chrono::{TimeZone, Utc};
use sqlx::postgres::PgPoolOptions;
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
async fn delete_receiving_order_is_owner_scoped_draft_only_and_audited(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 11, 12, 0, 0)
        .single()
        .expect("valid time");
    let order = repo
        .create_receiving_order(&ctx, receiving_order_req("ASN-DELETE-001"), now)
        .await
        .expect("draft order should create");

    let deleted = repo
        .delete_receiving_order(&ctx, order.id, now)
        .await
        .expect("draft order should delete");
    assert_eq!(deleted.id, order.id);
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM receiving_orders WHERE id = $1), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'delete' AND resource_id = $3)",
    )
    .bind(order.id)
    .bind(owner_id)
    .bind(order.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("delete evidence should query");
    assert_eq!(counts, (0, 1));
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

include!("wave3_postgres/wave3_postgres_part2.rs");
