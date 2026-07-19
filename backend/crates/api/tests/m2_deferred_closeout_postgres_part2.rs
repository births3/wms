use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{
    CancelReceivingOrderRequest, CreateReceivingOrderRequest, ForceCloseShortageRequest,
    InspectReceivingOrderRequest, PutawayRequest, ReceiveReceivingOrderRequest,
    ReceivingDashboardQuery, ReceivingOrderLine, ReceivingReceiptDetails, SignInspectionRequest,
    UpsertPutawayStrategyProfileRequest, RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND,
    RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
};

#[path = "support/auth.rs"]
mod auth_support;

fn context(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m2-deferred-closeout-test".to_string(),
        permissions: vec!["m2.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn request(document_type: &str, batch_no: Option<&str>) -> CreateReceivingOrderRequest {
    request_with_receipt_no(document_type, batch_no, None)
}

fn request_with_receipt_no(
    document_type: &str,
    batch_no: Option<&str>,
    receipt_no: Option<&str>,
) -> CreateReceivingOrderRequest {
    CreateReceivingOrderRequest {
        receipt_no: receipt_no
            .map(str::to_string)
            .unwrap_or_else(|| format!("M2-{}", Uuid::new_v4())),
        document_type: document_type.to_string(),
        supplier_id: Some(Uuid::new_v4()),
        warehouse_id: Uuid::new_v4(),
        external_ref: None,
        expected_arrival_at: Some(chrono::Utc::now() + chrono::Duration::days(1)),
        lines: vec![ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-M2-001".to_string(),
            expected_qty: 10,
            batch_no: batch_no.map(str::to_string),
            production_date: None,
            expiry_date: None,
        }],
    }
}

async fn seed_asn_references(
    pool: &PgPool,
    owner_id: Uuid,
    request: &mut CreateReceivingOrderRequest,
) {
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'M2 ASN 测试仓', 'normal', 'active') ON CONFLICT (owner_id, warehouse_code) DO NOTHING",
    )
    .bind(request.warehouse_id)
    .bind(owner_id)
    .bind(format!("M2-ASN-WH-{}", &request.warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed ASN warehouse");
    let supplier_id = request.supplier_id.expect("request supplier");
    let suffix = &supplier_id.to_string()[..8];
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, $3, 'M2 Test Supplier', $4, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("M2-SUP-{suffix}"))
    .bind(format!("M2-USCC-{suffix}"))
    .execute(pool)
    .await
    .expect("seed ASN supplier");
    let product_code = request
        .lines
        .first()
        .expect("request line")
        .product_code
        .clone();
    let product_id: Uuid = sqlx::query_scalar(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, attrs, status) VALUES ($1, $2, $3, 'M2 Test Product', '1 unit', 'normal', '{\"unit_volume_cm3\": 1}', 'active') ON CONFLICT (owner_id, product_code) DO UPDATE SET attrs = EXCLUDED.attrs, status = 'active' RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(&product_code)
    .fetch_one(pool)
    .await
    .expect("seed ASN product");
    for line in &mut request.lines {
        line.product_id = Some(product_id);
    }
}

async fn seed_putaway_location(pool: &PgPool, owner_id: Uuid) -> (Uuid, Uuid) {
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'M2 test warehouse', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("M2-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, $4, 'M2 test zone', 'normal', 'qualified_green', 'active')",
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
    .bind(format!("M2-LOC-{}", &location_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed location");
    (warehouse_id, location_id)
}

async fn seed_numbering_rule(pool: &PgPool, owner_id: Uuid) {
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'M2 test owner')",
    )
    .bind(owner_id)
    .bind(format!("M2OWNER-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed numbering owner");
    sqlx::query(
        "INSERT INTO document_number_rules (id, owner_id, document_type, rule_code, rule_name, template, reset_policy, sequence_width, enabled, created_at, updated_at) VALUES ($1, NULL, 'purchase_inbound', $2, 'M2 test ASN rule', 'ASN-{OWNER}-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, TRUE, $3, $3)",
    )
    .bind(Uuid::new_v4())
    .bind(format!("m2-test-asn-{}", &owner_id.to_string()[..8]))
    .bind(now)
    .execute(pool)
    .await
    .expect("seed numbering rule");
}

#[sqlx::test(migrations = "../../migrations")]
async fn sales_return_inspection_updates_each_batch_line(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut sales_return = request(RECEIVING_DOCUMENT_TYPE_SALES_RETURN, Some("B-001"));
    sales_return.lines.push(ReceivingOrderLine {
        line_no: 2,
        product_id: None,
        product_code: "P-M2-001".to_string(),
        expected_qty: 6,
        batch_no: Some("B-002".to_string()),
        production_date: None,
        expiry_date: None,
    });
    seed_asn_references(&pool, owner_id, &mut sales_return).await;
    let order = repository
        .create_receiving_order(&ctx, sales_return, chrono::Utc::now())
        .await
        .expect("create multi-batch sales return");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repository
        .receive_receiving_order_with_audit(
            &ctx,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 10,
                shortage_qty: 6,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: None,
                details: Some(ReceivingReceiptDetails {
                    temperature_control_method: Some("普通".to_string()),
                    vehicle_no: Some("沪A00000".to_string()),
                    origin: Some("发运地".to_string()),
                    departure_at: Some(chrono::Utc::now()),
                    arrival_at: Some(chrono::Utc::now()),
                    storage_at: Some(chrono::Utc::now()),
                    transport_mode: Some("公路".to_string()),
                    carrier: Some("承运商".to_string()),
                    contact_name: Some("送货人".to_string()),
                    contact_phone: Some("13800000000".to_string()),
                    contact_id_no: Some("310101199001011234".to_string()),
                    seal_checked: Some("已核对".to_string()),
                    filing_checked: Some("已核对".to_string()),
                }),
            },
            chrono::Utc::now(),
            "receive-sales-return-batches",
            None,
        )
        .await
        .expect("receive multi-batch sales return");

    for (batch_no, qty, key) in [
        ("B-001", 4_i64, "inspect-b-001"),
        ("B-002", 6_i64, "inspect-b-002"),
    ] {
        repository
            .inspect_receiving_order_with_audit(
                &ctx,
                order.id,
                InspectReceivingOrderRequest {
                    batch_no: batch_no.to_string(),
                    accepted_qty: qty,
                    rejected_qty: 0,
                    production_date: "2026-01-01".to_string(),
                    expiry_date: "2028-01-01".to_string(),
                    quality_status: STATUS_QUALIFIED.to_string(),
                    trace_codes: vec![format!("TRACE-{batch_no}")],
                    appearance_check: Some("完好".to_string()),
                    package_check: Some("完好".to_string()),
                    instruction_check: Some("有".to_string()),
                    label_check: Some("清晰".to_string()),
                    sampling_qty: Some(1),
                    approval_no: None,
                },
                chrono::NaiveDate::from_ymd_opt(2026, 7, 12).expect("valid date"),
                chrono::Utc::now(),
                key,
                None,
            )
            .await
            .expect("inspect each sales-return batch");
    }

    let batches: Vec<(i32, String, String)> = sqlx::query_as(
        "SELECT line_no, batch_no, expiry_date::TEXT FROM receiving_order_lines WHERE receiving_order_id = $1 ORDER BY line_no",
    )
    .bind(order.id)
    .fetch_all(&pool)
    .await
    .expect("read inspected lines");
    assert_eq!(
        batches,
        vec![
            (1, "B-001".to_string(), "2028-01-01".to_string()),
            (2, "B-002".to_string(), "2028-01-01".to_string())
        ]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_is_partial_until_all_accepted_quantity_is_committed(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let (warehouse_id, location_id) = seed_putaway_location(&pool, owner_id).await;
    let mut purchase = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut purchase).await;
    purchase.warehouse_id = warehouse_id;
    let order = repository
        .create_receiving_order(&ctx, purchase, chrono::Utc::now())
        .await
        .expect("create purchase inbound");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repository
        .receive_receiving_order_with_audit(
            &ctx,
            order.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 10,
                shortage_qty: 0,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: None,
                details: Some(ReceivingReceiptDetails {
                    temperature_control_method: Some("普通".to_string()),
                    vehicle_no: Some("沪A00000".to_string()),
                    origin: Some("发运地".to_string()),
                    departure_at: Some(chrono::Utc::now()),
                    arrival_at: Some(chrono::Utc::now()),
                    storage_at: Some(chrono::Utc::now()),
                    transport_mode: Some("公路".to_string()),
                    carrier: Some("承运商".to_string()),
                    contact_name: Some("送货人".to_string()),
                    contact_phone: Some("13800000000".to_string()),
                    contact_id_no: Some("310101199001011234".to_string()),
                    seal_checked: Some("已核对".to_string()),
                    filing_checked: Some("已核对".to_string()),
                }),
            },
            chrono::Utc::now(),
            "receive-putaway",
            None,
        )
        .await
        .expect("receive purchase inbound");
    repository
        .inspect_receiving_order_with_audit(
            &ctx,
            order.id,
            InspectReceivingOrderRequest {
                batch_no: "B-PUTAWAY-001".to_string(),
                accepted_qty: 10,
                rejected_qty: 0,
                production_date: "2026-01-01".to_string(),
                expiry_date: "2028-01-01".to_string(),
                quality_status: STATUS_QUALIFIED.to_string(),
                trace_codes: vec![],

                appearance_check: Some("完好".to_string()),
                package_check: Some("完好".to_string()),
                instruction_check: Some("有".to_string()),
                label_check: Some("清晰".to_string()),
                sampling_qty: Some(1),
                approval_no: None,
            },
            chrono::NaiveDate::from_ymd_opt(2026, 7, 12).expect("valid date"),
            chrono::Utc::now(),
            "inspect-putaway",
            None,
        )
        .await
        .expect("inspect putaway batch");
    sqlx::query("UPDATE receiving_orders SET status = 'putaway' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("restore putaway state after inspection");

    for (qty, key) in [(6_i64, "putaway-part-1"), (4_i64, "putaway-part-2")] {
        repository
            .putaway_receiving_order_and_inventory_with_audit(
                &ctx,
                order.id,
                PutawayRequest {
                    batch_no: "B-PUTAWAY-001".to_string(),
                    product_code: "P-M2-001".to_string(),
                    qty,
                    location_id,
                    location_code: sqlx::query_scalar(
                        "SELECT location_code FROM warehouse_locations WHERE id = $1",
                    )
                    .bind(location_id)
                    .fetch_one(&pool)
                    .await
                    .expect("read location code"),
                    quality_status: STATUS_QUALIFIED.to_string(),
                    lpn_code: None,
                },
                chrono::Utc::now(),
                key,
                None,
            )
            .await
            .expect("putaway partial quantity");
    }

    let status: String = sqlx::query_scalar("SELECT status FROM receiving_orders WHERE id = $1")
        .bind(order.id)
        .fetch_one(&pool)
        .await
        .expect("read final status");
    assert_eq!(status, "completed");
    let putaway_qty: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(qty), 0)::BIGINT FROM receiving_putaways WHERE receiving_order_id = $1",
    )
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("read putaway total");
    assert_eq!(putaway_qty, 10);
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_is_idempotent_and_audited_in_the_postgres_path(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    seed_numbering_rule(&pool, owner_id).await;
    let mut req = request_with_receipt_no(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None, Some(""));
    seed_asn_references(&pool, owner_id, &mut req).await;
    let audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "create",
        "M2",
        "receiving_order",
        "pending",
        None,
    );
    let first = repository
        .create_receiving_order_with_audit(
            &ctx,
            req.clone(),
            chrono::Utc::now(),
            "create-idempotency",
            audit.clone(),
        )
        .await
        .expect("create ASN");
    let replay = repository
        .create_receiving_order_with_audit(
            &ctx,
            req,
            chrono::Utc::now(),
            "create-idempotency",
            audit,
        )
        .await
        .expect("replay create ASN");
    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.id, replay.value.id);
    assert!(first.value.receipt_no.starts_with("ASN-M2OWNER-"));
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'create')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("read create idempotency evidence");
    assert_eq!(counts, (1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn dashboard_groups_real_postgres_receiving_statuses(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut first_request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut first_request).await;
    let first = repository
        .create_receiving_order(&ctx, first_request, chrono::Utc::now())
        .await
        .expect("create first dashboard order");
    let mut second_request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut second_request).await;
    let second = repository
        .create_receiving_order(&ctx, second_request, chrono::Utc::now())
        .await
        .expect("create second dashboard order");
    sqlx::query("UPDATE receiving_orders SET status = 'closed_rejected' WHERE id = $1")
        .bind(second.id)
        .execute(&pool)
        .await
        .expect("mark abnormal order");

    let rows = repository
        .list_receiving_dashboard(&ctx, &ReceivingDashboardQuery::default())
        .await
        .expect("read dashboard");
    assert!(rows
        .iter()
        .any(|row| row.status == "draft" && row.order_count == 1));
    let abnormal = rows
        .iter()
        .find(|row| row.status == "closed_rejected")
        .expect("abnormal dashboard row");
    assert!(abnormal.abnormal);
    assert_eq!(abnormal.expected_qty, 10);
    assert_ne!(first.id, second.id);
}

include!("m2_deferred_closeout/print_data.rs");
include!("m2_deferred_closeout/closeout_actions.rs");
