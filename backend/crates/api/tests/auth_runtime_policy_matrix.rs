use std::sync::{Arc, Mutex};

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, Method, Request},
};
use chrono::Utc;
use uuid::Uuid;
use wms_api::auth::{
    build_access_claims, classify_auth_operation, encode_access_token, AuthContext, AuthError,
    AuthOperationRisk, AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy,
    JWT_SECRET_ENV,
};

#[derive(Default)]
struct MatrixStore {
    state: Mutex<MatrixStoreState>,
}

#[derive(Default)]
struct MatrixStoreState {
    unavailable: bool,
    revoked: bool,
}

impl MatrixStore {
    fn set_unavailable(&self, unavailable: bool) {
        self.state.lock().expect("matrix store lock").unavailable = unavailable;
    }

    fn set_revoked(&self, revoked: bool) {
        self.state.lock().expect("matrix store lock").revoked = revoked;
    }
}

#[axum::async_trait]
impl AuthRevocationStore for MatrixStore {
    async fn jti_is_blacklisted(&self, _jti: &str) -> Result<bool, AuthRevocationStoreError> {
        let state = self.state.lock().expect("matrix store lock");
        if state.unavailable {
            return Err(AuthRevocationStoreError::Unavailable(
                "revocation store unavailable".to_string(),
            ));
        }
        Ok(state.revoked)
    }

    async fn permissions_changed_at(
        &self,
        _user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        let state = self.state.lock().expect("matrix store lock");
        if state.unavailable {
            return Err(AuthRevocationStoreError::Unavailable(
                "revocation store unavailable".to_string(),
            ));
        }
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

fn claims() -> wms_api::auth::Claims {
    build_access_claims(
        Uuid::new_v4(),
        Uuid::new_v4(),
        "matrix-user",
        vec!["m4.write".to_string()],
        "matrix-jti",
        Utc::now(),
    )
}

#[tokio::test]
async fn risk_matrix_covers_healthy_revoked_unavailable_and_recovery() {
    let store = Arc::new(MatrixStore::default());
    let policy = AuthRuntimePolicy::new(store.clone());
    let claims = claims();

    for risk in [
        AuthOperationRisk::Read,
        AuthOperationRisk::OrdinaryWrite,
        AuthOperationRisk::HighRiskWrite,
    ] {
        assert_eq!(policy.validate_claims_for(&claims, risk).await, Ok(()));
    }

    store.set_revoked(true);
    for risk in [
        AuthOperationRisk::Read,
        AuthOperationRisk::OrdinaryWrite,
        AuthOperationRisk::HighRiskWrite,
    ] {
        assert_eq!(
            policy.validate_claims_for(&claims, risk).await,
            Err(AuthError::TokenRevoked)
        );
    }

    store.set_revoked(false);
    store.set_unavailable(true);
    assert_eq!(
        policy
            .validate_claims_for(&claims, AuthOperationRisk::Read)
            .await,
        Ok(())
    );
    assert_eq!(
        policy
            .validate_claims_for(&claims, AuthOperationRisk::OrdinaryWrite)
            .await,
        Ok(())
    );
    assert_eq!(
        policy
            .validate_claims_for(&claims, AuthOperationRisk::HighRiskWrite)
            .await,
        Err(AuthError::RevocationStoreUnavailable)
    );

    store.set_unavailable(false);
    assert_eq!(
        policy
            .validate_claims_for(&claims, AuthOperationRisk::HighRiskWrite)
            .await,
        Ok(())
    );
}

#[test]
fn request_classifier_keeps_ordinary_writes_available() {
    assert_eq!(
        classify_auth_operation(&Method::GET, "/api/v1/auth/roles"),
        AuthOperationRisk::Read
    );
    assert_eq!(
        classify_auth_operation(&Method::POST, "/api/v1/auth/roles"),
        AuthOperationRisk::HighRiskWrite
    );
    assert_eq!(
        classify_auth_operation(&Method::POST, "/api/v1/m4/orders"),
        AuthOperationRisk::OrdinaryWrite
    );
    assert_eq!(
        classify_auth_operation(
            &Method::POST,
            "/api/v1/print-orchestration/suite-instances/1/category-pdfs/prepare"
        ),
        AuthOperationRisk::HighRiskWrite
    );
    assert_eq!(
        classify_auth_operation(
            &Method::POST,
            "/api/v1/print-orchestration/print-suites/versions"
        ),
        AuthOperationRisk::OrdinaryWrite
    );
    assert_eq!(
        classify_auth_operation(&Method::POST, "/api/v1/inventory/counts/1/approve"),
        AuthOperationRisk::HighRiskWrite
    );
    assert_eq!(
        classify_auth_operation(&Method::POST, "/api/v1/inventory/counts"),
        AuthOperationRisk::OrdinaryWrite
    );
    assert_eq!(
        classify_auth_operation(
            &Method::PUT,
            "/api/v1/inventory/status-transitions/available/quarantine"
        ),
        AuthOperationRisk::HighRiskWrite
    );
    assert_eq!(
        classify_auth_operation(&Method::POST, "/api/v1/inventory/alerts/1/handle"),
        AuthOperationRisk::HighRiskWrite
    );
}

#[tokio::test]
async fn extractor_applies_fail_closed_only_to_high_risk_writes() {
    std::env::set_var(JWT_SECRET_ENV, "matrix-secret");
    let claims = claims();
    let token = encode_access_token(&claims, "matrix-secret").expect("token should encode");
    let store = Arc::new(MatrixStore::default());
    store.set_unavailable(true);
    let policy = AuthRuntimePolicy::new(store);

    let mut high_risk_parts = Request::builder()
        .method("POST")
        .uri("/api/v1/inventory/counts/1/approve")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(())
        .expect("request should build")
        .into_parts()
        .0;
    high_risk_parts.extensions.insert(policy.clone());
    assert_eq!(
        AuthContext::from_request_parts(&mut high_risk_parts, &())
            .await
            .expect_err("high-risk write must fail closed"),
        AuthError::RevocationStoreUnavailable
    );

    let mut ordinary_parts = Request::builder()
        .method("POST")
        .uri("/api/v1/m4/orders")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(())
        .expect("request should build")
        .into_parts()
        .0;
    ordinary_parts.extensions.insert(policy);
    assert_eq!(
        AuthContext::from_request_parts(&mut ordinary_parts, &())
            .await
            .expect("ordinary write should remain available")
            .user_id,
        claims.sub
    );
}
