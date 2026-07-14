use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy,
        JWT_SECRET_ENV,
    },
    auth_handlers::{auth_router, AuthAppState},
};
use wms_domain::{
    AuthSessionListResponse, AuthUserStatusRequest, LoginRequest, LoginResponse,
    PasswordChangeRequest,
};

#[derive(Default)]
struct MemoryRevocations {
    blacklisted: Mutex<HashSet<String>>,
    changed_at: Mutex<HashMap<Uuid, i64>>,
}

#[axum::async_trait]
impl AuthRevocationStore for MemoryRevocations {
    async fn jti_is_blacklisted(&self, jti: &str) -> Result<bool, AuthRevocationStoreError> {
        Ok(self
            .blacklisted
            .lock()
            .expect("blacklist lock")
            .contains(jti))
    }

    async fn permissions_changed_at(
        &self,
        user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Ok(self
            .changed_at
            .lock()
            .expect("changed lock")
            .get(&user_id)
            .copied())
    }

    async fn blacklist_jti(
        &self,
        jti: &str,
        _ttl_seconds: u64,
    ) -> Result<(), AuthRevocationStoreError> {
        self.blacklisted
            .lock()
            .expect("blacklist lock")
            .insert(jti.to_string());
        Ok(())
    }

    async fn set_permissions_changed_at(
        &self,
        user_id: Uuid,
        changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        self.changed_at
            .lock()
            .expect("changed lock")
            .insert(user_id, changed_at_unix);
        Ok(())
    }
}

async fn seed_user(pool: &PgPool, owner_id: Uuid, user_id: Uuid, username: &str) {
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("会话测试货主")
        .execute(pool)
        .await
        .expect("owner should insert");
    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash) VALUES ($1, $2, $3, $4)")
        .bind(user_id)
        .bind(username)
        .bind(username)
        .bind(bcrypt::hash("CorrectHorse1!", 4).expect("password should hash"))
        .execute(pool)
        .await
        .expect("user should insert");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, true, true)")
        .bind(user_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("owner binding should insert");
}

fn app(pool: PgPool, store: Arc<MemoryRevocations>) -> axum::Router {
    auth_router(AuthAppState::new(pool)).layer(auth_runtime_layer(AuthRuntimePolicy::strict(store)))
}

async fn login(app: &axum::Router, owner_id: Uuid, username: &str) -> LoginResponse {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&LoginRequest {
                        owner_code: format!("OWNER-{owner_id}"),
                        username: username.to_string(),
                        password: "CorrectHorse1!".to_string(),
                    })
                    .expect("login request should encode"),
                ))
                .expect("login request should build"),
        )
        .await
        .expect("login should respond");
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("login body should read"),
    )
    .expect("login response should decode")
}

fn bearer(token: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("request should build")
}

#[sqlx::test(migrations = "../../migrations")]
async fn logout_is_idempotent_blacklists_token_and_writes_one_audit(pool: PgPool) {
    std::env::set_var(JWT_SECRET_ENV, "session-test-secret");
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_user(&pool, owner_id, user_id, "session-user").await;
    let store = Arc::new(MemoryRevocations::default());
    let app = app(pool.clone(), store.clone());
    let login = login(&app, owner_id, "session-user").await;

    let logout = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header("authorization", format!("Bearer {}", login.access_token))
                .body(Body::empty())
                .expect("logout request should build"),
        )
        .await
        .expect("logout should respond");
    assert_eq!(logout.status(), StatusCode::OK);

    let rejected = app
        .clone()
        .oneshot(bearer(&login.access_token, "/api/v1/auth/me"))
        .await
        .expect("revoked request should respond");
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);

    let replay = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header("authorization", format!("Bearer {}", login.access_token))
                .body(Body::empty())
                .expect("logout replay should build"),
        )
        .await
        .expect("logout replay should respond");
    assert_eq!(replay.status(), StatusCode::OK);
    assert!(store
        .blacklisted
        .lock()
        .expect("blacklist lock")
        .contains(&decode_jti(&login.access_token)));
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE owner_id=$1 AND action='auth.logout' AND jti=$2",
    )
    .bind(owner_id)
    .bind(decode_jti(&login.access_token))
    .fetch_one(&pool)
    .await
    .expect("logout audit count should query");
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn session_list_and_single_device_revoke_are_tenant_scoped(pool: PgPool) {
    std::env::set_var(JWT_SECRET_ENV, "session-test-secret");
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_user(&pool, owner_id, user_id, "session-user").await;
    let store = Arc::new(MemoryRevocations::default());
    let app = app(pool, store);
    let first = login(&app, owner_id, "session-user").await;
    let second = login(&app, owner_id, "session-user").await;

    let sessions = app
        .clone()
        .oneshot(bearer(&first.access_token, "/api/v1/auth/sessions"))
        .await
        .expect("session list should respond");
    assert_eq!(sessions.status(), StatusCode::OK);
    let sessions: AuthSessionListResponse = serde_json::from_slice(
        &to_bytes(sessions.into_body(), usize::MAX)
            .await
            .expect("session list body should read"),
    )
    .expect("session list should decode");
    assert_eq!(sessions.data.len(), 2);
    let second_session = sessions
        .data
        .iter()
        .find(|session| session.session_id == decode_jti(&second.access_token))
        .expect("second session should be listed");

    let revoke = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/auth/sessions/{}/revoke",
                    second_session.session_id
                ))
                .header("authorization", format!("Bearer {}", first.access_token))
                .body(Body::empty())
                .expect("session revoke should build"),
        )
        .await
        .expect("session revoke should respond");
    assert_eq!(revoke.status(), StatusCode::OK);
    assert_eq!(
        app.clone()
            .oneshot(bearer(&second.access_token, "/api/v1/auth/me"))
            .await
            .expect("revoked device request should respond")
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.oneshot(bearer(&first.access_token, "/api/v1/auth/me"))
            .await
            .expect("current device request should respond")
            .status(),
        StatusCode::OK
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn only_session_admin_can_kick_another_user(pool: PgPool) {
    std::env::set_var(JWT_SECRET_ENV, "session-test-secret");
    let owner_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    seed_user(&pool, owner_id, admin_id, "session-admin").await;
    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash) VALUES ($1, 'target-user', 'target-user', $2)")
        .bind(user_id)
        .bind(bcrypt::hash("CorrectHorse1!", 4).expect("password should hash"))
        .execute(&pool)
        .await
        .expect("target user should insert");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, true, true)")
        .bind(user_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("target binding should insert");
    let store = Arc::new(MemoryRevocations::default());
    let app = app(pool.clone(), store);
    let target = login(&app, owner_id, "target-user").await;

    let forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/auth/users/{user_id}/kick"))
                .header("authorization", format!("Bearer {}", target.access_token))
                .body(Body::empty())
                .expect("forbidden kick should build"),
        )
        .await
        .expect("forbidden kick should respond");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let allowed_token = issue_token(
        admin_id,
        owner_id,
        "session-admin",
        vec!["h1.sessions.manage"],
    );
    let kick = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/auth/users/{user_id}/kick"))
                .header("authorization", format!("Bearer {allowed_token}"))
                .body(Body::empty())
                .expect("kick should build"),
        )
        .await
        .expect("kick should respond");
    assert_eq!(kick.status(), StatusCode::OK);
    assert_eq!(
        app.oneshot(bearer(&target.access_token, "/api/v1/auth/me"))
            .await
            .expect("kicked request should respond")
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_event WHERE owner_id=$1 AND action='auth.token_revoked' AND resource_id=$2")
        .bind(owner_id)
        .bind(user_id.to_string())
        .fetch_one(&pool)
        .await
        .expect("kick audit count should query");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn password_change_and_disable_revoke_all_user_tokens(pool: PgPool) {
    std::env::set_var(JWT_SECRET_ENV, "session-test-secret");
    let owner_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    seed_user(&pool, owner_id, target_id, "password-target").await;
    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash) VALUES ($1, 'session-admin', 'session-admin', $2)")
        .bind(admin_id)
        .bind(bcrypt::hash("CorrectHorse1!", 4).expect("password should hash"))
        .execute(&pool)
        .await
        .expect("admin should insert");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, true, true)")
        .bind(admin_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("admin binding should insert");
    let store = Arc::new(MemoryRevocations::default());
    let app = app(pool.clone(), store);
    let target = login(&app, owner_id, "password-target").await;

    let password_change = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/auth/me/password")
                .header("authorization", format!("Bearer {}", target.access_token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&PasswordChangeRequest {
                        current_password: "CorrectHorse1!".to_string(),
                        new_password: "NewPassword1".to_string(),
                    })
                    .expect("password request should encode"),
                ))
                .expect("password request should build"),
        )
        .await
        .expect("password change should respond");
    assert_eq!(password_change.status(), StatusCode::OK);
    assert_eq!(
        app.clone()
            .oneshot(bearer(&target.access_token, "/api/v1/auth/me"))
            .await
            .expect("password-revoked request should respond")
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let admin_token = issue_token(
        admin_id,
        owner_id,
        "session-admin",
        vec!["h1.sessions.manage"],
    );
    let disable = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/auth/users/{target_id}/status"))
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&AuthUserStatusRequest {
                        status: "disabled".to_string(),
                    })
                    .expect("status request should encode"),
                ))
                .expect("status request should build"),
        )
        .await
        .expect("disable should respond");
    assert_eq!(disable.status(), StatusCode::OK);
    let fresh_target_token = issue_token(target_id, owner_id, "password-target", vec![]);
    assert_eq!(
        app.oneshot(bearer(&fresh_target_token, "/api/v1/auth/me"))
            .await
            .expect("disabled request should respond")
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let password_audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE owner_id=$1 AND action='auth.password.changed' AND resource_id=$2",
    )
    .bind(owner_id)
    .bind(target_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("password audit count should query");
    assert_eq!(password_audit, 1);
}

fn issue_token(user_id: Uuid, owner_id: Uuid, name: &str, permissions: Vec<&str>) -> String {
    let claims = wms_api::auth::build_access_claims(
        user_id,
        owner_id,
        name,
        permissions.into_iter().map(str::to_string).collect(),
        Uuid::new_v4().to_string(),
        Utc::now(),
    );
    wms_api::auth::encode_access_token(&claims, "session-test-secret").expect("token should encode")
}

fn decode_jti(token: &str) -> String {
    let claims = jsonwebtoken::decode::<wms_api::auth::Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret("session-test-secret".as_bytes()),
        &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256),
    )
    .expect("test token should decode")
    .claims;
    claims.jti
}
