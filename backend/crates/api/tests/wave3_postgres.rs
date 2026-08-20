use chrono::{TimeZone, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{
    CreateBillingAccountRequest, CreateBillingContractRequest, CreateBillingRuleRequest,
    CreateReceivingOrderRequest, InventoryBatchQuery, PutawayRequest, ReceiveReceivingOrderRequest,
    ReceivingOrderLine, ReceivingReceiptDetails, RejectReceivingOrderRequest,
    UpdateReceivingOrderRequest,
};

mod postgres_test_support;
use postgres_test_support::{ensure_audit_partition, seed_idle_lpn};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "postgres-test".to_string(),
        permissions: vec!["m2.write".to_string(), "m3.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
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
        expected_arrival_at: Some(Utc::now() + chrono::Duration::days(1)),
        lines: vec![ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-001".to_string(),
            expected_qty: 10.into(),
            batch_no: None,
            production_date: None,
            expiry_date: None,
        }],
    }
}

async fn seed_active_supplier_and_warehouse(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let supplier_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'Wave 3 测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("W3-OWNER-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
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
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, volume_cm3, status) VALUES ($1, $2, 'P-001', 'Active Product', '1 unit', 'normal_10_30', 1, 'active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed product");
    (supplier_id, warehouse_id)
}

async fn seed_location(pool: &PgPool, owner_id: Uuid, warehouse_id: Uuid) -> (Uuid, String) {
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let location_code = format!("M2-LOC-{}", &location_id.to_string()[..8]);
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, $4, 'M2 test zone', 'normal_10_30', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("M2-ZONE-{}", &zone_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, $5, 1, 1, 1, 100000, 0, 3, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .bind(&location_code)
    .execute(pool)
    .await
    .expect("seed location");
    (location_id, location_code)
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
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;
    let mut req = receiving_order_req_with("SR-PG-001", Some(supplier_id), warehouse_id);
    req.document_type = "sales_return".to_string();
    req.lines[0].batch_no = Some("B-SALES-RETURN-001".to_string());

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
    ensure_audit_partition(&pool, now).await;
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;
    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-DELETE-001", Some(supplier_id), warehouse_id),
            now,
        )
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
    let (supplier_id, warehouse_id) = seed_active_supplier_and_warehouse(&pool, owner_id).await;
    let order = repo
        .create_receiving_order(
            &ctx,
            receiving_order_req_with("ASN-PG-OWNER-001", Some(supplier_id), warehouse_id),
            now,
        )
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
                    expected_qty: 10.into(),
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
    assert_eq!(persisted.lines[0].product_id, order.lines[0].product_id);
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
