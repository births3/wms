use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Extension,
};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    h8_inbound::{h8_inbound_router, H8InboundAppState},
};

async fn seed_context(
    pool: &PgPool,
    owner_id: Uuid,
    api_key_id: Uuid,
    warehouse_id: Uuid,
    supplier_id: Uuid,
    product_code: &str,
) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H8 return REST test owner')",
    )
    .bind(owner_id)
    .bind(format!("H8-RET-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'H8 return REST test warehouse', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("H8-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, $3, 'H8 return REST test supplier', $4, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("H8-SUP-{}", &supplier_id.to_string()[..8]))
    .bind(format!("H8-USCC-{}", &supplier_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed supplier");
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, status) VALUES ($1, $2, $3, 'H8 return REST test product', '1 unit', 'normal', 'active')",
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
            $1, $2, 'H8-RET-REST', 'H8 return REST', ARRAY[$3]::uuid[],
            ARRAY['inbound'], ARRAY['return_order'], 'rest', $4, 'active',
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
        .uri("/api/v1/integration/erp-messages/inbound/return_order")
        .header("content-type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn return_order_rest_maps_persists_batch_and_replays_one_resource(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let supplier_id = Uuid::new_v4();
    let product_code = format!("H8-RET-P-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(
        &pool,
        owner_id,
        api_key_id,
        warehouse_id,
        supplier_id,
        &product_code,
    )
    .await;
    let state = H8InboundAppState::with_postgres(pool.clone());
    let app = h8_inbound_router(state.clone()).layer(Extension(AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "H8 return API Key".to_string(),
        permissions: vec!["m2.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: Some(warehouse_id),
    }));
    let external_ref = format!("ERP-RET-{}", &Uuid::new_v4().to_string()[..8]);
    let idempotency_key = format!("h8-return-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "warehouse_id": warehouse_id,
        "receipt_no": null,
        "document_type": "销售退货入库",
        "customer_id": supplier_id,
        "supplier_id": null,
        "product_code": product_code,
        "expected_qty": 2,
        "expected_arrival_at": Utc::now() + Duration::days(1),
        "batch_no": "ERP-ORIGINAL-BATCH-001"
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
    .expect("first body should be JSON");
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
    .expect("replay body should be JSON");
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["message_id"], first["message_id"]);
    assert_eq!(replay["wms_resource_id"], first["wms_resource_id"]);

    let mut changed = body.clone();
    changed["expected_qty"] = Value::from(3);
    let conflict = app
        .oneshot(request(&changed, &idempotency_key))
        .await
        .expect("changed replay should respond");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let denied = h8_inbound_router(state)
        .layer(Extension(AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "unbound return API Key".to_string(),
            permissions: vec!["m2.write".to_string()],
            jti: "unbound-return-api-key".to_string(),
            warehouse_scope: Some(warehouse_id),
        }))
        .oneshot(request(&body, &format!("denied-{idempotency_key}")))
        .await
        .expect("unbound request should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let evidence: (i64, String, String, String) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1 AND external_ref = $2),
            (SELECT document_type FROM receiving_orders WHERE owner_id = $1 AND external_ref = $2),
            (SELECT batch_no FROM receiving_order_lines l
              JOIN receiving_orders o ON o.id = l.receiving_order_id
             WHERE o.owner_id = $1 AND o.external_ref = $2),
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'return_order' AND external_ref = $2)
        "#,
    )
    .bind(owner_id)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load return evidence");
    assert_eq!(
        evidence,
        (
            1,
            "sales_return".to_string(),
            "ERP-ORIGINAL-BATCH-001".to_string(),
            "succeeded".to_string(),
        )
    );
    let lifecycle_actions: Vec<String> = sqlx::query_scalar(
        r#"SELECT action FROM audit_event
            WHERE owner_id=$1 AND resource_type='h8_erp_message' AND resource_id=$2
            ORDER BY id"#,
    )
    .bind(owner_id)
    .bind(first["message_id"].as_str().expect("message id"))
    .fetch_all(&pool)
    .await
    .expect("load H8 return lifecycle audits");
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
async fn return_order_rest_rejects_unmapped_type_before_business_write(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let supplier_id = Uuid::new_v4();
    let product_code = format!("H8-RET-P-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(
        &pool,
        owner_id,
        api_key_id,
        warehouse_id,
        supplier_id,
        &product_code,
    )
    .await;
    let external_ref = format!("ERP-RET-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "warehouse_id": warehouse_id,
        "receipt_no": null,
        "document_type": format!("未知销退类型-{}", Uuid::new_v4()),
        "customer_id": supplier_id,
        "supplier_id": null,
        "product_code": product_code,
        "expected_qty": 1,
        "expected_arrival_at": Utc::now() + Duration::days(1),
        "batch_no": "ERP-ORIGINAL-BATCH-002"
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(AuthContext {
            user_id: api_key_id,
            owner_id,
            actor_name: "H8 return API Key".to_string(),
            permissions: vec!["m2.write".to_string()],
            jti: format!("api-key:{api_key_id}"),
            warehouse_scope: Some(warehouse_id),
        }))
        .oneshot(request(&body, &format!("h8-return-{}", Uuid::new_v4())))
        .await
        .expect("unmapped request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (i64, String) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1),
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'return_order' AND external_ref = $2)
        "#,
    )
    .bind(owner_id)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load rejection evidence");
    assert_eq!(evidence, (0, "dead".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn return_order_rest_rejects_missing_original_batch_before_message_write(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let body = json!({
        "schema_version": "1",
        "external_ref": format!("ERP-RET-{}", Uuid::new_v4()),
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "warehouse_id": warehouse_id,
        "receipt_no": null,
        "document_type": "销售退货入库",
        "customer_id": Uuid::new_v4(),
        "supplier_id": null,
        "product_code": "H8-RET-P-MISSING-BATCH",
        "expected_qty": 1,
        "expected_arrival_at": Utc::now() + Duration::days(1),
        "batch_no": ""
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "H8 return API Key".to_string(),
            permissions: vec!["m2.write".to_string()],
            jti: "missing-batch-api-key".to_string(),
            warehouse_scope: Some(warehouse_id),
        }))
        .oneshot(request(&body, &format!("h8-return-{}", Uuid::new_v4())))
        .await
        .expect("missing batch request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM receiving_orders WHERE owner_id = $1), (SELECT COUNT(*) FROM h8_erp_messages WHERE owner_id = $1)",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("load missing batch evidence");
    assert_eq!(evidence, (0, 0));
}
