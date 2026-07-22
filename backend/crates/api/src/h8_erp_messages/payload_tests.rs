//! US-H8-003 AC16：完整报文处理器权限、生产接线与审计测试。

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;
use uuid::Uuid;
use wms_domain::UpdateH8PayloadRetentionPolicyRequest;

use crate::auth::AuthContext;

use super::{handlers::h8_erp_message_router, state::H8ErpMessageAppState};

fn test_ctx(owner_id: Uuid, writable: bool) -> AuthContext {
    AuthContext {
        user_id: Uuid::nil(),
        owner_id,
        actor_name: "tester".into(),
        permissions: if writable {
            vec![
                "h8.erp_connector.read".into(),
                "h8.erp_connector.write".into(),
            ]
        } else {
            vec!["h8.erp_connector.read".into()]
        },
        jti: "jti-test".into(),
        warehouse_scope: None,
    }
}

#[tokio::test]
async fn lifecycle_capture_and_decrypt_require_write_and_redact_audit() {
    let state = H8ErpMessageAppState::with_memory();
    let owner_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    state
        .payload_repository
        .update_policy(
            owner_id,
            &UpdateH8PayloadRetentionPolicyRequest {
                connector_id,
                enabled: true,
                retention_days: None,
                confirmed: true,
            },
            "admin",
            chrono::Utc::now(),
        )
        .await
        .unwrap();

    let key = "k".repeat(32);
    std::env::set_var("WMS_ENCRYPTION_MASTER_KEY", &key);
    let lifecycle = serde_json::json!({
        "stage": "receive",
        "result": "ok",
        "direction": "inbound",
        "message_type": "asn",
        "schema_version": "1",
        "external_ref": "ERP-ASN-PAYLOAD-1",
        "idempotency_key": "idem-payload-1",
        "correlation_id": "corr-payload-1",
        "channel": "interface_table",
        "connector_id": connector_id,
        "connector_code": "SELF-ERP",
        "config_version": 1,
        "payload": {"qty": 1}
    });
    let mut capture_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/integration/erp-messages/lifecycle")
        .header("content-type", "application/json")
        .body(Body::from(lifecycle.to_string()))
        .unwrap();
    capture_request
        .extensions_mut()
        .insert(test_ctx(owner_id, true));
    let response = h8_erp_message_router(state.clone())
        .oneshot(capture_request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let message_id: Uuid = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        state
            .payload_repository
            .payload_status(owner_id, message_id, chrono::Utc::now())
            .await
            .unwrap()
            .0
    );

    let payload_uri = format!("/api/v1/integration/erp-messages/{message_id}/payload");
    let mut denied_request = Request::builder()
        .uri(&payload_uri)
        .body(Body::empty())
        .unwrap();
    denied_request
        .extensions_mut()
        .insert(test_ctx(owner_id, false));
    let denied = h8_erp_message_router(state.clone())
        .oneshot(denied_request)
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    let mut decrypt_request = Request::builder()
        .uri(payload_uri)
        .body(Body::empty())
        .unwrap();
    decrypt_request
        .extensions_mut()
        .insert(test_ctx(owner_id, true));
    let decrypted = h8_erp_message_router(state.clone())
        .oneshot(decrypt_request)
        .await
        .unwrap();
    assert_eq!(decrypted.status(), StatusCode::OK);
    assert_eq!(decrypted.headers()["cache-control"], "no-store");
    let body = to_bytes(decrypted.into_body(), usize::MAX).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains(r#"\"qty\":1"#));

    let log = state.audit_log.lock().expect("audit log");
    assert!(log
        .events()
        .iter()
        .any(|event| event.action == "h8_exchange_receive"));
    let decrypt_audit = log
        .events()
        .iter()
        .find(|event| event.action == "h8_payload_decrypt")
        .expect("decrypt audit");
    assert!(decrypt_audit.diff.as_ref().is_some_and(|diff| diff
        .after
        .get("payload")
        .is_some_and(serde_json::Value::is_null)));
    drop(log);
    std::env::remove_var("WMS_ENCRYPTION_MASTER_KEY");
}
