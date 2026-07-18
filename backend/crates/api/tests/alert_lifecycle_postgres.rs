use std::sync::Arc;

use chrono::{Duration, TimeZone, Utc};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_api::{
    alert_definition_repository::PgAlertDefinitionRepository,
    alert_engine_job::run_once_with_provider,
    alert_lifecycle_service::{AlertLifecycleError, PgAlertLifecycleService},
    auth::AuthContext,
    h2_lifecycle::{publish_event, upsert_event_subscription},
    wechat_notify_service::{
        WechatProvider, WechatProviderError, WechatProviderFuture, WechatProviderRequest,
    },
};
use wms_domain::CreateAlertDefinitionRequest;

#[derive(Clone, Default)]
struct RecordingProvider {
    recipients: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone, Default)]
struct AlwaysRetryableProvider {
    attempts: Arc<Mutex<usize>>,
}

impl WechatProvider for AlwaysRetryableProvider {
    fn send<'a>(&'a self, _request: WechatProviderRequest) -> WechatProviderFuture<'a> {
        Box::pin(async move {
            *self.attempts.lock().await += 1;
            Err(WechatProviderError::Retryable(
                "simulated H4 outage".to_string(),
            ))
        })
    }
}

impl WechatProvider for RecordingProvider {
    fn send<'a>(&'a self, request: WechatProviderRequest) -> WechatProviderFuture<'a> {
        Box::pin(async move {
            self.recipients.lock().await.push(request.recipient);
            Ok(())
        })
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn h2_event_creates_notifies_and_silences_alert_instance(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 15, 8, 0, 0)
        .single()
        .expect("fixed alert timestamp should be valid");
    seed_owner_manager_and_h4(&pool, owner_id, manager_id).await;
    upsert_event_subscription(&pool, owner_id, "hal-alert-engine", "business.*", true, now)
        .await
        .expect("H-AL event subscription should configure");
    let definition = PgAlertDefinitionRepository::new(pool.clone())
        .create(
            owner_id,
            &CreateAlertDefinitionRequest {
                alert_code: "inventory.low.runtime".to_string(),
                name: "库存阈值运行告警".to_string(),
                event_type: "business.inventory.changed".to_string(),
                condition_expression: r#"{"field":"quantity","op":"lt","value":10}"#.to_string(),
                default_severity: "warning".to_string(),
                recipient_roles: vec!["warehouse_manager".to_string()],
                escalation_ref: None,
                silence_period_seconds: 300,
                is_disable_allowed: true,
                message_template: "库存低于阈值：{{product_code}}".to_string(),
                is_gsp_forced: false,
            },
            now,
        )
        .await
        .expect("runtime alert definition should create");

    publish_event(
        &pool,
        owner_id,
        "hal-runtime-event-1",
        "business.inventory.changed",
        "M3",
        "inventory_batch",
        "BATCH-001",
        serde_json::json!({
            "quantity": 5,
            "product_code": "P-001",
            "warehouse_id": Uuid::new_v4(),
            "resource_path": "/inventory/batches/BATCH-001"
        }),
        now,
    )
    .await
    .expect("business event should publish");
    let provider = RecordingProvider::default();
    assert_eq!(
        run_once_with_provider(&pool, now, &provider)
            .await
            .expect("H-AL job should process event"),
        1
    );

    let instance: (String, String, Vec<String>, serde_json::Value) = sqlx::query_as(
        "SELECT alert_code, status, recipients, event_payload FROM alert_instances WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("alert instance should persist");
    assert_eq!(instance.0, "inventory.low.runtime");
    assert_eq!(instance.1, "notified");
    assert_eq!(instance.2.len(), 1);
    assert_eq!(instance.3["product_code"], "P-001");
    assert_eq!(provider.recipients.lock().await.as_slice(), instance.2);

    publish_event(
        &pool,
        owner_id,
        "hal-runtime-event-2",
        "business.inventory.changed",
        "M3",
        "inventory_batch",
        "BATCH-001",
        serde_json::json!({"quantity": 4, "product_code": "P-001"}),
        now + Duration::minutes(1),
    )
    .await
    .expect("duplicate-window event should publish");
    run_once_with_provider(&pool, now + Duration::minutes(1), &provider)
        .await
        .expect("duplicate-window event should process");
    assert_eq!(alert_count(&pool, owner_id).await, 1);

    sqlx::query("UPDATE alert_definitions SET enabled = FALSE WHERE id = $1")
        .bind(definition.id)
        .execute(&pool)
        .await
        .expect("definition should disable for runtime test");
    publish_event(
        &pool,
        owner_id,
        "hal-runtime-event-3",
        "business.inventory.changed",
        "M3",
        "inventory_batch",
        "BATCH-002",
        serde_json::json!({"quantity": 3, "product_code": "P-002"}),
        now + Duration::minutes(6),
    )
    .await
    .expect("disabled-definition event should publish");
    run_once_with_provider(&pool, now + Duration::minutes(6), &provider)
        .await
        .expect("disabled-definition event should process");
    assert_eq!(alert_count(&pool, owner_id).await, 1);

    let lifecycle: Vec<String> = sqlx::query_scalar(
        "SELECT to_status FROM alert_lifecycle_events WHERE owner_id = $1 ORDER BY event_sequence",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("alert lifecycle should query");
    assert_eq!(lifecycle, vec!["triggered", "notified"]);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H-AL' AND resource_type = 'alert_instance'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("alert runtime audit should query");
    assert_eq!(audit_count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn recipient_acknowledges_handles_closes_ignores_and_stale_alerts_auto_close(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 15, 9, 0, 0)
        .single()
        .expect("fixed lifecycle timestamp should be valid");
    seed_owner_manager_and_h4(&pool, owner_id, manager_id).await;
    let definition_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM alert_definitions WHERE owner_id = $1 ORDER BY alert_code LIMIT 1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("owner bootstrap should seed alert definitions");
    let alert_id = insert_alert_instance(&pool, owner_id, definition_id, "notified", now).await;
    let ctx = AuthContext {
        user_id: manager_id,
        owner_id,
        actor_name: "仓库主管".to_string(),
        permissions: vec!["hal.alert.handle".to_string()],
        jti: "hal-lifecycle-test".to_string(),
        warehouse_scope: None,
    };
    let service = PgAlertLifecycleService::new(pool.clone());

    service
        .acknowledge(&ctx, alert_id, now + Duration::minutes(1))
        .await
        .expect("notified alert should acknowledge");
    service
        .record_handling(
            &ctx,
            alert_id,
            "已派养护员现场复核".to_string(),
            now + Duration::minutes(2),
        )
        .await
        .expect("acknowledged alert should record handling");
    service
        .close(
            &ctx,
            alert_id,
            "现场复核完成".to_string(),
            now + Duration::minutes(3),
        )
        .await
        .expect("handled alert should close");
    let status: String = sqlx::query_scalar("SELECT status FROM alert_instances WHERE id = $1")
        .bind(alert_id)
        .fetch_one(&pool)
        .await
        .expect("closed status should query");
    assert_eq!(status, "closed");

    let ignored_id = insert_alert_instance(&pool, owner_id, definition_id, "notified", now).await;
    let missing_reason = service
        .ignore(
            &ctx,
            ignored_id,
            "  ".to_string(),
            now + Duration::minutes(1),
        )
        .await
        .expect_err("ignore should require reason");
    assert_eq!(missing_reason, AlertLifecycleError::ReasonRequired);
    service
        .ignore(
            &ctx,
            ignored_id,
            "重复业务事件，经人工确认忽略".to_string(),
            now + Duration::minutes(2),
        )
        .await
        .expect("ignore with reason should succeed");

    let stale_id = insert_alert_instance(
        &pool,
        owner_id,
        definition_id,
        "notified",
        now - Duration::days(8),
    )
    .await;
    assert_eq!(
        service
            .auto_close_stale(now)
            .await
            .expect("stale lifecycle should auto close"),
        1
    );
    let stale: (String, Option<String>) =
        sqlx::query_as("SELECT status, close_reason FROM alert_instances WHERE id = $1")
            .bind(stale_id)
            .fetch_one(&pool)
            .await
            .expect("stale close result should query");
    assert_eq!(stale.0, "closed");
    assert_eq!(stale.1.as_deref(), Some("超时未关闭，系统自动关闭"));

    let resolved_id = insert_alert_instance(&pool, owner_id, definition_id, "notified", now).await;
    assert_eq!(
        service
            .close_resolved_by_resource(
                owner_id,
                "test_resource",
                &resolved_id.to_string(),
                now + Duration::minutes(4),
            )
            .await
            .expect("business recovery should auto close matching alert"),
        1
    );
    let resolved_reason: Option<String> =
        sqlx::query_scalar("SELECT close_reason FROM alert_instances WHERE id = $1")
            .bind(resolved_id)
            .fetch_one(&pool)
            .await
            .expect("resolved close reason should query");
    assert_eq!(
        resolved_reason.as_deref(),
        Some("业务事件解除，系统自动关闭")
    );

    let audit_actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_event WHERE owner_id = $1 AND module = 'H-AL' AND resource_type = 'alert_instance' ORDER BY occurred_at, id",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("lifecycle audit should query");
    assert!(audit_actions.contains(&"alert.acknowledged".to_string()));
    assert!(audit_actions.contains(&"alert.handling_recorded".to_string()));
    assert!(audit_actions.contains(&"alert.closed".to_string()));
    assert!(audit_actions.contains(&"alert.ignored".to_string()));
    assert!(audit_actions.contains(&"alert.auto_closed".to_string()));
    assert!(audit_actions.contains(&"alert.resolved".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn h4_retry_exhaustion_marks_failure_and_creates_secondary_alert(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 15, 10, 0, 0)
        .single()
        .expect("fixed retry timestamp should be valid");
    seed_owner_manager_and_h4(&pool, owner_id, manager_id).await;
    upsert_event_subscription(&pool, owner_id, "hal-alert-engine", "business.*", true, now)
        .await
        .expect("H-AL event subscription should configure");
    PgAlertDefinitionRepository::new(pool.clone())
        .create(
            owner_id,
            &CreateAlertDefinitionRequest {
                alert_code: "inventory.low.retry".to_string(),
                name: "库存阈值通知失败测试".to_string(),
                event_type: "business.inventory.changed".to_string(),
                condition_expression: r#"{"field":"quantity","op":"lt","value":10}"#.to_string(),
                default_severity: "warning".to_string(),
                recipient_roles: vec!["warehouse_manager".to_string()],
                escalation_ref: None,
                silence_period_seconds: 300,
                is_disable_allowed: true,
                message_template: "库存低于阈值".to_string(),
                is_gsp_forced: false,
            },
            now,
        )
        .await
        .expect("retry alert definition should create");
    publish_event(
        &pool,
        owner_id,
        "hal-runtime-retry-event",
        "business.inventory.changed",
        "M3",
        "inventory_batch",
        "BATCH-RETRY",
        serde_json::json!({"quantity": 1, "product_code": "P-RETRY"}),
        now,
    )
    .await
    .expect("retry test event should publish");

    let provider = AlwaysRetryableProvider::default();
    assert_eq!(
        run_once_with_provider(&pool, now, &provider)
            .await
            .expect("retry exhaustion should remain a handled alert outcome"),
        1
    );
    assert_eq!(*provider.attempts.lock().await, 4);
    let alerts: Vec<(String, String)> = sqlx::query_as(
        "SELECT alert_code, status FROM alert_instances WHERE owner_id = $1 ORDER BY alert_code",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("primary and secondary alerts should query");
    assert_eq!(
        alerts,
        vec![
            (
                "alert.notification_failed".to_string(),
                "triggered".to_string()
            ),
            (
                "inventory.low.retry".to_string(),
                "notification_failed".to_string()
            ),
        ]
    );
}

async fn seed_owner_manager_and_h4(pool: &PgPool, owner_id: Uuid, manager_id: Uuid) {
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '告警生命周期测试货主')")
        .bind(owner_id)
        .bind(format!("ALRT-{}", &owner_id.to_string()[..8]))
        .execute(pool)
        .await
        .expect("alert lifecycle owner should seed");
    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '仓库主管', 'test-hash', 'active')")
        .bind(manager_id)
        .bind(format!("alert-manager-{}", &manager_id.to_string()[..8]))
        .execute(pool)
        .await
        .expect("alert manager should seed");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)")
        .bind(manager_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("alert manager owner binding should seed");
    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'warehouse_manager'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("owner bootstrap should seed warehouse manager role");
    sqlx::query("INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3)")
        .bind(manager_id)
        .bind(owner_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("warehouse manager assignment should seed");
    sqlx::query(
        r#"
        INSERT INTO h4_notification_configs (
            id, owner_id, event_type, enabled, template, recipient_rule,
            channels, created_by, updated_by
        ) VALUES (
            $1, $2, 'business.inventory.changed', TRUE,
            '库存低于阈值：{{product_code}}',
            '{"roles":["warehouse_manager"]}'::jsonb,
            ARRAY['wechat']::text[], $3, $3
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(manager_id)
    .execute(pool)
    .await
    .expect("alert H4 config should seed");
    sqlx::query(
        r#"
        INSERT INTO h4_wechat_settings (
            id, owner_id, corp_id, agent_id, secret_alias, callback_token_alias,
            aes_key_alias, callback_url, approval_callback_path, enabled,
            retry_max_attempts, retry_interval_seconds, created_by, updated_by
        ) VALUES (
            $1, $2, 'corp', 'agent', 'secret', 'token', 'aes',
            'https://example.test/callback', '/api/v1/wechat-notify/callback', TRUE,
            3, 1, $3, $3
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(manager_id)
    .execute(pool)
    .await
    .expect("alert H4 settings should seed");
}

async fn alert_count(pool: &PgPool, owner_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM alert_instances WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_one(pool)
        .await
        .expect("alert count should query")
}

async fn insert_alert_instance(
    pool: &PgPool,
    owner_id: Uuid,
    definition_id: Uuid,
    status: &str,
    triggered_at: chrono::DateTime<Utc>,
) -> Uuid {
    let id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO event_bus_event (
            id, owner_id, idempotency_key, event_type, source_module,
            resource_type, resource_id, payload, created_at
        ) VALUES ($1, $2, $3, 'business.test', 'TEST', 'test_resource', $4, '{}'::jsonb, $5)
        "#,
    )
    .bind(event_id)
    .bind(owner_id)
    .bind(format!("test-event:{id}"))
    .bind(id.to_string())
    .bind(triggered_at)
    .execute(pool)
    .await
    .expect("test event should insert");
    sqlx::query(
        r#"
        INSERT INTO alert_instances (
            id, owner_id, alert_definition_id, alert_code, severity,
            event_id, event_type, resource_type, resource_id, event_payload,
            recipients, status, dedup_key, triggered_at, created_at, updated_at
        ) VALUES (
            $1, $2, $3, 'test.lifecycle', 'warning', $4,
            'business.test', 'test_resource', $5, '{}'::jsonb,
            ARRAY['manager'], $6, $7, $8, $8, $8
        )
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(definition_id)
    .bind(event_id)
    .bind(id.to_string())
    .bind(status)
    .bind(format!("test:{id}"))
    .bind(triggered_at)
    .execute(pool)
    .await
    .expect("test alert instance should insert");
    id
}
