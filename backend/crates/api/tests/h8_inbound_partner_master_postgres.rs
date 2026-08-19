use axum::{
    body::Body,
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

async fn seed_context(pool: &PgPool, owner_id: Uuid, api_key_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H8 partner test owner')",
    )
    .bind(owner_id)
    .bind(format!("H8-PARTNER-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed partner test owner");
    sqlx::query(
        r#"
        INSERT INTO h8_erp_connectors (
            id, owner_id, connector_code, connector_name, warehouse_ids,
            directions, message_types, channel_mode, api_key_id, status,
            config_version, first_activated_at, last_tested_version,
            last_tested_at, last_tested_succeeded
        ) VALUES (
            $1, $2, 'H8-PARTNER-REST', 'H8 partner REST', ARRAY[]::uuid[],
            ARRAY['inbound'], ARRAY['customer_master','supplier_master'], 'rest', $3, 'active',
            1, now(), 1, now(), TRUE
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(api_key_id)
    .execute(pool)
    .await
    .expect("seed partner connector");
}

fn context(owner_id: Uuid, api_key_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "H8 partner API Key".to_string(),
        permissions: vec!["m1.master_data.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: None,
    }
}

fn request(message_type: &str, body: Value, key: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/api/v1/integration/erp-messages/inbound/{message_type}"
        ))
        .header("content-type", "application/json")
        .header("Idempotency-Key", key)
        .body(Body::from(body.to_string()))
        .expect("partner request should build")
}

fn envelope(source_version: i64, external_ref: String) -> Value {
    json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "payload_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_version": source_version,
    })
}

#[sqlx::test(migrations = "../../migrations")]
async fn partner_master_applies_newer_versions_and_ignores_older_ones(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id).await;
    let app = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)));

    let customer_id = 41001_i64;
    for (version, name) in [(1_i64, "客户一"), (3, "客户三"), (2, "客户二")] {
        let body = json!({
            "entity_id": customer_id,
            "op_type": "U",
            "customer_code": "C-41001",
            "customer_name": name,
            "customer_type": "医院",
            "address": "注册地址",
            "contact_name": "张三",
            "contact_phone": "13800000000",
            "delivery_address": "收货地址",
            "delivery_contact": "李四",
            "delivery_phone": "13900000000",
            "delivery_mode": 1,
            "stop_send": false,
        });
        let mut body = body.as_object().expect("customer body object").clone();
        body.extend(
            envelope(version, format!("customer-{customer_id}-v{version}"))
                .as_object()
                .expect("envelope object")
                .clone(),
        );
        let response = app
            .clone()
            .oneshot(request(
                "customer_master",
                Value::Object(body),
                &format!("customer-{customer_id}-v{version}"),
            ))
            .await
            .expect("customer request should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let customer: (String, i64, Value) = sqlx::query_as(
        "SELECT customer_name, erp_source_version, erp_payload FROM customers WHERE owner_id=$1 AND erp_client_id=$2",
    )
    .bind(owner_id)
    .bind(customer_id)
    .fetch_one(&pool)
    .await
    .expect("customer snapshot should persist");
    assert_eq!(customer.0, "客户三");
    assert_eq!(customer.1, 3);
    assert_eq!(customer.2["delivery_address"], "收货地址");

    let supplier_id = 51001_i64;
    let mut supplier = json!({
        "entity_id": supplier_id,
        "op_type": "I",
        "supplier_code": "S-51001",
        "supplier_name": "供应商一",
        "address": "供应商地址",
        "contact_name": "王五",
        "contact_phone": "13700000000",
    })
    .as_object()
    .expect("supplier body object")
    .clone();
    supplier.extend(
        envelope(1, format!("supplier-{supplier_id}-v1"))
            .as_object()
            .expect("envelope object")
            .clone(),
    );
    let response = app
        .clone()
        .oneshot(request(
            "supplier_master",
            Value::Object(supplier),
            &format!("supplier-{supplier_id}-v1"),
        ))
        .await
        .expect("supplier request should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let supplier: (String, i64) = sqlx::query_as(
        "SELECT supplier_name, erp_source_version FROM suppliers WHERE owner_id=$1 AND erp_supplier_id=$2",
    )
    .bind(owner_id)
    .bind(supplier_id)
    .fetch_one(&pool)
    .await
    .expect("supplier snapshot should persist");
    assert_eq!(supplier, ("供应商一".to_string(), 1));

    let mut deletion = json!({
        "entity_id": supplier_id,
        "op_type": "D",
    })
    .as_object()
    .expect("supplier deletion object")
    .clone();
    deletion.extend(
        envelope(2, format!("supplier-{supplier_id}-v2"))
            .as_object()
            .expect("envelope object")
            .clone(),
    );
    let response = app
        .oneshot(request(
            "supplier_master",
            Value::Object(deletion),
            &format!("supplier-{supplier_id}-v2"),
        ))
        .await
        .expect("supplier deletion should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let status: (String, i64) = sqlx::query_as(
        "SELECT status, erp_source_version FROM suppliers WHERE owner_id=$1 AND erp_supplier_id=$2",
    )
    .bind(owner_id)
    .bind(supplier_id)
    .fetch_one(&pool)
    .await
    .expect("supplier deletion should persist");
    assert_eq!(status, ("disabled".to_string(), 2));
}
