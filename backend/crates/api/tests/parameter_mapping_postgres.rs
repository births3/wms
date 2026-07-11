use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header::AUTHORIZATION, Request, StatusCode},
};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, build_access_claims, encode_access_token, AuthRevocationStore,
        AuthRevocationStoreError, AuthRuntimePolicy, JWT_SECRET_ENV,
    },
    parameter_mapping::{parameter_mapping_router, ParameterMappingAppState},
};
use wms_domain::{ErrorResponse, ExecuteMappingResponse};

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

fn token(owner_id: Uuid, permissions: &[&str]) -> String {
    std::env::set_var(JWT_SECRET_ENV, "test-secret");
    let claims = build_access_claims(
        Uuid::new_v4(),
        owner_id,
        "parameter-mapping-test",
        permissions.iter().map(|value| value.to_string()).collect(),
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    encode_access_token(&claims, "test-secret").expect("token should encode")
}

fn request(token: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/parameter-mapping/execute")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"source_system": "ERP", "raw_payload": {"ITEM_NO": " P-001 "}}).to_string(),
        ))
        .expect("request should build")
}

fn app(pool: PgPool) -> axum::Router {
    parameter_mapping_router(ParameterMappingAppState::with_postgres(pool)).layer(
        auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(AllowAllRevocationStore))),
    )
}

#[sqlx::test(migrations = "../../migrations")]
async fn execute_route_is_reachable_and_appends_owner_scoped_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let response = app(pool.clone())
        .oneshot(request(&token(owner_id, &["mpm.execute"])))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ExecuteMappingResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should be mapping result");
    assert_eq!(payload.unresolved_fields, vec!["ITEM_NO"]);

    let owners: Vec<Uuid> = sqlx::query_scalar(
        "SELECT owner_id FROM audit_event WHERE action = 'execute_mapping' AND resource_id = $1",
    )
    .bind(payload.execution_id.to_string())
    .fetch_all(&pool)
    .await
    .expect("audit query should succeed");
    assert_eq!(owners, vec![owner_id]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn execute_route_requires_mpm_execute_permission(pool: PgPool) {
    let response = app(pool)
        .oneshot(request(&token(Uuid::new_v4(), &[])))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let error: ErrorResponse = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should read"),
    )
    .expect("response should be permission error");
    assert_eq!(error.code, "AUTH-005");
}
