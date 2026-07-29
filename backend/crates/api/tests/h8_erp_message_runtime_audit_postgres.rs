use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    h8_erp_messages::{h8_erp_message_router, H8ErpMessageAppState},
};

struct RuntimeFixture {
    auth: AuthContext,
    connector_id: Uuid,
    claim_message_id: Uuid,
    purge_message_id: Uuid,
}

async fn seed_runtime_fixture(pool: &PgPool) -> RuntimeFixture {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let claim_message_id = Uuid::new_v4();
    let purge_message_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$2)")
        .bind(owner_id)
        .bind(format!("H8-RUNTIME-AUDIT-{owner_id}"))
        .execute(pool)
        .await
        .expect("owner should insert");
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status)
        VALUES ($1,$2,'H8 Worker 审计测试用户','not-used-in-test','active')
        "#,
    )
    .bind(user_id)
    .bind(format!("h8-runtime-audit-{user_id}"))
    .execute(pool)
    .await
    .expect("user should insert");
    sqlx::query(
        r#"
        INSERT INTO h8_erp_connectors (
            id, owner_id, connector_code, connector_name, directions, message_types,
            channel_mode, api_base_url, bearer_secret_alias, status, config_version,
            first_activated_at, last_tested_version, last_tested_at, last_tested_succeeded
        )
        VALUES (
            $1,$2,'RUNTIME-AUDIT','Worker 审计连接器',ARRAY['outbound'],
            ARRAY['putaway_complete'],'rest','https://erp.example.com',
            'vault://h8/runtime-audit/token','active',1,now(),1,now(),true
        )
        "#,
    )
    .bind(connector_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("connector should insert");
    let now = Utc::now();
    for (message_id, external_ref, idempotency_key, correlation_id, status, updated_at) in [
        (
            claim_message_id,
            "ERP-CLAIM-AUDIT",
            "h8-claim-audit-source",
            "h8-claim-audit-correlation",
            "pending",
            now,
        ),
        (
            purge_message_id,
            "ERP-PURGE-AUDIT",
            "h8-purge-audit-source",
            "h8-purge-audit-correlation",
            "succeeded",
            now - Duration::days(8),
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO h8_erp_messages (
                id, owner_id, connector_id, connector_code, config_version,
                direction, message_type, schema_version, channel, external_ref,
                idempotency_key, correlation_id, sync_status, payload_digest,
                created_at, updated_at
            )
            VALUES (
                $1,$2,$3,'RUNTIME-AUDIT',1,'outbound','putaway_complete','1',
                'rest',$4,$5,$6,$7,'sha256:runtime-audit',now(),$8
            )
            "#,
        )
        .bind(message_id)
        .bind(owner_id)
        .bind(connector_id)
        .bind(external_ref)
        .bind(idempotency_key)
        .bind(correlation_id)
        .bind(status)
        .bind(updated_at)
        .execute(pool)
        .await
        .expect("message should insert");
    }
    sqlx::query(
        "INSERT INTO h8_erp_message_retention_policy (owner_id, retention_days) VALUES ($1,7)",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("message retention policy should insert");
    RuntimeFixture {
        auth: AuthContext {
            user_id,
            owner_id,
            actor_name: "H8 Worker 审计测试用户".to_string(),
            permissions: vec!["h8.erp_connector.write".to_string()],
            jti: format!("h8-runtime-audit-{user_id}"),
            warehouse_scope: None,
        },
        connector_id,
        claim_message_id,
        purge_message_id,
    }
}

fn json_post(auth: &AuthContext, path: &str, body: serde_json::Value) -> Request<Body> {
    let mut request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build");
    request.extensions_mut().insert(auth.clone());
    request
}

#[sqlx::test(migrations = "../../migrations")]
async fn update_h8_payload_retention_policy_purge_h8_erp_messages_set_h8_worker_claim_control_record_h8_worker_heartbeat_and_claim_h8_erp_message_write_audit(
    pool: PgPool,
) {
    let fixture = seed_runtime_fixture(&pool).await;
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));
    let requests = [
        json_post(
            &fixture.auth,
            "/api/v1/integration/erp-messages/payload-retention",
            serde_json::json!({
                "connector_id": fixture.connector_id,
                "enabled": false,
                "retention_days": 7,
                "confirmed": true
            }),
        ),
        json_post(
            &fixture.auth,
            "/api/v1/integration/erp-messages/worker-runtime/control",
            serde_json::json!({
                "connector_id": fixture.connector_id,
                "direction": "outbound",
                "paused": false,
                "reason": "治理证据：恢复认领",
                "paused_until": null,
                "confirmed": true
            }),
        ),
        json_post(
            &fixture.auth,
            "/api/v1/integration/erp-messages/worker-runtime/heartbeat",
            serde_json::json!({
                "worker_id": "worker-audit-1",
                "worker_version": "1.0.0",
                "connector_id": fixture.connector_id,
                "directions": ["outbound"],
                "current_claims": 0,
                "heartbeat_ttl_seconds": 60
            }),
        ),
        json_post(
            &fixture.auth,
            &format!(
                "/api/v1/integration/erp-messages/{}/claim",
                fixture.claim_message_id
            ),
            serde_json::json!({
                "worker_id": "worker-audit-1",
                "lease_seconds": 60
            }),
        ),
        json_post(
            &fixture.auth,
            "/api/v1/integration/erp-messages/purge",
            serde_json::json!({"confirmed": true}),
        ),
    ];
    for request in requests {
        let path = request.uri().path().to_string();
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("runtime mutation should respond");
        assert_eq!(response.status(), StatusCode::OK, "failed path: {path}");
    }

    for action in [
        "h8_payload_retention_update",
        "h8_worker_claim_resume",
        "h8_worker_heartbeat",
        "h8_message_claim",
        "h8_message_purge",
    ] {
        let audit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_event WHERE owner_id=$1 AND module='H8' AND action=$2",
        )
        .bind(fixture.auth.owner_id)
        .bind(action)
        .fetch_one(&pool)
        .await
        .expect("runtime audit count should load");
        assert_eq!(audit_count, 1, "audit should be unique for {action}");
    }
    let purged: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM h8_erp_messages WHERE owner_id=$1 AND id=$2")
            .bind(fixture.auth.owner_id)
            .bind(fixture.purge_message_id)
            .fetch_one(&pool)
            .await
            .expect("purged message count should load");
    assert_eq!(purged, 0);
}
