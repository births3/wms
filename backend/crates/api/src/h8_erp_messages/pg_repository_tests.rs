//! US-H8-003 AC9：真实 PostgreSQL 分维度统计证据。

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::{
    standard_retry_delay_millis, H8ErpMessage, H8ErpMessageListResponse, H8ErpMessageStats,
};

use crate::{
    h8_erp_connectors::{h8_erp_connector_router, H8ErpConnectorAppState},
    operation_context::OperationContext as AuthContext,
};

use super::{
    handlers::h8_erp_message_router, pg_repository::PgH8ErpMessageRepository,
    repository::H8ErpMessageRepository, state::H8ErpMessageAppState,
};

type AttemptRow = (
    i32,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    String,
    Option<String>,
    String,
);

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

fn warehouse_reader_ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "warehouse-reader".into(),
        permissions: vec!["h8.erp_connector.read".into()],
        jti: "h8-warehouse-reader-test".into(),
        warehouse_scope: None,
    }
}

fn warehouse_worker_ctx(owner_id: Uuid, user_id: Uuid) -> AuthContext {
    AuthContext {
        permissions: vec![
            "h8.erp_connector.read".into(),
            "h8.erp_connector.write".into(),
        ],
        ..warehouse_reader_ctx(owner_id, user_id)
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn jwt_warehouse_scopes_limit_message_list_detail_and_stats(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let allowed_warehouse = Uuid::new_v4();
    let second_allowed_warehouse = Uuid::new_v4();
    let denied_warehouse = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 warehouse scope test")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1,$2,'H8 warehouse reader','test-hash','active')")
        .bind(user_id)
        .bind(format!("h8-reader-{user_id}"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1,$2,TRUE,TRUE)")
        .bind(user_id)
        .bind(owner_id)
        .execute(&pool)
        .await
        .unwrap();
    for (warehouse_id, code) in [
        (allowed_warehouse, "H8-SCOPE-A"),
        (second_allowed_warehouse, "H8-SCOPE-B"),
        (denied_warehouse, "H8-SCOPE-C"),
    ] {
        sqlx::query("INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1,$2,$3,$3,'normal','active')")
            .bind(warehouse_id)
            .bind(owner_id)
            .bind(code)
            .execute(&pool)
            .await
            .unwrap();
    }
    for warehouse_id in [allowed_warehouse, second_allowed_warehouse] {
        sqlx::query("INSERT INTO auth_user_warehouse_scopes (user_id, owner_id, warehouse_id) VALUES ($1,$2,$3)")
            .bind(user_id)
            .bind(owner_id)
            .bind(warehouse_id)
            .execute(&pool)
            .await
            .unwrap();
    }

    let allowed_connector_id = Uuid::new_v4();
    let denied_connector_id = Uuid::new_v4();
    for (connector_id, code, warehouse_id) in [
        (allowed_connector_id, "H8-ROUTE-ALLOWED", allowed_warehouse),
        (denied_connector_id, "H8-ROUTE-DENIED", denied_warehouse),
    ] {
        sqlx::query(
            r#"INSERT INTO h8_erp_connectors
               (id, owner_id, connector_code, connector_name, warehouse_ids, directions,
                message_types, channel_mode, status, config_version)
               VALUES ($1,$2,$3,$3,ARRAY[$4],ARRAY['inbound'],ARRAY['asn'],
                       'interface_table','active',1)"#,
        )
        .bind(connector_id)
        .bind(owner_id)
        .bind(code)
        .bind(warehouse_id)
        .execute(&pool)
        .await
        .unwrap();
    }

    let route_app = h8_erp_connector_router(H8ErpConnectorAppState::with_postgres(pool.clone()));
    for (warehouse_id, expected_status) in [
        (allowed_warehouse, StatusCode::OK),
        (denied_warehouse, StatusCode::FORBIDDEN),
    ] {
        let mut request = Request::builder()
            .uri(format!(
                "/api/v1/config/erp-connectors/route-resolve?direction=inbound&message_type=asn&warehouse_id={warehouse_id}"
            ))
            .body(Body::empty())
            .unwrap();
        request
            .extensions_mut()
            .insert(warehouse_worker_ctx(owner_id, user_id));
        let response = route_app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected_status);
    }
    let mut request = Request::builder()
        .uri("/api/v1/config/erp-connectors/route-resolve?direction=inbound&message_type=asn")
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(warehouse_worker_ctx(owner_id, user_id));
    let response = route_app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let lifecycle_app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));
    for (warehouse_id, connector_id, suffix, expected_status) in [
        (
            allowed_warehouse,
            allowed_connector_id,
            "allowed",
            StatusCode::OK,
        ),
        (
            denied_warehouse,
            denied_connector_id,
            "denied",
            StatusCode::FORBIDDEN,
        ),
    ] {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "stage": "receive",
                    "result": "ok",
                    "direction": "inbound",
                    "message_type": "asn",
                    "schema_version": "1",
                    "channel": "interface_table",
                    "external_ref": format!("ERP-JWT-{suffix}"),
                    "idempotency_key": format!("idem-jwt-{suffix}"),
                    "correlation_id": format!("corr-jwt-{suffix}"),
                    "connector_id": connector_id,
                    "connector_code": format!("H8-ROUTE-{}", suffix.to_ascii_uppercase()),
                    "config_version": 1,
                    "warehouse_id": warehouse_id
                })
                .to_string(),
            ))
            .unwrap();
        request
            .extensions_mut()
            .insert(warehouse_worker_ctx(owner_id, user_id));
        let response = lifecycle_app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected_status);
        if expected_status == StatusCode::OK {
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let message: H8ErpMessage = serde_json::from_slice(&body).unwrap();
            assert_eq!(message.warehouse_id, Some(allowed_warehouse));
        }
    }

    let allowed_id = Uuid::new_v4();
    let second_allowed_id = Uuid::new_v4();
    let denied_id = Uuid::new_v4();
    let owner_level_id = Uuid::new_v4();
    for (id, warehouse_id, external_ref) in [
        (allowed_id, allowed_warehouse, "ERP-SCOPE-ALLOWED"),
        (
            second_allowed_id,
            second_allowed_warehouse,
            "ERP-SCOPE-ALLOWED-2",
        ),
        (denied_id, denied_warehouse, "ERP-SCOPE-DENIED"),
    ] {
        sqlx::query(
            r#"INSERT INTO h8_erp_messages
               (id, owner_id, warehouse_id, connector_code, direction, message_type,
                schema_version, channel, external_ref, idempotency_key, correlation_id,
                sync_status, retry_count, payload_digest)
               VALUES ($1,$2,$3,'SELF-ERP','inbound','asn','1','interface_table',$4,$4,$4,
                       'failed',0,'digest')"#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(warehouse_id)
        .bind(external_ref)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, connector_code, direction, message_type,
            schema_version, channel, external_ref, idempotency_key, correlation_id,
            sync_status, retry_count, payload_digest)
           VALUES ($1,$2,'SELF-ERP','inbound','product_master','1','rest',
                   'ERP-SCOPE-OWNER','ERP-SCOPE-OWNER','ERP-SCOPE-OWNER',
                   'failed',0,'digest')"#,
    )
    .bind(owner_level_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();

    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool));
    let mut request = Request::builder()
        .uri(format!("/api/v1/integration/erp-messages/{owner_level_id}"))
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(warehouse_worker_ctx(owner_id, user_id));
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let ctx = warehouse_reader_ctx(owner_id, user_id);

    let mut request = Request::builder()
        .uri("/api/v1/integration/erp-messages?created_from=1970-01-01T00:00:00Z")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ctx.clone());
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let listed: H8ErpMessageListResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(listed.data.len(), 3);
    assert!(listed.data.iter().any(|message| message.id == allowed_id));
    assert!(listed
        .data
        .iter()
        .any(|message| message.id == second_allowed_id));

    let mut request = Request::builder()
        .uri(format!(
            "/api/v1/integration/erp-messages?warehouse_id={denied_warehouse}"
        ))
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ctx.clone());
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let mut request = Request::builder()
        .uri(format!("/api/v1/integration/erp-messages/{denied_id}"))
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ctx.clone());
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let mut request = Request::builder()
        .uri("/api/v1/integration/erp-messages/stats")
        .body(Body::empty())
        .unwrap();
    request.extensions_mut().insert(ctx);
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let stats: H8ErpMessageStats = serde_json::from_slice(&body).unwrap();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.failed, 2);
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
            retry_count, next_retry_at, payload_digest)
           VALUES ($1,$2,$3,'SELF-ERP','inbound','asn','1','interface_table',
                   'ERP-REPLAY-1','idem-replay-1','corr-replay-1','failed',1,
                   now() + interval '1 hour','digest')"#,
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
    assert!(replay_requests[0].next_retry_at.is_none());

    let claimed = repository
        .claim(owner_id, message_id, "worker-1", 60, now)
        .await
        .unwrap();

    assert_eq!(claimed.claimed_by.as_deref(), Some("worker-1"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn archive_revision_retry_boundary_is_enforced_by_postgres(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 archive retry boundary test")
        .execute(&pool)
        .await
        .unwrap();

    let (max_attempts, deadline_seconds): (i32, i64) = sqlx::query_as(
        r#"INSERT INTO archive_revision_erp_feedback_outbox
           (id, owner_id, product_code, field_name, payload)
           VALUES ($1,$2,'P-1','approval_no','{}'::jsonb)
           RETURNING max_attempts,
                     EXTRACT(EPOCH FROM (deadline_at - created_at))::bigint"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((max_attempts, deadline_seconds), (5, 86_400));

    let invalid_attempts = sqlx::query(
        r#"INSERT INTO archive_revision_erp_feedback_outbox
           (id, owner_id, product_code, field_name, payload, max_attempts)
           VALUES ($1,$2,'P-2','approval_no','{}'::jsonb,6)"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(&pool)
    .await;
    assert!(invalid_attempts.is_err());

    let invalid_deadline = sqlx::query(
        r#"INSERT INTO archive_revision_erp_feedback_outbox
           (id, owner_id, product_code, field_name, payload, created_at, deadline_at)
           VALUES ($1,$2,'P-3','approval_no','{}'::jsonb,now(),now()+interval '25 hours')"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(&pool)
    .await;
    assert!(invalid_deadline.is_err());
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
            None,
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
            "result": if stage == "receive" { "received" } else { "preflight_rejected" },
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

    for (stage, result, expected_status, retry_scheduled) in [
        ("receive", "ok", "processing", false),
        (
            "final_failure",
            "Bearer supersecrettoken password=anotherlongsecret",
            "failed",
            true,
        ),
        ("receive", "ok", "processing", false),
        ("receipt", "ok", "succeeded", false),
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
            "channel": "interface_table",
            "wms_resource_id": (stage == "receipt").then_some("receiving-order-1")
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
        let returned: H8ErpMessage =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(
            returned.wms_resource_id.as_deref(),
            (stage == "receipt").then_some("receiving-order-1"),
            "stage={stage}"
        );

        let (status, next_retry_at, updated_at, wms_resource_id): (
            String,
            Option<DateTime<Utc>>,
            DateTime<Utc>,
            Option<String>,
        ) = sqlx::query_as(
            "SELECT sync_status, next_retry_at, updated_at, wms_resource_id FROM h8_erp_messages WHERE owner_id=$1 AND idempotency_key=$2",
        )
        .bind(owner_id)
        .bind("idem-retry-pg-1")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status, expected_status, "stage={stage}");
        if retry_scheduled {
            assert_eq!(
                next_retry_at.expect("retry should be scheduled") - updated_at,
                Duration::milliseconds(standard_retry_delay_millis(1, "idem-retry-pg-1"))
            );
        } else {
            assert!(next_retry_at.is_none(), "stage={stage}");
        }
        assert_eq!(
            wms_resource_id.as_deref(),
            (stage == "receipt").then_some("receiving-order-1"),
            "stage={stage}"
        );
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

    let attempts: Vec<AttemptRow> = sqlx::query_as(
        r#"SELECT attempt_no, channel, started_at, finished_at, result, error_summary, actor
           FROM h8_erp_message_attempts
           WHERE owner_id=$1
           ORDER BY attempt_no"#,
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!((attempts[0].0, attempts[0].4.as_str()), (1, "failed"));
    assert_eq!((attempts[1].0, attempts[1].4.as_str()), (2, "succeeded"));
    for attempt in &attempts {
        assert_eq!(attempt.1, "interface_table");
        assert!(attempt.3.is_some_and(|finished| finished >= attempt.2));
        assert!(attempt.6.starts_with("worker:"));
    }
    let failed_summary = attempts[0].5.as_deref().expect("failed attempt summary");
    for secret in ["supersecrettoken", "rettoken", "anotherlongsecret"] {
        assert!(!failed_summary.contains(secret), "attempt leaked {secret}");
    }
    assert!(attempts[1].5.is_none());
}
