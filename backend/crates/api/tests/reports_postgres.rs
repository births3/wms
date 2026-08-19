use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthRevocationStore,
        AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    reports_handlers::{reports_router, ReportsAppState},
};
use wms_domain::ReportQueryRequest;

struct AllowAllRevocationStore;

#[axum::async_trait]
impl AuthRevocationStore for AllowAllRevocationStore {
    async fn jti_is_blacklisted(&self, _jti: &str) -> Result<bool, AuthRevocationStoreError> {
        Ok(false)
    }

    async fn permissions_changed_at(
        &self,
        _user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Ok(None)
    }

    async fn blacklist_jti(
        &self,
        _jti: &str,
        _ttl_seconds: u64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }

    async fn set_permissions_changed_at(
        &self,
        _user_id: Uuid,
        _changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}

fn bearer_token(owner_id: Uuid) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "reports-reader",
        vec!["m6.report.read".to_string()],
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token")
}

#[sqlx::test(migrations = "../../migrations")]
async fn report_query_counts_owner_scoped_receiving_orders(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 6, 4, 10, 0, 0).single().unwrap();
    for (owner, code) in [(owner_a, "A1"), (owner_a, "A2"), (owner_b, "B1")] {
        sqlx::query(
            r#"
            INSERT INTO receiving_orders (
                id, owner_id, receipt_no, document_type, warehouse_id, status, created_at, updated_at
            ) VALUES ($1, $2, $3, 'purchase_inbound', $4, 'draft', $5, $5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner)
        .bind(code)
        .bind(Uuid::new_v4())
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed order");
    }

    let app = reports_router(ReportsAppState::with_postgres(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore)),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reports/query")
                .header(AUTHORIZATION, format!("Bearer {}", bearer_token(owner_a)))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ReportQueryRequest {
                        report_code: "m6_inbound_summary".to_string(),
                        filters: json!({}),
                        limit: Some(20),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .expect("report query");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["rows"][0]["values"]["count"], 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn report_query_counts_owner_scoped_outbound_orders(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 6, 4, 10, 0, 0).single().unwrap();
    for (owner, code) in [
        (owner_a, "OUT-A1"),
        (owner_a, "OUT-A2"),
        (owner_b, "OUT-B1"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO outbound_orders (
                id, owner_id, wms_order_no, customer_id,
                delivery_address_id, delivery_address_snapshot,
                warehouse_id, status, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, gen_random_uuid(), '{}'::jsonb, $5, 'confirmed', $6, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner)
        .bind(code)
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed outbound order");
    }

    let app = reports_router(ReportsAppState::with_postgres(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore)),
    ));
    for (owner, expected_count) in [(owner_a, 2), (owner_b, 1)] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/reports/query")
                    .header(AUTHORIZATION, format!("Bearer {}", bearer_token(owner)))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&ReportQueryRequest {
                            report_code: "m6_outbound_summary".to_string(),
                            filters: json!({}),
                            limit: Some(20),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("outbound report query");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["report_code"], "m6_outbound_summary");
        assert!(payload["generated_at"].is_string());
        assert_eq!(payload["page"]["count"], 1);
        assert_eq!(payload["rows"][0]["values"]["metric"], "outbound_orders");
        assert_eq!(payload["rows"][0]["values"]["owner_id"], owner.to_string());
        assert_eq!(payload["rows"][0]["values"]["count"], expected_count);
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn report_query_rejects_unknown_report_code(pool: PgPool) {
    let app = reports_router(ReportsAppState::with_postgres(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore)),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reports/query")
                .header(
                    AUTHORIZATION,
                    format!("Bearer {}", bearer_token(Uuid::new_v4())),
                )
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ReportQueryRequest {
                        report_code: "m6_unknown_summary".to_string(),
                        filters: json!({}),
                        limit: Some(20),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .expect("unknown report query");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["code"], "REPORT_UNSUPPORTED_CODE");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gsp_inbound_ledger_is_owner_scoped(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let now = Utc.with_ymd_and_hms(2026, 6, 4, 11, 0, 0).single().unwrap();
    for (owner, receipt_no) in [(owner_a, "ASN-RPT-A"), (owner_b, "ASN-RPT-B")] {
        let order_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO receiving_orders (
                id, owner_id, receipt_no, document_type, warehouse_id, status, created_at, updated_at
            ) VALUES ($1, $2, $3, 'purchase_inbound', $4, 'inspecting', $5, $5)
            "#,
        )
        .bind(order_id)
        .bind(owner)
        .bind(receipt_no)
        .bind(Uuid::new_v4())
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed order");
        sqlx::query(
            r#"
            INSERT INTO receiving_order_lines (
                id, receiving_order_id, owner_id, line_no, product_code, expected_qty
            ) VALUES ($1, $2, $3, 1, 'P-RPT', 10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(order_id)
        .bind(owner)
        .execute(&pool)
        .await
        .expect("seed line");
        sqlx::query(
            r#"
            INSERT INTO receiving_order_receipts (
                id, receiving_order_id, owner_id, actual_qty, shortage_qty, rejected_qty, occurred_at
            ) VALUES ($1, $2, $3, 10, 0, 0, $4)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(order_id)
        .bind(owner)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed receipt");
    }

    let app = reports_router(ReportsAppState::with_postgres(pool)).layer(auth_runtime_layer(
        AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore)),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reports/gsp/inbound-ledger")
                .header(AUTHORIZATION, format!("Bearer {}", bearer_token(owner_a)))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ReportQueryRequest {
                        report_code: "gsp_inbound_ledger".to_string(),
                        filters: json!({}),
                        limit: Some(20),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .expect("ledger query");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["page"]["count"], 1);
    assert_eq!(payload["rows"][0]["document_no"], "ASN-RPT-A");
    assert_eq!(payload["rows"][0]["product_code"], "P-RPT");
}
