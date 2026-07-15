//! H-AL alert-instance state transitions and seven-day safety closeout.

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

#[derive(Clone, Debug)]
pub struct PgAlertLifecycleService {
    pool: PgPool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertLifecycleError {
    NotFound,
    InvalidTransition,
    ReasonRequired,
    Database(String),
    Audit(String),
}

impl PgAlertLifecycleService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn acknowledge(
        &self,
        ctx: &AuthContext,
        alert_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AlertLifecycleError> {
        self.transition(
            ctx,
            alert_id,
            &["triggered", "notified", "timed_out", "escalated"],
            "acknowledged",
            "alert.acknowledged",
            None,
            now,
        )
        .await
    }

    pub async fn record_handling(
        &self,
        ctx: &AuthContext,
        alert_id: Uuid,
        description: String,
        now: DateTime<Utc>,
    ) -> Result<(), AlertLifecycleError> {
        let description = required_reason(description)?;
        self.transition(
            ctx,
            alert_id,
            &["acknowledged", "handling"],
            "handling",
            "alert.handling_recorded",
            Some(description),
            now,
        )
        .await
    }

    pub async fn close(
        &self,
        ctx: &AuthContext,
        alert_id: Uuid,
        reason: String,
        now: DateTime<Utc>,
    ) -> Result<(), AlertLifecycleError> {
        let reason = required_reason(reason)?;
        self.transition(
            ctx,
            alert_id,
            &["acknowledged", "handling"],
            "closed",
            "alert.closed",
            Some(reason),
            now,
        )
        .await
    }

    pub async fn ignore(
        &self,
        ctx: &AuthContext,
        alert_id: Uuid,
        reason: String,
        now: DateTime<Utc>,
    ) -> Result<(), AlertLifecycleError> {
        let reason = required_reason(reason)?;
        self.transition(
            ctx,
            alert_id,
            &["triggered", "notified", "timed_out", "escalated"],
            "ignored",
            "alert.ignored",
            Some(reason),
            now,
        )
        .await
    }

    pub async fn auto_close_stale(&self, now: DateTime<Utc>) -> Result<usize, AlertLifecycleError> {
        let ids: Vec<(Uuid, Uuid)> = sqlx::query_as(
            r#"
            SELECT owner_id, id
              FROM alert_instances
             WHERE status NOT IN ('closed', 'ignored')
               AND triggered_at <= $1
             ORDER BY triggered_at, id
            "#,
        )
        .bind(now - Duration::days(7))
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut closed = 0;
        for (owner_id, alert_id) in ids {
            let ctx = AuthContext {
                user_id: Uuid::nil(),
                owner_id,
                actor_name: "system-alert-engine".to_string(),
                permissions: Vec::new(),
                jti: format!("hal-auto-close:{alert_id}"),
            };
            match self
                .transition(
                    &ctx,
                    alert_id,
                    &[
                        "triggered",
                        "notified",
                        "acknowledged",
                        "handling",
                        "timed_out",
                        "escalated",
                        "notification_failed",
                    ],
                    "closed",
                    "alert.auto_closed",
                    Some("超时未关闭，系统自动关闭".to_string()),
                    now,
                )
                .await
            {
                Ok(()) => closed += 1,
                Err(AlertLifecycleError::InvalidTransition | AlertLifecycleError::NotFound) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(closed)
    }

    pub async fn close_resolved_by_resource(
        &self,
        owner_id: Uuid,
        resource_type: &str,
        resource_id: &str,
        now: DateTime<Utc>,
    ) -> Result<usize, AlertLifecycleError> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
              FROM alert_instances
             WHERE owner_id = $1 AND resource_type = $2 AND resource_id = $3
               AND status NOT IN ('closed', 'ignored')
             ORDER BY triggered_at, id
            "#,
        )
        .bind(owner_id)
        .bind(resource_type)
        .bind(resource_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut closed = 0;
        for alert_id in ids {
            let ctx = AuthContext {
                user_id: Uuid::nil(),
                owner_id,
                actor_name: "system-alert-engine".to_string(),
                permissions: Vec::new(),
                jti: format!("hal-resolved:{alert_id}"),
            };
            match self
                .transition(
                    &ctx,
                    alert_id,
                    &[
                        "triggered",
                        "notified",
                        "acknowledged",
                        "handling",
                        "timed_out",
                        "escalated",
                        "notification_failed",
                    ],
                    "closed",
                    "alert.resolved",
                    Some("业务事件解除，系统自动关闭".to_string()),
                    now,
                )
                .await
            {
                Ok(()) => closed += 1,
                Err(AlertLifecycleError::InvalidTransition | AlertLifecycleError::NotFound) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(closed)
    }

    async fn transition(
        &self,
        ctx: &AuthContext,
        alert_id: Uuid,
        allowed_from: &[&str],
        to_status: &str,
        audit_action: &str,
        description: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<(), AlertLifecycleError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let from_status: String = sqlx::query_scalar(
            "SELECT status FROM alert_instances WHERE owner_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(alert_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_error)?
        .ok_or(AlertLifecycleError::NotFound)?;
        if !allowed_from.contains(&from_status.as_str()) {
            return Err(AlertLifecycleError::InvalidTransition);
        }
        let action_description = description.as_deref();
        sqlx::query(
            r#"
            UPDATE alert_instances
               SET status = $3,
                   acknowledged_at = CASE WHEN $3 = 'acknowledged' THEN $4 ELSE acknowledged_at END,
                   handled_at = CASE WHEN $3 = 'handling' THEN $4 ELSE handled_at END,
                   closed_at = CASE WHEN $3 IN ('closed', 'ignored') THEN $4 ELSE closed_at END,
                   action_description = CASE WHEN $3 = 'handling' THEN $5 ELSE action_description END,
                   ignored_reason = CASE WHEN $3 = 'ignored' THEN $5 ELSE ignored_reason END,
                   close_reason = CASE WHEN $3 = 'closed' THEN $5 ELSE close_reason END,
                   updated_at = $4
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(alert_id)
        .bind(to_status)
        .bind(now)
        .bind(action_description)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        append_lifecycle(
            &mut tx,
            ctx,
            alert_id,
            &from_status,
            to_status,
            action_description,
            now,
        )
        .await?;
        append_alert_audit(
            &mut tx,
            ctx,
            alert_id,
            audit_action,
            serde_json::json!({
                "from_status": from_status,
                "to_status": to_status,
                "description": action_description,
            }),
            now,
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }
}

fn required_reason(value: String) -> Result<String, AlertLifecycleError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AlertLifecycleError::ReasonRequired);
    }
    Ok(value.chars().take(500).collect())
}

async fn append_lifecycle(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    alert_id: Uuid,
    from_status: &str,
    to_status: &str,
    description: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), AlertLifecycleError> {
    sqlx::query(
        r#"
        INSERT INTO alert_lifecycle_events (
            id, owner_id, alert_instance_id, from_status, to_status,
            action_description, actor_id, actor_name, occurred_at, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(alert_id)
    .bind(from_status)
    .bind(to_status)
    .bind(description)
    .bind(ctx.user_id)
    .bind(&ctx.actor_name)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

async fn append_alert_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    alert_id: Uuid,
    action: &str,
    after: Value,
    now: DateTime<Utc>,
) -> Result<(), AlertLifecycleError> {
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
            resource_type: "alert_instance".to_string(),
            resource_id: alert_id.to_string(),
            diff: Some(AuditDiff {
                before: serde_json::json!({}),
                after,
                changed_keys: vec!["status".to_string()],
            }),
            request_id: None,
            ip: None,
            user_agent: Some("wms-hal-lifecycle".to_string()),
        },
    )
    .await
    .map_err(|error| AlertLifecycleError::Audit(format!("{error:?}")))?;
    Ok(())
}

fn db_error(error: sqlx::Error) -> AlertLifecycleError {
    AlertLifecycleError::Database(error.to_string())
}
