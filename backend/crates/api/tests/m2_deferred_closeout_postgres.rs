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
            expected_qty: 10.into(),
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
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'M2 ASN 测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("M2-ASN-OWNER-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed ASN owner");
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
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, volume_cm3, attrs, status) VALUES ($1, $2, $3, 'M2 Test Product', '1 unit', 'normal_10_30', 1.0, '{\"unit_volume_cm3\": 1}', 'active') ON CONFLICT (owner_id, product_code) DO UPDATE SET volume_cm3 = EXCLUDED.volume_cm3, attrs = EXCLUDED.attrs, status = 'active' RETURNING id",
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
    .bind(format!("M2-LOC-{}", &location_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed location");
    (warehouse_id, location_id)
}

async fn seed_idle_lpn(pool: &PgPool, owner_id: Uuid, lpn_code: &str) {
    sqlx::query(
        r#"
        INSERT INTO lpn_containers (
            id, owner_id, lpn_code, container_type, status, current_lock_category, created_at, updated_at
        ) VALUES ($1, $2, $3, 'pallet', 'idle', 'qualified', now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(lpn_code)
    .execute(pool)
    .await
    .expect("idle LPN should seed");
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
async fn create_asn_requires_supplier_and_non_past_expected_arrival(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repository = PgWave3Repository::new(pool.clone());
    let now = chrono::Utc::now();

    let mut missing_supplier_request = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    missing_supplier_request.supplier_id = None;
    let missing_supplier = repository
        .create_receiving_order(&context(owner_id), missing_supplier_request, now)
        .await;
    assert!(matches!(
        missing_supplier,
        Err(Wave3RepositoryError::MissingSupplier)
    ));

    let mut missing_arrival = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut missing_arrival).await;
    missing_arrival.supplier_id = Some(Uuid::new_v4());
    missing_arrival.expected_arrival_at = None;
    let missing_arrival_result = repository
        .create_receiving_order(&context(owner_id), missing_arrival, now)
        .await;
    assert!(matches!(
        missing_arrival_result,
        Err(Wave3RepositoryError::MissingExpectedArrival)
    ));

    let mut past_arrival = request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, None);
    seed_asn_references(&pool, owner_id, &mut past_arrival).await;
    past_arrival.supplier_id = Some(Uuid::new_v4());
    past_arrival.expected_arrival_at = Some(now - chrono::Duration::days(1));
    let past_arrival_result = repository
        .create_receiving_order(&context(owner_id), past_arrival, now)
        .await;
    assert!(matches!(
        past_arrival_result,
        Err(Wave3RepositoryError::InvalidExpectedArrival)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn inspection_uses_actual_receipt_quantity_and_blocks_early_signature(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = context(owner_id);
    let first_signer_id = ctx.user_id;
    let second_signer_id = Uuid::new_v4();
    auth_support::seed_receiving_verifiers(&pool, owner_id, &[first_signer_id, second_signer_id])
        .await;
    let foreign_owner_id = Uuid::new_v4();
    let foreign_signer_id = Uuid::new_v4();
    auth_support::seed_receiving_verifiers(&pool, foreign_owner_id, &[foreign_signer_id]).await;
    let repository = PgWave3Repository::new(pool.clone());
    let mut create_request = request(
        RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
        Some("B-QUANTITY-GATE"),
    );
    seed_asn_references(&pool, owner_id, &mut create_request).await;
    let order = repository
        .create_receiving_order(&ctx, create_request, chrono::Utc::now())
        .await
        .expect("create quantity-gate order");
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
                actual_qty: 8.into(),
                shortage_qty: 2.into(),
                rejected_qty: 0.into(),
                arrival_temperature_celsius: None,
                exception_note: None,
                details: Some(ReceivingReceiptDetails {
                    delivery_qty: 10.into(),
                    second_receiver_id: None,
                    sales_return_batches: vec![wms_domain::SalesReturnReceivingBatch {
                        batch_no: "B-QUANTITY-GATE".to_string(),
                        quantity: 10.into(),
                        rejected_qty: 0.into(),
                        reject_reason: None,
                    }],
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
            "receive-quantity-gate",
            None,
        )
        .await
        .expect("receive quantity-gate order");

    let inspection = |accepted_qty: i64, key: &str| InspectReceivingOrderRequest {
        batch_no: "B-QUANTITY-GATE".to_string(),
        accepted_qty: accepted_qty.into(),
        rejected_qty: 0.into(),
        production_date: "2026-01-01".to_string(),
        expiry_date: "2028-01-01".to_string(),
        quality_status: STATUS_QUALIFIED.to_string(),
        trace_codes: vec![format!("TRACE-{key}")],
        appearance_check: Some("完好".to_string()),
        package_check: Some("完好".to_string()),
        instruction_check: Some("有".to_string()),
        label_check: Some("清晰".to_string()),
        sampling_qty: Some(1.into()),
        approval_no: None,
    };
    let over_actual = repository
        .inspect_receiving_order_with_audit(
            &ctx,
            order.id,
            inspection(9, "over"),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 12).expect("valid date"),
            chrono::Utc::now(),
            "inspect-over-actual",
            None,
        )
        .await;
    assert!(matches!(
        over_actual,
        Err(Wave3RepositoryError::QuantityClosureMismatch)
    ));

    repository
        .inspect_receiving_order_with_audit(
            &ctx,
            order.id,
            inspection(4, "first"),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 12).expect("valid date"),
            chrono::Utc::now(),
            "inspect-first-half",
            None,
        )
        .await
        .expect("inspect first half");
    let premature_signature = repository
        .sign_receiving_order_with_audit(
            &ctx,
            order.id,
            SignInspectionRequest {
                first_signer_id,
                second_signer_id: Some(foreign_signer_id),
                dual_required: true,
            },
            chrono::Utc::now(),
            "sign-before-complete",
            None,
        )
        .await;
    assert!(matches!(
        premature_signature,
        Err(Wave3RepositoryError::UnauthorizedSigner)
    ));

    let premature_signature = repository
        .sign_receiving_order_with_audit(
            &ctx,
            order.id,
            SignInspectionRequest {
                first_signer_id,
                second_signer_id: None,
                dual_required: true,
            },
            chrono::Utc::now(),
            "sign-before-complete-authorized",
            None,
        )
        .await;
    assert!(matches!(
        premature_signature,
        Err(Wave3RepositoryError::QuantityClosureMismatch)
    ));

    repository
        .inspect_receiving_order_with_audit(
            &ctx,
            order.id,
            inspection(4, "second"),
            chrono::NaiveDate::from_ymd_opt(2026, 7, 12).expect("valid date"),
            chrono::Utc::now(),
            "inspect-second-half",
            None,
        )
        .await
        .expect("inspect second half");

    // 客户端 dual_required=false 不能绕过 M-VR 双人策略；一次提交两名签字人被拒。
    let proxy_both = repository
        .sign_receiving_order_with_audit(
            &ctx,
            order.id,
            SignInspectionRequest {
                first_signer_id,
                second_signer_id: Some(second_signer_id),
                dual_required: false,
            },
            chrono::Utc::now(),
            "sign-client-proxy-both",
            None,
        )
        .await;
    assert!(matches!(
        proxy_both,
        Err(Wave3RepositoryError::UnauthorizedSigner)
    ));

    sqlx::query(
        "UPDATE products SET special_drug_category = 'narcotic' WHERE owner_id = $1 AND product_code = 'P-M2-001'",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("switch inbound product to approval-required category");
    let missing_approval = repository
        .sign_receiving_order_with_audit(
            &ctx,
            order.id,
            SignInspectionRequest {
                first_signer_id,
                second_signer_id: None,
                dual_required: false,
            },
            chrono::Utc::now(),
            "sign-missing-approval",
            None,
        )
        .await;
    assert!(matches!(
        missing_approval,
        Err(Wave3RepositoryError::DualPersonApprovalRequired)
    ));
    let approval_id = Uuid::new_v4();
    let approval_time = chrono::Utc::now();
    sqlx::query(
        r#"
        INSERT INTO h4_approval_records (
            id, owner_id, scenario, business_ref, dedupe_key, approver_user,
            process_id, callback_path, summary, status, external_approval_id,
            approved_by, approved_at, created_at, updated_at
        )
        VALUES ($1, $2, 'mvr.dual_person', $3, 'm2-inspection-approval', $4,
                'mvr-dual-person', '/api/v1/wechat-notify/approvals/callback',
                '特殊药品入库验收', 'approved', 'WX-M2-APPROVED', $4, $5, $5, $5)
        "#,
    )
    .bind(approval_id)
    .bind(owner_id)
    .bind(order.id.to_string())
    .bind(second_signer_id.to_string())
    .bind(approval_time)
    .execute(&pool)
    .await
    .expect("seed approved H4 dual-person approval");
    repository
        .sign_receiving_order_with_audit(
            &ctx,
            order.id,
            SignInspectionRequest {
                first_signer_id,
                second_signer_id: None,
                dual_required: true,
            },
            chrono::Utc::now(),
            "sign-first-after-approval",
            None,
        )
        .await
        .expect("first signature after approval");
    let mut second_ctx = ctx.clone();
    second_ctx.user_id = second_signer_id;
    repository
        .sign_receiving_order_with_audit(
            &second_ctx,
            order.id,
            SignInspectionRequest {
                first_signer_id,
                second_signer_id: Some(second_signer_id),
                dual_required: true,
            },
            chrono::Utc::now(),
            "sign-after-complete",
            None,
        )
        .await
        .expect("sign after complete inspection");
    let execution_evidence: (Uuid, Uuid) = sqlx::query_as(
        "SELECT strategy_rule_id, approval_record_id FROM receiving_inspection_signatures WHERE receiving_order_id = $1",
    )
    .bind(order.id)
    .fetch_one(&pool)
    .await
        .expect("inspection signature strategy should query");
    assert_eq!(execution_evidence.1, approval_id);
    let putaway_task: (String, String, String, i64, Uuid) = sqlx::query_as(
        r#"
        SELECT task_type_code, status, product_code, planned_qty::BIGINT, source_doc_id
          FROM warehouse_tasks
         WHERE owner_id = $1
           AND source_doc_type = 'receiving_order'
           AND source_doc_id = $2
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("inspection signature should create a putaway task");
    assert_eq!(
        putaway_task,
        (
            "putaway".to_string(),
            "pending_assignment".to_string(),
            "P-M2-001".to_string(),
            8,
            order.id,
        )
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn purchase_inbound_does_not_accept_a_prefilled_batch(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repository = PgWave3Repository::new(pool);

    let result = repository
        .create_receiving_order(
            &context(owner_id),
            request(RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, Some("B-001")),
            chrono::Utc::now(),
        )
        .await;

    assert!(matches!(
        result,
        Err(Wave3RepositoryError::InvalidBatchPolicy)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn sales_return_requires_the_original_batch(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repository = PgWave3Repository::new(pool);

    let result = repository
        .create_receiving_order(
            &context(owner_id),
            request(RECEIVING_DOCUMENT_TYPE_SALES_RETURN, None),
            chrono::Utc::now(),
        )
        .await;

    assert!(matches!(
        result,
        Err(Wave3RepositoryError::InvalidBatchPolicy)
    ));
}

include!("m2_deferred_closeout_postgres_included/part2.rs");
