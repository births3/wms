use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    wechat_notify_service::{H4RecordQuery, PgWechatNotifyService},
};
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

fn read_only_ctx(owner_id: Uuid, actor_name: &str) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: actor_name.to_string(),
        jti: Uuid::new_v4().to_string(),
        permissions: vec!["h4.notify.read".to_string()],
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_config_send_and_idempotency_write_records(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let ctx = ctx();
    let now = Utc::now();
    let empty_rule_error = service
        .upsert_config(
            &pool,
            &ctx,
            UpsertH4NotificationConfigRequest {
                event_type: "asn_arrived".to_string(),
                enabled: true,
                template: "ASN {{asn_no}} 已到货".to_string(),
                recipient_rule: serde_json::json!({}),
                channels: vec!["wechat".to_string()],
            },
            now,
            "h4-empty-rule",
        )
        .await
        .expect_err("empty recipient rule should fail");
    assert_eq!(format!("{empty_rule_error:?}"), "NoRecipients");

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
    assert!(sent.value.iter().all(|record| record.status == "failed"));
    assert!(sent.value.iter().all(|record| record.sent_at.is_none()));
    sqlx::query("UPDATE h4_notification_records SET status = 'success' WHERE id = $1")
        .bind(sent.value[0].id)
        .execute(&pool)
        .await
        .expect("record should become successful for resend guard test");
    let success_resend_error = service
        .resend_record(&pool, &ctx, sent.value[0].id, now, "h4-resend-success")
        .await
        .expect_err("successful notification must not be resent");
    assert_eq!(format!("{success_resend_error:?}"), "RecordNotResendable");

    sqlx::query("UPDATE h4_notification_records SET status = 'failed' WHERE id = $1")
        .bind(sent.value[0].id)
        .execute(&pool)
        .await
        .expect("record should become failed for resend test");
    let resent = service
        .resend_record(&pool, &ctx, sent.value[0].id, now, "h4-resend-failed")
        .await
        .expect("failed notification should resend");
    assert_eq!(resent.value.status, "failed");
    assert_eq!(resent.value.retry_count, 1);

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
    let mut ctx = ctx();
    ctx.actor_name = "manager".to_string();
    let now = Utc::now();
    let approval_request = CreateH4ApprovalRequest {
        scenario: "asn_cancel".to_string(),
        business_ref: "ASN-001".to_string(),
        dedupe_key: "ASN-001-cancel".to_string(),
        approver_user: ctx.user_id.to_string().to_uppercase(),
        process_id: "ww-process-1".to_string(),
        callback_path: "/api/v1/wechat-notify/approvals/callback".to_string(),
        summary: "ASN 作废审批".to_string(),
    };
    let mut display_name_request = approval_request.clone();
    display_name_request.business_ref = "ASN-DISPLAY-NAME".to_string();
    display_name_request.dedupe_key = "ASN-DISPLAY-NAME".to_string();
    display_name_request.approver_user = ctx.actor_name.clone();
    let display_name_error = service
        .create_approval(
            &pool,
            &ctx,
            display_name_request,
            now,
            "h4-approval-display-name",
        )
        .await
        .expect_err("approver must use immutable user id instead of display name");
    assert_eq!(format!("{display_name_error:?}"), "InvalidRequest");

    let approval = service
        .create_approval(&pool, &ctx, approval_request.clone(), now, "h4-approval-1")
        .await
        .expect("approval should create");
    assert_eq!(approval.value.status, "pending");
    let mut canonical_request = approval_request;
    canonical_request.approver_user = ctx.user_id.to_string();
    let creation_replay = service
        .create_approval(&pool, &ctx, canonical_request, now, "h4-approval-1")
        .await
        .expect("UUID case differences should replay approval creation");
    assert!(creation_replay.replayed);
    assert_eq!(creation_replay.value.id, approval.value.id);

    let mut attacker = ctx.clone();
    attacker.user_id = Uuid::new_v4();
    let forged_identity = service
        .apply_approval_callback(
            &pool,
            &attacker,
            approval.value.id,
            H4ApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: Some("伪造指定审批人".to_string()),
                approved_by: ctx.user_id.to_string().to_uppercase(),
                external_approval_id: Some("ww-approval-forged-identity".to_string()),
            },
            now,
            "h4-callback-forged-identity",
        )
        .await
        .expect_err("JWT actor must match the designated approver");
    assert_eq!(format!("{forged_identity:?}"), "InvalidRequest");

    let unauthorized = service
        .apply_approval_callback(
            &pool,
            &ctx,
            approval.value.id,
            H4ApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: Some("越权回写".to_string()),
                approved_by: Uuid::new_v4().to_string(),
                external_approval_id: Some("ww-approval-forged".to_string()),
            },
            now,
            "h4-callback-forged",
        )
        .await
        .expect_err("non-designated approver must be rejected");
    assert_eq!(format!("{unauthorized:?}"), "InvalidRequest");

    let callback = service
        .apply_approval_callback(
            &pool,
            &ctx,
            approval.value.id,
            H4ApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: Some("同意".to_string()),
                approved_by: ctx.user_id.to_string().to_uppercase(),
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
                approved_by: ctx.user_id.to_string(),
                external_approval_id: Some("ww-approval-1".to_string()),
            },
            now,
            "h4-callback-1",
        )
        .await
        .expect("callback should replay");
    assert!(replay.replayed);

    let audit_count_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'h4.approval.callback_applied'",
    )
    .bind(ctx.owner_id)
    .fetch_one(&pool)
    .await
    .expect("callback audit count should query");
    let second_key = service
        .apply_approval_callback(
            &pool,
            &ctx,
            approval.value.id,
            H4ApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: Some("重复回调".to_string()),
                approved_by: ctx.user_id.to_string(),
                external_approval_id: Some("ww-approval-1".to_string()),
            },
            now,
            "h4-callback-2",
        )
        .await
        .expect("completed callback should return existing result");
    assert!(second_key.replayed);
    assert_eq!(second_key.value.opinion.as_deref(), Some("同意"));
    let audit_count_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'h4.approval.callback_applied'",
    )
    .bind(ctx.owner_id)
    .fetch_one(&pool)
    .await
    .expect("callback audit count should query again");
    assert_eq!(audit_count_after, audit_count_before);

    let conflicting_conclusion = service
        .apply_approval_callback(
            &pool,
            &ctx,
            approval.value.id,
            H4ApprovalCallbackRequest {
                conclusion: "rejected".to_string(),
                opinion: Some("冲突终态".to_string()),
                approved_by: ctx.user_id.to_string(),
                external_approval_id: Some("ww-approval-1".to_string()),
            },
            now,
            "h4-callback-conflicting-conclusion",
        )
        .await
        .expect_err("same external approval cannot change terminal conclusion");
    assert_eq!(format!("{conflicting_conclusion:?}"), "IdempotencyConflict");
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_approval_creation_rejects_invalid_callback_path(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let ctx = ctx();
    let error = service
        .create_approval(
            &pool,
            &ctx,
            CreateH4ApprovalRequest {
                scenario: "asn_cancel".to_string(),
                business_ref: "ASN-INVALID-PATH".to_string(),
                dedupe_key: "ASN-INVALID-PATH".to_string(),
                approver_user: "manager".to_string(),
                process_id: "ww-process-1".to_string(),
                callback_path: "https://evil.example/callback".to_string(),
                summary: "非法回调路径".to_string(),
            },
            Utc::now(),
            "h4-approval-invalid-path",
        )
        .await
        .expect_err("callback path must be an internal absolute path");

    assert_eq!(format!("{error:?}"), "InvalidRequest");
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_record_query_limits_read_only_users_to_their_notifications(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let admin = ctx();
    let now = Utc::now();
    service
        .upsert_config(
            &pool,
            &admin,
            UpsertH4NotificationConfigRequest {
                event_type: "asn_arrived".to_string(),
                enabled: true,
                template: "ASN {{asn_no}} 已到货".to_string(),
                recipient_rule: serde_json::json!({ "users": ["operator-a"] }),
                channels: vec!["wechat".to_string()],
            },
            now,
            "h4-scope-config",
        )
        .await
        .expect("config should upsert");
    service
        .send_notification(
            &pool,
            &admin,
            SendH4NotificationRequest {
                event_type: "asn_arrived".to_string(),
                dedupe_key: "ASN-SCOPE".to_string(),
                recipients: vec!["operator-a".to_string(), "operator-b".to_string()],
                payload: serde_json::json!({ "asn_no": "ASN-SCOPE" }),
            },
            now,
            "h4-scope-send",
        )
        .await
        .expect("records should persist");

    let user = read_only_ctx(admin.owner_id, "operator-a");
    let records = service
        .list_records(
            &pool,
            &user,
            H4RecordQuery {
                event_type: None,
                recipient: None,
                status: None,
                from: None,
                to: None,
                limit: None,
            },
        )
        .await
        .expect("read-only user should list own records");

    assert_eq!(records.data.len(), 1);
    assert_eq!(records.data[0].recipient, "operator-a");

    let mut config_writer = user;
    config_writer
        .permissions
        .push("h4.notify.write".to_string());
    let writer_records = service
        .list_records(
            &pool,
            &config_writer,
            H4RecordQuery {
                event_type: None,
                recipient: None,
                status: None,
                from: None,
                to: None,
                limit: None,
            },
        )
        .await
        .expect("config writer should still list only own records");
    assert_eq!(writer_records.data.len(), 1);

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H4 scope owner')",
    )
    .bind(admin.owner_id)
    .bind(format!("H4-SCOPE-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("scope owner should insert");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash) VALUES ($1, $2, 'H4 admin', 'not-used')",
    )
    .bind(admin.user_id)
    .bind(format!("h4-admin-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("scope admin user should insert");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id) VALUES ($1, $2)")
        .bind(admin.user_id)
        .bind(admin.owner_id)
        .execute(&pool)
        .await
        .expect("scope admin owner binding should insert");
    let role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'SYSTEM_ADMIN', '系统管理员')",
    )
    .bind(role_id)
    .bind(admin.owner_id)
    .execute(&pool)
    .await
    .expect("scope system admin role should insert");
    sqlx::query("INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3)")
        .bind(admin.user_id)
        .bind(admin.owner_id)
        .bind(role_id)
        .execute(&pool)
        .await
        .expect("scope system admin role should bind");
    let admin_records = service
        .list_records(
            &pool,
            &admin,
            H4RecordQuery {
                event_type: None,
                recipient: None,
                status: None,
                from: None,
                to: None,
                limit: None,
            },
        )
        .await
        .expect("system admin should list all owner records");
    assert_eq!(admin_records.data.len(), 2);
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

    let tested = service
        .test_wechat_settings(&pool, &ctx, now)
        .await
        .expect("settings test should pass");
    assert_eq!(tested.status, "success");

    let record_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM h4_notification_records WHERE owner_id = $1")
            .bind(ctx.owner_id)
            .fetch_one(&pool)
            .await
            .expect("record count should query");
    assert_eq!(record_count.0, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_wechat_settings_reject_invalid_callback_urls(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let ctx = ctx();
    let request = UpsertH4WechatSettingsRequest {
        corp_id: "ww-corp-demo".to_string(),
        agent_id: "1000002".to_string(),
        secret_alias: "h4/wechat/agent_secret".to_string(),
        callback_token_alias: "h4/wechat/callback_token".to_string(),
        aes_key_alias: "h4/wechat/aes_key".to_string(),
        callback_url: "not-a-url".to_string(),
        approval_callback_path: "callback-without-leading-slash".to_string(),
        enabled: true,
        retry_max_attempts: 3,
        retry_interval_seconds: 60,
    };

    let error = service
        .upsert_wechat_settings(&pool, &ctx, request, Utc::now(), "h4-invalid-url")
        .await
        .expect_err("invalid callback URL should fail");

    assert_eq!(format!("{error:?}"), "InvalidRequest");
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_wechat_settings_test_requires_existing_settings(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let ctx = ctx();

    let error = service
        .test_wechat_settings(&pool, &ctx, Utc::now())
        .await
        .expect_err("settings test should fail without settings");
    assert_eq!(format!("{error:?}"), "WechatSettingsNotFound");
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_wechat_settings_test_revalidates_saved_settings(pool: PgPool) {
    let service = PgWechatNotifyService::new();
    let ctx = ctx();
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO h4_wechat_settings (
            id, owner_id, corp_id, agent_id, secret_alias, callback_token_alias,
            aes_key_alias, callback_url, approval_callback_path, enabled,
            retry_max_attempts, retry_interval_seconds, created_by, updated_by,
            created_at, updated_at, version
        )
        VALUES ($1, $2, 'ww-corp', '1000002', 'secret-alias', 'token-alias',
                'aes-alias', '', '/callback', TRUE, 3, 60, $3, $3, $4, $4, 1)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(ctx.user_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("invalid legacy settings should insert for test");

    let error = service
        .test_wechat_settings(&pool, &ctx, now)
        .await
        .expect_err("saved settings should be revalidated");

    assert_eq!(format!("{error:?}"), "InvalidRequest");
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_permissions_are_granted_to_system_admin(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    sqlx::query("ALTER TABLE auth_roles DISABLE TRIGGER auth_roles_grant_system_admin_permissions")
        .execute(&pool)
        .await
        .expect("automatic system admin grant should be disabled for migration 002 test");
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'H4 test owner')",
    )
    .bind(owner_id)
    .bind(format!("H4-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("owner should insert");
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'system_admin', '系统管理员')",
    )
    .bind(role_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("system admin role should insert");
    let count_before_backfill: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM auth_role_permissions WHERE role_id = $1")
            .bind(role_id)
            .fetch_one(&pool)
            .await
            .expect("pre-backfill permissions should query");
    assert_eq!(
        count_before_backfill, 0,
        "test must prove migration 002 performs the backfill"
    );
    sqlx::raw_sql(include_str!(
        "../../../migrations/202607100002_h4_system_admin_permissions.sql"
    ))
    .execute(&pool)
    .await
    .expect("H4 permission migration should backfill existing system admin");
    sqlx::query("ALTER TABLE auth_roles ENABLE TRIGGER auth_roles_grant_system_admin_permissions")
        .execute(&pool)
        .await
        .expect("automatic system admin grant should be restored");

    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM auth_role_permissions role_permission
          JOIN auth_roles role ON role.id = role_permission.role_id
          JOIN auth_permissions permission ON permission.id = role_permission.permission_id
         WHERE role.role_code = 'system_admin'
           AND permission.permission_code = ANY($1)
        "#,
    )
    .bind(vec![
        "h4.notify.read",
        "h4.notify.write",
        "h4.notify.send",
        "h4.approval.write",
    ])
    .fetch_one(&pool)
    .await
    .expect("system admin H4 permissions should query");

    assert_eq!(count, 4);
}

#[sqlx::test(migrations = "../../migrations")]
async fn system_admin_created_after_migrations_receives_all_registered_permissions(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'Late role owner')",
    )
    .bind(owner_id)
    .bind(format!("LATE-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("owner should insert");
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'system_admin', '系统管理员')",
    )
    .bind(role_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("system admin role should insert after migrations");

    let permission_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM auth_role_permissions WHERE role_id = $1")
            .bind(role_id)
            .fetch_one(&pool)
            .await
            .expect("system admin permissions should query");
    let registered_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM auth_permissions")
        .fetch_one(&pool)
        .await
        .expect("registered permissions should query");

    assert_eq!(permission_count, registered_count);
    assert!(registered_count > 0);

    let rename_error =
        sqlx::query("UPDATE auth_roles SET role_code = 'warehouse_manager' WHERE id = $1")
            .bind(role_id)
            .execute(&pool)
            .await
            .expect_err("built-in system_admin role code must be immutable");
    assert!(rename_error
        .to_string()
        .contains("system_admin role_code is immutable"));

    let second_owner_id = Uuid::new_v4();
    let operator_role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'Reserved role owner')",
    )
    .bind(second_owner_id)
    .bind(format!("RESERVED-{}", Uuid::new_v4()))
    .execute(&pool)
    .await
    .expect("second owner should insert");
    sqlx::query(
        "INSERT INTO auth_roles (id, owner_id, role_code, role_name) VALUES ($1, $2, 'warehouse_operator', '仓库操作员')",
    )
    .bind(operator_role_id)
    .bind(second_owner_id)
    .execute(&pool)
    .await
    .expect("operator role should insert");
    let promotion_error =
        sqlx::query("UPDATE auth_roles SET role_code = 'system_admin' WHERE id = $1")
            .bind(operator_role_id)
            .execute(&pool)
            .await
            .expect_err("system_admin role code must be reserved");
    assert!(promotion_error
        .to_string()
        .contains("system_admin role_code is immutable"));
}
