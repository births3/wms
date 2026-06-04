use chrono::{NaiveDate, TimeZone, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    wave5_repository::{PgWave5Repository, Wave5RepositoryError},
};
use wms_domain::{
    CalculateBillingChargesRequest, ConfirmBillingStatementRequest,
    ConfirmContainerRecoveryRequest, CreateCrossdockPlanRequest, CreatePackJobRequest,
    CreatePackingStationRequest, CreateRetailReplenishmentSuggestionRequest,
    GenerateBillingStatementRequest, IngestTransitTemperatureRequest, PrintWaybillRequest,
    ReceiveTmsDispatchRequest, WeighPackJobRequest,
};

static MIGRATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct SeededOutboundOrder {
    id: Uuid,
    customer_id: Uuid,
}

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "wave5-postgres-test".to_string(),
        permissions: vec![
            "m-pk.write".to_string(),
            "m8.write".to_string(),
            "m9.write".to_string(),
            "m10.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
    }
}

async fn migrated_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL is required for wave5_postgres tests");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect postgres");
    let _guard = MIGRATION_LOCK.lock().await;
    let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let migrator = sqlx::migrate::Migrator::new(migrations_path.as_path())
        .await
        .expect("load migrations");
    migrator.run(&pool).await.expect("run migrations");
    pool
}

async fn seed_outbound_order(
    pool: &PgPool,
    owner_id: Uuid,
    order_no: &str,
    now: chrono::DateTime<Utc>,
) -> SeededOutboundOrder {
    let id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, wms_order_no, erp_order_no, customer_id, warehouse_id,
            required_ship_at, status, short_pick, created_at, updated_at
        )
        VALUES ($1, $2, $3, NULL, $4, $5, NULL, 'confirmed', FALSE, $6, $6)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(order_no)
    .bind(customer_id)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("seed outbound order");
    SeededOutboundOrder { id, customer_id }
}

async fn seed_billing_contract(
    pool: &PgPool,
    owner_id: Uuid,
    charge_item: &str,
    unit_price_cents: i64,
    now: chrono::DateTime<Utc>,
) -> Uuid {
    let account_id = Uuid::new_v4();
    let contract_id = Uuid::new_v4();
    let valid_from = NaiveDate::from_ymd_opt(2026, 6, 1).expect("valid date");
    let valid_to = NaiveDate::from_ymd_opt(2026, 6, 30).expect("valid date");
    sqlx::query(
        r#"
        INSERT INTO billing_accounts (
            id, owner_id, account_code, account_name, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'Wave 5 Billing Account', 'active', $4, $4)
        "#,
    )
    .bind(account_id)
    .bind(owner_id)
    .bind(format!("BILL-{}", owner_id.simple()))
    .bind(now)
    .execute(pool)
    .await
    .expect("seed billing account");
    sqlx::query(
        r#"
        INSERT INTO billing_contracts (
            id, owner_id, account_id, contract_no, valid_from, valid_to,
            status, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $7)
        "#,
    )
    .bind(contract_id)
    .bind(owner_id)
    .bind(account_id)
    .bind(format!("CONTRACT-{}", owner_id.simple()))
    .bind(valid_from)
    .bind(valid_to)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed billing contract");
    sqlx::query(
        r#"
        INSERT INTO billing_rules (
            id, owner_id, contract_id, charge_item, unit, unit_price_cents,
            billing_cycle, effective_from, effective_to, created_at
        )
        VALUES ($1, $2, $3, $4, 'job', $5, 'monthly', $6, $7, $8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(contract_id)
    .bind(charge_item)
    .bind(unit_price_cents)
    .bind(valid_from)
    .bind(valid_to)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed billing rule");
    contract_id
}

async fn owner_row_count(pool: &PgPool, table: &str, owner_id: Uuid) -> i64 {
    let sql = format!("SELECT COUNT(*)::BIGINT FROM {table} WHERE owner_id = $1");
    sqlx::query_scalar(&sql)
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .expect("count owner rows")
}

async fn audit_count(pool: &PgPool, owner_id: Uuid) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
          FROM audit_event
         WHERE owner_id = $1
           AND module = ANY($2)
        "#,
    )
    .bind(owner_id)
    .bind(vec!["M-PK", "M8", "M9", "M10"])
    .fetch_one(pool)
    .await
    .expect("count audit events")
}

#[tokio::test]
async fn wave5_owner_isolation() {
    let pool = migrated_pool().await;
    let repo = PgWave5Repository::new(pool.clone());
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let ctx_a = ctx(owner_a);
    let ctx_b = ctx(owner_b);
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 9, 0, 0)
        .single()
        .expect("valid time");
    let outbound_a =
        seed_outbound_order(&pool, owner_a, &format!("W5-OWN-{}", owner_a.simple()), now).await;
    let contract_a = seed_billing_contract(&pool, owner_a, "packing_operation", 125, now).await;

    let station_req = CreatePackingStationRequest {
        station_code: format!("PK-{}", owner_a.simple()),
        station_name: "Wave 5 Packing Station".to_string(),
        printer_code: Some("PRN-W5-01".to_string()),
        scale_code: Some("SCL-W5-01".to_string()),
        temperature_zone: "normal".to_string(),
    };
    let station = repo
        .create_packing_station(
            &ctx_a,
            station_req.clone(),
            now,
            "wave5-owner-station",
            None,
        )
        .await
        .expect("create station");
    assert!(!station.replayed);
    let replayed_station = repo
        .create_packing_station(&ctx_a, station_req, now, "wave5-owner-station", None)
        .await
        .expect("replay station");
    assert!(replayed_station.replayed);
    assert_eq!(replayed_station.value.id, station.value.id);

    let pack_req = CreatePackJobRequest {
        outbound_order_id: outbound_a.id,
        station_id: Some(station.value.id),
        job_no: format!("PKJOB-{}", owner_a.simple()),
        pack_mode: "station".to_string(),
        recommended_box_type: "M".to_string(),
        actual_box_type: "M".to_string(),
        adjustment_reason: None,
        outbound_lpn: format!("LPN-{}", owner_a.simple()),
        trace_codes: vec!["TC-W5-OWNER-001".to_string()],
    };
    let pack_job = repo
        .create_pack_job(&ctx_a, pack_req.clone(), now, "wave5-owner-pack", None)
        .await
        .expect("create pack job")
        .value;
    assert_eq!(pack_job.owner_id, owner_a);
    let cross_owner_pack = repo
        .create_pack_job(&ctx_b, pack_req, now, "wave5-owner-cross-pack", None)
        .await
        .expect_err("cross owner outbound order must not be visible");
    assert_eq!(cross_owner_pack, Wave5RepositoryError::NotFound);

    let suggestion = repo
        .create_replenishment_suggestion(
            &ctx_a,
            CreateRetailReplenishmentSuggestionRequest {
                store_id: Uuid::new_v4(),
                product_code: "P-W5-OWNER".to_string(),
                period_key: "2026-W23".to_string(),
                min_qty: 10,
                max_qty: 40,
                current_qty: 12,
                in_transit_qty: 4,
                daily_sales_avg: 3,
            },
            now,
            "wave5-owner-replenishment",
            None,
        )
        .await
        .expect("create replenishment suggestion")
        .value;
    assert_eq!(suggestion.owner_id, owner_a);

    let charge_req = CalculateBillingChargesRequest {
        contract_id: contract_a,
        period_start: "2026-06-01".to_string(),
        period_end: "2026-06-30".to_string(),
        charge_item: "packing_operation".to_string(),
        quantity: 2,
        source_refs: vec![format!("packing_job:{}", pack_job.id)],
    };
    let charge = repo
        .calculate_period_charges(&ctx_a, charge_req.clone(), now, "wave5-owner-charge", None)
        .await
        .expect("calculate charge")
        .value;
    assert_eq!(charge.amount_cents, 250);
    let cross_owner_charge = repo
        .calculate_period_charges(&ctx_b, charge_req, now, "wave5-owner-cross-charge", None)
        .await
        .expect_err("cross owner contract rule must not be visible");
    assert_eq!(cross_owner_charge, Wave5RepositoryError::NotFound);

    let dispatch_req = ReceiveTmsDispatchRequest {
        dispatch_no: format!("DSP-{}", owner_a.simple()),
        outbound_order_id: outbound_a.id,
        delivery_provider_type: "third_party_express".to_string(),
        vehicle_no: None,
        plate_no: Some("ZJ-A12345".to_string()),
        driver_user_id: None,
        carrier_code: Some("SF".to_string()),
        waybill_no: Some("SF-W5-OWNER".to_string()),
        version: 1,
        scheduled_load_at: Some(now),
    };
    let dispatch = repo
        .receive_tms_dispatch(
            &ctx_a,
            dispatch_req.clone(),
            now,
            "wave5-owner-dispatch",
            None,
        )
        .await
        .expect("receive dispatch")
        .value;
    assert_eq!(dispatch.owner_id, owner_a);
    let cross_owner_dispatch = repo
        .receive_tms_dispatch(
            &ctx_b,
            dispatch_req,
            now,
            "wave5-owner-cross-dispatch",
            None,
        )
        .await
        .expect_err("cross owner outbound order must not be visible to tms");
    assert_eq!(cross_owner_dispatch, Wave5RepositoryError::NotFound);

    assert_eq!(owner_row_count(&pool, "packing_stations", owner_a).await, 1);
    assert_eq!(owner_row_count(&pool, "packing_jobs", owner_a).await, 1);
    assert_eq!(
        owner_row_count(&pool, "retail_replenishment_suggestions", owner_a).await,
        1
    );
    assert_eq!(
        owner_row_count(&pool, "billing_charge_calculations", owner_a).await,
        1
    );
    assert_eq!(owner_row_count(&pool, "tms_dispatches", owner_a).await, 1);
    assert_eq!(owner_row_count(&pool, "packing_jobs", owner_b).await, 0);
    assert_eq!(
        owner_row_count(&pool, "billing_charge_calculations", owner_b).await,
        0
    );
    assert_eq!(owner_row_count(&pool, "tms_dispatches", owner_b).await, 0);
    assert_eq!(
        owner_row_count(&pool, "idempotency_request", owner_a).await,
        5
    );
    assert_eq!(audit_count(&pool, owner_a).await, 5);
}

#[tokio::test]
async fn chain_store_replenishment_to_packing_tms_and_billing() {
    let pool = migrated_pool().await;
    let repo = PgWave5Repository::new(pool.clone());
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 10, 0, 0)
        .single()
        .expect("valid time");
    let outbound = seed_outbound_order(
        &pool,
        owner_id,
        &format!("W5-CHAIN-{}", owner_id.simple()),
        now,
    )
    .await;
    let contract_id = seed_billing_contract(&pool, owner_id, "packing_operation", 125, now).await;
    let store_id = Uuid::new_v4();
    let product_code = "P-W5-CHAIN".to_string();

    let suggestion = repo
        .create_replenishment_suggestion(
            &ctx,
            CreateRetailReplenishmentSuggestionRequest {
                store_id,
                product_code: product_code.clone(),
                period_key: "2026-W23".to_string(),
                min_qty: 10,
                max_qty: 40,
                current_qty: 8,
                in_transit_qty: 5,
                daily_sales_avg: 4,
            },
            now,
            "wave5-chain-replenishment",
            None,
        )
        .await
        .expect("create replenishment suggestion")
        .value;
    assert_eq!(suggestion.suggested_qty, 27);

    let crossdock = repo
        .create_crossdock_plan(
            &ctx,
            CreateCrossdockPlanRequest {
                asn_id: Uuid::new_v4(),
                outbound_order_id: outbound.id,
                store_id,
                product_code: product_code.clone(),
                qty: suggestion.suggested_qty,
            },
            now,
            "wave5-chain-crossdock",
            None,
        )
        .await
        .expect("create crossdock")
        .value;
    assert_eq!(crossdock.status, "planned");

    let station = repo
        .create_packing_station(
            &ctx,
            CreatePackingStationRequest {
                station_code: format!("PK-CHAIN-{}", owner_id.simple()),
                station_name: "Wave 5 Chain Packing Station".to_string(),
                printer_code: Some("PRN-W5-CHAIN".to_string()),
                scale_code: Some("SCL-W5-CHAIN".to_string()),
                temperature_zone: "normal".to_string(),
            },
            now,
            "wave5-chain-station",
            None,
        )
        .await
        .expect("create station")
        .value;
    let pack_job = repo
        .create_pack_job(
            &ctx,
            CreatePackJobRequest {
                outbound_order_id: outbound.id,
                station_id: Some(station.id),
                job_no: format!("PKJOB-CHAIN-{}", owner_id.simple()),
                pack_mode: "station".to_string(),
                recommended_box_type: "M".to_string(),
                actual_box_type: "M".to_string(),
                adjustment_reason: None,
                outbound_lpn: format!("LPN-CHAIN-{}", owner_id.simple()),
                trace_codes: vec!["TC-W5-CHAIN-001".to_string()],
            },
            now,
            "wave5-chain-pack",
            None,
        )
        .await
        .expect("create pack job")
        .value;
    let weighed = repo
        .weigh_pack_job(
            &ctx,
            pack_job.id,
            WeighPackJobRequest {
                actual_weight_grams: 1000,
                theoretical_weight_grams: 1000,
                tolerance_percent: 5,
                override_reason: None,
            },
            now,
            "wave5-chain-weigh",
            None,
        )
        .await
        .expect("weigh pack job")
        .value;
    assert_eq!(weighed.status, "weighed");
    let waybill = repo
        .print_pack_job_waybill(
            &ctx,
            pack_job.id,
            PrintWaybillRequest {
                carrier_code: "SF".to_string(),
                waybill_no: Some("SF-W5-CHAIN".to_string()),
            },
            now,
            "wave5-chain-waybill",
            None,
        )
        .await
        .expect("print waybill")
        .value;
    assert_eq!(waybill.waybill_no.as_deref(), Some("SF-W5-CHAIN"));

    let dispatch = repo
        .receive_tms_dispatch(
            &ctx,
            ReceiveTmsDispatchRequest {
                dispatch_no: format!("DSP-CHAIN-{}", owner_id.simple()),
                outbound_order_id: outbound.id,
                delivery_provider_type: "third_party_express".to_string(),
                vehicle_no: None,
                plate_no: Some("ZJ-A12345".to_string()),
                driver_user_id: None,
                carrier_code: Some("SF".to_string()),
                waybill_no: waybill.waybill_no.clone(),
                version: 1,
                scheduled_load_at: Some(now + chrono::Duration::minutes(30)),
            },
            now,
            "wave5-chain-dispatch",
            None,
        )
        .await
        .expect("receive tms dispatch")
        .value;
    let reading = repo
        .ingest_transit_temperature(
            &ctx,
            IngestTransitTemperatureRequest {
                dispatch_id: dispatch.id,
                device_code: "TEMP-W5-CHAIN".to_string(),
                plate_no: "ZJ-A12345".to_string(),
                measured_at: now,
                temperature_celsius: 4.2,
                humidity_percent: Some(55.0),
                is_exceeded: false,
                external_trace_url: Some("https://tms.example.invalid/traces/W5".to_string()),
            },
            now,
            "wave5-chain-temperature",
            None,
        )
        .await
        .expect("ingest transit temperature")
        .value;
    assert!(!reading.is_exceeded);
    let recovery = repo
        .confirm_container_recovery(
            &ctx,
            ConfirmContainerRecoveryRequest {
                container_lpn: format!("BOX-{}", owner_id.simple()),
                dispatch_id: Some(dispatch.id),
                customer_id: outbound.customer_id,
                delivery_provider_type: "third_party_express".to_string(),
                shipped_at: Some(now),
            },
            now,
            "wave5-chain-recovery",
            None,
        )
        .await
        .expect("confirm container recovery")
        .value;
    assert_eq!(recovery.status, "recovered");

    let charge = repo
        .calculate_period_charges(
            &ctx,
            CalculateBillingChargesRequest {
                contract_id,
                period_start: "2026-06-01".to_string(),
                period_end: "2026-06-30".to_string(),
                charge_item: "packing_operation".to_string(),
                quantity: 1,
                source_refs: vec![
                    format!("retail_suggestion:{}", suggestion.id),
                    format!("packing_job:{}", pack_job.id),
                    format!("tms_dispatch:{}", dispatch.id),
                ],
            },
            now,
            "wave5-chain-charge",
            None,
        )
        .await
        .expect("calculate billing charge")
        .value;
    assert_eq!(charge.amount_cents, 125);
    let statement = repo
        .generate_billing_statement(
            &ctx,
            GenerateBillingStatementRequest {
                contract_id,
                period_start: "2026-06-01".to_string(),
                period_end: "2026-06-30".to_string(),
                charge_ids: vec![charge.id],
            },
            now,
            "wave5-chain-statement",
            None,
        )
        .await
        .expect("generate billing statement")
        .value;
    assert_eq!(statement.total_amount_cents, 125);
    let confirmed = repo
        .confirm_billing_statement(
            &ctx,
            statement.id,
            ConfirmBillingStatementRequest {
                confirmation_note: Some("chain scenario accepted".to_string()),
            },
            now,
            "wave5-chain-confirm-statement",
            None,
        )
        .await
        .expect("confirm billing statement")
        .value;
    assert_eq!(confirmed.status, "confirmed");

    assert_eq!(
        owner_row_count(&pool, "retail_replenishment_suggestions", owner_id).await,
        1
    );
    assert_eq!(owner_row_count(&pool, "crossdock_plans", owner_id).await, 1);
    assert_eq!(owner_row_count(&pool, "packing_jobs", owner_id).await, 1);
    assert_eq!(
        owner_row_count(&pool, "transit_temperature_readings", owner_id).await,
        1
    );
    assert_eq!(
        owner_row_count(&pool, "container_recoveries", owner_id).await,
        1
    );
    assert_eq!(
        owner_row_count(&pool, "billing_statements", owner_id).await,
        1
    );
    assert_eq!(audit_count(&pool, owner_id).await, 12);
}
