use chrono::{Duration, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
    wave3_repository::PgWave3Repository,
    wave5_repository::PgWave5Repository,
};
use wms_domain::{
    CalculateBillingChargesRequest, ChangeInventoryStatusRequest, ConfirmBillingStatementRequest,
    CreateBillingAccountRequest, CreateBillingContractRequest, CreateBillingRuleRequest,
    CreateReceivingOrderRequest, GenerateBillingStatementRequest,
    IngestTemperatureExcursionRequest, IngestTemperatureReadingRequest,
    InspectReceivingOrderRequest, PutawayRequest, ReceiveReceivingOrderRequest, ReceivingOrderLine,
    ReceivingReceiptDetails, SignInspectionRequest,
};

#[path = "support/auth.rs"]
mod auth_support;
mod postgres_test_support;

use postgres_test_support::ensure_audit_partition;

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

fn audit(ctx: &AuthContext, action: &str, module: &str, resource_type: &str) -> AuditWriteRequest {
    AuditWriteRequest::from_auth_context(ctx, action, module, resource_type, "", None)
}

fn receiving_order_req(receipt_no: &str) -> CreateReceivingOrderRequest {
    CreateReceivingOrderRequest {
        receipt_no: receipt_no.to_string(),
        document_type: "purchase_inbound".to_string(),
        supplier_id: Some(Uuid::new_v4()),
        warehouse_id: Uuid::new_v4(),
        external_ref: Some(format!("ERP-{receipt_no}")),
        expected_arrival_at: Some(Utc::now() + Duration::days(1)),
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

async fn seed_product(pool: &PgPool, owner_id: Uuid, product_code: &str) {
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, volume_cm3, status) VALUES ($1, $2, $3, 'Evidence Product', '1 unit', 'normal_10_30', 1, 'active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_code)
    .execute(pool)
    .await
    .expect("seed evidence product");
}

async fn seed_logger(pool: &PgPool, owner_id: Uuid, now: chrono::DateTime<Utc>) {
    sqlx::query(
        "INSERT INTO cold_chain_devices (id, owner_id, device_code, device_type, status, created_at, updated_at) VALUES ($1, $2, 'CC-PG-001', 'temperature_logger', 'active', $3, $3)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed logger");
}

async fn seed_rate(
    repo: &PgWave3Repository,
    ctx: &AuthContext,
    now: chrono::DateTime<Utc>,
) -> Uuid {
    let account = repo
        .create_billing_account(
            ctx,
            CreateBillingAccountRequest {
                account_code: "BILL-PG-EVIDENCE".to_string(),
                account_name: "Billing evidence".to_string(),
            },
            now,
        )
        .await
        .expect("create billing account");
    let contract = repo
        .create_billing_contract(
            ctx,
            CreateBillingContractRequest {
                account_id: account.id,
                contract_no: "CONTRACT-PG-EVIDENCE".to_string(),
                valid_from: "2026-06-01".to_string(),
                valid_to: "2026-06-30".to_string(),
            },
            now,
        )
        .await
        .expect("create billing contract");
    repo.create_billing_rule(
        ctx,
        CreateBillingRuleRequest {
            contract_id: contract.id,
            charge_item: "storage".to_string(),
            unit: "pallet_day".to_string(),
            unit_price_cents: 125.into(),
            billing_cycle: "monthly".to_string(),
            effective_from: "2026-06-01".to_string(),
            effective_to: "2026-06-30".to_string(),
        },
        now,
    )
    .await
    .expect("create billing rule");
    contract.id
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_billing_account_with_audit_persists_audit_event(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc::now();

    let account = repo
        .create_billing_account_with_audit(
            &ctx,
            CreateBillingAccountRequest {
                account_code: "BILL-AUDIT-001".to_string(),
                account_name: "Billing audit account".to_string(),
            },
            now,
            audit(&ctx, "create_account", "M9", "billing_account"),
        )
        .await
        .expect("create billing account with audit");

    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM billing_accounts WHERE id = $1), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'create_account' AND resource_id = $3)",
    )
    .bind(account.id)
    .bind(owner_id)
    .bind(account.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("query billing account and audit evidence");
    assert_eq!(counts, (1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_billing_rule_with_audit_persists_audit_event(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc::now();
    let account = repo
        .create_billing_account(
            &ctx,
            CreateBillingAccountRequest {
                account_code: "BILL-AUDIT-002".to_string(),
                account_name: "Billing rule audit account".to_string(),
            },
            now,
        )
        .await
        .expect("create billing account");
    let contract = repo
        .create_billing_contract(
            &ctx,
            CreateBillingContractRequest {
                account_id: account.id,
                contract_no: "CONTRACT-AUDIT-002".to_string(),
                valid_from: "2026-06-01".to_string(),
                valid_to: "2026-06-30".to_string(),
            },
            now,
        )
        .await
        .expect("create billing contract");

    let rule = repo
        .create_billing_rule_with_audit(
            &ctx,
            CreateBillingRuleRequest {
                contract_id: contract.id,
                charge_item: "storage".to_string(),
                unit: "pallet_day".to_string(),
                unit_price_cents: 125.into(),
                billing_cycle: "monthly".to_string(),
                effective_from: "2026-06-01".to_string(),
                effective_to: "2026-06-30".to_string(),
            },
            now,
            "m9-rule-audit-key",
            audit(&ctx, "create_rule", "M9", "billing_rule"),
        )
        .await
        .expect("create billing rule with audit")
        .value;

    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM billing_rules WHERE id = $1), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'create_rule' AND resource_id = $3)",
    )
    .bind(rule.id)
    .bind(owner_id)
    .bind(rule.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("query billing rule and audit evidence");
    assert_eq!(counts, (1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn expired_idempotency_key_is_not_replayed(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 11, 30, 0)
        .single()
        .expect("valid time");
    let batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, 'P-001', 'B202606', $3, $4, 10, 0, $5, $6, 'A-01-01', FALSE, $7, $7)
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
    .bind(STATUS_QUALIFIED)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed inventory batch");

    let req = ChangeInventoryStatusRequest {
        batch_id,
        target_status: STATUS_QUARANTINED.to_string(),
        reason: "temperature exception".to_string(),
        approval_source: "温度超标事件".to_string(),
        approval_id: "TEMP-001".to_string(),
    };
    let first = repo
        .change_inventory_status_with_audit(&ctx, req.clone(), now, "idem-status-ttl", None)
        .await
        .expect("first status change should succeed");
    assert!(!first.replayed);
    assert_eq!(first.value.status, STATUS_QUARANTINED);

    sqlx::query(
        r#"
        UPDATE idempotency_request
           SET expires_at = $3,
               response_body = jsonb_set(response_body, '{status}', '"stale"'::jsonb)
         WHERE owner_id = $1 AND idempotency_key = $2
        "#,
    )
    .bind(owner_id)
    .bind("idem-status-ttl")
    .bind(now - Duration::minutes(1))
    .execute(&pool)
    .await
    .expect("expire and poison idempotency response");

    let retry = repo
        .change_inventory_status_with_audit(
            &ctx,
            req,
            now + Duration::minutes(1),
            "idem-status-ttl",
            None,
        )
        .await
        .expect("expired idempotency key should allow a fresh execution");

    assert!(!retry.replayed);
    assert_eq!(retry.value.status, STATUS_QUARANTINED);
    let stored_status: String = sqlx::query_scalar(
        "SELECT response_body->>'status' FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("idem-status-ttl")
    .fetch_one(&pool)
    .await
    .expect("stored status");
    assert_eq!(stored_status, STATUS_QUARANTINED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn inbound_chain_persists_inventory_movement_and_audit_end_to_end(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let second_signer_id = Uuid::new_v4();
    auth_support::seed_receiving_verifiers(&pool, owner_id, &[ctx.user_id, second_signer_id]).await;
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 7, 12, 9, 0, 0)
        .single()
        .expect("valid time");
    ensure_audit_partition(&pool, now).await;
    let supplier_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    sqlx::query("INSERT INTO suppliers (id,owner_id,supplier_code,supplier_name,uscc,status) VALUES ($1,$2,'SUP-CHAIN','链路供应商','USCC-CHAIN','active')")
        .bind(supplier_id).bind(owner_id).execute(&pool).await.expect("seed supplier");
    sqlx::query("INSERT INTO warehouses (id,owner_id,warehouse_code,warehouse_name,warehouse_type,status) VALUES ($1,$2,'WH-CHAIN','链路仓','normal','active')")
        .bind(warehouse_id).bind(owner_id).execute(&pool).await.expect("seed warehouse");
    seed_product(&pool, owner_id, "P-001").await;
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let location_code = format!("M2-CHAIN-LOC-{}", &location_id.to_string()[..8]);
    sqlx::query("INSERT INTO warehouse_zones (id,owner_id,warehouse_id,zone_code,zone_name,temperature_zone,quality_color,status) VALUES ($1,$2,$3,$4,'链路合格区','normal_10_30','qualified_green','active')")
        .bind(zone_id).bind(owner_id).bind(warehouse_id)
        .bind(format!("M2-CHAIN-ZONE-{}", &zone_id.to_string()[..8]))
        .execute(&pool).await.expect("seed zone");
    sqlx::query("INSERT INTO warehouse_locations (id,owner_id,warehouse_id,zone_id,location_code,row_no,column_no,layer_no,max_volume_cm3,used_volume_cm3,max_sku_count,location_type,status) VALUES ($1,$2,$3,$4,$5,1,1,1,100000,0,3,'storage','available')")
        .bind(location_id).bind(owner_id).bind(warehouse_id).bind(zone_id)
        .bind(&location_code).execute(&pool).await.expect("seed location");
    let mut request = receiving_order_req("ASN-CHAIN-001");
    request.supplier_id = Some(supplier_id);
    request.warehouse_id = warehouse_id;
    request.lines[0].batch_no = None;
    request.lines[0].production_date = None;
    request.lines[0].expiry_date = None;
    let order = repo
        .create_receiving_order(&ctx, request, now)
        .await
        .expect("create ASN");
    repo.release_receiving_order_with_audit(
        &ctx,
        order.id,
        now,
        Some("chain-release"),
        Some(audit(&ctx, "release", "M2", "receiving_order")),
    )
    .await
    .expect("release ASN");
    repo.receive_receiving_order_with_audit(
        &ctx,
        order.id,
        ReceiveReceivingOrderRequest {
            actual_qty: 10.into(),
            shortage_qty: 0.into(),
            rejected_qty: 0.into(),
            arrival_temperature_celsius: Some(5.0),
            exception_note: None,
            details: Some(ReceivingReceiptDetails {
                delivery_qty: 10.into(),
                second_receiver_id: None,
                sales_return_batches: vec![],
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
        now,
        "chain-receive",
        Some(audit(&ctx, "receive", "M2", "receiving_receipt")),
    )
    .await
    .expect("receive ASN");
    repo.inspect_receiving_order_with_audit(
        &ctx,
        order.id,
        InspectReceivingOrderRequest {
            batch_no: "B-CHAIN-001".into(),
            accepted_qty: 10.into(),
            rejected_qty: 0.into(),
            production_date: "2026-01-01".into(),
            expiry_date: "2028-01-01".into(),
            quality_status: STATUS_QUALIFIED.into(),
            trace_codes: vec!["TRACE-CHAIN-001".into()],

            appearance_check: Some("完好".to_string()),
            package_check: Some("完好".to_string()),
            instruction_check: Some("有".to_string()),
            label_check: Some("清晰".to_string()),
            sampling_qty: Some(1.into()),
            approval_no: None,
        },
        now.date_naive(),
        now,
        "chain-inspect",
        Some(audit(&ctx, "inspect", "M2", "receiving_inspection")),
    )
    .await
    .expect("inspect ASN");
    repo.sign_receiving_order_with_audit(
        &ctx,
        order.id,
        SignInspectionRequest {
            first_signer_id: ctx.user_id,
            second_signer_id: None,
            dual_required: true,
        },
        now,
        "chain-sign-first",
        Some(audit(&ctx, "sign", "M2", "receiving_inspection_signature")),
    )
    .await
    .expect("first sign ASN");
    let mut second_ctx = ctx.clone();
    second_ctx.user_id = second_signer_id;
    repo.sign_receiving_order_with_audit(
        &second_ctx,
        order.id,
        SignInspectionRequest {
            first_signer_id: ctx.user_id,
            second_signer_id: Some(second_signer_id),
            dual_required: true,
        },
        now,
        "chain-sign-second",
        Some(audit(
            &second_ctx,
            "sign",
            "M2",
            "receiving_inspection_signature",
        )),
    )
    .await
    .expect("second sign ASN");
    repo.putaway_receiving_order_and_inventory_with_audit(
        &ctx,
        order.id,
        PutawayRequest {
            batch_no: "B-CHAIN-001".into(),
            product_code: "P-001".into(),
            qty: 10.into(),
            location_id,
            location_code,
            quality_status: STATUS_QUALIFIED.into(),
            lpn_code: None,
            witness_id: None,
        },
        now,
        "chain-putaway",
        Some(audit(&ctx, "putaway", "M2", "receiving_order")),
    )
    .await
    .expect("putaway ASN");

    let result: (String, i64, i64, i64, i64) = sqlx::query_as(r#"SELECT
        (SELECT status FROM receiving_orders WHERE owner_id=$1 AND id=$2),
        (SELECT COALESCE(SUM(qty_on_hand),0)::BIGINT FROM inventory_batches WHERE owner_id=$1 AND batch_no='B-CHAIN-001'),
        (SELECT COUNT(*) FROM inventory_movements WHERE owner_id=$1 AND source_document_id=$2),
        (SELECT COUNT(*) FROM audit_event WHERE owner_id=$1 AND action IN ('release','receive','inspect','sign','putaway')),
        (SELECT COUNT(*) FROM idempotency_request WHERE owner_id=$1 AND idempotency_key LIKE 'chain-%')"#)
        .bind(owner_id).bind(order.id).fetch_one(&pool).await.expect("query complete chain");
    // release/receive/inspect/sign×2/putaway → 6 审计与 6 幂等键。
    assert_eq!(result, ("completed".into(), 10, 1, 6, 6));
}

#[sqlx::test(migrations = "../../migrations")]
async fn ingest_cold_chain_readings_and_excursions_replay_without_duplicate_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 13, 0, 0)
        .single()
        .expect("valid time");
    seed_logger(&pool, owner_id, now).await;

    let reading_req = IngestTemperatureReadingRequest {
        device_code: "CC-PG-001".to_string(),
        temperature_celsius: 4.2,
        humidity_percent: Some(55.0),
        captured_at: now - Duration::minutes(1),
        external_report_url: None,
        out_of_range: false,
    };
    let reading = repo
        .ingest_temperature_reading_with_audit(
            &ctx,
            reading_req.clone(),
            now,
            "idem-reading-1",
            Some(audit(&ctx, "ingest_reading", "M5", "temperature_reading")),
        )
        .await
        .expect("ingest reading");
    let reading_replay = repo
        .ingest_temperature_reading_with_audit(
            &ctx,
            reading_req,
            now,
            "idem-reading-1",
            Some(audit(&ctx, "ingest_reading", "M5", "temperature_reading")),
        )
        .await
        .expect("replay reading");
    assert_eq!(reading.value.id, reading_replay.value.id);
    assert!(reading_replay.replayed);

    let excursion_req = IngestTemperatureExcursionRequest {
        external_event_id: "EX-PG-001".to_string(),
        device_code: "CC-PG-001".to_string(),
        location_code: Some("COLD-01".to_string()),
        started_at: now - Duration::minutes(10),
        ended_at: Some(now - Duration::minutes(2)),
        min_temperature_celsius: Some(1.0),
        max_temperature_celsius: Some(9.0),
        affected_batch_ids: vec![],
    };
    let excursion = repo
        .ingest_temperature_excursion_with_audit(
            &ctx,
            excursion_req.clone(),
            now,
            "idem-cold-2",
            Some(audit(
                &ctx,
                "ingest_excursion",
                "M5",
                "temperature_excursion",
            )),
        )
        .await
        .expect("ingest excursion");
    let excursion_replay = repo
        .ingest_temperature_excursion_with_audit(
            &ctx,
            excursion_req,
            now,
            "idem-cold-2",
            Some(audit(
                &ctx,
                "ingest_excursion",
                "M5",
                "temperature_excursion",
            )),
        )
        .await
        .expect("replay excursion");
    assert_eq!(excursion.value.id, excursion_replay.value.id);
    assert!(excursion_replay.replayed);

    let counts: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"SELECT
            (SELECT COUNT(*) FROM temperature_readings WHERE owner_id = $1),
            (SELECT COUNT(*) FROM temperature_excursion_events WHERE owner_id = $1),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'idem-reading-1'),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'idem-cold-2'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ingest_reading'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'ingest_excursion')"#,
    ).bind(owner_id).fetch_one(&pool).await.expect("cold-chain evidence counts");
    assert_eq!(counts, (1, 1, 1, 1, 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_charge_statement_and_confirmation_replay_without_duplicate_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let wave3 = PgWave3Repository::new(pool.clone());
    let repo = PgWave5Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 14, 0, 0)
        .single()
        .expect("valid time");
    let contract_id = seed_rate(&wave3, &ctx, now).await;

    let charge_req = CalculateBillingChargesRequest {
        contract_id,
        period_start: "2026-06-01".to_string(),
        period_end: "2026-06-30".to_string(),
        charge_item: "storage".to_string(),
        quantity: 2.into(),
        source_refs: vec!["putaway:PG-EVIDENCE".to_string()],
    };
    let charge = repo
        .calculate_period_charges(
            &ctx,
            charge_req.clone(),
            now,
            "idem-charge-1",
            Some(audit(
                &ctx,
                "calculate_period_charges",
                "M9",
                "billing_charge_calculation",
            )),
        )
        .await
        .expect("calculate charge");
    let charge_replay = repo
        .calculate_period_charges(
            &ctx,
            charge_req,
            now,
            "idem-charge-1",
            Some(audit(
                &ctx,
                "calculate_period_charges",
                "M9",
                "billing_charge_calculation",
            )),
        )
        .await
        .expect("replay charge");
    assert_eq!(charge.value.id, charge_replay.value.id);
    assert!(charge_replay.replayed);

    let statement_req = GenerateBillingStatementRequest {
        contract_id,
        period_start: "2026-06-01".to_string(),
        period_end: "2026-06-30".to_string(),
        charge_ids: vec![charge.value.id],
    };
    let statement = repo
        .generate_billing_statement(
            &ctx,
            statement_req.clone(),
            now,
            "idem-statement-1",
            Some(audit(
                &ctx,
                "generate_billing_statement",
                "M9",
                "billing_statement",
            )),
        )
        .await
        .expect("generate statement");
    let statement_replay = repo
        .generate_billing_statement(
            &ctx,
            statement_req,
            now,
            "idem-statement-1",
            Some(audit(
                &ctx,
                "generate_billing_statement",
                "M9",
                "billing_statement",
            )),
        )
        .await
        .expect("replay statement");
    assert_eq!(statement.value.id, statement_replay.value.id);
    assert!(statement_replay.replayed);

    let confirm_req = ConfirmBillingStatementRequest {
        confirmation_note: Some("checked".to_string()),
    };
    let confirmed = repo
        .confirm_billing_statement(
            &ctx,
            statement.value.id,
            confirm_req.clone(),
            now,
            "idem-confirm-statement-1",
            Some(audit(
                &ctx,
                "confirm_billing_statement",
                "M9",
                "billing_statement",
            )),
        )
        .await
        .expect("confirm statement");
    let confirmed_replay = repo
        .confirm_billing_statement(
            &ctx,
            statement.value.id,
            confirm_req,
            now,
            "idem-confirm-statement-1",
            Some(audit(
                &ctx,
                "confirm_billing_statement",
                "M9",
                "billing_statement",
            )),
        )
        .await
        .expect("replay confirmation");
    assert_eq!(confirmed.value.id, confirmed_replay.value.id);
    assert!(confirmed_replay.replayed);

    let counts: (i64, i64, i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"SELECT
            (SELECT COUNT(*) FROM billing_charge_calculations WHERE owner_id = $1),
            (SELECT COUNT(*) FROM billing_statements WHERE owner_id = $1),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'idem-charge-1'),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'idem-statement-1'),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'idem-confirm-statement-1'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'calculate_period_charges'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'generate_billing_statement'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'confirm_billing_statement'),
            (SELECT version FROM billing_statements WHERE owner_id = $1 AND id = $2)"#,
    ).bind(owner_id).bind(statement.value.id).fetch_one(&pool).await.expect("billing evidence counts");
    assert_eq!(counts, (1, 1, 1, 1, 1, 1, 1, 1, 2));
}
