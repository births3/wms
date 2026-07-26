use axum::{
    body::{to_bytes, Body},
    extract::{Path, Query, State},
    http::{HeaderMap, Request, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::{
    ChangeInventoryStatusRequest, CreateReceivingOrderRequest, ExpireInventoryBatchesRequest,
    IngestTemperatureExcursionRequest, IngestTemperatureReadingRequest,
    InspectReceivingOrderRequest, InventoryBatchQuery, PutawayInventoryRequest, PutawayRequest,
    ReceiveReceivingOrderRequest, ReceivingOrderLine, ReceivingReceiptDetails,
    SignInspectionRequest, UpdateReceivingOrderRequest,
};

use super::{
    change_inventory_batch_status_handler, ingest_temperature_excursion_handler,
    ingest_temperature_reading_handler, inspect_receiving_order_handler,
    isolate_expired_inventory_batches_handler, list_inventory_batches_handler,
    putaway_inventory_batch_handler, putaway_receiving_order_handler,
    receive_receiving_order_handler, sha256_hex, sign_receiving_order_handler,
    update_receiving_order_handler, wave3_router, ExternalApiKeyConfig, Wave3AppState,
    Wave3HandlerError, EXTERNAL_API_KEY_HEADER, IDEMPOTENCY_KEY_HEADER,
    INVENTORY_BATCHES_SMOKE_FLAG,
};
use crate::{
    auth::{AuthContext, AuthError},
    config_center::{
        ConfigCenterAppState, ConfigCenterError, FeatureFlagSource, CONFIG_FLAG_MISSING_CODE,
    },
    feature_flags::FeatureFlagRegistry,
    inventory::{InventoryError, STATUS_QUALIFIED, STATUS_QUARANTINED, STATUS_UNQUALIFIED},
};

fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "tester".to_string(),
        permissions: permissions
            .iter()
            .map(|permission| permission.to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}
fn receiving_line() -> ReceivingOrderLine {
    ReceivingOrderLine {
        line_no: 1,
        product_id: None,
        product_code: "P-001".to_string(),
        expected_qty: 10,
        batch_no: None,
        production_date: None,
        expiry_date: None,
    }
}

async fn seed_active_supplier_and_product(
    pool: &PgPool,
    owner_id: Uuid,
    supplier_id: Uuid,
    product_code: &str,
) {
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, $3, 'Active Supplier', $4, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("SUP-HANDLER-{}", &supplier_id.to_string()[..8]))
    .bind(format!("USCC-HANDLER-{}", &supplier_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed supplier");
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, attrs, status) VALUES ($1, $2, $3, 'Active Product', '1 unit', 'normal', '{\"unit_volume_cm3\": 1}', 'active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_code)
    .execute(pool)
    .await
    .expect("seed product");
}

fn inventory_putaway_req() -> PutawayInventoryRequest {
    PutawayInventoryRequest {
        product_code: "P-001".to_string(),
        batch_no: "B202606".to_string(),
        production_date: "2026-01-01".to_string(),
        expiry_date: "2028-01-01".to_string(),
        qty: 10,
        quality_status: STATUS_QUALIFIED.to_string(),
        location_id: Uuid::new_v4(),
        location_code: "A-01-01".to_string(),
        source_receiving_order_id: Uuid::new_v4(),
    }
}

fn config_center_smoke_registry() -> FeatureFlagRegistry {
    FeatureFlagRegistry::from_toml_str(&format!(
        r#"
            [[flags]]
            key = "{INVENTORY_BATCHES_SMOKE_FLAG}"
            owner = "platform"
            created_at = 2026-06-07
            cleanup_by = 2026-09-05
            enabled = true
            "#
    ))
    .expect("valid smoke flag registry")
}

fn idempotency_headers(key: &'static str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(IDEMPOTENCY_KEY_HEADER, key.parse().expect("valid header"));
    headers
}

fn external_auth_headers(idempotency_key: &'static str, api_key: &'static str) -> HeaderMap {
    let mut headers = idempotency_headers(idempotency_key);
    headers.insert(
        EXTERNAL_API_KEY_HEADER,
        api_key.parse().expect("valid header"),
    );
    headers
}

fn external_api_key_config(owner_id: Uuid, api_key: &str) -> ExternalApiKeyConfig {
    ExternalApiKeyConfig {
        key_sha256: sha256_hex(api_key.as_bytes()),
        owner_id,
        actor_name: "external-cold-chain-test".to_string(),
    }
}

async fn error_response(error: Wave3HandlerError) -> (StatusCode, wms_domain::ErrorResponse) {
    let response = error.into_response();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let payload = serde_json::from_slice(&body).expect("error response should be json");
    (status, payload)
}

async fn seed_cold_chain_device(pool: &PgPool, owner_id: Uuid, device_code: &str) {
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 14, 0, 0)
        .single()
        .expect("valid time");
    sqlx::query(
        r#"
	            INSERT INTO cold_chain_devices (
	                id, owner_id, device_code, device_type,
	                installed_at_location_code, calibration_due_at, status, created_at, updated_at
	            )
	            VALUES ($1, $2, $3, 'thermometer', 'CC-01', NULL, 'active', $4, $4)
	            "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(device_code)
    .bind(now)
    .execute(pool)
    .await
    .expect("seed cold-chain device");
}

#[test]
fn wave3_router_registers_first_batch_handlers() {
    let _router = wave3_router(Wave3AppState::default());
}

#[tokio::test]
async fn inventory_trace_route_is_registered_with_runtime_path_syntax() {
    let response = wave3_router(Wave3AppState::default())
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/inventory/batches/{}/trace",
                    Uuid::new_v4()
                ))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should respond");

    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_receiving_order_requires_permission_and_appends_audit() {
    let owner_id = Uuid::new_v4();
    let state = Wave3AppState::default();
    let authorized = ctx(owner_id, &["m2.write"]);
    let created = {
        let mut store = state.inbound_store.lock().await;
        store
            .create(
                &authorized,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-UPDATE-001".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: Some(Uuid::new_v4()),
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: Some(Utc::now() + chrono::Duration::days(1)),
                    lines: vec![receiving_line()],
                },
                Utc::now(),
            )
            .expect("receiving order should be created")
    };

    let denied = update_receiving_order_handler(
        ctx(owner_id, &[]),
        State(state.clone()),
        Path(created.id),
        idempotency_headers("update-denied"),
        Json(UpdateReceivingOrderRequest {
            supplier_id: None,
            warehouse_id: None,
            external_ref: Some(Some("ERP-UPDATED".to_string())),
            expected_arrival_at: None,
            lines: None,
        }),
    )
    .await;
    assert!(matches!(
        denied,
        Err(Wave3HandlerError::Auth(AuthError::PermissionDenied(permission)))
            if permission == "m2.write"
    ));

    let missing_key = update_receiving_order_handler(
        authorized.clone(),
        State(state.clone()),
        Path(created.id),
        HeaderMap::new(),
        Json(UpdateReceivingOrderRequest {
            supplier_id: None,
            warehouse_id: None,
            external_ref: Some(Some("ERP-UPDATED".to_string())),
            expected_arrival_at: None,
            lines: None,
        }),
    )
    .await;
    assert!(matches!(
        missing_key,
        Err(Wave3HandlerError::MissingIdempotencyKey)
    ));

    let updated = update_receiving_order_handler(
        authorized,
        State(state.clone()),
        Path(created.id),
        idempotency_headers("update-authorized"),
        Json(UpdateReceivingOrderRequest {
            supplier_id: None,
            warehouse_id: None,
            external_ref: Some(Some("ERP-UPDATED".to_string())),
            expected_arrival_at: None,
            lines: None,
        }),
    )
    .await
    .expect("authorized update should succeed")
    .0;

    assert_eq!(updated.external_ref.as_deref(), Some("ERP-UPDATED"));
    let audit = state.audit_log.lock().await;
    assert_eq!(audit.events().len(), 1);
    assert_eq!(audit.events()[0].action, "update");
    let diff = audit.events()[0]
        .diff
        .as_ref()
        .expect("update audit should record before and after values");
    assert!(diff.changed_keys.contains(&"external_ref".to_string()));
    assert_eq!(diff.before["external_ref"], serde_json::Value::Null);
    assert_eq!(diff.after["external_ref"], "ERP-UPDATED");
    drop(audit);
}

#[tokio::test]
async fn inbound_receive_handler_requires_permission_and_appends_audit() {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m2.write"]);
    let denied_ctx = ctx(owner_id, &[]);
    let state = Wave3AppState::default();
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
        .single()
        .expect("valid time");
    let order = {
        let mut store = state.inbound_store.lock().await;
        let created = store
            .create(
                &authorized,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-HANDLER-001".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: Some(Uuid::new_v4()),
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: Some(now + chrono::Duration::days(1)),
                    lines: vec![receiving_line()],
                },
                now,
            )
            .expect("create order");
        store
            .release(&authorized, created.id, now)
            .expect("release order")
    };

    let req = ReceiveReceivingOrderRequest {
        actual_qty: 8,
        shortage_qty: 2,
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
    };
    let denied = receive_receiving_order_handler(
        denied_ctx,
        State(state.clone()),
        Path(order.id),
        HeaderMap::new(),
        Json(req.clone()),
    )
    .await
    .expect_err("permission should be required");
    assert!(matches!(
        denied,
        Wave3HandlerError::Auth(AuthError::PermissionDenied(permission))
            if permission == "m2.write"
    ));
    assert!(state.audit_log.lock().await.events().is_empty());

    let missing_idempotency_key = receive_receiving_order_handler(
        authorized.clone(),
        State(state.clone()),
        Path(order.id),
        HeaderMap::new(),
        Json(req.clone()),
    )
    .await
    .expect_err("idempotency key should be required for fallback writes");
    assert!(matches!(
        missing_idempotency_key,
        Wave3HandlerError::MissingIdempotencyKey
    ));
    assert!(state.audit_log.lock().await.events().is_empty());

    let Json(receipt) = receive_receiving_order_handler(
        authorized.clone(),
        State(state.clone()),
        Path(order.id),
        idempotency_headers("fallback-receive-1"),
        Json(req),
    )
    .await
    .expect("authorized receive should succeed");

    assert_eq!(receipt.actual_qty, 8);
    let audit_log = state.audit_log.lock().await;
    assert_eq!(audit_log.events().len(), 1);
    assert_eq!(audit_log.events()[0].action, "receive");
    assert_eq!(audit_log.events()[0].module, "M2");
    assert_eq!(audit_log.events()[0].resource_id, order.id.to_string());
    audit_log
        .verify_hash_chain()
        .expect("audit hash chain should verify");
}

#[sqlx::test(migrations = "../../migrations")]
async fn postgres_receive_handler_writes_business_idempotency_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m2.write"]);
    let state = Wave3AppState::with_postgres(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
        .single()
        .expect("valid time");
    let repository = state
        .wave3_repository
        .as_ref()
        .expect("postgres repository");
    let (supplier_id, warehouse_id) = (Uuid::new_v4(), Uuid::new_v4());
    seed_active_supplier_and_product(&pool, owner_id, supplier_id, "P-001").await;
    sqlx::query("INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1,$2,'WH-H1','W','normal','active')").bind(warehouse_id).bind(owner_id).execute(&pool).await.expect("seed warehouse");
    let order = repository
        .create_receiving_order(
            &authorized,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-HANDLER-PG-001".to_string(),
                document_type: "purchase_inbound".to_string(),
                supplier_id: Some(supplier_id),
                warehouse_id,
                external_ref: None,
                expected_arrival_at: Some(now + chrono::Duration::days(1)),
                lines: vec![receiving_line()],
            },
            now,
        )
        .await
        .expect("create order");
    repository
        .release_receiving_order(&authorized, order.id, now)
        .await
        .expect("release order");

    let req = ReceiveReceivingOrderRequest {
        actual_qty: 8,
        shortage_qty: 2,
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
    };
    let Json(receipt) = receive_receiving_order_handler(
        authorized.clone(),
        State(state.clone()),
        Path(order.id),
        idempotency_headers("handler-receive-1"),
        Json(req.clone()),
    )
    .await
    .expect("postgres receive should succeed");
    let Json(replay) = receive_receiving_order_handler(
        authorized,
        State(state.clone()),
        Path(order.id),
        idempotency_headers("handler-receive-1"),
        Json(req),
    )
    .await
    .expect("same idempotency key should replay");

    assert_eq!(receipt.id, replay.id);
    let counts: (i64, i64, i64) = sqlx::query_as(
        r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_order_receipts WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'receive')
            "#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn postgres_putaway_handler_commits_inventory_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let authorized = ctx(owner_id, &["m2.write", "m2.putaway.write"]);
    let state = Wave3AppState::with_postgres(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 4, 11, 0, 0)
        .single()
        .expect("valid time");
    let repository = state
        .wave3_repository
        .as_ref()
        .expect("postgres repository");
    let supplier_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let location_code = format!("M2-HANDLER-LOC-{}", &location_id.to_string()[..8]);
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '上架处理器测试货主') ON CONFLICT (id) DO NOTHING")
        .bind(owner_id)
        .bind(format!("M2-PUTAWAY-{}", &owner_id.to_string()[..8]))
        .execute(&pool)
        .await
        .expect("seed putaway owner");
    sqlx::query("INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1,$2,'WH-HANDLER-PUTAWAY','W','normal','active')")
            .bind(warehouse_id).bind(owner_id).execute(&pool).await.expect("seed warehouse");
    sqlx::query("INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1,$2,$3,'M2-HANDLER-ZONE','Z','normal','qualified_green','active')")
            .bind(zone_id).bind(owner_id).bind(warehouse_id).execute(&pool).await.expect("seed zone");
    sqlx::query("INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1,$2,$3,$4,$5,1,1,1,100000,0,3,'storage','available')")
            .bind(location_id).bind(owner_id).bind(warehouse_id).bind(zone_id).bind(&location_code)
            .execute(&pool).await.expect("seed location");
    seed_active_supplier_and_product(&pool, owner_id, supplier_id, "P-001").await;
    let order = repository
        .create_receiving_order(
            &authorized,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-HANDLER-PG-002".to_string(),
                document_type: "purchase_inbound".to_string(),
                supplier_id: Some(supplier_id),
                warehouse_id,
                external_ref: None,
                expected_arrival_at: Some(now + chrono::Duration::days(1)),
                lines: vec![receiving_line()],
            },
            now,
        )
        .await
        .expect("create order");
    sqlx::query("UPDATE receiving_orders SET status = 'released' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare released state");
    repository
        .receive_receiving_order_with_audit(
            &authorized,
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
            now,
            "handler-putaway-receive",
            None,
        )
        .await
        .expect("receive before inspection");
    repository
        .inspect_receiving_order_with_audit(
            &authorized,
            order.id,
            InspectReceivingOrderRequest {
                batch_no: "B202606".to_string(),
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
            now.date_naive(),
            now,
            "handler-putaway-inspect",
            None,
        )
        .await
        .expect("inspect before putaway");
    sqlx::query("UPDATE receiving_orders SET status = 'putaway' WHERE id = $1")
        .bind(order.id)
        .execute(&pool)
        .await
        .expect("prepare putaway state");

    let Json(putaway) = putaway_receiving_order_handler(
        authorized,
        State(state),
        Path(order.id),
        idempotency_headers("handler-putaway-1"),
        Json(PutawayRequest {
            batch_no: "B202606".to_string(),
            product_code: "P-001".to_string(),
            qty: 10,
            location_id,
            location_code,
            quality_status: crate::inventory::STATUS_QUALIFIED.to_string(),
                    lpn_code: None,
        }),
    )
    .await
    .expect("postgres putaway should succeed");

    assert_eq!(putaway.receiving_order_id, order.id);
    let counts: (i64, i64, i64, String) = sqlx::query_as(
        r#"
            SELECT
                (SELECT COUNT(*) FROM receiving_putaways WHERE receiving_order_id = $1),
                (SELECT COUNT(*) FROM inventory_batches WHERE owner_id = $2),
                (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND action = 'putaway'),
                (SELECT status FROM receiving_orders WHERE id = $1)
            "#,
    )
    .bind(order.id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, 1, 1, "completed".to_string()));
}
