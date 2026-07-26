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

#[path = "support/h9.rs"]
mod h9_support;
use h9_support::seed_outbound_route_binding;

async fn seed_context(pool: &PgPool, owner_id: Uuid, api_key_id: Uuid, warehouse_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H8 outbound REST test owner')",
    )
    .bind(owner_id)
    .bind(format!("H8-OUT-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'H8 outbound REST test warehouse', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("H8-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed warehouse");
    sqlx::query(
        r#"
        INSERT INTO h8_erp_connectors (
            id, owner_id, connector_code, connector_name, warehouse_ids,
            directions, message_types, channel_mode, api_key_id, status,
            config_version, first_activated_at, last_tested_version,
            last_tested_at, last_tested_succeeded
        )
        VALUES (
            $1, $2, 'H8-OUT-REST', 'H8 outbound REST', ARRAY[$3]::uuid[],
            ARRAY['inbound'], ARRAY['outbound_order'], 'rest', $4, 'active',
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
        .uri("/api/v1/integration/erp-messages/inbound/outbound_order")
        .header("content-type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_order_rest_maps_persists_and_replays_one_business_resource(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id, warehouse_id).await;
    let ctx = AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "H8 outbound API Key".to_string(),
        permissions: vec!["m4.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: Some(warehouse_id),
    };
    let state = H8InboundAppState::with_postgres(pool.clone());
    let app = h8_inbound_router(state.clone()).layer(Extension(ctx));
    let external_ref = format!("ERP-OUT-{}", &Uuid::new_v4().to_string()[..8]);
    let idempotency_key = format!("h8-out-{}", Uuid::new_v4());
    let customer_id = Uuid::new_v4();
    let delivery_address_id =
        seed_outbound_route_binding(&pool, owner_id, warehouse_id, customer_id, Utc::now()).await;
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "warehouse_id": warehouse_id,
        "wms_order_no": format!("H8-O-{}", &Uuid::new_v4().to_string()[..8]),
        "document_type": "销售出库",
        "erp_order_no": format!("ERP-O-{}", &Uuid::new_v4().to_string()[..8]),
        "customer_id": customer_id,
        "delivery_address_id": delivery_address_id,
        "product_code": "H8-OUT-P-001",
        "batch_no": "H8-OUT-B-001",
        "planned_qty": 2,
        "required_ship_at": Utc::now() + Duration::days(1)
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
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["message_id"], first["message_id"]);
    assert_eq!(replay["wms_resource_id"], first["wms_resource_id"]);

    let message_id = first["message_id"]
        .as_str()
        .expect("message id")
        .parse::<Uuid>()
        .expect("message id should be UUID");
    sqlx::query(
        "UPDATE h8_erp_messages SET sync_status = 'processing', wms_resource_id = NULL, completed_at = NULL WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(message_id)
    .execute(&pool)
    .await
    .expect("simulate business commit before H8 final status");
    let recovered = app
        .clone()
        .oneshot(request(&body, &idempotency_key))
        .await
        .expect("recovery request should respond");
    assert_eq!(recovered.status(), StatusCode::OK);
    let recovered: Value = serde_json::from_slice(
        &to_bytes(recovered.into_body(), usize::MAX)
            .await
            .expect("recovery body should read"),
    )
    .expect("recovery body should be json");
    assert_eq!(recovered["message_id"], first["message_id"]);
    assert_eq!(recovered["wms_resource_id"], first["wms_resource_id"]);
    assert_eq!(recovered["status"], "succeeded");
    assert_eq!(recovered["replayed"], true);

    let mut changed = body.clone();
    changed["planned_qty"] = Value::from(3);
    let conflict = app
        .oneshot(request(&changed, &idempotency_key))
        .await
        .expect("changed replay should respond");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let denied = h8_inbound_router(state)
        .layer(Extension(AuthContext {
            user_id: api_key_id,
            owner_id,
            actor_name: "H8 ASN-only API Key".to_string(),
            permissions: vec!["m2.write".to_string()],
            jti: "asn-only-api-key".to_string(),
            warehouse_scope: Some(warehouse_id),
        }))
        .oneshot(request(&body, &format!("denied-{idempotency_key}")))
        .await
        .expect("wrong scope request should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let evidence: (i64, i64, String, String) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM outbound_orders WHERE owner_id = $1 AND erp_order_no = $2),
            (SELECT COUNT(*) FROM outbound_order_lines l
               JOIN outbound_orders o ON o.id = l.outbound_order_id AND o.owner_id = l.owner_id
              WHERE o.owner_id = $1 AND o.erp_order_no = $2),
            (SELECT document_type FROM outbound_orders WHERE owner_id = $1 AND erp_order_no = $2),
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'outbound_order' AND external_ref = $3)
        "#,
    )
    .bind(owner_id)
    .bind(body["erp_order_no"].as_str().expect("ERP order number"))
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load outbound evidence");
    assert_eq!(
        evidence,
        (1, 1, "sales_outbound".to_string(), "succeeded".to_string())
    );
    let lifecycle_actions: Vec<String> = sqlx::query_scalar(
        r#"SELECT action FROM audit_event
            WHERE owner_id=$1 AND resource_type='h8_erp_message' AND resource_id=$2
            ORDER BY id"#,
    )
    .bind(owner_id)
    .bind(message_id.to_string())
    .fetch_all(&pool)
    .await
    .expect("load H8 outbound lifecycle audits");
    assert_eq!(
        lifecycle_actions,
        vec![
            "h8_exchange_receive",
            "h8_exchange_convert",
            "h8_exchange_business_api",
            "h8_exchange_receipt",
            "h8_exchange_receive",
            "h8_exchange_convert",
            "h8_exchange_business_api",
            "h8_exchange_receipt",
        ]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_order_rest_rejects_unmapped_document_type_before_business_write(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id, warehouse_id).await;
    let ctx = AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "H8 outbound API Key".to_string(),
        permissions: vec!["m4.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: Some(warehouse_id),
    };
    let external_ref = format!("ERP-OUT-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "warehouse_id": warehouse_id,
        "wms_order_no": null,
        "document_type": format!("未知出库类型-{}", Uuid::new_v4()),
        "erp_order_no": null,
        "customer_id": Uuid::new_v4(),
        "delivery_address_id": Uuid::new_v4(),
        "product_code": "H8-OUT-P-INVALID",
        "batch_no": "H8-OUT-B-INVALID",
        "planned_qty": 1,
        "required_ship_at": null
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(ctx))
        .oneshot(request(&body, &format!("h8-out-{}", Uuid::new_v4())))
        .await
        .expect("unmapped request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (i64, String, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM outbound_orders WHERE owner_id = $1),
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'outbound_order' AND external_ref = $2),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND module = 'H8' AND action = 'h8_message_dead')
        "#,
    )
    .bind(owner_id)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load rejection evidence");
    assert_eq!(evidence, (0, "dead".to_string(), 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_same_outbound_order_returns_one_message_and_business_resource(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id, warehouse_id).await;
    let app = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone())).layer(Extension(
        AuthContext {
            user_id: api_key_id,
            owner_id,
            actor_name: "H8 concurrent API Key".to_string(),
            permissions: vec!["m4.write".to_string()],
            jti: format!("api-key:{api_key_id}"),
            warehouse_scope: Some(warehouse_id),
        },
    ));
    let external_ref = format!("ERP-OUT-{}", Uuid::new_v4());
    let idempotency_key = format!("h8-out-{}", Uuid::new_v4());
    let customer_id = Uuid::new_v4();
    let delivery_address_id =
        seed_outbound_route_binding(&pool, owner_id, warehouse_id, customer_id, Utc::now()).await;
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "warehouse_id": warehouse_id,
        "wms_order_no": null,
        "document_type": "销售出库",
        "erp_order_no": null,
        "customer_id": customer_id,
        "delivery_address_id": delivery_address_id,
        "product_code": "H8-OUT-P-CONCURRENT",
        "batch_no": "H8-OUT-B-CONCURRENT",
        "planned_qty": 1,
        "required_ship_at": null
    });

    let (first, second) = tokio::join!(
        app.clone().oneshot(request(&body, &idempotency_key)),
        app.oneshot(request(&body, &idempotency_key))
    );
    let first = first.expect("first concurrent request");
    let second = second.expect("second concurrent request");
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    let first: Value = serde_json::from_slice(
        &to_bytes(first.into_body(), usize::MAX)
            .await
            .expect("first concurrent body"),
    )
    .expect("first concurrent JSON");
    let second: Value = serde_json::from_slice(
        &to_bytes(second.into_body(), usize::MAX)
            .await
            .expect("second concurrent body"),
    )
    .expect("second concurrent JSON");
    assert_eq!(first["message_id"], second["message_id"]);
    assert_eq!(first["wms_resource_id"], second["wms_resource_id"]);

    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM h8_erp_messages WHERE owner_id = $1 AND external_ref = $2), (SELECT COUNT(*) FROM outbound_orders WHERE owner_id = $1 AND erp_order_no = $2)",
    )
    .bind(owner_id)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load concurrent evidence");
    assert_eq!(counts, (1, 1));
}
