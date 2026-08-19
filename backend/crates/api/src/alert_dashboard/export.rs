use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{AlertInstanceListQuery, SendH4NotificationRequest};

use crate::{
    audit::{append_event, AuditDiff, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
    wechat_notify_service::{PgWechatNotifyService, WechatProvider},
};

use super::{db_error, AlertDashboardError};

const MAX_EXPORT_ROWS: i64 = 100_000;

#[derive(Debug, FromRow)]
struct ExportRow {
    alert_code: String,
    severity: String,
    status: String,
    resource_type: String,
    resource_id: String,
    recipients: Vec<String>,
    triggered_at: DateTime<Utc>,
    acknowledged_at: Option<DateTime<Utc>>,
    closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ExportJobRow {
    id: Uuid,
    owner_id: Uuid,
    requested_by: Uuid,
    format: String,
    filters: Value,
    row_count: i64,
    download_token: Uuid,
    recipient_email: Option<String>,
    created_at: DateTime<Utc>,
}

pub async fn process_queued_exports_with_provider(
    pool: &PgPool,
    now: DateTime<Utc>,
    provider: &dyn WechatProvider,
) -> Result<usize, AlertDashboardError> {
    let jobs: Vec<ExportJobRow> = sqlx::query_as(
        r#"
        SELECT id, owner_id, requested_by, format, filters, row_count,
               download_token, recipient_email, created_at
          FROM alert_report_exports
         WHERE status = 'queued'
         ORDER BY created_at, id
         LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(db_error)?;
    let mut completed = 0;
    for job in jobs {
        let claimed = sqlx::query(
            "UPDATE alert_report_exports SET status = 'processing', updated_at = $2 WHERE id = $1 AND status = 'queued'",
        )
        .bind(job.id)
        .bind(now)
        .execute(pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if claimed == 0 {
            continue;
        }
        let query: AlertInstanceListQuery = serde_json::from_value(job.filters.clone())
            .map_err(|error| AlertDashboardError::Serialize(error.to_string()))?;
        let (content, content_type, filename) =
            generate_export(pool, job.owner_id, &job.format, &query).await?;
        sqlx::query(
            r#"
            UPDATE alert_report_exports
               SET status = 'ready', content = $2, content_type = $3, filename = $4,
                   updated_at = $5, completed_at = $5
             WHERE id = $1
            "#,
        )
        .bind(job.id)
        .bind(content)
        .bind(content_type)
        .bind(filename)
        .bind(now)
        .execute(pool)
        .await
        .map_err(db_error)?;
        let email_status = notify_recipient(pool, &job, now, provider).await?;
        append_export_audit(pool, &job, email_status, now).await?;
        completed += 1;
    }
    Ok(completed)
}

async fn notify_recipient(
    pool: &PgPool,
    job: &ExportJobRow,
    now: DateTime<Utc>,
    provider: &dyn WechatProvider,
) -> Result<Option<&'static str>, AlertDashboardError> {
    let Some(email) = job.recipient_email.as_ref() else {
        return Ok(None);
    };
    let ctx = system_context(job);
    let result = PgWechatNotifyService::new()
        .send_notification_with_provider(
            pool,
            &ctx,
            SendH4NotificationRequest {
                event_type: "hal.alert.export.ready".to_string(),
                dedupe_key: format!("alert-export:{}", job.id),
                recipients: vec![email.clone()],
                payload: serde_json::json!({
                    "download_url": format!("/api/v1/alerts/exports/{}/download", job.download_token),
                    "row_count": job.row_count.min(MAX_EXPORT_ROWS),
                    "expires_at": job.created_at + Duration::days(7),
                }),
            },
            now,
            &format!("hal-export-notify:{}", job.id),
            provider,
        )
        .await;
    let status = match result {
        Ok(result) if result.value.iter().all(|record| record.status == "success") => "sent",
        _ => "failed",
    };
    sqlx::query(
        "UPDATE alert_report_exports SET email_notification_status = $2, updated_at = $3 WHERE id = $1",
    )
    .bind(job.id)
    .bind(status)
    .bind(now)
    .execute(pool)
    .await
    .map_err(db_error)?;
    Ok(Some(status))
}

async fn append_export_audit(
    pool: &PgPool,
    job: &ExportJobRow,
    email_status: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), AlertDashboardError> {
    let ctx = system_context(job);
    append_event(
        pool,
        &AuditWriteRequest {
            occurred_at: now,
            actor_id: ctx.user_id,
            actor_name: ctx.actor_name,
            owner_id: ctx.owner_id,
            jti: ctx.jti,
            action: "alert.export.completed".to_string(),
            module: "H-AL".to_string(),
            resource_type: "alert_report_export".to_string(),
            resource_id: job.id.to_string(),
            diff: Some(AuditDiff {
                before: serde_json::json!({"status": "queued"}),
                after: serde_json::json!({"status": "ready", "email_status": email_status}),
                changed_keys: vec!["status".to_string()],
            }),
            request_id: None,
            ip: None,
            user_agent: Some("wms-hal-export-worker".to_string()),
        },
    )
    .await
    .map_err(|error| AlertDashboardError::Audit(format!("{error:?}")))?;
    Ok(())
}

fn system_context(job: &ExportJobRow) -> AuthContext {
    AuthContext {
        user_id: job.requested_by,
        owner_id: job.owner_id,
        actor_name: "system-alert-export".to_string(),
        permissions: Vec::new(),
        jti: format!("hal-export:{}", job.id),
        warehouse_scope: None,
    }
}

pub(super) fn export_mode(row_count: i64) -> &'static str {
    if row_count > MAX_EXPORT_ROWS {
        "queued"
    } else {
        "ready"
    }
}

pub(super) async fn count_rows(
    pool: &PgPool,
    owner_id: Uuid,
    query: &AlertInstanceListQuery,
) -> Result<i64, AlertDashboardError> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT FROM alert_instances
         WHERE owner_id = $1
           AND ($2::UUID IS NULL OR warehouse_id = $2)
           AND ($3::TEXT IS NULL OR severity = $3)
           AND ($4::TEXT IS NULL OR status = $4)
           AND ($5::TEXT IS NULL OR alert_code = $5)
           AND ($6::TIMESTAMPTZ IS NULL OR triggered_at >= $6)
           AND ($7::TIMESTAMPTZ IS NULL OR triggered_at <= $7)
        "#,
    )
    .bind(owner_id)
    .bind(query.warehouse_id)
    .bind(query.severity.as_deref())
    .bind(query.status.as_deref())
    .bind(query.alert_code.as_deref())
    .bind(query.from)
    .bind(query.to)
    .fetch_one(pool)
    .await
    .map_err(db_error)
}

pub(super) async fn generate_export(
    pool: &PgPool,
    owner_id: Uuid,
    format: &str,
    query: &AlertInstanceListQuery,
) -> Result<(Vec<u8>, String, String), AlertDashboardError> {
    let rows: Vec<ExportRow> = sqlx::query_as(
        r#"
        SELECT alert_code, severity, status, resource_type, resource_id,
               recipients, triggered_at, acknowledged_at, closed_at
          FROM alert_instances
         WHERE owner_id = $1
           AND ($2::UUID IS NULL OR warehouse_id = $2)
           AND ($3::TEXT IS NULL OR severity = $3)
           AND ($4::TEXT IS NULL OR status = $4)
           AND ($5::TEXT IS NULL OR alert_code = $5)
           AND ($6::TIMESTAMPTZ IS NULL OR triggered_at >= $6)
           AND ($7::TIMESTAMPTZ IS NULL OR triggered_at <= $7)
         ORDER BY triggered_at DESC, id DESC
         LIMIT 100000
        "#,
    )
    .bind(owner_id)
    .bind(query.warehouse_id)
    .bind(query.severity.as_deref())
    .bind(query.status.as_deref())
    .bind(query.alert_code.as_deref())
    .bind(query.from)
    .bind(query.to)
    .fetch_all(pool)
    .await
    .map_err(db_error)?;
    let tabular = export_text(query, &rows);
    match format {
        "excel" => Ok((
            [b"\xEF\xBB\xBF".as_slice(), tabular.as_bytes()].concat(),
            "application/vnd.ms-excel; charset=utf-8".to_string(),
            "alert-report.xls".to_string(),
        )),
        "pdf" => Ok((
            crate::pdf_document::render_text_pdf(&tabular),
            "application/pdf".to_string(),
            "alert-report.pdf".to_string(),
        )),
        _ => Err(AlertDashboardError::InvalidExportFormat),
    }
}

fn export_text(query: &AlertInstanceListQuery, rows: &[ExportRow]) -> String {
    let mut output = format!(
        "filters\t{}\nalert_code\tseverity\tstatus\tresource\trecipients\ttriggered_at\tacknowledged_at\tclosed_at\n",
        serde_json::to_string(query).unwrap_or_else(|_| "{}".to_string())
    );
    for row in rows {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}:{}\t{}\t{}\t{}\t{}\n",
            row.alert_code,
            row.severity,
            row.status,
            row.resource_type,
            row.resource_id,
            row.recipients.join(","),
            row.triggered_at,
            row.acknowledged_at
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.closed_at
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_above_one_hundred_thousand_rows_is_async() {
        assert_eq!(export_mode(100_000), "ready");
        assert_eq!(export_mode(100_001), "queued");
    }

    #[test]
    fn generated_pdf_has_pdf_header_and_eof() {
        let pdf = crate::pdf_document::render_text_pdf("alert report");
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }
}
