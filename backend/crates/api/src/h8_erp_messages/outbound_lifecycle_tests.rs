use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::H8ErpMessage;

use crate::auth::AuthContext;

use super::{handlers::h8_erp_message_router, state::H8ErpMessageAppState};

fn worker_ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "h8-outbound-worker-test".into(),
        permissions: vec!["h8.erp_connector.write".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn receipt_ctx(owner_id: Uuid, api_key_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: api_key_id,
        owner_id,
        actor_name: "h8-receipt-api-key-test".into(),
        permissions: vec!["h8.erp_receipt.write".into()],
        jti: format!("api-key:{api_key_id}"),
        warehouse_scope: None,
    }
}

async fn seed_receipt_connector(
    pool: &PgPool,
    owner_id: Uuid,
    connector_id: Uuid,
    api_key_id: Uuid,
    connector_code: &str,
) {
    sqlx::query(
        r#"INSERT INTO h8_erp_connectors
           (id, owner_id, connector_code, connector_name, directions, message_types,
            channel_mode, api_key_id, status, config_version)
           VALUES ($1,$2,$3,$3,ARRAY['outbound'],ARRAY['putaway_complete'],
                   'rest',$4,'active',1)"#,
    )
    .bind(connector_id)
    .bind(owner_id)
    .bind(connector_code)
    .bind(api_key_id)
    .execute(pool)
    .await
    .expect("receipt connector should seed");
}

#[sqlx::test(migrations = "../../migrations")]
async fn rest_business_receipt_rejects_another_connector_api_key(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let bound_api_key_id = Uuid::new_v4();
    let other_api_key_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 REST 回执冻结 API Key 测试")
        .execute(&pool)
        .await
        .unwrap();
    seed_receipt_connector(
        &pool,
        owner_id,
        connector_id,
        bound_api_key_id,
        "ERP-RECEIPT-BOUND",
    )
    .await;
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, connector_id, connector_code, config_version, direction,
            message_type, schema_version, channel, external_ref, idempotency_key,
            correlation_id, sync_status, retry_count, payload_digest)
           VALUES ($1,$2,$3,'ERP-RECEIPT-BOUND',1,'outbound','putaway_complete','1',
                   'rest','PUTAWAY-BOUND-1','idem-putaway-bound-1',
                   'corr-putaway-bound-1','awaiting_receipt',0,'digest')"#,
    )
    .bind(message_id)
    .bind(owner_id)
    .bind(connector_id)
    .execute(&pool)
    .await
    .unwrap();
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));
    let mut request = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/integration/erp-messages/{message_id}/receipt"
        ))
        .header("content-type", "application/json")
        .header("Idempotency-Key", "idem-putaway-bound-1")
        .body(Body::from(
            serde_json::json!({
                "result": "ok",
                "schema_version": "1",
                "correlation_id": "corr-putaway-bound-1"
            })
            .to_string(),
        ))
        .unwrap();
    request
        .extensions_mut()
        .insert(receipt_ctx(owner_id, other_api_key_id));
    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let status: String = sqlx::query_scalar("SELECT sync_status FROM h8_erp_messages WHERE id=$1")
        .bind(message_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "awaiting_receipt");
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_send_waits_for_business_receipt_and_duplicate_receipt_is_stable(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 出站两级回执测试")
        .execute(&pool)
        .await
        .expect("owner should seed");
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));
    let ctx = worker_ctx(owner_id);
    let mut first_acked_at = None;

    for (stage, expected_status) in [
        ("receive", "processing"),
        ("send", "awaiting_receipt"),
        ("receipt", "acked"),
        ("receipt", "acked"),
    ] {
        let body = serde_json::json!({
            "stage": stage,
            "result": "ok",
            "direction": "outbound",
            "message_type": "putaway_complete",
            "schema_version": "1",
            "external_ref": "PUTAWAY-OUT-1",
            "idempotency_key": "idem-putaway-out-1",
            "correlation_id": "corr-putaway-out-1",
            "channel": "rest"
        });
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("request should build");
        request.extensions_mut().insert(ctx.clone());
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("lifecycle should respond");
        assert_eq!(response.status(), StatusCode::OK, "stage={stage}");
        let message: H8ErpMessage = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("response body should read"),
        )
        .expect("response should be an H8 message");
        assert_eq!(message.sync_status, expected_status, "stage={stage}");
        if stage == "receipt" {
            let acked_at = message
                .acked_at
                .expect("business receipt should set acked_at");
            if let Some(first) = first_acked_at {
                assert_eq!(
                    acked_at, first,
                    "duplicate receipt must not rewrite ack time"
                );
            } else {
                first_acked_at = Some(acked_at);
            }
        }
    }

    let message_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM h8_erp_messages WHERE owner_id=$1 AND idempotency_key=$2",
    )
    .bind(owner_id)
    .bind("idem-putaway-out-1")
    .fetch_one(&pool)
    .await
    .expect("message count should query");
    assert_eq!(message_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn rest_business_receipt_acks_once_with_original_binding(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 REST 业务回执测试")
        .execute(&pool)
        .await
        .expect("owner should seed");
    seed_receipt_connector(&pool, owner_id, connector_id, api_key_id, "ERP-RECEIPT-ACK").await;
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));
    let worker = worker_ctx(owner_id);
    let receipt = receipt_ctx(owner_id, api_key_id);
    let body = serde_json::json!({
        "stage": "receive",
        "result": "ok",
        "direction": "outbound",
        "message_type": "putaway_complete",
        "schema_version": "1",
        "external_ref": "PUTAWAY-REST-1",
        "idempotency_key": "idem-putaway-rest-1",
        "correlation_id": "corr-putaway-rest-1",
        "channel": "rest",
        "connector_id": connector_id,
        "connector_code": "ERP-RECEIPT-ACK",
        "config_version": 1
    });
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/lifecycle")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build");
    request.extensions_mut().insert(worker.clone());
    let response = app.clone().oneshot(request).await.unwrap();
    let message: H8ErpMessage =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();

    let send = serde_json::json!({
        "stage": "send",
        "result": "ok",
        "direction": "outbound",
        "message_type": "putaway_complete",
        "schema_version": "1",
        "external_ref": "PUTAWAY-REST-1",
        "idempotency_key": "idem-putaway-rest-1",
        "correlation_id": "corr-putaway-rest-1",
        "channel": "rest",
        "connector_id": connector_id,
        "connector_code": "ERP-RECEIPT-ACK",
        "config_version": 1,
        "message_id": message.id
    });
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/lifecycle")
        .header("content-type", "application/json")
        .body(Body::from(send.to_string()))
        .unwrap();
    request.extensions_mut().insert(worker);
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::OK
    );

    let mut invalid_success = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/integration/erp-messages/{}/receipt",
            message.id
        ))
        .header("content-type", "application/json")
        .header("Idempotency-Key", "idem-putaway-rest-1")
        .body(Body::from(
            serde_json::json!({
                "result": "ok",
                "error_summary": "must not turn a success receipt into rejection",
                "schema_version": "1",
                "correlation_id": "corr-putaway-rest-1"
            })
            .to_string(),
        ))
        .unwrap();
    invalid_success.extensions_mut().insert(receipt.clone());
    assert_eq!(
        app.clone().oneshot(invalid_success).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );

    let mut first_acked_at = None;
    for _ in 0..2 {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/v1/integration/erp-messages/{}/receipt",
                message.id
            ))
            .header("content-type", "application/json")
            .header("Idempotency-Key", "idem-putaway-rest-1")
            .body(Body::from(
                serde_json::json!({
                    "result": "ok",
                    "schema_version": "1",
                    "correlation_id": "corr-putaway-rest-1"
                })
                .to_string(),
            ))
            .unwrap();
        request.extensions_mut().insert(receipt.clone());
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let acked: H8ErpMessage =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(acked.sync_status, "acked");
        let acked_at = acked.acked_at.expect("receipt should set acked_at");
        if let Some(first) = first_acked_at {
            assert_eq!(acked_at, first);
        } else {
            first_acked_at = Some(acked_at);
        }
    }

    let mut invalid_terminal_success = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/integration/erp-messages/{}/receipt",
            message.id
        ))
        .header("content-type", "application/json")
        .header("Idempotency-Key", "idem-putaway-rest-1")
        .body(Body::from(
            serde_json::json!({
                "result": "ok",
                "error_summary": "must remain invalid after ack",
                "schema_version": "1",
                "correlation_id": "corr-putaway-rest-1"
            })
            .to_string(),
        ))
        .unwrap();
    invalid_terminal_success.extensions_mut().insert(receipt);
    assert_eq!(
        app.oneshot(invalid_terminal_success)
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn rest_business_rejection_requires_original_binding_and_enters_dead_once(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 REST 拒绝回执测试")
        .execute(&pool)
        .await
        .unwrap();
    seed_receipt_connector(
        &pool,
        owner_id,
        connector_id,
        api_key_id,
        "ERP-RECEIPT-REJECT",
    )
    .await;
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, connector_id, connector_code, config_version, direction,
            message_type, schema_version, channel, external_ref, idempotency_key,
            correlation_id, sync_status, retry_count, payload_digest)
           VALUES ($1,$2,$3,'ERP-RECEIPT-REJECT',1,'outbound','putaway_complete','1',
                   'rest','PUTAWAY-REST-REJECT-1',
                   'idem-putaway-rest-reject-1','corr-putaway-rest-reject-1',
                   'awaiting_receipt',0,'digest')"#,
    )
    .bind(message_id)
    .bind(owner_id)
    .bind(connector_id)
    .execute(&pool)
    .await
    .unwrap();
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));
    let ctx = receipt_ctx(owner_id, api_key_id);
    let body = serde_json::json!({
        "result": "rejected",
        "error_summary": "ERP business validation rejected",
        "schema_version": "1",
        "correlation_id": "corr-putaway-rest-reject-1"
    });

    let mut mismatched = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/integration/erp-messages/{message_id}/receipt"
        ))
        .header("content-type", "application/json")
        .header("Idempotency-Key", "wrong-key")
        .body(Body::from(body.to_string()))
        .unwrap();
    mismatched.extensions_mut().insert(ctx.clone());
    assert_eq!(
        app.clone().oneshot(mismatched).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );

    for _ in 0..2 {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/v1/integration/erp-messages/{message_id}/receipt"
            ))
            .header("content-type", "application/json")
            .header("Idempotency-Key", "idem-putaway-rest-reject-1")
            .body(Body::from(body.to_string()))
            .unwrap();
        request.extensions_mut().insert(ctx.clone());
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let rejected: H8ErpMessage =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(rejected.sync_status, "dead");
    }

    let mut invalid_terminal_rejection = Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/integration/erp-messages/{message_id}/receipt"
        ))
        .header("content-type", "application/json")
        .header("Idempotency-Key", "idem-putaway-rest-reject-1")
        .body(Body::from(
            serde_json::json!({
                "result": "rejected",
                "schema_version": "1",
                "correlation_id": "corr-putaway-rest-reject-1"
            })
            .to_string(),
        ))
        .unwrap();
    invalid_terminal_rejection.extensions_mut().insert(ctx);
    assert_eq!(
        app.clone()
            .oneshot(invalid_terminal_rejection)
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );

    let audit_actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_event WHERE owner_id=$1 ORDER BY occurred_at, id",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        audit_actions,
        vec!["h8_exchange_receipt", "h8_message_dead"]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn receipt_timeout_retries_original_message_then_exhausts_to_dead(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 回执超时测试")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, direction, message_type, schema_version, channel, external_ref,
            idempotency_key, correlation_id, sync_status, retry_count, payload_digest)
           VALUES ($1,$2,'outbound','putaway_complete','1','rest','PUTAWAY-TIMEOUT-1',
                   'idem-putaway-timeout-1','corr-putaway-timeout-1',
                   'processing',0,'digest')"#,
    )
    .bind(message_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));
    let ctx = worker_ctx(owner_id);
    let lifecycle = |stage: &str, result: &str| {
        serde_json::json!({
            "stage": stage,
            "result": result,
            "direction": "outbound",
            "message_type": "putaway_complete",
            "schema_version": "1",
            "external_ref": "PUTAWAY-TIMEOUT-1",
            "idempotency_key": "idem-putaway-timeout-1",
            "correlation_id": "corr-putaway-timeout-1",
            "channel": "rest",
            "message_id": message_id
        })
    };

    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/lifecycle")
        .header("content-type", "application/json")
        .body(Body::from(lifecycle("send", "ok").to_string()))
        .unwrap();
    request.extensions_mut().insert(ctx.clone());
    let response = app.clone().oneshot(request).await.unwrap();
    let mut message: H8ErpMessage =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(message.sync_status, "awaiting_receipt");
    assert!(message.next_retry_at.is_some());

    for retry in 1..=5 {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(
                lifecycle("final_failure", "business receipt timeout").to_string(),
            ))
            .unwrap();
        request.extensions_mut().insert(ctx.clone());
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        message =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(message.retry_count, retry);
        assert_eq!(message.id, message_id);
        assert_eq!(message.idempotency_key, "idem-putaway-timeout-1");
        if retry == 5 {
            assert_eq!(message.sync_status, "dead");
            assert!(message.next_retry_at.is_none());
            break;
        }
        assert_eq!(message.sync_status, "processing");

        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(lifecycle("receive", "ok").to_string()))
            .unwrap();
        request.extensions_mut().insert(ctx.clone());
        let response = app.clone().oneshot(request).await.unwrap();
        message =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(message.sync_status, "processing");

        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/integration/erp-messages/lifecycle")
            .header("content-type", "application/json")
            .body(Body::from(lifecycle("send", "ok").to_string()))
            .unwrap();
        request.extensions_mut().insert(ctx.clone());
        let response = app.clone().oneshot(request).await.unwrap();
        message =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(message.sync_status, "awaiting_receipt");
        assert!(message.next_retry_at.is_some());
    }

    let dead_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id=$1 AND action='h8_message_dead'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(dead_audits, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn dead_audit_failure_rolls_back_lifecycle_status_and_attempt(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("H8 lifecycle atomic audit test")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"INSERT INTO h8_erp_messages
           (id, owner_id, direction, message_type, schema_version, channel, external_ref,
            idempotency_key, correlation_id, sync_status, retry_count, payload_digest)
           VALUES ($1,$2,'outbound','putaway_complete','1','rest','PUTAWAY-ATOMIC-1',
                   'idem-putaway-atomic-1','corr-putaway-atomic-1',
                   'awaiting_receipt',4,'digest')"#,
    )
    .bind(message_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION reject_h8_dead_audit() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.action = 'h8_message_dead' THEN
                RAISE EXCEPTION 'forced H8 dead audit failure';
            END IF;
            RETURN NEW;
        END;
        $$;
        CREATE TRIGGER reject_h8_dead_audit
        BEFORE INSERT ON audit_event
        FOR EACH ROW EXECUTE FUNCTION reject_h8_dead_audit();
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));
    let body = serde_json::json!({
        "stage": "final_failure",
        "result": "business receipt timeout",
        "direction": "outbound",
        "message_type": "putaway_complete",
        "schema_version": "1",
        "external_ref": "PUTAWAY-ATOMIC-1",
        "idempotency_key": "idem-putaway-atomic-1",
        "correlation_id": "corr-putaway-atomic-1",
        "channel": "rest",
        "message_id": message_id
    });
    let mut request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/lifecycle")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    request.extensions_mut().insert(worker_ctx(owner_id));

    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let evidence: (String, i32, i64, i64) = sqlx::query_as(
        r#"SELECT sync_status, retry_count,
                  (SELECT COUNT(*) FROM audit_event WHERE owner_id=$2),
                  (SELECT COUNT(*) FROM h8_erp_message_attempts WHERE message_id=$1)
             FROM h8_erp_messages WHERE id=$1"#,
    )
    .bind(message_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(evidence, ("awaiting_receipt".into(), 4, 0, 0));
}
