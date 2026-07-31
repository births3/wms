use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthRevocationStore,
        AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    express::{express_router, ExpressAppState},
};
use wms_domain::ExpressWaybill;

struct AllowAllRevocationStore;

#[axum::async_trait]
impl AuthRevocationStore for AllowAllRevocationStore {
    async fn jti_is_blacklisted(&self, _: &str) -> Result<bool, AuthRevocationStoreError> {
        Ok(false)
    }

    async fn permissions_changed_at(
        &self,
        _: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Ok(None)
    }

    async fn blacklist_jti(&self, _: &str, _: u64) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }

    async fn set_permissions_changed_at(
        &self,
        _: Uuid,
        _: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}

fn bearer_token(owner_id: Uuid) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "express-postgres-test",
        vec!["h5.express.write".to_string()],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token should encode")
}

fn post(path: &str, token: &str, key: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("Idempotency-Key", key)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn express_carrier_rule_waybill_cancel_writes_audit_and_replays(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $3)")
        .bind(owner_id)
        .bind(format!("EXP-{}", &owner_id.simple().to_string()[..8]))
        .bind("快递测试货主")
        .execute(&pool)
        .await
        .expect("owner should insert");
    let token = bearer_token(owner_id);
    let app = express_router(ExpressAppState::with_postgres(pool.clone())).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    );

    let carrier = json!({
        "carrier_code": "SF",
        "carrier_name": "顺丰",
        "api_url": "https://express.example.invalid",
        "api_key_alias": "h5/sf/key",
        "api_secret_alias": "h5/sf/secret",
        "account_no": "WMS",
        "enabled": true,
        "priority": 1,
        "conditions": {}
    });
    let response = app
        .clone()
        .oneshot(post(
            "/api/v1/express/carriers",
            &token,
            "express-carrier-1",
            carrier,
        ))
        .await
        .expect("carrier request should complete");
    assert_eq!(response.status(), StatusCode::OK);

    let rule = json!({
        "rule_code": "NORMAL-SF",
        "rule_name": "普通件顺丰",
        "delivery_provider_type": "third_party_express",
        "carrier_code": "SF",
        "priority": 1,
        "conditions": {},
        "fallback_strategy": "manual",
        "enabled": true,
        "effective_from": null,
        "effective_to": null
    });
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(post(
                "/api/v1/express/routing-rules",
                &token,
                "express-rule-1",
                rule.clone(),
            ))
            .await
            .expect("routing-rule request should complete");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let waybill_request = json!({
        "outbound_order_id": null,
        "package_no": "PKG-H5-001",
        "carrier_code": "SF",
        "requested_waybill_no": "SF-H5-001",
        "sender_name": "上海仓",
        "sender_mobile": "13800000000",
        "sender_address": "上海市",
        "receiver_name": "客户",
        "receiver_mobile": "13900000000",
        "receiver_address": "杭州市",
        "weight_grams": 1000,
        "volume_cm3": 2000,
        "package_count": 1
    });
    let response = app
        .clone()
        .oneshot(post(
            "/api/v1/express/waybills",
            &token,
            "express-waybill-1",
            waybill_request,
        ))
        .await
        .expect("waybill request should complete");
    assert_eq!(response.status(), StatusCode::OK);
    let waybill: ExpressWaybill = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("waybill body should read"),
    )
    .expect("waybill body should parse");

    let response = app
        .clone()
        .oneshot(post(
            &format!("/api/v1/express/waybills/{}/cancel", waybill.waybill_no),
            &token,
            "express-cancel-1",
            json!({"reason": "客户取消"}),
        ))
        .await
        .expect("cancel request should complete");
    assert_eq!(response.status(), StatusCode::OK);
    sqlx::query(
        "UPDATE idempotency_request SET method = 'PATCH', path = '/wrong-path' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("express-cancel-1")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be mutable for the regression check");
    let metadata_conflict = app
        .clone()
        .oneshot(post(
            &format!("/api/v1/express/waybills/{}/cancel", waybill.waybill_no),
            &token,
            "express-cancel-1",
            json!({"reason": "客户取消"}),
        ))
        .await
        .expect("metadata conflict request should complete");
    assert_eq!(metadata_conflict.status(), StatusCode::CONFLICT);

    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM h5_express_carriers WHERE owner_id = $1),
            (SELECT COUNT(*) FROM h5_express_routing_rules WHERE owner_id = $1),
            (SELECT COUNT(*) FROM h5_express_waybills WHERE owner_id = $1),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H5')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("express PostgreSQL and audit_event should query");
    assert_eq!(counts, (1, 1, 1, 4));
    let idempotency_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("idempotency records should query");
    assert_eq!(
        idempotency_count, 4,
        "same-key replay must not duplicate writes"
    );
}
