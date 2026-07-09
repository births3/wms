use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{auth::AuthContext, wechat_notify_service::PgWechatNotifyService};
use wms_domain::{
    CreateH4ApprovalRequest, H4ApprovalCallbackRequest, SendH4NotificationRequest,
    UpsertH4NotificationConfigRequest, UpsertH4WechatSettingsRequest,
};

fn ctx() -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id: Uuid::new_v4(),
        actor_name: "h4-tester".to_string(),
        jti: Uuid::new_v4().to_string(),
        permissions: vec![
            "h4.notify.read".to_string(),
            "h4.notify.write".to_string(),
            "h4.notify.send".to_string(),
            "h4.approval.write".to_string(),
        ],
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_config_send_and_idempotency_write_records(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let ctx = ctx();
    let now = Utc::now();
    let config = service
        .upsert_config(
            &pool,
            &ctx,
            UpsertH4NotificationConfigRequest {
                event_type: "asn_arrived".to_string(),
                enabled: true,
                template: "ASN {{asn_no}} 已到货".to_string(),
                recipient_rule: serde_json::json!({ "roles": ["warehouse_manager"] }),
                channels: vec!["wechat".to_string()],
            },
            now,
            "h4-config-1",
        )
        .await
        .expect("config should upsert");
    assert!(!config.replayed);

    let sent = service
        .send_notification(
            &pool,
            &ctx,
            SendH4NotificationRequest {
                event_type: "asn_arrived".to_string(),
                dedupe_key: "ASN-001".to_string(),
                recipients: vec!["manager".to_string(), "qa".to_string()],
                payload: serde_json::json!({ "asn_no": "ASN-001" }),
            },
            now,
            "h4-send-1",
        )
        .await
        .expect("notification should send");
    assert_eq!(sent.value.len(), 2);

    let replay = service
        .send_notification(
            &pool,
            &ctx,
            SendH4NotificationRequest {
                event_type: "asn_arrived".to_string(),
                dedupe_key: "ASN-001".to_string(),
                recipients: vec!["manager".to_string(), "qa".to_string()],
                payload: serde_json::json!({ "asn_no": "ASN-001" }),
            },
            now,
            "h4-send-1",
        )
        .await
        .expect("notification should replay");
    assert!(replay.replayed);

    let record_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM h4_notification_records WHERE owner_id = $1")
            .bind(ctx.owner_id)
            .fetch_one(&pool)
            .await
            .expect("record count should query");
    assert_eq!(record_count.0, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_approval_callback_is_idempotent(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let ctx = ctx();
    let now = Utc::now();
    let approval = service
        .create_approval(
            &pool,
            &ctx,
            CreateH4ApprovalRequest {
                scenario: "asn_cancel".to_string(),
                business_ref: "ASN-001".to_string(),
                dedupe_key: "ASN-001-cancel".to_string(),
                approver_user: "manager".to_string(),
                process_id: "ww-process-1".to_string(),
                callback_path: "/api/v1/wechat-notify/approvals/callback".to_string(),
                summary: "ASN 作废审批".to_string(),
            },
            now,
            "h4-approval-1",
        )
        .await
        .expect("approval should create");
    assert_eq!(approval.value.status, "pending");

    let callback = service
        .apply_approval_callback(
            &pool,
            &ctx,
            approval.value.id,
            H4ApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: Some("同意".to_string()),
                approved_by: "manager".to_string(),
                external_approval_id: Some("ww-approval-1".to_string()),
            },
            now,
            "h4-callback-1",
        )
        .await
        .expect("callback should apply");
    assert_eq!(callback.value.status, "approved");

    let replay = service
        .apply_approval_callback(
            &pool,
            &ctx,
            approval.value.id,
            H4ApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: Some("同意".to_string()),
                approved_by: "manager".to_string(),
                external_approval_id: Some("ww-approval-1".to_string()),
            },
            now,
            "h4-callback-1",
        )
        .await
        .expect("callback should replay");
    assert!(replay.replayed);
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_wechat_settings_are_owner_scoped_and_idempotent(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let ctx = ctx();
    let now = Utc::now();
    let request = UpsertH4WechatSettingsRequest {
        corp_id: "ww-corp-demo".to_string(),
        agent_id: "1000002".to_string(),
        secret_alias: "h4/wechat/agent_secret".to_string(),
        callback_token_alias: "h4/wechat/callback_token".to_string(),
        aes_key_alias: "h4/wechat/aes_key".to_string(),
        callback_url: "https://wms.example.com/api/v1/wechat-notify/approvals/callback".to_string(),
        approval_callback_path: "/api/v1/wechat-notify/approvals/{approval_id}/callback"
            .to_string(),
        enabled: true,
        retry_max_attempts: 3,
        retry_interval_seconds: 60,
    };

    let saved = service
        .upsert_wechat_settings(&pool, &ctx, request.clone(), now, "h4-settings-1")
        .await
        .expect("settings should upsert");
    assert_eq!(saved.value.corp_id, "ww-corp-demo");
    assert!(!saved.replayed);

    let replay = service
        .upsert_wechat_settings(&pool, &ctx, request, now, "h4-settings-1")
        .await
        .expect("settings should replay");
    assert!(replay.replayed);

    let loaded = service
        .get_wechat_settings(&pool, &ctx)
        .await
        .expect("settings should load");
    assert_eq!(
        loaded.data.expect("settings exists").secret_alias,
        "h4/wechat/agent_secret"
    );
}
