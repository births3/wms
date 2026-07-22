//! US-H8-003 AC9：真实 PostgreSQL 分维度统计证据。

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

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
        permissions: vec!["h8.erp_connector.write".into()],
        jti: "h8-preflight-test".into(),
        warehouse_scope: None,
    }
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
            Some("interface_table"),
            None,
            Some("ERP-ASN-STATS-1"),
            Some("ERP-ASN-STATS-1"),
            Some("ERP-ASN-STATS-1"),
            Some(now - Duration::minutes(1)),
            Some(now + Duration::minutes(1)),
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
