use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    wave3_handlers::{wave3_router, Wave3AppState},
};

fn ctx(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m2-adversarial-test".to_string(),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn forbidden_inbound(
    pool: PgPool,
    method: &str,
    uri: String,
    permissions: &[&str],
    body: String,
) {
    let app = wave3_router(Wave3AppState::with_postgres(pool));
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header("idempotency-key", "m2-adversarial-forbidden")
        .body(Body::from(body))
        .expect("m2 adversarial request should build");
    request
        .extensions_mut()
        .insert(ctx(Uuid::new_v4(), permissions));
    let response = app
        .oneshot(request)
        .await
        .expect("m2 adversarial route should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn inbound_write_http_requires_m2_write_permission(pool: PgPool) {
    let order_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let signer_id = Uuid::new_v4();
    forbidden_inbound(
        pool.clone(),
        "POST",
        "/api/v1/inbound/receiving-orders".to_string(),
        &["m2.read"],
        format!(
            r#"{{"receipt_no":"M2-ADV","document_type":"purchase_inbound","warehouse_id":"{warehouse_id}","expected_arrival_at":"2026-08-22T00:00:00Z","lines":[{{"line_no":1,"product_code":"P-M2-001","expected_qty":"10"}}]}}"#
        ),
    )
    .await;
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/receive"),
        &["m2.read"],
        r#"{"actual_qty":"10","shortage_qty":"0","rejected_qty":"0"}"#.to_string(),
    )
    .await;
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/inspect"),
        &["m2.read"],
        r#"{"batch_no":"B-1","accepted_qty":"1","rejected_qty":"0","production_date":"2026-01-01","expiry_date":"2028-01-01","quality_status":"qualified","trace_codes":[]}"#.to_string(),
    )
    .await;
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/sign"),
        &["m2.read"],
        format!(r#"{{"first_signer_id":"{signer_id}","dual_required":false}}"#),
    )
    .await;
    forbidden_inbound(
        pool,
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/reject"),
        &["m2.read"],
        r#"{"reason":"质量拒收"}"#.to_string(),
    )
    .await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn putaway_http_requires_putaway_write_permission(pool: PgPool) {
    let order_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    forbidden_inbound(
        pool.clone(),
        "POST",
        format!("/api/v1/inbound/receiving-orders/{order_id}/putaway"),
        &["m2.write", "m2.read"],
        format!(
            r#"{{"batch_no":"B-1","product_code":"P-M2-001","qty":"1","location_id":"{location_id}","location_code":"A01-01-01-01","quality_status":"qualified"}}"#
        ),
    )
    .await;
    forbidden_inbound(
        pool,
        "PUT",
        "/api/v1/inbound/putaway-strategy-profiles".to_string(),
        &["m2.write", "m2.read"],
        r#"{"profile_code":"default","profile_name":"默认上架"}"#.to_string(),
    )
    .await;
}
