use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    drug_inspection_handlers::{drug_inspection_router, DrugInspectionAppState},
};

fn context(owner_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "di-platform-test".to_string(),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable"),
    )
    .expect("response should be JSON")
}

fn request(
    method: &str,
    path: &str,
    ctx: AuthContext,
    idempotency_key: Option<&str>,
    payload: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_TYPE, "application/json");
    if let Some(idempotency_key) = idempotency_key {
        builder = builder.header("Idempotency-Key", idempotency_key);
    }
    let mut request = builder
        .body(Body::from(payload.to_string()))
        .expect("request should build");
    request.extensions_mut().insert(ctx);
    request
}

async fn seed_owner(pool: &PgPool, owner_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("DI-{}", owner_id.simple()))
    .bind("药检平台测试货主")
    .execute(pool)
    .await
    .expect("test owner should insert");
}

fn api_key_payload(code: &str) -> Value {
    json!({
        "platform_code": code,
        "platform_name": format!("药检平台 {code}"),
        "api_url": "https://inspection.example.test/api",
        "auth_method": "api_key",
        "api_key_alias": format!("vault://wms/di/{code}/api-key"),
        "username": null,
        "password_alias": null,
        "timeout_seconds": 30,
        "status": "testing"
    })
}

#[sqlx::test(migrations = "../../migrations")]
async fn platform_config_is_owner_scoped_idempotent_audited_and_redacted(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    seed_owner(&pool, owner_a).await;
    seed_owner(&pool, owner_b).await;
    let app = drug_inspection_router(DrugInspectionAppState::with_postgres(pool.clone()));
    let write_permissions = ["m-di.platform.read", "m-di.platform.write"];

    let first_payload = api_key_payload("platform-a");
    let first = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/drug-inspection/platforms",
            context(owner_a, &write_permissions),
            Some("di-platform-a-1"),
            first_payload.clone(),
        ))
        .await
        .expect("first platform request should complete");
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = response_json(first).await;
    let first_id = first_body["id"].as_str().expect("platform id should exist");
    assert_eq!(first_body["api_key_configured"], true);
    assert_eq!(first_body.get("api_key_alias"), None);
    assert!(!first_body.to_string().contains("vault://"));

    let replay = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/drug-inspection/platforms",
            context(owner_a, &write_permissions),
            Some("di-platform-a-1"),
            first_payload.clone(),
        ))
        .await
        .expect("same-key replay should complete");
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(response_json(replay).await["id"], first_id);

    let mut changed_payload = first_payload.clone();
    changed_payload["timeout_seconds"] = json!(60);
    let conflict = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/drug-inspection/platforms",
            context(owner_a, &write_permissions),
            Some("di-platform-a-1"),
            changed_payload,
        ))
        .await
        .expect("idempotency conflict request should complete");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(conflict).await["code"],
        "M_DI_PLATFORM_IDEMPOTENCY_CONFLICT"
    );

    let account_payload = json!({
        "platform_code": "platform-b",
        "platform_name": "药检平台 B",
        "api_url": "https://inspection-b.example.test/query",
        "auth_method": "username_password",
        "api_key_alias": null,
        "username": "inspection-user",
        "password_alias": "vault://wms/di/platform-b/password",
        "timeout_seconds": 120,
        "status": "testing"
    });
    let account = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/drug-inspection/platforms",
            context(owner_a, &write_permissions),
            Some("di-platform-b-1"),
            account_payload,
        ))
        .await
        .expect("account/password platform should complete");
    assert_eq!(account.status(), StatusCode::OK);
    let account_body = response_json(account).await;
    let account_id = account_body["id"].as_str().expect("account platform id");
    assert_eq!(account_body["username"], "inspection-user");
    assert_eq!(account_body["password_configured"], true);
    assert_eq!(account_body.get("password_alias"), None);
    assert!(!account_body.to_string().contains("vault://"));

    let connected = app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!("/api/v1/drug-inspection/platforms/{first_id}/status"),
            context(owner_a, &write_permissions),
            Some("di-platform-a-connected"),
            json!({"status": "connected"}),
        ))
        .await
        .expect("status change should complete");
    assert_eq!(connected.status(), StatusCode::OK);
    assert_eq!(response_json(connected).await["status"], "connected");

    let disabled = app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!("/api/v1/drug-inspection/platforms/{first_id}/status"),
            context(owner_a, &write_permissions),
            Some("di-platform-a-disabled"),
            json!({"status": "disabled"}),
        ))
        .await
        .expect("disable request should complete");
    assert_eq!(disabled.status(), StatusCode::OK);
    assert_eq!(response_json(disabled).await["status"], "disabled");

    let owner_a_list = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/v1/drug-inspection/platforms",
            context(owner_a, &["m-di.platform.read"]),
            None,
            json!({}),
        ))
        .await
        .expect("owner A list should complete");
    assert_eq!(owner_a_list.status(), StatusCode::OK);
    assert_eq!(response_json(owner_a_list).await["page"]["count"], 2);

    let owner_b_list = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/v1/drug-inspection/platforms",
            context(owner_b, &["m-di.platform.read"]),
            None,
            json!({}),
        ))
        .await
        .expect("owner B list should complete");
    assert_eq!(owner_b_list.status(), StatusCode::OK);
    assert_eq!(response_json(owner_b_list).await["page"]["count"], 0);

    let cross_owner_status = app
        .clone()
        .oneshot(request(
            "PATCH",
            &format!("/api/v1/drug-inspection/platforms/{account_id}/status"),
            context(owner_b, &write_permissions),
            Some("di-cross-owner-status"),
            json!({"status": "disabled"}),
        ))
        .await
        .expect("cross-owner status request should complete");
    assert_eq!(cross_owner_status.status(), StatusCode::NOT_FOUND);

    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM drug_inspection_platforms WHERE owner_id = $1),
            (SELECT COUNT(*) FROM drug_inspection_platforms WHERE owner_id = $2),
            (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND resource_type = 'drug_inspection_platform'),
            (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'DI')
        "#,
    )
    .bind(owner_a)
    .bind(owner_b)
    .fetch_one(&pool)
    .await
    .expect("DI database evidence should query");
    assert_eq!(counts, (2, 0, 4, 4));

    let audit_diff: String = sqlx::query_scalar(
        "SELECT diff::TEXT FROM audit_event WHERE owner_id = $1 AND module = 'DI' ORDER BY id DESC LIMIT 1",
    )
    .bind(owner_a)
    .fetch_one(&pool)
    .await
    .expect("DI audit diff should query");
    assert!(!audit_diff.contains("vault://"));

    let stored_refs: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT api_key_alias, password_alias FROM drug_inspection_platforms WHERE owner_id = $1 ORDER BY platform_code",
    )
    .bind(owner_a)
    .fetch_one(&pool)
    .await
    .expect("credential references should query");
    assert_eq!(
        stored_refs.0.as_deref(),
        Some("vault://wms/di/platform-a/api-key")
    );
    assert_eq!(stored_refs.1, None);
}

#[sqlx::test(migrations = "../../migrations")]
async fn platform_config_rejects_invalid_input_and_missing_idempotency(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    seed_owner(&pool, owner_id).await;
    let app = drug_inspection_router(DrugInspectionAppState::with_postgres(pool));

    let missing_key = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/drug-inspection/platforms",
            context(owner_id, &["m-di.platform.write"]),
            None,
            api_key_payload("missing-key"),
        ))
        .await
        .expect("missing key request should complete");
    assert_eq!(missing_key.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(missing_key).await["code"],
        "M_DI_PLATFORM_IDEMPOTENCY_REQUIRED"
    );

    let invalid_url = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/drug-inspection/platforms",
            context(owner_id, &["m-di.platform.write"]),
            Some("di-invalid-url"),
            json!({
                "platform_code": "invalid-url",
                "platform_name": "非法地址",
                "api_url": "ftp://inspection.example.test",
                "auth_method": "api_key",
                "api_key_alias": "vault://wms/di/invalid/api-key",
                "username": null,
                "password_alias": null,
                "timeout_seconds": 30,
                "status": "testing"
            }),
        ))
        .await
        .expect("invalid URL request should complete");
    assert_eq!(invalid_url.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response_json(invalid_url).await["code"],
        "M_DI_PLATFORM_API_URL_INVALID"
    );

    let inline_secret = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/drug-inspection/platforms",
            context(owner_id, &["m-di.platform.write"]),
            Some("di-inline-secret"),
            {
                let mut payload = api_key_payload("inline-secret");
                payload["api_key_alias"] = json!("actual-api-key");
                payload
            },
        ))
        .await
        .expect("inline secret request should complete");
    assert_eq!(inline_secret.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response_json(inline_secret).await["code"],
        "M_DI_PLATFORM_CREDENTIAL_REF_INVALID"
    );

    let invalid_timeout = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/drug-inspection/platforms",
            context(owner_id, &["m-di.platform.write"]),
            Some("di-invalid-timeout"),
            {
                let mut payload = api_key_payload("invalid-timeout");
                payload["timeout_seconds"] = json!(301);
                payload
            },
        ))
        .await
        .expect("invalid timeout request should complete");
    assert_eq!(invalid_timeout.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response_json(invalid_timeout).await["code"],
        "M_DI_PLATFORM_TIMEOUT_INVALID"
    );
}
