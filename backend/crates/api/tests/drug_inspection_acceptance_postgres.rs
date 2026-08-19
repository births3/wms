use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    inventory::STATUS_QUALIFIED,
    quality_liaison::PgQualityLiaisonRepository,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
    wave4_repository::PgWave4Repository,
};
use wms_domain::{
    InspectReceivingOrderRequest, QualityLiaisonApprovalCallbackRequest, ShipOutboundOrderRequest,
};

struct Fixture {
    owner_id: Uuid,
    user_id: Uuid,
    reviewer_user_id: Uuid,
    product_id: Uuid,
    supplier_id: Uuid,
    warehouse_id: Uuid,
    location_id: Uuid,
}

fn context(fixture: &Fixture) -> AuthContext {
    AuthContext {
        user_id: fixture.user_id,
        owner_id: fixture.owner_id,
        actor_name: "药检验收测试员".to_string(),
        permissions: vec!["m2.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_fixture(pool: &PgPool) -> Fixture {
    let fixture = Fixture {
        owner_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        reviewer_user_id: Uuid::new_v4(),
        product_id: Uuid::new_v4(),
        supplier_id: Uuid::new_v4(),
        warehouse_id: Uuid::new_v4(),
        location_id: Uuid::new_v4(),
    };
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name)
         VALUES ($1, $2, '药检验收测试货主')",
    )
    .bind(fixture.owner_id)
    .bind(format!("DI-ACCEPT-{}", fixture.owner_id.simple()))
    .execute(pool)
    .await
    .expect("owner should seed");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status)
         VALUES ($1, $2, '药检验收测试员', 'test-hash', 'active')",
    )
    .bind(fixture.user_id)
    .bind(format!("di-accept-{}", fixture.user_id.simple()))
    .execute(pool)
    .await
    .expect("user should seed");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status)
         VALUES ($1, $2, '药检验收审核员', 'test-hash', 'active')",
    )
    .bind(fixture.reviewer_user_id)
    .bind(format!("di-reviewer-{}", fixture.reviewer_user_id.simple()))
    .execute(pool)
    .await
    .expect("reviewer should seed");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
         VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(fixture.user_id)
    .bind(fixture.owner_id)
    .execute(pool)
    .await
    .expect("binding should seed");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
         VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(fixture.reviewer_user_id)
    .bind(fixture.owner_id)
    .execute(pool)
    .await
    .expect("reviewer binding should seed");
    sqlx::query(
        "INSERT INTO products (
            id, owner_id, product_code, product_name, special_drug_category,
            specification, storage_condition, status
         )
         VALUES ($1, $2, 'P-DI-ACCEPT', '药检验收药品', 'none',
                 '10mg', 'normal_10_30', 'active')",
    )
    .bind(fixture.product_id)
    .bind(fixture.owner_id)
    .execute(pool)
    .await
    .expect("product should seed");
    sqlx::query(
        "INSERT INTO suppliers (
            id, owner_id, supplier_code, supplier_name, uscc, status
         )
         VALUES ($1, $2, 'S-DI-ACCEPT', '药检验收供应商', $3, 'active')",
    )
    .bind(fixture.supplier_id)
    .bind(fixture.owner_id)
    .bind(format!(
        "USCC{}",
        &fixture.supplier_id.simple().to_string()[..14]
    ))
    .execute(pool)
    .await
    .expect("supplier should seed");
    sqlx::query(
        "INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type, status
         )
         VALUES ($1, $2, 'W-DI-ACCEPT', '药检验收仓', 'pharma', 'active')",
    )
    .bind(fixture.warehouse_id)
    .bind(fixture.owner_id)
    .execute(pool)
    .await
    .expect("warehouse should seed");
    let zone_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name,
            temperature_zone, quality_color, status
         )
         VALUES ($1, $2, $3, 'DI-ACCEPT-ZONE', '药检验收库区',
                 'normal_10_30', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(fixture.owner_id)
    .bind(fixture.warehouse_id)
    .execute(pool)
    .await
    .expect("zone should seed");
    sqlx::query(
        "INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code,
            row_no, column_no, layer_no, max_volume_cm3,
            used_volume_cm3, max_sku_count, location_type, status
         )
         VALUES ($1, $2, $3, $4, 'DI-ACCEPT-LOC',
                 1, 1, 1, 100000, 0, 10, 'storage', 'available')",
    )
    .bind(fixture.location_id)
    .bind(fixture.owner_id)
    .bind(fixture.warehouse_id)
    .bind(zone_id)
    .execute(pool)
    .await
    .expect("location should seed");
    sqlx::query(
        "INSERT INTO quality_liaison_types (
            id, owner_id, type_code, type_name, approval_template_id,
            approver_user_id, timeout_seconds, enabled, created_by
         )
         VALUES ($1, $2, 'inbound_unqualified', '入库不合格',
                 'TPL-DI-ACCEPT', $3, 3600, TRUE, $3)",
    )
    .bind(Uuid::new_v4())
    .bind(fixture.owner_id)
    .bind(fixture.reviewer_user_id)
    .execute(pool)
    .await
    .expect("quality liaison type should seed");
    fixture
}

fn reviewer_context(fixture: &Fixture) -> AuthContext {
    AuthContext {
        user_id: fixture.reviewer_user_id,
        owner_id: fixture.owner_id,
        actor_name: "药检验收质量审批员".to_string(),
        permissions: vec!["mql.quality-liaison.approve".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_inventory_batch(pool: &PgPool, fixture: &Fixture, batch_no: &str) -> Uuid {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code
         )
         VALUES ($1, $2, 'P-DI-ACCEPT', $3, '2026-01-01', '2028-01-01',
                 10, 0, 'qualified', $4, 'DI-ACCEPT-LOC')",
    )
    .bind(batch_id)
    .bind(fixture.owner_id)
    .bind(batch_no)
    .bind(fixture.location_id)
    .execute(pool)
    .await
    .expect("inventory batch should seed");
    batch_id
}

async fn seed_inspecting_asn(pool: &PgPool, fixture: &Fixture, receipt_no: &str) -> Uuid {
    let asn_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO receiving_orders (
            id, owner_id, receipt_no, document_type, supplier_id,
            warehouse_id, status
         )
         VALUES ($1, $2, $3, 'purchase_inbound', $4, $5, 'inspecting')",
    )
    .bind(asn_id)
    .bind(fixture.owner_id)
    .bind(receipt_no)
    .bind(fixture.supplier_id)
    .bind(fixture.warehouse_id)
    .execute(pool)
    .await
    .expect("ASN should seed");
    sqlx::query(
        "INSERT INTO receiving_order_lines (
            id, receiving_order_id, owner_id, line_no, product_id,
            product_code, expected_qty
         )
         VALUES ($1, $2, $3, 1, $4, 'P-DI-ACCEPT', 10)",
    )
    .bind(Uuid::new_v4())
    .bind(asn_id)
    .bind(fixture.owner_id)
    .bind(fixture.product_id)
    .execute(pool)
    .await
    .expect("ASN line should seed");
    sqlx::query(
        "INSERT INTO receiving_order_receipts (
            id, receiving_order_id, owner_id, actual_qty, shortage_qty,
            rejected_qty, occurred_at
         )
         VALUES ($1, $2, $3, 10, 0, 0, now())",
    )
    .bind(Uuid::new_v4())
    .bind(asn_id)
    .bind(fixture.owner_id)
    .execute(pool)
    .await
    .expect("receipt should seed");
    asn_id
}

async fn seed_report(
    pool: &PgPool,
    fixture: &Fixture,
    asn_id: Uuid,
    batch_no: &str,
    qualified: bool,
    copy_status: &str,
) -> Uuid {
    let report_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let attachment_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO attachments (
            id, owner_id, module, entity_type, entity_id, file_name,
            content_type, size_bytes, storage_key, sha256, uploaded_by
         )
         VALUES ($1, $2, 'M-DI', 'drug_inspection', $3, 'report.pdf',
                 'application/pdf', 12, $4, $5, $6)",
    )
    .bind(attachment_id)
    .bind(fixture.owner_id)
    .bind(report_id)
    .bind(format!("test/{attachment_id}.pdf"))
    .bind(format!("hash-{attachment_id}"))
    .bind(fixture.user_id)
    .execute(pool)
    .await
    .expect("attachment should seed");
    sqlx::query(
        "INSERT INTO drug_inspection_reports (
            id, owner_id, product_id, batch_no, created_by
         )
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(report_id)
    .bind(fixture.owner_id)
    .bind(fixture.product_id)
    .bind(batch_no)
    .bind(fixture.user_id)
    .execute(pool)
    .await
    .expect("report should seed");
    sqlx::query(
        "INSERT INTO drug_inspection_report_versions (
            id, report_id, owner_id, version_number, report_no,
            original_file_id, original_file_hash, source, processing_mode,
            qualified, status, uploaded_by, reviewed_by, reviewed_at,
            review_result, customer_copy_status
         )
         VALUES ($1, $2, $3, 1, $4, $5, $6, 'manual_upload', 'none',
                 $7, 'confirmed', $8, $9, now(), 'confirmed', $10)",
    )
    .bind(version_id)
    .bind(report_id)
    .bind(fixture.owner_id)
    .bind(format!("REPORT-{batch_no}"))
    .bind(attachment_id)
    .bind(format!("hash-{attachment_id}"))
    .bind(qualified)
    .bind(fixture.user_id)
    .bind(fixture.reviewer_user_id)
    .bind(copy_status)
    .execute(pool)
    .await
    .expect("report version should seed");
    sqlx::query(
        "UPDATE drug_inspection_reports
         SET current_version_id = $2
         WHERE id = $1",
    )
    .bind(report_id)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("current report should update");
    sqlx::query(
        "INSERT INTO drug_inspection_asn_links (
            id, owner_id, asn_id, batch_no, report_id,
            source_version_id, source, linked_by
         )
         VALUES ($1, $2, $3, $4, $5, $6, 'reused', $7)",
    )
    .bind(Uuid::new_v4())
    .bind(fixture.owner_id)
    .bind(asn_id)
    .bind(batch_no)
    .bind(report_id)
    .bind(version_id)
    .bind(fixture.user_id)
    .execute(pool)
    .await
    .expect("ASN report link should seed");
    version_id
}

fn inspect_request(batch_no: &str) -> InspectReceivingOrderRequest {
    InspectReceivingOrderRequest {
        batch_no: batch_no.to_string(),
        accepted_qty: 10.into(),
        rejected_qty: 0.into(),
        production_date: "2026-01-01".to_string(),
        expiry_date: "2028-01-01".to_string(),
        quality_status: STATUS_QUALIFIED.to_string(),
        trace_codes: vec![],
        appearance_check: Some("完好".to_string()),
        package_check: Some("完好".to_string()),
        instruction_check: Some("有".to_string()),
        label_check: Some("清晰".to_string()),
        sampling_qty: Some(1.into()),
        approval_no: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn acceptance_warns_or_blocks_missing_and_rejects_unqualified_without_blocking_copy_failure(
    pool: PgPool,
) {
    let fixture = seed_fixture(&pool).await;
    let ctx = context(&fixture);
    let repo = PgWave3Repository::new(pool.clone());
    sqlx::query(
        "INSERT INTO drug_inspection_requirement_rules (
            id, owner_id, special_drug_category, missing_behavior,
            enabled, updated_by
         )
         VALUES ($1, $2, 'none', 'warning', TRUE, $3)",
    )
    .bind(Uuid::new_v4())
    .bind(fixture.owner_id)
    .bind(fixture.user_id)
    .execute(&pool)
    .await
    .expect("requirement rule should seed");

    let warning_asn = seed_inspecting_asn(&pool, &fixture, "ASN-DI-WARNING").await;
    let warned = repo
        .inspect_receiving_order_with_audit(
            &ctx,
            warning_asn,
            inspect_request("B-WARNING"),
            Utc::now().date_naive(),
            Utc::now(),
            "di-accept-warning",
            None,
        )
        .await
        .expect("warning rule should record missing report and continue acceptance");
    assert_eq!(warned.value.batch_no, "B-WARNING");
    let warning_result: String = sqlx::query_scalar(
        "SELECT result
         FROM drug_inspection_acceptance_validations
         WHERE receiving_order_id = $1",
    )
    .bind(warning_asn)
    .fetch_one(&pool)
    .await
    .expect("warning validation should persist");
    assert_eq!(warning_result, "missing_warning");

    sqlx::query(
        "UPDATE drug_inspection_requirement_rules
         SET missing_behavior = 'block', version = version + 1, updated_at = now()
         WHERE owner_id = $1 AND special_drug_category = 'none'",
    )
    .bind(fixture.owner_id)
    .execute(&pool)
    .await
    .expect("requirement rule should switch to block");

    let missing_asn = seed_inspecting_asn(&pool, &fixture, "ASN-DI-MISSING").await;
    let missing = repo
        .inspect_receiving_order_with_audit(
            &ctx,
            missing_asn,
            inspect_request("B-MISSING"),
            Utc::now().date_naive(),
            Utc::now(),
            "di-accept-missing",
            None,
        )
        .await;
    assert!(matches!(
        missing,
        Err(Wave3RepositoryError::DrugInspectionMissingBlocked)
    ));
    let missing_result: String = sqlx::query_scalar(
        "SELECT result
         FROM drug_inspection_acceptance_validations
         WHERE receiving_order_id = $1",
    )
    .bind(missing_asn)
    .fetch_one(&pool)
    .await
    .expect("missing validation should persist");
    assert_eq!(missing_result, "missing_blocked");

    let unqualified_asn = seed_inspecting_asn(&pool, &fixture, "ASN-DI-UNQUALIFIED").await;
    let affected_batch_id = seed_inventory_batch(&pool, &fixture, "B-UNQUALIFIED").await;
    seed_report(
        &pool,
        &fixture,
        unqualified_asn,
        "B-UNQUALIFIED",
        false,
        "available",
    )
    .await;
    let unqualified = repo
        .inspect_receiving_order_with_audit(
            &ctx,
            unqualified_asn,
            inspect_request("B-UNQUALIFIED"),
            Utc::now().date_naive(),
            Utc::now(),
            "di-accept-unqualified",
            None,
        )
        .await;
    assert!(matches!(
        unqualified,
        Err(Wave3RepositoryError::DrugInspectionUnqualifiedBlocked)
    ));
    let liaison_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT
         FROM quality_liaison_orders
         WHERE owner_id = $1 AND trigger_source = 'm-di.acceptance'",
    )
    .bind(fixture.owner_id)
    .fetch_one(&pool)
    .await
    .expect("quality liaison should count");
    assert_eq!(liaison_count, 1);
    let liaison_id: Uuid = sqlx::query_scalar(
        "SELECT id
         FROM quality_liaison_orders
         WHERE owner_id = $1 AND trigger_source = 'm-di.acceptance'",
    )
    .bind(fixture.owner_id)
    .fetch_one(&pool)
    .await
    .expect("quality liaison should query");
    let liaison_repository = PgQualityLiaisonRepository::new(pool.clone());
    let approval_request = QualityLiaisonApprovalCallbackRequest {
        conclusion: "approved".to_string(),
        opinion: "批准隔离同商品同批号库存".to_string(),
        external_approval_id: "DI-ACCEPT-APPROVAL-001".to_string(),
    };
    let approved = liaison_repository
        .apply_approval_callback(
            &reviewer_context(&fixture),
            liaison_id,
            approval_request.clone(),
            Utc::now(),
            "di-accept-unqualified-approval",
        )
        .await
        .expect("quality liaison approval should quarantine inventory");
    assert_eq!(approved.value.status, "approved");
    let replayed = liaison_repository
        .apply_approval_callback(
            &reviewer_context(&fixture),
            liaison_id,
            approval_request,
            Utc::now(),
            "di-accept-unqualified-approval",
        )
        .await
        .expect("quality liaison approval should replay");
    assert!(replayed.replayed);
    let inventory_result: (String, String, String) = sqlx::query_as(
        "SELECT batch.status, change.approval_source, change.approval_id
         FROM inventory_batches AS batch
         JOIN inventory_status_changes AS change
           ON change.owner_id = batch.owner_id AND change.batch_id = batch.id
         WHERE batch.owner_id = $1 AND batch.id = $2",
    )
    .bind(fixture.owner_id)
    .bind(affected_batch_id)
    .fetch_one(&pool)
    .await
    .expect("M3 quality liaison status change should query");
    assert_eq!(
        inventory_result,
        (
            "quarantined".to_string(),
            "quality_liaison".to_string(),
            liaison_id.to_string(),
        )
    );
    let erp_feedback_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT
         FROM inventory_status_erp_feedback_outbox
         WHERE owner_id = $1 AND batch_id = $2",
    )
    .bind(fixture.owner_id)
    .bind(affected_batch_id)
    .fetch_one(&pool)
    .await
    .expect("M3 ERP feedback should query");
    assert_eq!(erp_feedback_count, 1);

    let qualified_asn = seed_inspecting_asn(&pool, &fixture, "ASN-DI-COPY-FAILED").await;
    seed_report(
        &pool,
        &fixture,
        qualified_asn,
        "B-COPY-FAILED",
        true,
        "failed",
    )
    .await;
    let accepted = repo
        .inspect_receiving_order_with_audit(
            &ctx,
            qualified_asn,
            inspect_request("B-COPY-FAILED"),
            Utc::now().date_naive(),
            Utc::now(),
            "di-accept-copy-failed",
            None,
        )
        .await
        .expect("failed customer copy must not block qualified report acceptance");
    assert_eq!(accepted.value.batch_no, "B-COPY-FAILED");

    seed_inventory_batch(&pool, &fixture, "B-COPY-FAILED").await;
    let customer_id = Uuid::new_v4();
    let address_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO customers (
            id, owner_id, customer_code, customer_name, customer_type, status
         )
         VALUES ($1, $2, 'DI-SHIP-CUSTOMER', '药检发货测试客户', 'customer', 'active')",
    )
    .bind(customer_id)
    .bind(fixture.owner_id)
    .execute(&pool)
    .await
    .expect("shipping customer should seed");
    sqlx::query(
        "INSERT INTO customer_addresses (
            id, owner_id, customer_id, province, city, district,
            detail_address, contact_name, contact_phone, is_default
         )
         VALUES ($1, $2, $3, '上海市', '上海市', '浦东新区',
                 '药检发货测试地址', '收货人', '13800000000', TRUE)",
    )
    .bind(address_id)
    .bind(fixture.owner_id)
    .bind(customer_id)
    .execute(&pool)
    .await
    .expect("shipping address should seed");
    let outbound_order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, customer_id,
            delivery_address_id, delivery_address_snapshot, warehouse_id,
            status, short_pick
         )
         VALUES ($1, $2, 'sales_outbound', 'OUT-DI-COPY-FAILED', $3, $4,
                 $5, $6, 'reviewed', FALSE)",
    )
    .bind(outbound_order_id)
    .bind(fixture.owner_id)
    .bind(customer_id)
    .bind(address_id)
    .bind(serde_json::json!({
        "province": "上海市",
        "city": "上海市",
        "district": "浦东新区",
        "detail_address": "药检发货测试地址",
        "contact_name": "收货人",
        "contact_phone": "13800000000"
    }))
    .bind(fixture.warehouse_id)
    .execute(&pool)
    .await
    .expect("outbound order should seed");
    sqlx::query(
        "INSERT INTO outbound_order_lines (
            id, outbound_order_id, owner_id, line_no, product_code,
            batch_no, planned_qty, picked_qty, reviewed_qty, shipped_qty,
            short_pick_qty
         )
         VALUES ($1, $2, $3, 1, 'P-DI-ACCEPT', 'B-COPY-FAILED',
                 10, 10, 10, 0, 0)",
    )
    .bind(Uuid::new_v4())
    .bind(outbound_order_id)
    .bind(fixture.owner_id)
    .execute(&pool)
    .await
    .expect("outbound line should seed");
    let shipped = PgWave4Repository::new(pool.clone())
        .ship_outbound_order(
            &ctx,
            outbound_order_id,
            ShipOutboundOrderRequest {
                delivery_provider_type: "own_fleet".to_string(),
                vehicle_no: Some("DI-VEHICLE-001".to_string()),
                plate_no: "沪A12345".to_string(),
                driver_user_id: Some(fixture.user_id),
                courier_name: None,
                courier_phone: None,
                signature_attachment_id: None,
                loading_temperature_celsius: None,
                cold_chain_packages: Vec::new(),
                package_count: 1,
            },
            Utc::now(),
            "di-copy-failed-shipping",
            None,
        )
        .await
        .expect("failed customer copy must not block M4 shipping");
    assert_eq!(shipped.value.status, "shipped");
    let failed_copy_status: String = sqlx::query_scalar(
        "SELECT version.customer_copy_status
         FROM drug_inspection_asn_links AS link
         JOIN drug_inspection_reports AS report ON report.id = link.report_id
         JOIN drug_inspection_report_versions AS version
           ON version.id = report.current_version_id
         WHERE link.owner_id = $1 AND link.asn_id = $2 AND link.batch_no = 'B-COPY-FAILED'",
    )
    .bind(fixture.owner_id)
    .bind(qualified_asn)
    .fetch_one(&pool)
    .await
    .expect("failed customer copy should remain queryable after shipping");
    assert_eq!(failed_copy_status, "failed");
    let portal_order_projection: (i64, i64) = sqlx::query_as(
        "SELECT
            COUNT(DISTINCT event.id)::BIGINT,
            COUNT(DISTINCT delivery.id)::BIGINT
         FROM event_bus_event AS event
         LEFT JOIN event_bus_delivery AS delivery
           ON delivery.owner_id = event.owner_id
          AND delivery.event_id = event.id
         LEFT JOIN event_bus_subscription AS subscription
           ON subscription.owner_id = delivery.owner_id
          AND subscription.id = delivery.subscription_id
          AND subscription.subscriber_key = 'mdi-customer-portal'
         WHERE event.owner_id = $1
           AND event.event_type = 'portal.customer_order.snapshot'
           AND event.resource_id = $2",
    )
    .bind(fixture.owner_id)
    .bind(outbound_order_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("shipped order portal projection should query");
    assert_eq!(portal_order_projection, (1, 1));
}
