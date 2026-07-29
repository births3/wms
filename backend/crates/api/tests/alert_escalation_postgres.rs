use std::sync::Arc;

use chrono::{Duration, TimeZone, Utc};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_api::{
    alert_escalation::{
        run_escalations_once_with_provider, AlertEscalationError, PgAlertEscalationRepository,
    },
    auth::AuthContext,
    wechat_notify_service::{WechatProvider, WechatProviderFuture, WechatProviderRequest},
};
use wms_domain::{AlertEscalationLevelDraft, UpsertAlertEscalationRuleRequest};

#[derive(Clone, Default)]
struct RecordingProvider {
    recipients: Arc<Mutex<Vec<String>>>,
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
async fn upsert_alert_escalation_rule_limits_levels_and_job_escalates_once_then_stops_after_ack(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 15, 10, 0, 0)
        .single()
        .expect("fixed escalation timestamp should be valid");
    seed_owner_manager_and_h4(&pool, owner_id, manager_id).await;
    let ctx = AuthContext {
        user_id: manager_id,
        owner_id,
        actor_name: "告警管理员".to_string(),
        permissions: vec!["hal.escalation.write".to_string()],
        jti: "hal-escalation-test".to_string(),
        warehouse_scope: None,
    };
    let repository = PgAlertEscalationRepository::new(pool.clone());
    let invalid = rule_request(
        "too-many",
        vec![
            level(1, 1800, "warehouse_manager"),
            level(2, 7200, "warehouse_manager"),
            level(3, 86400, "system_admin"),
            level(4, 172800, "system_admin"),
        ],
    );
    assert_eq!(
        repository
            .upsert(&ctx, invalid, now)
            .await
            .expect_err("more than three escalation levels must fail"),
        AlertEscalationError::TooManyLevels
    );
    repository
        .upsert(
            &ctx,
            rule_request(
                "critical-default",
                vec![
                    level(1, 1800, "warehouse_manager"),
                    level(2, 7200, "warehouse_manager"),
                    level(3, 86400, "system_admin"),
                ],
            ),
            now,
        )
        .await
        .expect("three-level escalation rule should save");
    let rule_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event
         WHERE owner_id = $1
           AND action = 'alert.escalation_rule.upserted'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("escalation rule audit should query");
    assert_eq!(rule_audit_count, 1);
    let alert_id = seed_escalating_alert(
        &pool,
        owner_id,
        "critical-default",
        now - Duration::minutes(31),
    )
    .await;
    let provider = RecordingProvider::default();
    assert_eq!(
        run_escalations_once_with_provider(&pool, now, &provider)
            .await
            .expect("due alert should escalate"),
        1
    );
    let state: (String, i32) =
        sqlx::query_as("SELECT status, escalation_level FROM alert_instances WHERE id = $1")
            .bind(alert_id)
            .fetch_one(&pool)
            .await
            .expect("escalated alert should query");
    assert_eq!(state, ("escalated".to_string(), 1));
    let recipients = provider.recipients.lock().await.clone();
    assert!(recipients.contains(&"primary-operator".to_string()));
    assert!(recipients
        .iter()
        .any(|recipient| recipient.starts_with("alert-manager-")));
    assert_eq!(
        run_escalations_once_with_provider(&pool, now, &provider)
            .await
            .expect("same level should be idempotent"),
        0
    );

    sqlx::query(
        "UPDATE alert_instances SET status = 'acknowledged', acknowledged_at = $2 WHERE id = $1",
    )
    .bind(alert_id)
    .bind(now + Duration::minutes(1))
    .execute(&pool)
    .await
    .expect("test should acknowledge alert");
    assert_eq!(
        run_escalations_once_with_provider(&pool, now + Duration::hours(3), &provider)
            .await
            .expect("acknowledged alert should stop escalation"),
        0
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn l3_repeats_every_24_hours_and_empty_off_hours_route_falls_back_to_system_admin(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 15, 22, 0, 0)
        .single()
        .expect("fixed off-hours timestamp should be valid");
    seed_owner_manager_and_h4(&pool, owner_id, manager_id).await;
    seed_system_admin(&pool, owner_id, admin_id).await;
    let ctx = AuthContext {
        user_id: manager_id,
        owner_id,
        actor_name: "告警管理员".to_string(),
        permissions: vec!["hal.escalation.write".to_string()],
        jti: "hal-l3-repeat-test".to_string(),
        warehouse_scope: None,
    };
    let mut request = rule_request(
        "l3-repeat",
        vec![
            level(1, 1800, "warehouse_manager"),
            level(2, 7200, "warehouse_manager"),
            level(3, 86400, "system_admin"),
        ],
    );
    request.off_hours_handler_roles = vec!["departed_night_manager".to_string()];
    PgAlertEscalationRepository::new(pool.clone())
        .upsert(&ctx, request, now)
        .await
        .expect("L3 repeat rule should save");
    let alert_id =
        seed_escalating_alert(&pool, owner_id, "l3-repeat", now - Duration::days(2)).await;
    sqlx::query(
        "UPDATE alert_instances SET status = 'escalated', escalation_level = 3, last_escalated_at = $2 WHERE id = $1",
    )
    .bind(alert_id)
    .bind(now - Duration::hours(25))
    .execute(&pool)
    .await
    .expect("alert should seed at L3");
    let provider = RecordingProvider::default();
    assert_eq!(
        run_escalations_once_with_provider(&pool, now, &provider)
            .await
            .expect("L3 should repeat after 24 hours"),
        1
    );
    let recipients = provider.recipients.lock().await.clone();
    assert!(recipients.contains(&"primary-operator".to_string()));
    assert!(recipients
        .iter()
        .any(|recipient| recipient.starts_with("alert-admin-")));
    assert_eq!(
        run_escalations_once_with_provider(&pool, now, &provider)
            .await
            .expect("same L3 repeat bucket should be idempotent"),
        0
    );
    assert_eq!(
        run_escalations_once_with_provider(&pool, now + Duration::hours(24), &provider)
            .await
            .expect("L3 should send again after another 24 hours"),
        1
    );
}

fn level(level: i32, threshold_seconds: i64, role: &str) -> AlertEscalationLevelDraft {
    AlertEscalationLevelDraft {
        level,
        threshold_seconds,
        recipient_roles: vec![role.to_string()],
    }
}

fn rule_request(
    rule_code: &str,
    levels: Vec<AlertEscalationLevelDraft>,
) -> UpsertAlertEscalationRuleRequest {
    UpsertAlertEscalationRuleRequest {
        rule_code: rule_code.to_string(),
        rule_name: "关键告警默认升级".to_string(),
        notify_lower_levels: true,
        off_hours_start: "18:00".to_string(),
        off_hours_end: "08:00".to_string(),
        off_hours_handler_roles: vec!["system_admin".to_string()],
        holiday_dates: Vec::new(),
        enabled: true,
        levels,
    }
}

async fn seed_owner_manager_and_h4(pool: &PgPool, owner_id: Uuid, manager_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '升级测试货主')",
    )
    .bind(owner_id)
    .bind(format!("ALES-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("owner should seed");
    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '告警主管', 'test-hash', 'active')")
        .bind(manager_id)
        .bind(format!("alert-manager-{}", &manager_id.to_string()[..8]))
        .execute(pool)
        .await
        .expect("manager should seed");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)")
        .bind(manager_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("manager binding should seed");
    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'warehouse_manager'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("warehouse manager role should exist");
    sqlx::query("INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3)")
        .bind(manager_id)
        .bind(owner_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("manager role should assign");
    sqlx::query(
        r#"
        INSERT INTO h4_notification_configs (
            id, owner_id, event_type, enabled, template, recipient_rule,
            channels, created_by, updated_by
        ) VALUES ($1, $2, 'business.inventory.changed', TRUE, '升级：{{product_code}}',
                  '{}'::jsonb, ARRAY['wechat','email']::text[], $3, $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(manager_id)
    .execute(pool)
    .await
    .expect("H4 config should seed");
    sqlx::query(
        r#"
        INSERT INTO h4_wechat_settings (
            id, owner_id, corp_id, agent_id, secret_alias, callback_token_alias,
            aes_key_alias, callback_url, approval_callback_path, enabled,
            retry_max_attempts, retry_interval_seconds, created_by, updated_by
        ) VALUES ($1, $2, 'corp', 'agent', 'secret', 'token', 'aes',
                  'https://example.test/callback', '/callback', TRUE, 3, 1, $3, $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(manager_id)
    .execute(pool)
    .await
    .expect("H4 settings should seed");
}

async fn seed_system_admin(pool: &PgPool, owner_id: Uuid, admin_id: Uuid) {
    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '系统管理员', 'test-hash', 'active')")
        .bind(admin_id)
        .bind(format!("alert-admin-{}", &admin_id.to_string()[..8]))
        .execute(pool)
        .await
        .expect("system admin should seed");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)")
        .bind(admin_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("system admin binding should seed");
    let role_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM auth_roles WHERE owner_id = $1 AND lower(role_code) = 'system_admin'",
    )
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .expect("owner bootstrap should seed system admin role");
    sqlx::query("INSERT INTO auth_user_roles (user_id, owner_id, role_id) VALUES ($1, $2, $3)")
        .bind(admin_id)
        .bind(owner_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("system admin role should assign");
}

async fn seed_escalating_alert(
    pool: &PgPool,
    owner_id: Uuid,
    escalation_ref: &str,
    triggered_at: chrono::DateTime<Utc>,
) -> Uuid {
    let definition_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO alert_definitions (
            id, owner_id, alert_code, name, event_type, condition_expression,
            default_severity, recipient_roles, escalation_ref, silence_period_seconds,
            is_disable_allowed, enabled, message_template, message_templates, is_gsp_forced
        ) VALUES ($1, $2, 'escalation.test', '升级测试告警', 'business.inventory.changed', '{}',
                  'critical', ARRAY['warehouse_manager'], $3, 300, TRUE, TRUE,
                  '升级测试', '{"zh-CN":"升级测试"}'::jsonb, FALSE)
        "#,
    )
    .bind(definition_id)
    .bind(owner_id)
    .bind(escalation_ref)
    .execute(pool)
    .await
    .expect("escalating definition should seed");
    let event_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO event_bus_event (id, owner_id, idempotency_key, event_type, source_module, resource_type, resource_id, payload, created_at) VALUES ($1, $2, $3, 'business.inventory.changed', 'M3', 'inventory_batch', 'BATCH-ESC', '{\"product_code\":\"P-ESC\"}'::jsonb, $4)",
    )
    .bind(event_id)
    .bind(owner_id)
    .bind(format!("esc:{event_id}"))
    .bind(triggered_at)
    .execute(pool)
    .await
    .expect("escalation event should seed");
    let alert_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO alert_instances (
            id, owner_id, alert_definition_id, alert_code, severity, event_id,
            event_type, resource_type, resource_id, event_payload, recipients,
            status, dedup_key, triggered_at, notified_at, created_at, updated_at
        ) VALUES ($1, $2, $3, 'escalation.test', 'critical', $4,
                  'business.inventory.changed', 'inventory_batch', 'BATCH-ESC',
                  '{"product_code":"P-ESC"}'::jsonb, ARRAY['primary-operator'],
                  'notified', $5, $6, $6, $6, $6)
        "#,
    )
    .bind(alert_id)
    .bind(owner_id)
    .bind(definition_id)
    .bind(event_id)
    .bind(format!("esc-alert:{alert_id}"))
    .bind(triggered_at)
    .execute(pool)
    .await
    .expect("escalating alert should seed");
    alert_id
}
