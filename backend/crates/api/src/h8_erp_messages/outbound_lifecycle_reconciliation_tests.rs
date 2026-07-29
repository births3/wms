use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use chrono::Utc;
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
        actor_name: "h8-mrc-worker-test".into(),
        permissions: vec!["h8.erp_connector.write".into()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn reconciliation_waits_for_h8_ack_and_keeps_exception_isolated_on_dead(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let actor = worker_ctx(owner_id);
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1,$2,$3)")
        .bind(owner_id)
        .bind(format!("OWNER-{owner_id}"))
        .bind("M-RC H8 回执测试")
        .execute(&pool)
        .await
        .unwrap();
    let run_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO reconciliation_runs
         (id, owner_id, window_key, request_hash, snapshot_at, created_by)
         VALUES ($1,$2,'h8-receipt','hash',now(),$3)",
    )
    .bind(run_id)
    .bind(owner_id)
    .bind(actor.user_id)
    .execute(&pool)
    .await
    .unwrap();
    let app = h8_erp_message_router(H8ErpMessageAppState::with_postgres(pool.clone()));

    for (suffix, terminal, expected) in [
        ("ACK", "receipt", "resolved"),
        ("DEAD", "dead", "exception"),
    ] {
        let item_id = Uuid::new_v4();
        let outbox_id = Uuid::new_v4();
        let idempotency_key = format!("out:reconciliation_erp_feedback_outbox:{outbox_id}");
        sqlx::query(
            "INSERT INTO reconciliation_items
             (id, owner_id, run_id, product_code, batch_no, wms_qty, erp_qty,
              difference_qty, difference_type, resolution_status, disposition, resolved_by)
             VALUES ($1,$2,$3,$4,'B1',10,8,2,'wms_more',
                     'erp_feedback_pending','wms_truth',$5)",
        )
        .bind(item_id)
        .bind(owner_id)
        .bind(run_id)
        .bind(format!("P-{suffix}"))
        .bind(actor.user_id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO reconciliation_erp_feedback_outbox
             (id, owner_id, recon_doc_no, payload)
             VALUES ($1,$2,$3,'{}')",
        )
        .bind(outbox_id)
        .bind(owner_id)
        .bind(item_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let mut message_id = None;
        for stage in ["receive", "send"] {
            let body = serde_json::json!({
                "stage": stage,
                "result": "ok",
                "direction": "outbound",
                "message_type": "reconciliation_diff",
                "schema_version": "1",
                "external_ref": format!("RC-{suffix}"),
                "idempotency_key": idempotency_key,
                "correlation_id": format!("corr-{suffix}"),
                "channel": "rest"
            });
            let mut request = Request::builder()
                .method(Method::POST)
                .uri("/api/v1/integration/erp-messages/lifecycle")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap();
            request.extensions_mut().insert(actor.clone());
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let message: H8ErpMessage =
                serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                    .unwrap();
            message_id = Some(message.id);
        }

        if terminal == "receipt" {
            let body = serde_json::json!({
                "stage": "receipt",
                "result": "ok",
                "direction": "outbound",
                "message_type": "reconciliation_diff",
                "schema_version": "1",
                "external_ref": format!("RC-{suffix}"),
                "idempotency_key": idempotency_key,
                "correlation_id": format!("corr-{suffix}"),
                "channel": "rest"
            });
            let mut request = Request::builder()
                .method(Method::POST)
                .uri("/api/v1/integration/erp-messages/lifecycle")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap();
            request.extensions_mut().insert(actor.clone());
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::OK
            );
        } else {
            let mut request = Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/integration/erp-messages/{}/dead",
                    message_id.unwrap()
                ))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"error_summary":"ERP rejected"}"#))
                .unwrap();
            request.extensions_mut().insert(actor.clone());
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::OK
            );
        }

        let state: (String, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
            "SELECT resolution_status, resolved_at
               FROM reconciliation_items
              WHERE owner_id=$1 AND id=$2",
        )
        .bind(owner_id)
        .bind(item_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(state.0, expected);
        assert_eq!(state.1.is_some(), expected == "resolved");
        let audit_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_event
              WHERE owner_id=$1
                AND module='M-RC'
                AND action='advance_reconciliation_h8_receipt'
                AND resource_type='reconciliation_item'
                AND resource_id=$2",
        )
        .bind(owner_id)
        .bind(item_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit_count, 1, "terminal={terminal}");
    }
}
