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

async fn seed_context(pool: &PgPool, owner_id: Uuid, api_key_id: Uuid, product_code: &str) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H8 product change REST owner')",
    )
    .bind(owner_id)
    .bind(format!("H8-PC-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed owner");
    sqlx::query(
        r#"
        INSERT INTO h8_erp_connectors (
            id, owner_id, connector_code, connector_name, warehouse_ids,
            directions, message_types, channel_mode, api_key_id, status,
            config_version, first_activated_at, last_tested_version,
            last_tested_at, last_tested_succeeded
        )
        VALUES (
            $1, $2, 'H8-CHANGE-REST', 'H8 product change REST', ARRAY[]::uuid[],
            ARRAY['inbound'], ARRAY['product_change'], 'rest', $3, 'active',
            1, now(), 1, now(), TRUE
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(api_key_id)
    .execute(pool)
    .await
    .expect("seed connector");
    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, dosage_form,
            storage_condition, special_drug_category, status, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'H8 变更前商品', '10mg*10片', '胶囊剂',
                'normal_10_30', 'none', 'active', now(), now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(product_code)
    .execute(pool)
    .await
    .expect("seed product");
}

async fn seed_archive_closeout(pool: &PgPool, owner_id: Uuid, product_code: &str) -> (Uuid, Uuid) {
    let user_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let asn_id = Uuid::new_v4();
    let receipt_id = Uuid::new_v4();
    let liaison_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash) VALUES ($1, $2, 'H8 archive approver', 'test-only')",
    )
    .bind(user_id)
    .bind(format!("h8-archive-{}", &user_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed archive user");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'H8 archive warehouse', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("H8-AR-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed archive warehouse");
    sqlx::query(
        "UPDATE products SET approval_no = 'OLD-001' WHERE owner_id = $1 AND product_code = $2",
    )
    .bind(owner_id)
    .bind(product_code)
    .execute(pool)
    .await
    .expect("seed old approval number");
    sqlx::query(
        "INSERT INTO receiving_orders (id, owner_id, receipt_no, document_type, warehouse_id, status) VALUES ($1, $2, 'ASN-H8-ARCHIVE', 'purchase_inbound', $3, 'archive_replenishing')",
    )
    .bind(asn_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("seed archive ASN");
    sqlx::query(
        "INSERT INTO receiving_order_lines (id, receiving_order_id, owner_id, line_no, product_code, expected_qty) VALUES ($1, $2, $3, 1, $4, 1)",
    )
    .bind(Uuid::new_v4())
    .bind(asn_id)
    .bind(owner_id)
    .bind(product_code)
    .execute(pool)
    .await
    .expect("seed archive ASN line");
    sqlx::query(
        "INSERT INTO receiving_order_receipts (id, receiving_order_id, owner_id, actual_qty, shortage_qty, rejected_qty, occurred_at) VALUES ($1, $2, $3, 1, 0, 0, now())",
    )
    .bind(receipt_id)
    .bind(asn_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed archive receipt");
    sqlx::query(
        r#"
        INSERT INTO quality_liaison_types (
            id, owner_id, type_code, type_name, approval_template_id,
            approver_user_id, timeout_seconds, created_by
        ) VALUES ($1, $2, 'archive_revision', '档案补录', 'h8-test', $3, 3600, $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed archive liaison type");
    sqlx::query(
        r#"
        INSERT INTO quality_liaison_orders (
            id, owner_id, liaison_no, type_code, related_document_type,
            related_document_no, problem_description, disposition_suggestion,
            trigger_source, business_payload, status, created_by
        ) VALUES (
            $1, $2, 'QL-H8-ARCHIVE', 'archive_revision', 'asn',
            'ASN-H8-ARCHIVE', '批准文号不一致', '以 ERP 结果为准',
            'M2', $3, 'pending_erp_sync', $4
        )
        "#,
    )
    .bind(liaison_id)
    .bind(owner_id)
    .bind(json!({
        "action": "publish_archive_revision",
        "warehouse_id": warehouse_id,
        "asn_id": asn_id,
        "receipt_record_id": receipt_id,
        "product_code": product_code,
        "field_name": "approval_number",
        "current_value": "OLD-001",
        "new_value": "NEW-001",
        "photo_evidence_urls": ["https://files.example.test/h8-archive.jpg"]
    }))
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed pending archive liaison");
    sqlx::query(
        r#"
        INSERT INTO archive_revision_erp_feedback_outbox (
            id, owner_id, liaison_id, asn_id, receipt_record_id,
            product_code, field_name, payload, status, created_at, updated_at, deadline_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, 'approval_number', '{}',
            'succeeded', now(), now(), now() + interval '24 hours'
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(liaison_id)
    .bind(asn_id)
    .bind(receipt_id)
    .bind(product_code)
    .execute(pool)
    .await
    .expect("seed succeeded archive outbox");
    (liaison_id, asn_id)
}

fn context(owner_id: Uuid, api_key_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "H8 product change API Key".to_string(),
        permissions: vec!["m1.master_data.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: None,
    }
}

fn request(body: &Value, idempotency_key: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/integration/erp-messages/inbound/product_change")
        .header("content-type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_maps_updates_and_replays_one_resource(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let app = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)));
    let external_ref = format!("ERP-PC-{}", Uuid::new_v4());
    let idempotency_key = format!("h8-change-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "product_id": null,
        "product_code": product_code,
        "field_name": "dosage_form",
        "new_value": "薄膜衣片",
        "liaison_id": null,
        "asn_id": null
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
    changed["new_value"] = Value::String("不同剂型".to_string());
    let conflict = app
        .oneshot(request(&changed, &idempotency_key))
        .await
        .expect("changed replay should respond");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let evidence: (String, i64, String, String, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            dosage_form,
            version,
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_change' AND external_ref = $3),
            (SELECT COALESCE(warehouse_id::text, 'owner-level') FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_change' AND external_ref = $3),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'update_product' AND resource_id = products.id::text),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id = $1 AND idempotency_key = $4),
            (SELECT COUNT(*) FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_change' AND external_ref = $3)
          FROM products
         WHERE owner_id = $1 AND product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .bind(&idempotency_key)
    .fetch_one(&pool)
    .await
    .expect("load product change evidence");
    assert_eq!(
        evidence,
        (
            "片剂".to_string(),
            2,
            "succeeded".to_string(),
            "owner-level".to_string(),
            1,
            1,
            1,
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
    .expect("load H8 product change lifecycle audits");
    assert_eq!(
        lifecycle_actions,
        vec![
            "h8_exchange_receive",
            "h8_exchange_convert",
            "h8_exchange_business_api",
            "h8_exchange_receipt",
        ]
    );
    let trace: (i64, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        r#"
        SELECT COUNT(*), MAX(field_name), MAX(source_value), MAX(target_value)
          FROM product_mapping_traces trace
          JOIN products product
            ON product.owner_id = trace.owner_id AND product.id = trace.product_id
         WHERE product.owner_id = $1 AND product.product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .fetch_one(&pool)
    .await
    .expect("load product change mapping trace");
    assert_eq!(
        trace,
        (
            1,
            Some("dosage_form".to_string()),
            Some("薄膜衣片".to_string()),
            Some("片剂".to_string()),
        )
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_updates_physical_dimensions_atomically(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let body = json!({
        "schema_version": "1",
        "external_ref": format!("ERP-PC-{}", Uuid::new_v4()),
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "product_id": null,
        "product_code": product_code,
        "field_name": "physical_dimensions",
        "physical_dimensions": {
            "length_mm": 120.5,
            "width_mm": 45.0,
            "height_mm": 30.25
        },
        "liaison_id": null,
        "asn_id": null
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(
            &body,
            &format!("h8-change-dimensions-{}", Uuid::new_v4()),
        ))
        .await
        .expect("atomic dimensions change should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let dimensions: (Option<f64>, Option<f64>, Option<f64>, i64) = sqlx::query_as(
        "SELECT length_mm, width_mm, height_mm, version FROM products WHERE owner_id = $1 AND product_code = $2",
    )
    .bind(owner_id)
    .bind(&product_code)
    .fetch_one(&pool)
    .await
    .expect("load updated dimensions");
    assert_eq!(dimensions, (Some(120.5), Some(45.0), Some(30.25), 2));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_rejects_individual_dimension_field(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let body = json!({
        "schema_version": "1",
        "external_ref": format!("ERP-PC-{}", Uuid::new_v4()),
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "product_id": null,
        "product_code": product_code,
        "field_name": "length_mm",
        "new_value": "120.5",
        "liaison_id": null,
        "asn_id": null
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(
            &body,
            &format!("h8-change-one-dimension-{}", Uuid::new_v4()),
        ))
        .await
        .expect("individual dimension change should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let dimensions: (Option<f64>, Option<f64>, Option<f64>, i64) = sqlx::query_as(
        "SELECT length_mm, width_mm, height_mm, version FROM products WHERE owner_id = $1 AND product_code = $2",
    )
    .bind(owner_id)
    .bind(&product_code)
    .fetch_one(&pool)
    .await
    .expect("load unchanged dimensions");
    assert_eq!(dimensions, (None, None, None, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_replaces_packaging_with_mpm_traces(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let idempotency_key = format!("h8-change-packaging-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": format!("ERP-PC-{}", Uuid::new_v4()),
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "product_id": null,
        "product_code": product_code,
        "field_name": "packaging_levels",
        "new_value": serde_json::to_string(&json!([
            {
                "unit": "支",
                "ratio_to_base": 1,
                "is_base": true,
                "is_default": false,
                "sort_order": 1
            },
            {
                "unit": "盒",
                "ratio_to_base": 10,
                "is_base": false,
                "is_default": true,
                "sort_order": 2
            }
        ])).expect("packaging JSON"),
        "liaison_id": null,
        "asn_id": null
    });
    let app = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)));

    let first = app
        .clone()
        .oneshot(request(&body, &idempotency_key))
        .await
        .expect("packaging change should respond");
    assert_eq!(first.status(), StatusCode::OK);
    let replay = app
        .oneshot(request(&body, &idempotency_key))
        .await
        .expect("packaging replay should respond");
    assert_eq!(replay.status(), StatusCode::OK);

    let evidence: (i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM product_packaging_levels level
              WHERE level.owner_id = product.owner_id AND level.product_id = product.id),
            (SELECT COUNT(*) FROM product_packaging_levels level
              WHERE level.owner_id = product.owner_id AND level.product_id = product.id
                AND level.is_base),
            (SELECT COUNT(*) FROM product_packaging_levels level
              WHERE level.owner_id = product.owner_id AND level.product_id = product.id
                AND level.is_default),
            (SELECT COUNT(*) FROM product_mapping_traces trace
              WHERE trace.owner_id = product.owner_id AND trace.product_id = product.id
                AND trace.field_name LIKE 'packaging_levels[%].unit_code'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = product.owner_id AND resource_id = product.id::text
                AND action = 'update_product')
          FROM products product
         WHERE product.owner_id = $1 AND product.product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .fetch_one(&pool)
    .await
    .expect("load packaging change evidence");
    assert_eq!(evidence, (2, 1, 1, 2, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_rejects_unmapped_storage_before_business_write(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let external_ref = format!("ERP-PC-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "product_id": null,
        "product_code": product_code,
        "field_name": "storage_condition",
        "new_value": format!("未知储存条件-{}", Uuid::new_v4()),
        "liaison_id": null,
        "asn_id": null
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(&body, &format!("h8-change-{}", Uuid::new_v4())))
        .await
        .expect("request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (String, i64, String, i64) = sqlx::query_as(
        r#"
        SELECT
            storage_condition,
            version,
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_change' AND external_ref = $3),
            (SELECT COUNT(*) FROM parameter_mapping_queue
              WHERE owner_id = $1 AND source_record_id = $3)
          FROM products
         WHERE owner_id = $1 AND product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load failed product change evidence");
    assert_eq!(evidence, ("normal".to_string(), 1, "dead".to_string(), 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_maps_external_status_before_update(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let external_ref = format!("ERP-PC-{}", Uuid::new_v4());
    let idempotency_key = format!("h8-change-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "product_id": null,
        "product_code": product_code,
        "field_name": "status",
        "new_value": "停用",
        "liaison_id": null,
        "asn_id": null
    });
    let app = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)));

    for expected_replayed in [false, true] {
        let response = app
            .clone()
            .oneshot(request(&body, &idempotency_key))
            .await
            .expect("status change should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let response: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("status response should read"),
        )
        .expect("status response should be JSON");
        assert_eq!(response["replayed"], expected_replayed);
    }

    let evidence: (String, i64, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            status,
            version,
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_change' AND external_ref = $3),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'update_product' AND resource_id = products.id::text),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id = $1 AND idempotency_key = $4)
          FROM products
         WHERE owner_id = $1 AND product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .bind(&idempotency_key)
    .fetch_one(&pool)
    .await
    .expect("load status mapping evidence");
    assert_eq!(
        evidence,
        ("disabled".to_string(), 2, "succeeded".to_string(), 1, 1)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_rejects_unmapped_status_before_business_write(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let external_ref = format!("ERP-PC-{}", Uuid::new_v4());
    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(
            &json!({
                "schema_version": "1",
                "external_ref": external_ref,
                "correlation_id": format!("corr-{}", Uuid::new_v4()),
                "occurred_at": Utc::now(),
                "product_id": null,
                "product_code": product_code,
                "field_name": "status",
                "new_value": "待映射",
                "liaison_id": null,
                "asn_id": null
            }),
            &format!("h8-change-{}", Uuid::new_v4()),
        ))
        .await
        .expect("unmapped status should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (String, i64, String, i64) = sqlx::query_as(
        r#"
        SELECT
            status,
            version,
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_change' AND external_ref = $3),
            (SELECT COUNT(*) FROM parameter_mapping_queue
              WHERE owner_id = $1 AND source_record_id = $3)
          FROM products
         WHERE owner_id = $1 AND product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load failed status mapping evidence");
    assert_eq!(evidence, ("active".to_string(), 1, "dead".to_string(), 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_rejects_special_drug_category_change_without_product_write(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let external_ref = format!("ERP-PC-{}", Uuid::new_v4());
    let idempotency_key = format!("h8-special-category-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "product_id": null,
        "product_code": product_code,
        "field_name": "special_drug_category",
        "new_value": "麻醉药品",
        "liaison_id": null,
        "asn_id": null
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(&body, &idempotency_key))
        .await
        .expect("special category change should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (String, i64, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            product.special_drug_category,
            product.version,
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_change' AND external_ref = $3),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'update_product'
                AND resource_id = product.id::text),
            (SELECT COUNT(*) FROM idempotency_request
              WHERE owner_id = $1 AND idempotency_key = $4)
          FROM products product
         WHERE product.owner_id = $1 AND product.product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .bind(&idempotency_key)
    .fetch_one(&pool)
    .await
    .expect("rejected special category evidence should query");
    assert_eq!(evidence, ("none".to_string(), 1, "dead".to_string(), 0, 0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_change_rest_completes_archive_liaison_and_unlocks_asn(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let product_code = format!("H8-PC-{}", &Uuid::new_v4().to_string()[..8]);
    seed_context(&pool, owner_id, api_key_id, &product_code).await;
    let (liaison_id, asn_id) = seed_archive_closeout(&pool, owner_id, &product_code).await;
    let body = json!({
        "schema_version": "1",
        "external_ref": format!("ERP-PC-{}", Uuid::new_v4()),
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "product_id": null,
        "product_code": product_code,
        "field_name": "approval_number",
        "new_value": "NEW-001",
        "liaison_id": liaison_id,
        "asn_id": asn_id
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(
            &body,
            &format!("h8-archive-change-{}", Uuid::new_v4()),
        ))
        .await
        .expect("archive product change should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let evidence: (String, String, String, String, i64) = sqlx::query_as(
        r#"
        SELECT
            product.approval_no,
            liaison.status,
            receiving_order.status,
            message.sync_status,
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'complete_archive_revision_sync'
                AND resource_id = $2)
          FROM products product
          JOIN quality_liaison_orders liaison
            ON liaison.owner_id = product.owner_id AND liaison.id = $3
          JOIN receiving_orders receiving_order
            ON receiving_order.owner_id = product.owner_id AND receiving_order.id = $4
          JOIN h8_erp_messages message
            ON message.owner_id = product.owner_id
           AND message.message_type = 'product_change'
           AND message.external_ref = $5
         WHERE product.owner_id = $1 AND product.product_code = $6
        "#,
    )
    .bind(owner_id)
    .bind(liaison_id.to_string())
    .bind(liaison_id)
    .bind(asn_id)
    .bind(body["external_ref"].as_str().expect("external ref"))
    .bind(&product_code)
    .fetch_one(&pool)
    .await
    .expect("load archive closeout evidence");
    assert_eq!(
        evidence,
        (
            "NEW-001".to_string(),
            "landed".to_string(),
            "inspecting".to_string(),
            "succeeded".to_string(),
            1,
        )
    );
}
