//! US-H8-003 AC9：真实 PostgreSQL 分维度统计证据。

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::H8ErpMessageListResponse;

use crate::auth::AuthContext;

use super::{
    handlers::h8_erp_message_router, pg_repository::PgH8ErpMessageRepository,
    repository::H8ErpMessageRepository, state::H8ErpMessageAppState,
};

fn test_ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "h8-worker-test".into(),
        permissions: vec![
            "h8.erp_connector.read".into(),
            "h8.erp_connector.write".into(),
        ],
        jti: "h8-preflight-test".into(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_cursor_is_stable_with_equal_created_at_in_postgres(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 cursor test")
        .execute(&pool)
        .await
        .unwrap();
    let created_at = Utc::now();
    for value in 1..=3 {
        let id = Uuid::from_u128(value);
        let external_ref = format!("ERP-CURSOR-{value}");
        sqlx::query(
            r#"INSERT INTO h8_erp_messages
               (id, owner_id, connector_code, direction, message_type, schema_version,
                channel, external_ref, idempotency_key, correlation_id, sync_status,
                retry_count, payload_digest, created_at, updated_at)
               VALUES ($1,$2,'SELF-ERP','inbound','asn','1','interface_table',$3,$3,$3,
                       'pending',0,'digest',$4,$4)"#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(external_ref)
        .bind(created_at)
        .execute(&pool)
        .await
        .unwrap();
    }
    let state = H8ErpMessageAppState::with_postgres(pool);

    let mut request = Request::builder()
        .uri("/api/v1/integration/erp-messages?limit=2")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner_id));
    let response = h8_erp_message_router(state.clone())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let first: H8ErpMessageListResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        first
            .data
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![Uuid::from_u128(3), Uuid::from_u128(2)]
    );
    let cursor = first.page.next_cursor.expect("next cursor");

    let mut request = Request::builder()
        .uri(format!(
            "/api/v1/integration/erp-messages?limit=2&cursor={cursor}"
        ))
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(test_ctx(owner_id));
    let response = h8_erp_message_router(state).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let second: H8ErpMessageListResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(second.data.len(), 1);
    assert_eq!(second.data[0].id, Uuid::from_u128(1));
    assert!(second.page.next_cursor.is_none());
}

#[sqlx::test(migrations = "../../migrations")]
async fn replay_marker_can_be_claimed_immediately_in_postgres(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 replay claim test")
        .execute(&pool)
        .await
        .unwrap();
    let connector_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO h8_erp_connectors
           (id, owner_id, connector_code, connector_name, directions, message_types, channel_mode)
           VALUES ($1,$2,'SELF-ERP','Self ERP',ARRAY['inbound'],ARRAY['asn'],'interface_table')"#,
    )
    .bind(connector_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();
    let message_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, connector_id, connector_code, direction, message_type, schema_version,
            channel, external_ref, idempotency_key, correlation_id, sync_status,
            retry_count, payload_digest)
           VALUES ($1,$2,$3,'SELF-ERP','inbound','asn','1','interface_table',
                   'ERP-REPLAY-1','idem-replay-1','corr-replay-1','failed',1,'digest')"#,
    )
    .bind(message_id)
    .bind(owner_id)
    .bind(connector_id)
    .execute(&pool)
    .await
    .unwrap();
    let repository = PgH8ErpMessageRepository::new(pool);
    let now = Utc::now();
    repository
        .replay(owner_id, message_id, "manual fix", "admin", now)
        .await
        .unwrap();

    let replay_requests = repository
        .list(
            owner_id,
            Some("inbound"),
            Some("asn"),
            Some("processing"),
            None,
            Some(connector_id),
            Some("interface_table"),
            true,
            None,
            None,
            Some("idem-replay-1"),
            None,
            Some(now - Duration::minutes(1)),
            Some(now + Duration::minutes(1)),
            None,
            200,
        )
        .await
        .unwrap();
    assert_eq!(replay_requests.len(), 1);
    assert_eq!(replay_requests[0].id, message_id);

    let claimed = repository
        .claim(owner_id, message_id, "worker-1", 60, now)
        .await
        .unwrap();

    assert_eq!(claimed.claimed_by.as_deref(), Some("worker-1"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn stats_filter_real_rows_and_attempt_latency(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 stats test")
        .execute(&pool)
        .await
        .unwrap();

    let selected_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();
    for (id, code, channel, message_type, status, external_ref) in [
        (
            selected_id,
            "SELF-ERP",
            "interface_table",
            "asn",
            "succeeded",
            "ERP-ASN-STATS-1",
        ),
        (
            other_id,
            "OTHER-ERP",
            "rest",
            "outbound_order",
            "dead",
            "ERP-OUT-STATS-1",
        ),
    ] {
        sqlx::query(
            r#"INSERT INTO h8_erp_messages
               (id, owner_id, connector_code, direction, message_type, schema_version,
                channel, external_ref, idempotency_key, correlation_id, sync_status,
                retry_count, payload_digest)
               VALUES ($1,$2,$3,'inbound',$4,'1',$5,$6,$6,$6,$7,1,'digest')"#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(code)
        .bind(message_type)
        .bind(channel)
        .bind(external_ref)
        .bind(status)
        .execute(&pool)
        .await
        .unwrap();
    }

    let now = Utc::now();
    for (message_id, elapsed_ms) in [(selected_id, 100_i64), (other_id, 10_000_i64)] {
        sqlx::query(
            r#"INSERT INTO h8_erp_message_attempts
               (id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
                result, actor)
               VALUES ($1,$2,$3,1,'interface_table',$4,$5,'succeeded','worker')"#,
        )
        .bind(Uuid::new_v4())
        .bind(message_id)
        .bind(owner_id)
        .bind(now)
        .bind(now + Duration::milliseconds(elapsed_ms))
        .execute(&pool)
        .await
        .unwrap();
    }

    let repository = PgH8ErpMessageRepository::new(pool);
    let listed = repository
        .list(
            owner_id,
            None,
            Some("asn"),
            None,
            Some("SELF-ERP"),
            None,
            Some("interface_table"),
            false,
            None,
            Some("ERP-ASN-STATS-1"),
            Some("ERP-ASN-STATS-1"),
            Some("ERP-ASN-STATS-1"),
            Some(now - Duration::minutes(1)),
            Some(now + Duration::minutes(1)),
            None,
            200,
        )
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, selected_id);

    let stats = repository
        .stats(
            owner_id,
            Some("SELF-ERP"),
            Some("interface_table"),
            Some("asn"),
        )
        .await
        .unwrap();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.succeeded, 1);
    assert_eq!(stats.dead, 0);
    assert_eq!(stats.retry_total, 1);
    assert_eq!(stats.p95_latency_ms, 100);
}

#[sqlx::test(migrations = "../../migrations")]
async fn preflight_failure_persists_receive_and_final_failure_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 preflight audit test")
        .execute(&pool)
        .await
        .unwrap();
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));

    for stage in ["receive", "final_failure"] {
        let body = serde_json::json!({
            "stage": stage,
            "result": "preflight_rejected",
            "direction": "inbound",
            "message_type": "asn",
            "schema_version": "999",
            "external_ref": "ERP-BAD-SCHEMA-PG-1",
            "idempotency_key": "idem-bad-schema-pg-1",
            "correlation_id": "corr-bad-schema-pg-1",
            "channel": "interface_table"
        });
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        request.extensions_mut().insert(test_ctx(owner_id));
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_event WHERE owner_id=$1 ORDER BY occurred_at, id",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        actions,
        vec!["h8_exchange_receive", "h8_exchange_final_failure"]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn inbound_lifecycle_persists_failure_retry_and_success_status(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 lifecycle status test")
        .execute(&pool)
        .await
        .unwrap();
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));

    for (stage, result, expected_status) in [
        ("receive", "ok", "processing"),
        (
            "final_failure",
            "Bearer supersecrettoken password=anotherlongsecret",
            "failed",
        ),
        ("receive", "ok", "processing"),
        ("receipt", "ok", "succeeded"),
    ] {
        let body = serde_json::json!({
            "stage": stage,
            "result": result,
            "direction": "inbound",
            "message_type": "asn",
            "schema_version": "1",
            "external_ref": "ERP-RETRY-PG-1",
            "idempotency_key": "idem-retry-pg-1",
            "correlation_id": "corr-retry-pg-1",
            "channel": "interface_table"
        });
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        request.extensions_mut().insert(test_ctx(owner_id));
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "stage={stage}");

        let status: String = sqlx::query_scalar(
            "SELECT sync_status FROM h8_erp_messages WHERE owner_id=$1 AND idempotency_key=$2",
        )
        .bind(owner_id)
        .bind("idem-retry-pg-1")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status, expected_status, "stage={stage}");
        if stage == "final_failure" {
            let summary: String = sqlx::query_scalar(
                "SELECT last_error_summary FROM h8_erp_messages WHERE owner_id=$1 AND idempotency_key=$2",
            )
            .bind(owner_id)
            .bind("idem-retry-pg-1")
            .fetch_one(&pool)
            .await
            .unwrap();
            let audit: String = sqlx::query_scalar(
                "SELECT diff::text FROM audit_event WHERE owner_id=$1 AND action='h8_exchange_final_failure' ORDER BY occurred_at DESC LIMIT 1",
            )
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .unwrap();
            for secret in ["supersecrettoken", "rettoken", "anotherlongsecret"] {
                assert!(!summary.contains(secret), "message leaked {secret}");
                assert!(!audit.contains(secret), "audit leaked {secret}");
            }
        }
    }
}
