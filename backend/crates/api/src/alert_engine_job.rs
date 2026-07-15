//! H-AL event-bus consumer: definition matching, silence/dedup and H4 delegation.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{matches_alert_condition, SendH4NotificationRequest};

use crate::{
    alert_lifecycle_service::PgAlertLifecycleService,
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    wechat_notify_service::{
        PgWechatNotifyService, WechatProvider, WechatProviderError, WechatProviderFuture,
        WechatProviderRequest,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertEngineJobError {
    Database(String),
    Audit(String),
    Notification(String),
}

#[derive(Debug, FromRow)]
struct PendingEvent {
    delivery_id: Uuid,
    event_id: Uuid,
    owner_id: Uuid,
    event_type: String,
    resource_type: String,
    resource_id: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RuntimeDefinition {
    id: Uuid,
    alert_code: String,
    condition_expression: String,
    default_severity: String,
    recipient_roles: Vec<String>,
    silence_period_seconds: i64,
}

struct NotificationPlan {
    instance_id: Uuid,
    definition_id: Uuid,
    event_id: Uuid,
    owner_id: Uuid,
    event_type: String,
    dedup_key: String,
    recipients: Vec<String>,
    payload: Value,
}

pub async fn run_once_with_provider(
    pool: &PgPool,
    now: DateTime<Utc>,
    provider: &dyn WechatProvider,
) -> Result<usize, AlertEngineJobError> {
    let mut created = 0_usize;
    loop {
        let mut tx = pool.begin().await.map_err(db_error)?;
        let Some(event) = lock_pending_event(&mut tx, now).await? else {
            tx.rollback().await.map_err(db_error)?;
            break;
        };
        let resolution = is_resolution_event(&event).then(|| {
            (
                event.owner_id,
                event.resource_type.clone(),
                event.resource_id.clone(),
            )
        });
        let plans = if resolution.is_some() {
            Vec::new()
        } else {
            build_notification_plans(&mut tx, &event, now).await?
        };
        sqlx::query(
            "UPDATE event_bus_delivery SET status = 'delivered', last_error = NULL, next_attempt_at = NULL, updated_at = $2 WHERE id = $1",
        )
        .bind(event.delivery_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        if let Some((owner_id, resource_type, resource_id)) = resolution {
            PgAlertLifecycleService::new(pool.clone())
                .close_resolved_by_resource(owner_id, &resource_type, &resource_id, now)
                .await
                .map_err(|error| AlertEngineJobError::Database(format!("{error:?}")))?;
        }
        created += plans.len();
        for plan in plans {
            notify(pool, plan, now, provider).await?;
        }
    }
    Ok(created)
}

fn is_resolution_event(event: &PendingEvent) -> bool {
    event.event_type.ends_with(".resolved")
        || event.event_type.ends_with(".recovered")
        || event.payload.get("resolved").and_then(Value::as_bool) == Some(true)
}

async fn lock_pending_event(
    tx: &mut Transaction<'_, Postgres>,
    now: DateTime<Utc>,
) -> Result<Option<PendingEvent>, AlertEngineJobError> {
    sqlx::query_as(
        r#"
        SELECT delivery.id AS delivery_id, event.id AS event_id, event.owner_id,
               event.event_type, event.resource_type, event.resource_id,
               event.payload, event.created_at
          FROM event_bus_delivery delivery
          JOIN event_bus_subscription subscription ON subscription.id = delivery.subscription_id
          JOIN event_bus_event event ON event.id = delivery.event_id
         WHERE subscription.subscriber_key = 'hal-alert-engine'
           AND subscription.active = TRUE
           AND delivery.status = 'pending'
           AND COALESCE(delivery.next_attempt_at, delivery.created_at) <= $1
         ORDER BY delivery.created_at, delivery.id
         FOR UPDATE OF delivery SKIP LOCKED
         LIMIT 1
        "#,
    )
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)
}

async fn build_notification_plans(
    tx: &mut Transaction<'_, Postgres>,
    event: &PendingEvent,
    now: DateTime<Utc>,
) -> Result<Vec<NotificationPlan>, AlertEngineJobError> {
    let definitions: Vec<RuntimeDefinition> = sqlx::query_as(
        r#"
        SELECT id, alert_code, condition_expression, default_severity,
               recipient_roles, silence_period_seconds
          FROM alert_definitions
         WHERE owner_id = $1 AND event_type = $2 AND enabled = TRUE
         ORDER BY alert_code
        "#,
    )
    .bind(event.owner_id)
    .bind(&event.event_type)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_error)?;
    let mut plans = Vec::new();
    for definition in definitions {
        let matches =
            match matches_alert_condition(&definition.condition_expression, &event.payload) {
                Ok(matches) => matches,
                Err(error) => {
                    tracing::warn!(
                        alert_code = %definition.alert_code,
                        event_id = %event.event_id,
                        error = ?error,
                        "invalid H-AL condition; event skipped"
                    );
                    false
                }
            };
        if !matches {
            continue;
        }
        let dedup_key = dedup_key(&definition, event);
        let recipients =
            resolve_recipients(tx, event.owner_id, &definition.recipient_roles).await?;
        let instance_id = Uuid::new_v4();
        let inserted = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO alert_instances (
                id, owner_id, alert_definition_id, alert_code, severity,
                event_id, event_type, resource_type, resource_id, resource_path,
                warehouse_id, event_payload, recipients, status, dedup_key,
                triggered_at, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, 'triggered', $14, $15, $15, $15
            )
            ON CONFLICT (owner_id, dedup_key) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(instance_id)
        .bind(event.owner_id)
        .bind(definition.id)
        .bind(&definition.alert_code)
        .bind(&definition.default_severity)
        .bind(event.event_id)
        .bind(&event.event_type)
        .bind(&event.resource_type)
        .bind(&event.resource_id)
        .bind(event.payload.get("resource_path").and_then(Value::as_str))
        .bind(
            event
                .payload
                .get("warehouse_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok()),
        )
        .bind(&event.payload)
        .bind(&recipients)
        .bind(&dedup_key)
        .bind(now)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_error)?;
        if inserted.is_none() {
            continue;
        }
        sqlx::query(
            "INSERT INTO alert_definition_triggers (id, alert_definition_id, event_type, occurred_at, payload) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(definition.id)
        .bind(&event.event_type)
        .bind(now)
        .bind(&event.payload)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
        append_lifecycle(
            tx,
            event.owner_id,
            instance_id,
            None,
            "triggered",
            None,
            now,
        )
        .await?;
        append_alert_audit(
            tx,
            event.owner_id,
            "alert.triggered",
            instance_id,
            serde_json::json!({"alert_code": definition.alert_code, "dedup_key": dedup_key}),
            now,
        )
        .await?;
        plans.push(NotificationPlan {
            instance_id,
            definition_id: definition.id,
            event_id: event.event_id,
            owner_id: event.owner_id,
            event_type: event.event_type.clone(),
            dedup_key,
            recipients,
            payload: event.payload.clone(),
        });
    }
    Ok(plans)
}

async fn notify(
    pool: &PgPool,
    plan: NotificationPlan,
    now: DateTime<Utc>,
    provider: &dyn WechatProvider,
) -> Result<(), AlertEngineJobError> {
    let ctx = AuthContext {
        user_id: Uuid::nil(),
        owner_id: plan.owner_id,
        actor_name: "system-alert-engine".to_string(),
        permissions: Vec::new(),
        jti: format!("hal:{}", plan.instance_id),
    };
    let retrying_provider = RetryingProvider {
        inner: provider,
        retry_attempts: 3,
    };
    let result = PgWechatNotifyService::new()
        .send_notification_with_provider(
            pool,
            &ctx,
            SendH4NotificationRequest {
                event_type: plan.event_type.clone(),
                dedupe_key: plan.dedup_key.clone(),
                recipients: plan.recipients.clone(),
                payload: plan.payload.clone(),
            },
            now,
            &format!("hal-notify:{}", plan.instance_id),
            &retrying_provider,
        )
        .await
        .map_err(|error| AlertEngineJobError::Notification(format!("{error:?}")))?;
    let notified = result.value.iter().all(|record| record.status == "success");
    let status = if notified {
        "notified"
    } else {
        "notification_failed"
    };
    let mut tx = pool.begin().await.map_err(db_error)?;
    sqlx::query(
        "UPDATE alert_instances SET status = $3, notified_at = CASE WHEN $3 = 'notified' THEN $4 ELSE NULL END, updated_at = $4 WHERE owner_id = $1 AND id = $2",
    )
    .bind(plan.owner_id)
    .bind(plan.instance_id)
    .bind(status)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    append_lifecycle(
        &mut tx,
        plan.owner_id,
        plan.instance_id,
        Some("triggered"),
        status,
        None,
        now,
    )
    .await?;
    append_alert_audit(
        &mut tx,
        plan.owner_id,
        if notified {
            "alert.notified"
        } else {
            "alert.notification_failed"
        },
        plan.instance_id,
        serde_json::json!({"status": status}),
        now,
    )
    .await?;
    if !notified {
        create_notification_failure_alert(&mut tx, &plan, now).await?;
    }
    tx.commit().await.map_err(db_error)?;
    Ok(())
}

struct RetryingProvider<'a> {
    inner: &'a dyn WechatProvider,
    retry_attempts: usize,
}

impl WechatProvider for RetryingProvider<'_> {
    fn send<'a>(&'a self, request: WechatProviderRequest) -> WechatProviderFuture<'a> {
        Box::pin(async move {
            for attempt in 0..=self.retry_attempts {
                match self.inner.send(request.clone()).await {
                    Ok(()) => return Ok(()),
                    Err(WechatProviderError::Retryable(_)) if attempt < self.retry_attempts => {}
                    Err(error) => return Err(error),
                }
            }
            unreachable!("retry loop always returns on final attempt")
        })
    }
}

async fn create_notification_failure_alert(
    tx: &mut Transaction<'_, Postgres>,
    plan: &NotificationPlan,
    now: DateTime<Utc>,
) -> Result<(), AlertEngineJobError> {
    let id = Uuid::new_v4();
    let dedup_key = format!("alert.notification_failed:{}", plan.instance_id);
    let inserted = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO alert_instances (
            id, owner_id, alert_definition_id, alert_code, severity,
            event_id, event_type, resource_type, resource_id, resource_path,
            event_payload, recipients, status, dedup_key,
            triggered_at, created_at, updated_at
        ) VALUES (
            $1, $2, $3, 'alert.notification_failed', 'critical',
            $4, 'hal.alert.notification_failed', 'alert_instance', $5, $6,
            $7, $8, 'triggered', $9, $10, $10, $10
        )
        ON CONFLICT (owner_id, dedup_key) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(plan.owner_id)
    .bind(plan.definition_id)
    .bind(plan.event_id)
    .bind(plan.instance_id.to_string())
    .bind(format!("/alerts/{}", plan.instance_id))
    .bind(serde_json::json!({
        "failed_alert_id": plan.instance_id,
        "reason": "H4 notification retry exhausted",
    }))
    .bind(&plan.recipients)
    .bind(&dedup_key)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    if inserted.is_some() {
        append_lifecycle(
            tx,
            plan.owner_id,
            id,
            None,
            "triggered",
            Some("H4 通知重试 3 次仍失败"),
            now,
        )
        .await?;
        append_alert_audit(
            tx,
            plan.owner_id,
            "alert.notification_failure_secondary_triggered",
            id,
            serde_json::json!({"failed_alert_id": plan.instance_id}),
            now,
        )
        .await?;
    }
    Ok(())
}

async fn resolve_recipients(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    roles: &[String],
) -> Result<Vec<String>, AlertEngineJobError> {
    let mut recipients: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT users.username
          FROM auth_user_roles user_role
          JOIN auth_roles role ON role.id = user_role.role_id AND role.owner_id = $1
          JOIN auth_users users ON users.id = user_role.user_id AND users.status = 'active'
          JOIN auth_user_owner_bindings binding
            ON binding.user_id = users.id AND binding.owner_id = $1 AND binding.is_active = TRUE
         WHERE user_role.owner_id = $1 AND role.role_code = ANY($2)
         ORDER BY users.username
        "#,
    )
    .bind(owner_id)
    .bind(roles)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_error)?;
    if recipients.is_empty() {
        recipients = sqlx::query_scalar(
            r#"
            SELECT DISTINCT users.username
              FROM auth_user_roles user_role
              JOIN auth_roles role ON role.id = user_role.role_id AND role.owner_id = $1
              JOIN auth_users users ON users.id = user_role.user_id AND users.status = 'active'
             WHERE user_role.owner_id = $1 AND lower(role.role_code) = 'system_admin'
             ORDER BY users.username
            "#,
        )
        .bind(owner_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(db_error)?;
    }
    Ok(recipients)
}

fn dedup_key(definition: &RuntimeDefinition, event: &PendingEvent) -> String {
    let window = if definition.silence_period_seconds > 0 {
        event.created_at.timestamp() / definition.silence_period_seconds
    } else {
        event.created_at.timestamp_micros()
    };
    format!(
        "{}:{}:{}:{}",
        definition.alert_code, event.resource_type, event.resource_id, window
    )
}

async fn append_lifecycle(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    instance_id: Uuid,
    from_status: Option<&str>,
    to_status: &str,
    description: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), AlertEngineJobError> {
    sqlx::query(
        r#"
        INSERT INTO alert_lifecycle_events (
            id, owner_id, alert_instance_id, from_status, to_status,
            action_description, actor_id, actor_name, occurred_at, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, NULL, 'system-alert-engine', $7, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(instance_id)
    .bind(from_status)
    .bind(to_status)
    .bind(description)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

async fn append_alert_audit(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    action: &str,
    instance_id: Uuid,
    after: Value,
    now: DateTime<Utc>,
) -> Result<(), AlertEngineJobError> {
    append_event_in_tx(
        tx,
        &AuditWriteRequest {
            occurred_at: now,
            actor_id: Uuid::nil(),
            actor_name: "system-alert-engine".to_string(),
            owner_id,
            jti: format!("hal:{}:{action}", instance_id),
            action: action.to_string(),
            module: "H-AL".to_string(),
            resource_type: "alert_instance".to_string(),
            resource_id: instance_id.to_string(),
            diff: Some(AuditDiff {
                before: serde_json::json!({}),
                after,
                changed_keys: vec!["status".to_string()],
            }),
            request_id: None,
            ip: None,
            user_agent: Some("wms-hal-engine".to_string()),
        },
    )
    .await
    .map_err(|error| AlertEngineJobError::Audit(format!("{error:?}")))?;
    Ok(())
}

fn db_error(error: sqlx::Error) -> AlertEngineJobError {
    AlertEngineJobError::Database(error.to_string())
}

pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        let provider = crate::wechat_notify_service::UnconfiguredWechatProvider;
        let lifecycle = PgAlertLifecycleService::new(pool.clone());
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let now = Utc::now();
            if let Err(error) = run_once_with_provider(&pool, now, &provider).await {
                tracing::error!(error = ?error, "H-AL event consumer failed");
            }
            if let Err(error) = lifecycle.auto_close_stale(now).await {
                tracing::error!(error = ?error, "H-AL stale alert closeout failed");
            }
            if let Err(error) =
                crate::alert_escalation::run_escalations_once_with_provider(&pool, now, &provider)
                    .await
            {
                tracing::error!(error = ?error, "H-AL escalation worker failed");
            }
            if let Err(error) =
                crate::alert_dashboard::process_queued_exports_with_provider(&pool, now, &provider)
                    .await
            {
                tracing::error!(error = ?error, "H-AL export worker failed");
            }
        }
    });
}
