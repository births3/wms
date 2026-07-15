//! H-AL escalation-rule persistence and due-alert worker.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Timelike, Utc, Weekday};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    AlertEscalationRule, SendH4NotificationRequest, UpsertAlertEscalationRuleRequest,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    wechat_notify_service::{PgWechatNotifyService, WechatProvider},
};

#[derive(Clone, Debug)]
pub struct PgAlertEscalationRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertEscalationError {
    TooManyLevels,
    InvalidRule,
    Database(String),
    Audit(String),
    Notification(String),
}

#[derive(Debug, FromRow)]
struct RuleRow {
    id: Uuid,
    owner_id: Uuid,
    rule_code: String,
    rule_name: String,
    notify_lower_levels: bool,
    off_hours_start: NaiveTime,
    off_hours_end: NaiveTime,
    off_hours_handler_roles: Vec<String>,
    holiday_dates: Vec<NaiveDate>,
    enabled: bool,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct CandidateRow {
    alert_id: Uuid,
    owner_id: Uuid,
    event_type: String,
    event_payload: Value,
    recipients: Vec<String>,
    status: String,
    escalation_level: i32,
    triggered_at: DateTime<Utc>,
    last_escalated_at: Option<DateTime<Utc>>,
    rule_id: Uuid,
    notify_lower_levels: bool,
    off_hours_start: NaiveTime,
    off_hours_end: NaiveTime,
    off_hours_handler_roles: Vec<String>,
    holiday_dates: Vec<NaiveDate>,
}

impl PgAlertEscalationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(
        &self,
        ctx: &AuthContext,
        request: UpsertAlertEscalationRuleRequest,
        now: DateTime<Utc>,
    ) -> Result<AlertEscalationRule, AlertEscalationError> {
        let (start, end) = validate_rule(&request)?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let row: RuleRow = sqlx::query_as(
            r#"
            INSERT INTO alert_escalation_rules (
                id, owner_id, rule_code, rule_name, notify_lower_levels,
                off_hours_start, off_hours_end, off_hours_handler_roles,
                holiday_dates, enabled, created_by, updated_by, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11, $12, $12)
            ON CONFLICT (owner_id, rule_code)
            DO UPDATE SET
                rule_name = EXCLUDED.rule_name,
                notify_lower_levels = EXCLUDED.notify_lower_levels,
                off_hours_start = EXCLUDED.off_hours_start,
                off_hours_end = EXCLUDED.off_hours_end,
                off_hours_handler_roles = EXCLUDED.off_hours_handler_roles,
                holiday_dates = EXCLUDED.holiday_dates,
                enabled = EXCLUDED.enabled,
                updated_by = EXCLUDED.updated_by,
                updated_at = EXCLUDED.updated_at,
                version = alert_escalation_rules.version + 1
            RETURNING id, owner_id, rule_code, rule_name, notify_lower_levels,
                      off_hours_start, off_hours_end, off_hours_handler_roles,
                      holiday_dates, enabled, version, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(request.rule_code.trim())
        .bind(request.rule_name.trim())
        .bind(request.notify_lower_levels)
        .bind(start)
        .bind(end)
        .bind(&request.off_hours_handler_roles)
        .bind(&request.holiday_dates)
        .bind(request.enabled)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
        sqlx::query("DELETE FROM alert_escalation_levels WHERE owner_id = $1 AND rule_id = $2")
            .bind(ctx.owner_id)
            .bind(row.id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for level in &request.levels {
            sqlx::query(
                r#"
                INSERT INTO alert_escalation_levels (
                    id, owner_id, rule_id, level_no, threshold_seconds,
                    recipient_roles, created_at, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(row.id)
            .bind(level.level)
            .bind(level.threshold_seconds)
            .bind(&level.recipient_roles)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        }
        append_audit(
            &mut tx,
            ctx,
            "alert.escalation_rule.upserted",
            "alert_escalation_rule",
            row.id,
            serde_json::json!({"rule_code": row.rule_code, "levels": request.levels}),
            now,
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(AlertEscalationRule {
            id: row.id,
            owner_id: row.owner_id,
            rule_code: row.rule_code,
            rule_name: row.rule_name,
            notify_lower_levels: row.notify_lower_levels,
            off_hours_start: row.off_hours_start.format("%H:%M").to_string(),
            off_hours_end: row.off_hours_end.format("%H:%M").to_string(),
            off_hours_handler_roles: row.off_hours_handler_roles,
            holiday_dates: row.holiday_dates,
            enabled: row.enabled,
            levels: request.levels,
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn list(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<AlertEscalationRule>, AlertEscalationError> {
        let rows: Vec<RuleRow> = sqlx::query_as(
            r#"
            SELECT id, owner_id, rule_code, rule_name, notify_lower_levels,
                   off_hours_start, off_hours_end, off_hours_handler_roles,
                   holiday_dates, enabled, version, created_at, updated_at
              FROM alert_escalation_rules
             WHERE owner_id = $1
             ORDER BY rule_code
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut rules = Vec::with_capacity(rows.len());
        for row in rows {
            let levels: Vec<(i32, i64, Vec<String>)> = sqlx::query_as(
                "SELECT level_no, threshold_seconds, recipient_roles FROM alert_escalation_levels WHERE owner_id = $1 AND rule_id = $2 ORDER BY level_no",
            )
            .bind(owner_id)
            .bind(row.id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
            rules.push(AlertEscalationRule {
                id: row.id,
                owner_id: row.owner_id,
                rule_code: row.rule_code,
                rule_name: row.rule_name,
                notify_lower_levels: row.notify_lower_levels,
                off_hours_start: row.off_hours_start.format("%H:%M").to_string(),
                off_hours_end: row.off_hours_end.format("%H:%M").to_string(),
                off_hours_handler_roles: row.off_hours_handler_roles,
                holiday_dates: row.holiday_dates,
                enabled: row.enabled,
                levels: levels
                    .into_iter()
                    .map(|(level, threshold_seconds, recipient_roles)| {
                        wms_domain::AlertEscalationLevelDraft {
                            level,
                            threshold_seconds,
                            recipient_roles,
                        }
                    })
                    .collect(),
                version: row.version,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }
        Ok(rules)
    }
}

fn validate_rule(
    request: &UpsertAlertEscalationRuleRequest,
) -> Result<(NaiveTime, NaiveTime), AlertEscalationError> {
    if request.levels.len() > 3 {
        return Err(AlertEscalationError::TooManyLevels);
    }
    if request.rule_code.trim().is_empty()
        || request.rule_name.trim().is_empty()
        || request.levels.is_empty()
    {
        return Err(AlertEscalationError::InvalidRule);
    }
    let mut previous_threshold = 0_i64;
    for (index, level) in request.levels.iter().enumerate() {
        if level.level != index as i32 + 1
            || level.threshold_seconds <= previous_threshold
            || level
                .recipient_roles
                .iter()
                .all(|role| role.trim().is_empty())
        {
            return Err(AlertEscalationError::InvalidRule);
        }
        previous_threshold = level.threshold_seconds;
    }
    let start = NaiveTime::parse_from_str(&request.off_hours_start, "%H:%M")
        .map_err(|_| AlertEscalationError::InvalidRule)?;
    let end = NaiveTime::parse_from_str(&request.off_hours_end, "%H:%M")
        .map_err(|_| AlertEscalationError::InvalidRule)?;
    Ok((start, end))
}

pub async fn run_escalations_once_with_provider(
    pool: &PgPool,
    now: DateTime<Utc>,
    provider: &dyn WechatProvider,
) -> Result<usize, AlertEscalationError> {
    let candidates: Vec<CandidateRow> = sqlx::query_as(
        r#"
        SELECT instance.id AS alert_id, instance.owner_id,
               instance.event_type, instance.event_payload, instance.recipients,
               instance.status, instance.escalation_level, instance.triggered_at,
               instance.last_escalated_at, rule.id AS rule_id,
               rule.notify_lower_levels, rule.off_hours_start, rule.off_hours_end,
               rule.off_hours_handler_roles, rule.holiday_dates
          FROM alert_instances instance
          JOIN alert_definitions definition ON definition.id = instance.alert_definition_id
          JOIN alert_escalation_rules rule
            ON rule.owner_id = instance.owner_id AND rule.rule_code = definition.escalation_ref
         WHERE instance.status IN ('triggered', 'notified', 'timed_out', 'escalated')
           AND rule.enabled = TRUE
         ORDER BY instance.triggered_at, instance.id
         LIMIT 200
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_error)?;
    let mut count = 0;
    for candidate in candidates {
        if process_candidate(pool, candidate, now, provider).await? {
            count += 1;
        }
    }
    Ok(count)
}

async fn process_candidate(
    pool: &PgPool,
    candidate: CandidateRow,
    now: DateTime<Utc>,
    provider: &dyn WechatProvider,
) -> Result<bool, AlertEscalationError> {
    let levels: Vec<(i32, i64, Vec<String>)> = sqlx::query_as(
        "SELECT level_no, threshold_seconds, recipient_roles FROM alert_escalation_levels WHERE owner_id = $1 AND rule_id = $2 ORDER BY level_no",
    )
    .bind(candidate.owner_id)
    .bind(candidate.rule_id)
    .fetch_all(pool)
    .await
    .map_err(db_error)?;
    let elapsed = (now - candidate.triggered_at).num_seconds().max(0);
    let repeat_l3 = candidate.escalation_level == 3
        && candidate
            .last_escalated_at
            .is_some_and(|last| now - last >= Duration::hours(24));
    let target_level = if repeat_l3 {
        3
    } else {
        candidate.escalation_level + 1
    };
    let Some((_, threshold, configured_roles)) =
        levels.iter().find(|(level, _, _)| *level == target_level)
    else {
        return Ok(false);
    };
    if !repeat_l3 && elapsed < *threshold {
        return Ok(false);
    }
    let repeat_key = if repeat_l3 {
        format!("l3-repeat:{}", now.timestamp().div_euclid(86_400))
    } else {
        format!("level:{target_level}")
    };
    let roles = if is_off_hours(&candidate, now) && !candidate.off_hours_handler_roles.is_empty() {
        &candidate.off_hours_handler_roles
    } else {
        configured_roles
    };
    let mut tx = pool.begin().await.map_err(db_error)?;
    let mut recipients = resolve_recipients(&mut tx, candidate.owner_id, roles).await?;
    if candidate.notify_lower_levels {
        recipients.extend(candidate.recipients.iter().cloned());
    }
    recipients.sort();
    recipients.dedup();
    let escalation_event_id = Uuid::new_v4();
    let inserted = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO alert_escalation_events (
            id, owner_id, alert_instance_id, level_no, repeat_key,
            recipients, elapsed_seconds, reason, occurred_at, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
        ON CONFLICT (alert_instance_id, repeat_key) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(escalation_event_id)
    .bind(candidate.owner_id)
    .bind(candidate.alert_id)
    .bind(target_level)
    .bind(&repeat_key)
    .bind(&recipients)
    .bind(elapsed)
    .bind(if repeat_l3 {
        "L3 24 小时持续提醒"
    } else {
        "未确认超时升级"
    })
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error)?;
    if inserted.is_none() {
        tx.rollback().await.map_err(db_error)?;
        return Ok(false);
    }
    sqlx::query(
        "UPDATE alert_instances SET status = 'escalated', escalation_level = $3, recipients = $4, last_escalated_at = $5, updated_at = $5 WHERE owner_id = $1 AND id = $2",
    )
    .bind(candidate.owner_id)
    .bind(candidate.alert_id)
    .bind(target_level)
    .bind(&recipients)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    if !repeat_l3 {
        append_lifecycle(
            &mut tx,
            &candidate,
            &candidate.status,
            "timed_out",
            target_level,
            now,
        )
        .await?;
        append_runtime_audit(
            &mut tx,
            &candidate,
            "alert.timed_out",
            serde_json::json!({"elapsed_seconds": elapsed, "target_level": target_level}),
            now,
        )
        .await?;
    }
    append_lifecycle(
        &mut tx,
        &candidate,
        if repeat_l3 { "escalated" } else { "timed_out" },
        "escalated",
        target_level,
        now,
    )
    .await?;
    append_runtime_audit(
        &mut tx,
        &candidate,
        "alert.escalated",
        serde_json::json!({
            "level": target_level,
            "elapsed_seconds": elapsed,
            "recipients": recipients,
            "repeat": repeat_l3,
        }),
        now,
    )
    .await?;
    tx.commit().await.map_err(db_error)?;

    let ctx = AuthContext {
        user_id: Uuid::nil(),
        owner_id: candidate.owner_id,
        actor_name: "system-alert-escalation".to_string(),
        permissions: Vec::new(),
        jti: format!("hal-escalation:{}:{repeat_key}", candidate.alert_id),
    };
    PgWechatNotifyService::new()
        .send_notification_with_provider(
            pool,
            &ctx,
            SendH4NotificationRequest {
                event_type: candidate.event_type,
                dedupe_key: format!("escalation:{}:{repeat_key}", candidate.alert_id),
                recipients,
                payload: candidate.event_payload,
            },
            now,
            &format!("hal-escalation:{escalation_event_id}"),
            provider,
        )
        .await
        .map_err(|error| AlertEscalationError::Notification(format!("{error:?}")))?;
    Ok(true)
}

fn is_off_hours(candidate: &CandidateRow, now: DateTime<Utc>) -> bool {
    let local_time = NaiveTime::from_hms_opt(now.hour(), now.minute(), now.second())
        .expect("UTC time components are valid");
    let overnight = candidate.off_hours_start > candidate.off_hours_end;
    let outside = if overnight {
        local_time >= candidate.off_hours_start || local_time < candidate.off_hours_end
    } else {
        local_time >= candidate.off_hours_start && local_time < candidate.off_hours_end
    };
    outside
        || matches!(now.weekday(), Weekday::Sat | Weekday::Sun)
        || candidate.holiday_dates.contains(&now.date_naive())
}

async fn resolve_recipients(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    roles: &[String],
) -> Result<Vec<String>, AlertEscalationError> {
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

async fn append_lifecycle(
    tx: &mut Transaction<'_, Postgres>,
    candidate: &CandidateRow,
    from_status: &str,
    to_status: &str,
    level: i32,
    now: DateTime<Utc>,
) -> Result<(), AlertEscalationError> {
    sqlx::query(
        r#"
        INSERT INTO alert_lifecycle_events (
            id, owner_id, alert_instance_id, from_status, to_status,
            action_description, actor_id, actor_name, occurred_at, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, NULL, 'system-alert-escalation', $7, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(candidate.owner_id)
    .bind(candidate.alert_id)
    .bind(from_status)
    .bind(to_status)
    .bind(format!("升级到 L{level}"))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

async fn append_runtime_audit(
    tx: &mut Transaction<'_, Postgres>,
    candidate: &CandidateRow,
    action: &str,
    after: Value,
    now: DateTime<Utc>,
) -> Result<(), AlertEscalationError> {
    let ctx = AuthContext {
        user_id: Uuid::nil(),
        owner_id: candidate.owner_id,
        actor_name: "system-alert-escalation".to_string(),
        permissions: Vec::new(),
        jti: format!("hal-escalation:{}:{action}", candidate.alert_id),
    };
    append_audit(
        tx,
        &ctx,
        action,
        "alert_instance",
        candidate.alert_id,
        after,
        now,
    )
    .await
}

async fn append_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    after: Value,
    now: DateTime<Utc>,
) -> Result<(), AlertEscalationError> {
    append_event_in_tx(
        tx,
        &AuditWriteRequest {
            occurred_at: now,
            actor_id: ctx.user_id,
            actor_name: ctx.actor_name.clone(),
            owner_id: ctx.owner_id,
            jti: ctx.jti.clone(),
            action: action.to_string(),
            module: "H-AL".to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            diff: Some(AuditDiff {
                before: serde_json::json!({}),
                after,
                changed_keys: vec!["escalation_level".to_string()],
            }),
            request_id: None,
            ip: None,
            user_agent: Some("wms-hal-escalation".to_string()),
        },
    )
    .await
    .map_err(|error| AlertEscalationError::Audit(format!("{error:?}")))?;
    Ok(())
}

fn db_error(error: sqlx::Error) -> AlertEscalationError {
    AlertEscalationError::Database(error.to_string())
}
