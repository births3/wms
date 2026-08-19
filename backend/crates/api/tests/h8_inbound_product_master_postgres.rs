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

async fn seed_context(pool: &PgPool, owner_id: Uuid, api_key_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H8 product REST test owner')",
    )
    .bind(owner_id)
    .bind(format!("H8-PM-{}", &owner_id.to_string()[..8]))
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
            $1, $2, 'H8-PRODUCT-REST', 'H8 product REST', ARRAY[]::uuid[],
            ARRAY['inbound'], ARRAY['product_master'], 'rest', $3, 'active',
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
}

fn context(owner_id: Uuid, api_key_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "H8 product API Key".to_string(),
        permissions: vec!["m1.master_data.write".to_string()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: None,
    }
}

fn request(body: &Value, idempotency_key: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/integration/erp-messages/inbound/product_master")
        .header("content-type", "application/json")
        .header("Idempotency-Key", idempotency_key)
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_master_rest_rejects_missing_spec_without_writes(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id).await;
    let product_code = format!("H8-PM-NO-SPEC-{}", &Uuid::new_v4().to_string()[..8]);
    let body = json!({
        "schema_version": "1",
        "external_ref": format!("ERP-PM-{}", Uuid::new_v4()),
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "payload_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_version": 1,
        "entity_id": 1001,
        "op_type": "I",
        "product_code": product_code,
        "product_name": "H8 缺规格商品",
        "special_drug_category": "普通药品",
        "packaging_levels": [
            {"unit": "盒", "ratio_to_base": 1, "is_base": true, "is_default": true, "sort_order": 1}
        ],
        "storage_condition": "常温"
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(
            &body,
            &format!("h8-product-no-spec-{}", Uuid::new_v4()),
        ))
        .await
        .expect("request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let writes: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM products WHERE owner_id = $1 AND product_code = $2),
            (SELECT COUNT(*) FROM h8_erp_messages WHERE owner_id = $1),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1)
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .fetch_one(&pool)
    .await
    .expect("missing spec write evidence should query");
    assert_eq!(writes, (0, 0, 0, 0));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_master_business_validation_failure_enters_dead_with_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id).await;
    let external_ref = format!("ERP-PM-INVALID-{}", Uuid::new_v4());
    let product_code = format!("H8-PM-INVALID-{}", &Uuid::new_v4().to_string()[..8]);
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "payload_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_version": 1,
        "entity_id": 1002,
        "op_type": "I",
        "product_code": product_code,
        "product_name": "H8 非法包装商品",
        "spec": "10mg*30片",
        "special_drug_category": "普通药品",
        "packaging_levels": [
            {"unit": "盒", "ratio_to_base": 1, "is_base": false, "is_default": true, "sort_order": 1}
        ],
        "storage_condition": "常温"
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(
            &body,
            &format!("h8-product-invalid-{}", Uuid::new_v4()),
        ))
        .await
        .expect("request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (i64, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM products WHERE owner_id = $1 AND product_code = $2),
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_master' AND external_ref = $3),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'h8_exchange_final_failure'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'h8_message_dead')
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load product validation failure evidence");
    assert_eq!(evidence, (0, "dead".to_string(), 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_master_rest_maps_persists_and_replays_one_resource(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id).await;
    let state = H8InboundAppState::with_postgres(pool.clone());
    let app = h8_inbound_router(state.clone()).layer(Extension(context(owner_id, api_key_id)));
    let external_ref = format!("ERP-PM-{}", Uuid::new_v4());
    let product_code = format!("H8-PM-{}", &Uuid::new_v4().to_string()[..8]);
    let idempotency_key = format!("h8-product-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "payload_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_version": 1,
        "entity_id": 1003,
        "op_type": "I",
        "product_code": product_code,
        "product_name": "H8 REST 商品",
        "approval_no": "国药准字 H8",
        "spec": "10mg*30片",
        "dosage_form": "薄膜衣片",
        "manufacturer": "H8 制药",
        "special_drug_category": "普通药品",
        "udi_code": "06912345678901",
        "electronic_regulatory_code": "H8-REG-001",
        "length_mm": 120.0,
        "width_mm": 80.0,
        "height_mm": 50.0,
        "weight_g": 350.5,
        "packaging_levels": [
            {"unit": "支", "ratio_to_base": 1, "is_base": true, "is_default": false, "sort_order": 1},
            {"unit": "盒", "ratio_to_base": 12, "is_base": false, "is_default": true, "sort_order": 2},
            {"unit": "件", "ratio_to_base": 120, "is_base": false, "is_default": false, "sort_order": 3}
        ],
        "storage_condition": "2-8℃避光保存"
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
    changed["product_name"] = Value::String("不同商品".to_string());
    changed["payload_digest"] = Value::String(
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
    );
    let conflict = app
        .oneshot(request(&changed, &idempotency_key))
        .await
        .expect("changed replay should respond");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let evidence: (i64, String, Option<String>, String, String, i64, i64, i64) = sqlx::query_as(
        r#"
            SELECT
                (SELECT COUNT(*) FROM products WHERE owner_id = $1 AND product_code = $2),
                (SELECT storage_condition FROM products WHERE owner_id = $1 AND product_code = $2),
                (SELECT dosage_form FROM products WHERE owner_id = $1 AND product_code = $2),
                (SELECT sync_status FROM h8_erp_messages
                  WHERE owner_id = $1 AND message_type = 'product_master' AND external_ref = $3),
                (SELECT COALESCE(warehouse_id::text, 'owner-level') FROM h8_erp_messages
                  WHERE owner_id = $1 AND message_type = 'product_master' AND external_ref = $3),
                (SELECT COUNT(*) FROM audit_event
                  WHERE owner_id = $1 AND action = 'apply_erp_master_snapshot'),
                (SELECT COUNT(*) FROM idempotency_request
                  WHERE owner_id = $1 AND idempotency_key = $4),
                (SELECT COUNT(*) FROM h8_erp_messages
                  WHERE owner_id = $1 AND message_type = 'product_master' AND external_ref = $3)
            "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .bind(&idempotency_key)
    .fetch_one(&pool)
    .await
    .expect("load product master evidence");
    assert_eq!(
        evidence,
        (
            1,
            "cold".to_string(),
            Some("片剂".to_string()),
            "succeeded".to_string(),
            "owner-level".to_string(),
            1,
            0,
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
    .expect("load H8 product master lifecycle audits");
    assert_eq!(
        lifecycle_actions,
        vec![
            "h8_exchange_receive",
            "h8_exchange_convert",
            "h8_exchange_business_api",
            "h8_exchange_receipt",
        ]
    );
    let complete_contract: (String, String, Option<String>, Option<f64>, i64, i64) =
        sqlx::query_as(
            r#"
            SELECT special_drug_category, udi_code, electronic_regulatory_code, volume_cm3,
                   (SELECT COUNT(*) FROM product_packaging_levels
                     WHERE owner_id = $1 AND product_id = products.id),
                   (SELECT COUNT(*) FROM product_mapping_traces
                     WHERE owner_id = $1 AND product_id = products.id)
              FROM products
             WHERE owner_id = $1 AND product_code = $2
            "#,
        )
        .bind(owner_id)
        .bind(&product_code)
        .fetch_one(&pool)
        .await
        .expect("load complete product contract");
    assert_eq!(
        complete_contract,
        (
            "none".to_string(),
            "06912345678901".to_string(),
            Some("H8-REG-001".to_string()),
            Some(480.0),
            3,
            6,
        )
    );

    let scoped = h8_inbound_router(state.clone())
        .layer(Extension(AuthContext {
            warehouse_scope: Some(Uuid::new_v4()),
            ..context(owner_id, api_key_id)
        }))
        .oneshot(request(&body, &format!("scoped-{idempotency_key}")))
        .await
        .expect("warehouse-scoped request should respond");
    assert_eq!(scoped.status(), StatusCode::FORBIDDEN);

    let denied = h8_inbound_router(state)
        .layer(Extension(context(owner_id, Uuid::new_v4())))
        .oneshot(request(&body, &format!("denied-{idempotency_key}")))
        .await
        .expect("unbound request should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_master_rest_rejects_required_unmapped_values_without_product_write(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id).await;
    let external_ref = format!("ERP-PM-{}", Uuid::new_v4());
    let product_code = format!("H8-PM-{}", &Uuid::new_v4().to_string()[..8]);
    let unresolved_storage = format!("未知储存条件-{}", Uuid::new_v4());
    let unresolved_category = format!("未知药品类别-{}", Uuid::new_v4());
    let unresolved_unit = format!("未知包装单位-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "payload_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_version": 1,
        "entity_id": 1004,
        "op_type": "I",
        "product_code": product_code,
        "product_name": "H8 未映射商品",
        "approval_no": null,
        "spec": "10mg*30片",
        "dosage_form": null,
        "manufacturer": null,
        "special_drug_category": unresolved_category,
        "packaging_levels": [
            {"unit": unresolved_unit, "ratio_to_base": 1, "is_base": true, "is_default": true, "sort_order": 1}
        ],
        "storage_condition": unresolved_storage
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(&body, &format!("h8-product-{}", Uuid::new_v4())))
        .await
        .expect("request should respond");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let evidence: (String, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT sync_status FROM h8_erp_messages
              WHERE owner_id = $1 AND message_type = 'product_master' AND external_ref = $3),
            (SELECT COUNT(*) FROM parameter_mapping_queue
              WHERE owner_id = $1 AND source_record_id = $3),
            (SELECT COUNT(*) FROM products
              WHERE owner_id = $1 AND product_code = $2),
            (SELECT COUNT(*) FROM product_mapping_traces
              WHERE owner_id = $1)
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .fetch_one(&pool)
    .await
    .expect("load rejected product evidence");
    assert_eq!(evidence, ("dead".to_string(), 3, 0, 0));

    let mut mapped_body = body;
    mapped_body["external_ref"] = json!(format!("ERP-PM-{}", Uuid::new_v4()));
    mapped_body["correlation_id"] = json!(format!("corr-{}", Uuid::new_v4()));
    mapped_body["payload_digest"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    mapped_body["source_version"] = json!(2);
    mapped_body["op_type"] = json!("U");
    mapped_body["storage_condition"] = json!("常温");
    mapped_body["special_drug_category"] = json!("普通药品");
    mapped_body["packaging_levels"][0]["unit"] = json!("盒");
    let activated = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(
            &mapped_body,
            &format!("h8-product-activate-{}", Uuid::new_v4()),
        ))
        .await
        .expect("mapped replacement should create product");
    assert_eq!(activated.status(), StatusCode::OK);

    let activated_evidence: (String, String, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT product.status, product.storage_condition,
               product.special_drug_category,
               (SELECT COUNT(*) FROM product_packaging_levels
                 WHERE owner_id = $1 AND product_id = product.id),
               (SELECT COUNT(*) FROM audit_event
                 WHERE owner_id = $1
                   AND resource_id = product.id::text
                   AND action = 'apply_erp_master_snapshot')
          FROM products product
         WHERE product.owner_id = $1 AND product.product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .fetch_one(&pool)
    .await
    .expect("load mapped product");
    assert_eq!(
        activated_evidence,
        (
            "active".to_string(),
            "normal".to_string(),
            "none".to_string(),
            1,
            1,
        )
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn product_master_rest_keeps_unmapped_dosage_form_active_with_trace(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    seed_context(&pool, owner_id, api_key_id).await;
    let external_ref = format!("ERP-PM-{}", Uuid::new_v4());
    let product_code = format!("H8-PM-{}", &Uuid::new_v4().to_string()[..8]);
    let dosage_form = format!("未知剂型-{}", Uuid::new_v4());
    let body = json!({
        "schema_version": "1",
        "external_ref": external_ref,
        "correlation_id": format!("corr-{}", Uuid::new_v4()),
        "occurred_at": Utc::now(),
        "payload_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "source_version": 1,
        "entity_id": 1005,
        "op_type": "I",
        "product_code": product_code,
        "product_name": "H8 未映射剂型商品",
        "approval_no": null,
        "spec": "10mg*30片",
        "dosage_form": dosage_form,
        "manufacturer": null,
        "special_drug_category": "普通药品",
        "packaging_levels": [
            {"unit": "盒", "ratio_to_base": 1, "is_base": true, "is_default": true, "sort_order": 1}
        ],
        "storage_condition": "常温"
    });

    let response = h8_inbound_router(H8InboundAppState::with_postgres(pool.clone()))
        .layer(Extension(context(owner_id, api_key_id)))
        .oneshot(request(&body, &format!("h8-product-{}", Uuid::new_v4())))
        .await
        .expect("request should respond");
    assert_eq!(response.status(), StatusCode::OK);

    let evidence: (String, Option<String>, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            product.status,
            product.dosage_form,
            (SELECT COUNT(*)
               FROM parameter_mapping_queue queue
               JOIN parameter_mapping_dictionaries dictionary
                 ON dictionary.id = queue.dictionary_id
              WHERE queue.owner_id = $1
                AND queue.source_record_id = $3
                AND dictionary.dict_code = 'dosage_form'),
            (SELECT COUNT(*)
               FROM product_mapping_traces trace
              WHERE trace.owner_id = $1
                AND trace.product_id = product.id
                AND trace.field_name = 'dosage_form'
                AND trace.source_value = $4
                AND trace.target_value = $4)
          FROM products product
         WHERE product.owner_id = $1 AND product.product_code = $2
        "#,
    )
    .bind(owner_id)
    .bind(&product_code)
    .bind(&external_ref)
    .bind(&dosage_form)
    .fetch_one(&pool)
    .await
    .expect("load unmapped dosage-form evidence");
    assert_eq!(evidence, ("active".to_string(), Some(dosage_form), 1, 1));
}
