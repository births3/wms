use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension,
};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    h8_inbound::{h8_inbound_router, H8InboundAppState},
};

async fn seed_asn_context(
    pool: &PgPool,
    owner_id: Uuid,
    api_key_id: Uuid,
    warehouse_id: Uuid,
    supplier_id: Uuid,
    product_code: &str,
) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H8 ASN test owner')",
    )
    .bind(owner_id)
    .bind(format!("H8-ASN-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'H8 ASN test warehouse', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("H8-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, $3, 'H8 ASN test supplier', $4, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("H8-SUP-{}", &supplier_id.to_string()[..8]))
    .bind(format!("H8-USCC-{}", &supplier_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed supplier");
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, status) VALUES ($1, $2, $3, 'H8 ASN test product', '1 unit', 'normal_10_30', 'active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_code)
    .execute(pool)
    .await
    .expect("seed product");
    sqlx::query(
        r#"
        INSERT INTO h8_erp_connectors (
            id, owner_id, connector_code, connector_name, warehouse_ids,
            directions, message_types, channel_mode, api_key_id, status,
            config_version, first_activated_at, last_tested_version,
            last_tested_at, last_tested_succeeded
        )
        VALUES (
            $1, $2, 'H8-ASN-REST', 'H8 ASN REST', ARRAY[$3]::uuid[],
            ARRAY['inbound'], ARRAY['asn'], 'rest', $4, 'active',
            1, now(), 1, now(), TRUE
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(api_key_id)
    .execute(pool)
    .await
    .expect("seed connector");
}

fn request(body: &Value, idempotency_key: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/integration/erp-messages/inbound/asn")
        .header("content-type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn asn_rest_maps_persists_and_replays_one_business_resource(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let supplier_id = Uuid::new_v4();
    let product_code = format!("H8-P-{}", &Uuid::new_v4().to_string()[..8]);
    seed_asn_context(
        &pool,
        owner_id,
        api_key_id,
        warehouse_id,
        supplier_id,
        &product_code,
    )
    .await;
    let ctx = AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "H8 API Key".to_string(),
        permissions: vec!["m2.write".to_string(), "h8.erp_connector.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: Some(warehouse_id),
    };
    let state = H8InboundAppState::with_postgres(pool.clone());
    let app = h8_inbound_router(state.clone()).layer(Extension(ctx));
    let external_ref = format!("ERP-ASN-{}", &Uuid::new_v4().to_string()[..8]);
    let idempotency_key = format!("h8-asn-{}", Uuid::new_v4());
    let correlation_id = format!("corr-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": correlation_id,
        "occurred_at": Utc::now(),
        "payload_digest": "a".repeat(64),
        "source_version": null,
        "erp_bill_id": 9001,
        "erp_bill_code": external_ref,
        "revision": 1,
        "order_type": 1,
        "partner_type": "supplier",
        "partner_code": format!("H8-SUP-{}", &supplier_id.to_string()[..8]),
        "depot_code": format!("H8-WH-{}", &warehouse_id.to_string()[..8]),
        "business_date": Utc::now().date_naive(),
        "note_code": null,
        "lines": [{
            "line_no": 1,
            "product_code": product_code,
            "expected_qty": "2.0000",
            "batch_no": null,
            "production_date": null,
            "expiry_date": null
        }]
    });

    let first = app
        .clone()
        .oneshot(request(&body, &idempotency_key))
        .await
        .expect("first request should respond");
    assert_eq!(first.status(), StatusCode::OK);
    let first: Value = serde_json::from_slice(
        &to_bytes(first.into_body(), usize::MAX)
            .await
            .expect("first body should read"),
    )
    .expect("first body should be json");
    assert_eq!(first["status"], "succeeded");
    assert_eq!(first["replayed"], false);

    let replay = app
        .clone()
        .oneshot(request(&body, &idempotency_key))
        .await
        .expect("replay request should respond");
    assert_eq!(replay.status(), StatusCode::OK);
    let replay: Value = serde_json::from_slice(
        &to_bytes(replay.into_body(), usize::MAX)
            .await
            .expect("replay body should read"),
    )
    .expect("replay body should be json");
    assert_eq!(replay["status"], "succeeded");
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["message_id"], first["message_id"]);
    assert_eq!(replay["wms_resource_id"], first["wms_resource_id"]);

    let mut changed = body.clone();
    changed["payload_digest"] = Value::from("b".repeat(64));
    changed["lines"][0]["expected_qty"] = Value::from("3.0000");
    let conflict = app
        .oneshot(request(&changed, &idempotency_key))
        .await
        .expect("changed replay should respond");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let denied = h8_inbound_router(state)
        .layer(Extension(AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "another H8 API Key".to_string(),
            permissions: vec!["m2.write".to_string(), "h8.erp_connector.write".to_string()],
            jti: "another-api-key".to_string(),
            warehouse_scope: Some(warehouse_id),
        }))
        .oneshot(request(&body, &idempotency_key))
        .await
        .expect("unbound key request should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let receiving_orders: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1 AND erp_bill_code = $2 AND erp_revision = 1",
    )
    .bind(owner_id)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("count receiving orders");
    assert_eq!(receiving_orders, 1);
    let document_type: String = sqlx::query_scalar(
        "SELECT document_type FROM receiving_orders WHERE owner_id = $1 AND erp_bill_code = $2 AND erp_revision = 1",
    )
    .bind(owner_id)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load mapped document type");
    assert_eq!(document_type, "purchase_inbound");
    let stored_correlation: String = sqlx::query_scalar(
        "SELECT erp_correlation_id FROM receiving_orders WHERE owner_id = $1 AND erp_bill_code = $2",
    )
    .bind(owner_id)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load ERP correlation id");
    assert_eq!(stored_correlation, correlation_id);
    let message_status: String = sqlx::query_scalar(
        "SELECT sync_status FROM h8_erp_messages WHERE owner_id = $1 AND message_type = 'asn' AND external_ref = $2",
    )
    .bind(owner_id)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load message status");
    assert_eq!(message_status, "succeeded");
    let lifecycle_actions: Vec<String> = sqlx::query_scalar(
        r#"SELECT action FROM audit_event
            WHERE owner_id=$1 AND resource_type='h8_erp_message'
              AND resource_id=$2
            ORDER BY id"#,
    )
    .bind(owner_id)
    .bind(first["message_id"].as_str().expect("message id"))
    .fetch_all(&pool)
    .await
    .expect("load H8 lifecycle audit chain");
    assert_eq!(
        lifecycle_actions,
        vec![
            "h8_exchange_receive",
            "h8_exchange_convert",
            "h8_exchange_business_api",
            "h8_exchange_receipt",
        ]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn asn_rest_rejects_pending_mapping_product_without_business_writes(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let supplier_id = Uuid::new_v4();
    let product_code = format!("H8-P-PENDING-{}", &Uuid::new_v4().to_string()[..8]);
    seed_asn_context(
        &pool,
        owner_id,
        api_key_id,
        warehouse_id,
        supplier_id,
        &product_code,
    )
    .await;
    sqlx::query(
        "UPDATE products SET status = 'pending_mapping' WHERE owner_id = $1 AND product_code = $2",
    )
    .bind(owner_id)
    .bind(&product_code)
    .execute(&pool)
    .await
    .expect("mark product pending mapping");

    let external_ref = format!("ERP-ASN-PENDING-{}", &Uuid::new_v4().to_string()[..8]);
    let idempotency_key = format!("h8-asn-pending-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "payload_digest": "c".repeat(64),
        "source_version": null,
        "erp_bill_id": 9002,
        "erp_bill_code": external_ref,
        "revision": 1,
        "order_type": 1,
        "partner_type": "supplier",
        "partner_code": format!("H8-SUP-{}", &supplier_id.to_string()[..8]),
        "depot_code": format!("H8-WH-{}", &warehouse_id.to_string()[..8]),
        "business_date": Utc::now().date_naive(),
        "note_code": null,
        "lines": [{
            "line_no": 1,
            "product_code": product_code,
            "expected_qty": "2.0000",
            "batch_no": null,
            "production_date": null,
            "expiry_date": null
        }]
    });
    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(AuthContext {
            user_id: api_key_id,
            owner_id,
            actor_name: "H8 API Key".to_string(),
            permissions: vec!["m2.write".to_string(), "h8.erp_connector.write".to_string()],
            jti: format!("api-key:{api_key_id}"),
            warehouse_scope: Some(warehouse_id),
        }))
        .oneshot(request(&body, &idempotency_key))
        .await
        .expect("pending product ASN should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (i64, i64, String) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id = $1 AND idempotency_key = $2),
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'asn' AND external_ref = $3)
        "#,
    )
    .bind(owner_id)
    .bind(&idempotency_key)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("pending product rejection evidence should query");
    assert_eq!(evidence, (0, 0, "dead".to_string()));
}
