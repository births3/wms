use chrono::{Duration, TimeZone, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use wms_api::{
    alert_dashboard::{process_queued_exports_with_provider, PgAlertDashboardService},
    auth::AuthContext,
    wechat_notify_service::{WechatProvider, WechatProviderFuture, WechatProviderRequest},
};
use wms_domain::{AlertInstanceListQuery, CreateAlertExportRequest};

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
async fn dashboard_statistics_gsp_report_and_excel_pdf_exports_are_audited(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 15, 12, 0, 0)
        .single()
        .expect("fixed dashboard timestamp should be valid");
    seed_owner_user(&pool, owner_id, user_id).await;
    let definition_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM alert_definitions WHERE owner_id = $1 AND alert_code = 'qualification_expiry_30d'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("GSP alert definition should exist");
    insert_alert(
        &pool,
        owner_id,
        definition_id,
        "critical",
        "acknowledged",
        now - Duration::minutes(20),
        Some(60),
        1,
        "manager-a",
    )
    .await;
    insert_alert(
        &pool,
        owner_id,
        definition_id,
        "warning",
        "closed",
        now - Duration::minutes(10),
        Some(120),
        0,
        "manager-a",
    )
    .await;
    insert_alert(
        &pool,
        owner_id,
        definition_id,
        "info",
        "notified",
        now - Duration::minutes(5),
        None,
        0,
        "manager-b",
    )
    .await;
    let ctx = AuthContext {
        user_id,
        owner_id,
        actor_name: "GSP 审计员".to_string(),
        permissions: vec![
            "hal.alert.read".to_string(),
            "hal.alert.read.all".to_string(),
            "hal.alert.report".to_string(),
        ],
        jti: "hal-dashboard-test".to_string(),
        warehouse_scope: None,
    };
    let service = PgAlertDashboardService::new(pool.clone());

    let active = service
        .list_active(&ctx, AlertInstanceListQuery::default(), now)
        .await
        .expect("active dashboard should query");
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].severity, "critical");
    assert!(active.iter().all(|alert| alert.status != "closed"));

    let statistics = service
        .statistics(&ctx, AlertInstanceListQuery::default(), now)
        .await
        .expect("monthly alert statistics should query");
    assert_eq!(statistics.monthly.len(), 1);
    assert_eq!(statistics.monthly[0].triggered_count, 3);
    assert!((statistics.monthly[0].acknowledgement_rate - 2.0 / 3.0).abs() < 0.0001);
    assert_eq!(statistics.monthly[0].average_response_seconds, Some(90.0));
    assert!((statistics.monthly[0].escalation_rate - 1.0 / 3.0).abs() < 0.0001);
    assert_eq!(statistics.alert_type_top10[0].count, 3);
    assert_eq!(statistics.recipient_top10[0].key, "manager-b");
    assert_eq!(statistics.recipient_top10[0].unacknowledged_count, 1);
    assert!(!statistics.possibly_stale);

    sqlx::query("ALTER TABLE alert_instances RENAME TO alert_instances_unavailable")
        .execute(&pool)
        .await
        .expect("live statistics source should become unavailable for fallback test");
    let cached_statistics = service
        .statistics(
            &ctx,
            AlertInstanceListQuery::default(),
            now + Duration::seconds(1),
        )
        .await
        .expect("same-scope statistics should fall back to the latest snapshot");
    assert!(cached_statistics.possibly_stale);
    assert_eq!(cached_statistics.generated_at, statistics.generated_at);
    assert_eq!(cached_statistics.monthly[0].triggered_count, 3);
    sqlx::query("ALTER TABLE alert_instances_unavailable RENAME TO alert_instances")
        .execute(&pool)
        .await
        .expect("live statistics source should be restored for remaining assertions");

    let gsp = service
        .gsp_report(&ctx, AlertInstanceListQuery::default(), now)
        .await
        .expect("GSP lifecycle report should query");
    assert_eq!(gsp.data.len(), 3);
    assert!(gsp.data.iter().all(|record| record
        .lifecycle_events
        .as_array()
        .is_some_and(|events| !events.is_empty())));

    let excel = service
        .create_export(
            &ctx,
            CreateAlertExportRequest {
                format: "excel".to_string(),
                filters: AlertInstanceListQuery::default(),
                recipient_email: None,
            },
            now,
        )
        .await
        .expect("small Excel export should generate synchronously");
    assert_eq!(excel.status, "ready");
    let excel_token = download_token(&pool, excel.id).await;
    let (excel_bytes, excel_type, _) = service
        .download(owner_id, excel_token, now)
        .await
        .expect("Excel download should be ready");
    assert!(excel_bytes.starts_with(b"\xEF\xBB\xBF"));
    assert!(excel_type.contains("excel"));
    assert!(String::from_utf8_lossy(&excel_bytes).contains("filters"));

    let pdf = service
        .create_export(
            &ctx,
            CreateAlertExportRequest {
                format: "pdf".to_string(),
                filters: AlertInstanceListQuery::default(),
                recipient_email: None,
            },
            now,
        )
        .await
        .expect("small PDF export should generate synchronously");
    let pdf_token = download_token(&pool, pdf.id).await;
    let (pdf_bytes, pdf_type, _) = service
        .download(owner_id, pdf_token, now)
        .await
        .expect("PDF download should be ready");
    assert!(pdf_bytes.starts_with(b"%PDF-1.4"));
    assert_eq!(pdf_type, "application/pdf");

    let audit_actions: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_event WHERE owner_id = $1 AND module = 'H-AL' AND resource_type = 'alert_report' ORDER BY occurred_at, id",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("dashboard query audit should query");
    assert!(audit_actions.contains(&"alert.dashboard.queried".to_string()));
    assert!(audit_actions.contains(&"alert.statistics.queried".to_string()));
    assert!(audit_actions.contains(&"alert.gsp_report.queried".to_string()));
    assert_eq!(
        audit_actions
            .iter()
            .filter(|action| action.as_str() == "alert.export.requested")
            .count(),
        2
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn queued_large_export_generates_at_most_100k_and_emails_download_link(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 15, 13, 0, 0)
        .single()
        .expect("fixed async export timestamp should be valid");
    seed_owner_user(&pool, owner_id, user_id).await;
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
    .bind(user_id)
    .execute(&pool)
    .await
    .expect("export H4 settings should seed");
    let export_id = Uuid::new_v4();
    let download_token = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO alert_report_exports (
            id, owner_id, requested_by, format, status, filters, row_count,
            download_token, recipient_email, email_notification_status,
            created_at, updated_at, expires_at
        ) VALUES ($1, $2, $3, 'excel', 'queued', '{}'::jsonb, 100001,
                  $4, 'auditor@example.test', 'pending', $5, $5, $5 + INTERVAL '7 days')
        "#,
    )
    .bind(export_id)
    .bind(owner_id)
    .bind(user_id)
    .bind(download_token)
    .bind(now)
    .execute(&pool)
    .await
    .expect("queued export should seed");
    let provider = RecordingProvider::default();
    assert_eq!(
        process_queued_exports_with_provider(&pool, now + Duration::seconds(1), &provider)
            .await
            .expect("queued export worker should complete"),
        1
    );
    let state: (String, Option<String>, bool) = sqlx::query_as(
        "SELECT status, email_notification_status, content IS NOT NULL FROM alert_report_exports WHERE id = $1",
    )
    .bind(export_id)
    .fetch_one(&pool)
    .await
    .expect("completed export should query");
    assert_eq!(state, ("ready".to_string(), Some("sent".to_string()), true));
    assert_eq!(
        provider.recipients.lock().await.as_slice(),
        ["auditor@example.test"]
    );
    let h4_payload: String = sqlx::query_scalar(
        "SELECT content FROM h4_notification_records WHERE owner_id = $1 AND event_type = 'hal.alert.export.ready'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("email download notification should persist");
    assert!(h4_payload.contains(&download_token.to_string()));
}

async fn seed_owner_user(pool: &PgPool, owner_id: Uuid, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '告警看板测试货主')",
    )
    .bind(owner_id)
    .bind(format!("ALDB-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("dashboard owner should seed");
    sqlx::query("INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, 'GSP 审计员', 'test-hash', 'active')")
        .bind(user_id)
        .bind(format!("gsp-auditor-{}", &user_id.to_string()[..8]))
        .execute(pool)
        .await
        .expect("dashboard user should seed");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)")
        .bind(user_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("dashboard user binding should seed");
}

#[allow(clippy::too_many_arguments)]
async fn insert_alert(
    pool: &PgPool,
    owner_id: Uuid,
    definition_id: Uuid,
    severity: &str,
    status: &str,
    triggered_at: chrono::DateTime<Utc>,
    response_seconds: Option<i64>,
    escalation_level: i32,
    recipient: &str,
) -> Uuid {
    let event_id = Uuid::new_v4();
    sqlx::query("INSERT INTO event_bus_event (id, owner_id, idempotency_key, event_type, source_module, resource_type, resource_id, payload, created_at) VALUES ($1, $2, $3, 'qualification.expiry', 'M1', 'supplier', $4, '{}'::jsonb, $5)")
        .bind(event_id)
        .bind(owner_id)
        .bind(format!("dashboard:{event_id}"))
        .bind(event_id.to_string())
        .bind(triggered_at)
        .execute(pool)
        .await
        .expect("dashboard source event should seed");
    let id = Uuid::new_v4();
    let acknowledged_at = response_seconds.map(|seconds| triggered_at + Duration::seconds(seconds));
    let closed_at = (status == "closed").then(|| triggered_at + Duration::minutes(4));
    sqlx::query(
        r#"
        INSERT INTO alert_instances (
            id, owner_id, alert_definition_id, alert_code, severity, event_id,
            event_type, resource_type, resource_id, event_payload, recipients,
            status, dedup_key, escalation_level, triggered_at, notified_at,
            acknowledged_at, closed_at, created_at, updated_at
        ) VALUES ($1, $2, $3, 'qualification_expiry_30d', $4, $5,
                  'qualification.expiry', 'supplier', $6, '{}'::jsonb, ARRAY[$7],
                  $8, $9, $10, $11, $11, $12, $13, $11, $11)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(definition_id)
    .bind(severity)
    .bind(event_id)
    .bind(id.to_string())
    .bind(recipient)
    .bind(status)
    .bind(format!("dashboard-alert:{id}"))
    .bind(escalation_level)
    .bind(triggered_at)
    .bind(acknowledged_at)
    .bind(closed_at)
    .execute(pool)
    .await
    .expect("dashboard alert should seed");
    sqlx::query(
        "INSERT INTO alert_lifecycle_events (id, owner_id, alert_instance_id, from_status, to_status, actor_name, occurred_at, created_at) VALUES ($1, $2, $3, NULL, $4, 'dashboard-test', $5, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(id)
    .bind(status)
    .bind(triggered_at)
    .execute(pool)
    .await
    .expect("dashboard lifecycle should seed");
    id
}

async fn download_token(pool: &PgPool, export_id: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT download_token FROM alert_report_exports WHERE id = $1")
        .bind(export_id)
        .fetch_one(pool)
        .await
        .expect("download token should query")
}
