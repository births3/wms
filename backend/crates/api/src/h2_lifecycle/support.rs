use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::audit::{append_event_in_tx, AuditDiff, AuditWriteRequest};

use super::types::H2LifecycleError;

pub(super) async fn append_system_audit_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    action: &str,
    resource_type: &str,
    resource_id: &str,
    after: Value,
    now: DateTime<Utc>,
    actor_name: &str,
) -> Result<(), H2LifecycleError> {
    append_event_in_tx(
        tx,
        &AuditWriteRequest {
            occurred_at: now,
            actor_id: Uuid::nil(),
            actor_name: actor_name.to_string(),
            owner_id,
            jti: format!("{actor_name}:{resource_id}"),
            action: action.to_string(),
            module: "H2".to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            diff: Some(AuditDiff {
                before: serde_json::json!({}),
                after,
                changed_keys: vec!["status".to_string()],
            }),
            request_id: None,
            ip: None,
            user_agent: Some("wms-h2-lifecycle".to_string()),
        },
    )
    .await
    .map_err(|error| H2LifecycleError::Audit(format!("{error:?}")))?;
    Ok(())
}

pub(super) fn subtract_months(date: NaiveDate, months: i32) -> Result<NaiveDate, H2LifecycleError> {
    add_months(date, -months)
}

pub(super) fn add_months(date: NaiveDate, months: i32) -> Result<NaiveDate, H2LifecycleError> {
    let total_months = date.year() * 12 + date.month0() as i32 + months;
    let year = total_months.div_euclid(12);
    let month0 = total_months.rem_euclid(12);
    let month = (month0 + 1) as u32;
    let day = date.day().min(days_in_month(year, month)?);
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| H2LifecycleError::InvalidInput("invalid date".to_string()))
}

fn days_in_month(year: i32, month: u32) -> Result<u32, H2LifecycleError> {
    let first_next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| H2LifecycleError::InvalidInput("invalid month".to_string()))?;
    first_next_month
        .pred_opt()
        .map(|date| date.day())
        .ok_or_else(|| H2LifecycleError::InvalidInput("invalid month predecessor".to_string()))
}

pub(super) fn matches_event_pattern(pattern: &str, event_type: &str) -> bool {
    pattern == "*"
        || pattern == event_type
        || pattern
            .strip_suffix(".*")
            .is_some_and(|prefix| event_type.starts_with(&format!("{prefix}.")))
}

pub(super) fn db_error(error: sqlx::Error) -> H2LifecycleError {
    H2LifecycleError::Database(error.to_string())
}
