//! H-AL active dashboard, monthly statistics, GSP lifecycle report and governed exports.

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{
    AlertChangeEvent, AlertExportJob, AlertInstance, AlertInstanceListQuery, AlertMonthlyMetric,
    AlertRankingItem, AlertStatisticsResponse, CreateAlertExportRequest, GspAlertLifecycleRecord,
    GspAlertLifecycleReport, PageMeta,
};

use crate::{
    alert_instance_repository::{AlertInstanceRepositoryError, PgAlertInstanceRepository},
    audit::{append_event, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

mod export;

pub use export::process_queued_exports_with_provider;
use export::{count_rows, export_mode, generate_export};

#[derive(Clone, Debug)]
pub struct PgAlertDashboardService {
    pool: PgPool,
    instances: PgAlertInstanceRepository,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertDashboardError {
    NotFound,
    RangeTooLarge,
    WarehouseScopeRequired,
    WarehouseScopeDenied,
    InvalidExportFormat,
    Database(String),
    Audit(String),
    Serialize(String),
    Notification(String),
}

#[derive(Debug, FromRow)]
struct MonthlyRow {
    month: String,
    triggered_count: i64,
    acknowledgement_rate: f64,
    average_response_seconds: Option<f64>,
    escalation_rate: f64,
}

#[derive(Debug, FromRow)]
struct RankingRow {
    key: String,
    count: i64,
    average_response_seconds: Option<f64>,
    unacknowledged_count: i64,
}

impl PgAlertDashboardService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            instances: PgAlertInstanceRepository::new(pool.clone()),
            pool,
        }
    }

    pub async fn list_active(
        &self,
        ctx: &AuthContext,
        query: AlertInstanceListQuery,
        now: DateTime<Utc>,
    ) -> Result<Vec<AlertInstance>, AlertDashboardError> {
        let mut query = self.authorized_query(ctx, query).await?;
        query.active_only = Some(true);
        let data = self
            .instances
            .list(ctx.owner_id, &query)
            .await
            .map_err(repository_error)?;
        self.audit_query(ctx, "alert.dashboard.queried", &query, now)
            .await?;
        Ok(data)
    }

    pub async fn statistics(
        &self,
        ctx: &AuthContext,
        query: AlertInstanceListQuery,
        now: DateTime<Utc>,
    ) -> Result<AlertStatisticsResponse, AlertDashboardError> {
        let mut query = self.authorized_query(ctx, query).await?;
        let cache_key_query = query.clone();
        let to = query.to.unwrap_or(now);
        let from = query.from.unwrap_or(to - Duration::days(30));
        if to < from || to - from > Duration::days(366) {
            return Err(AlertDashboardError::RangeTooLarge);
        }
        query.from = Some(from);
        query.to = Some(to);
        let filter_key = serde_json::to_string(&cache_key_query)
            .map_err(|error| AlertDashboardError::Serialize(error.to_string()))?;
        let statistics = match self
            .query_live_statistics(ctx.owner_id, &query, from, to, now)
            .await
        {
            Ok(statistics) => {
                self.store_statistics_snapshot(ctx.owner_id, &filter_key, &query, &statistics, now)
                    .await?;
                statistics
            }
            Err(live_error @ AlertDashboardError::Database(_)) => {
                let Some(mut cached) = self
                    .load_statistics_snapshot(ctx.owner_id, &filter_key)
                    .await?
                else {
                    return Err(live_error);
                };
                cached.possibly_stale = true;
                cached
            }
            Err(error) => return Err(error),
        };
        self.audit_query(ctx, "alert.statistics.queried", &query, now)
            .await?;
        Ok(statistics)
    }

    async fn query_live_statistics(
        &self,
        owner_id: Uuid,
        query: &AlertInstanceListQuery,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<AlertStatisticsResponse, AlertDashboardError> {
        let monthly: Vec<MonthlyRow> = sqlx::query_as(
            r#"
            SELECT to_char(date_trunc('month', triggered_at), 'YYYY-MM') AS month,
                   COUNT(*)::BIGINT AS triggered_count,
                   COALESCE(COUNT(*) FILTER (WHERE acknowledged_at IS NOT NULL)::DOUBLE PRECISION
                       / NULLIF(COUNT(*), 0), 0) AS acknowledgement_rate,
                   AVG(EXTRACT(EPOCH FROM (acknowledged_at - triggered_at)))
                       FILTER (WHERE acknowledged_at IS NOT NULL)::DOUBLE PRECISION
                       AS average_response_seconds,
                   COALESCE(COUNT(*) FILTER (WHERE escalation_level > 0)::DOUBLE PRECISION
                       / NULLIF(COUNT(*), 0), 0) AS escalation_rate
              FROM alert_instances
             WHERE owner_id = $1
               AND triggered_at BETWEEN $2 AND $3
               AND ($4::UUID IS NULL OR warehouse_id = $4)
               AND ($5::TEXT IS NULL OR severity = $5)
               AND ($6::TEXT IS NULL OR status = $6)
               AND ($7::TEXT IS NULL OR alert_code = $7)
             GROUP BY date_trunc('month', triggered_at)
             ORDER BY date_trunc('month', triggered_at)
            "#,
        )
        .bind(owner_id)
        .bind(from)
        .bind(to)
        .bind(query.warehouse_id)
        .bind(query.severity.as_deref())
        .bind(query.status.as_deref())
        .bind(query.alert_code.as_deref())
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let alert_type_top10: Vec<RankingRow> = sqlx::query_as(
            r#"
            SELECT alert_code AS key, COUNT(*)::BIGINT AS count,
                   AVG(EXTRACT(EPOCH FROM (acknowledged_at - triggered_at)))
                       FILTER (WHERE acknowledged_at IS NOT NULL)::DOUBLE PRECISION
                       AS average_response_seconds,
                   COUNT(*) FILTER (WHERE acknowledged_at IS NULL)::BIGINT AS unacknowledged_count
              FROM alert_instances
             WHERE owner_id = $1 AND triggered_at BETWEEN $2 AND $3
               AND ($4::UUID IS NULL OR warehouse_id = $4)
             GROUP BY alert_code
             ORDER BY count DESC, alert_code
             LIMIT 10
            "#,
        )
        .bind(owner_id)
        .bind(from)
        .bind(to)
        .bind(query.warehouse_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let recipient_top10: Vec<RankingRow> = sqlx::query_as(
            r#"
            SELECT recipient AS key, COUNT(*)::BIGINT AS count,
                   AVG(EXTRACT(EPOCH FROM (instance.acknowledged_at - instance.triggered_at)))
                       FILTER (WHERE instance.acknowledged_at IS NOT NULL)::DOUBLE PRECISION
                       AS average_response_seconds,
                   COUNT(*) FILTER (WHERE instance.acknowledged_at IS NULL)::BIGINT
                       AS unacknowledged_count
              FROM alert_instances instance
              CROSS JOIN LATERAL unnest(instance.recipients) recipient
             WHERE instance.owner_id = $1 AND instance.triggered_at BETWEEN $2 AND $3
               AND ($4::UUID IS NULL OR instance.warehouse_id = $4)
             GROUP BY recipient
             ORDER BY unacknowledged_count DESC, average_response_seconds ASC NULLS LAST, recipient
             LIMIT 10
            "#,
        )
        .bind(owner_id)
        .bind(from)
        .bind(to)
        .bind(query.warehouse_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(AlertStatisticsResponse {
            generated_at: now,
            possibly_stale: false,
            monthly: monthly.into_iter().map(map_monthly).collect(),
            alert_type_top10: alert_type_top10.into_iter().map(map_ranking).collect(),
            recipient_top10: recipient_top10.into_iter().map(map_ranking).collect(),
        })
    }

    async fn store_statistics_snapshot(
        &self,
        owner_id: Uuid,
        filter_key: &str,
        filters: &AlertInstanceListQuery,
        statistics: &AlertStatisticsResponse,
        now: DateTime<Utc>,
    ) -> Result<(), AlertDashboardError> {
        let filters = serde_json::to_value(filters)
            .map_err(|error| AlertDashboardError::Serialize(error.to_string()))?;
        let payload = serde_json::to_value(statistics)
            .map_err(|error| AlertDashboardError::Serialize(error.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO alert_statistics_snapshots (
                owner_id, filter_key, filters, payload, generated_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (owner_id, filter_key) DO UPDATE
               SET filters = EXCLUDED.filters,
                   payload = EXCLUDED.payload,
                   generated_at = EXCLUDED.generated_at,
                   updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(owner_id)
        .bind(filter_key)
        .bind(filters)
        .bind(payload)
        .bind(statistics.generated_at)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(())
    }

    async fn load_statistics_snapshot(
        &self,
        owner_id: Uuid,
        filter_key: &str,
    ) -> Result<Option<AlertStatisticsResponse>, AlertDashboardError> {
        let payload: Option<Value> = sqlx::query_scalar(
            "SELECT payload FROM alert_statistics_snapshots WHERE owner_id = $1 AND filter_key = $2",
        )
        .bind(owner_id)
        .bind(filter_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        payload
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| AlertDashboardError::Serialize(error.to_string()))
    }

    pub async fn gsp_report(
        &self,
        ctx: &AuthContext,
        query: AlertInstanceListQuery,
        now: DateTime<Utc>,
    ) -> Result<GspAlertLifecycleReport, AlertDashboardError> {
        let mut query = self.authorized_query(ctx, query).await?;
        let to = query.to.unwrap_or(now);
        let from = query.from.unwrap_or(to - Duration::days(30));
        query.from = Some(from);
        query.to = Some(to);
        let ids: Vec<(Uuid, Value)> = sqlx::query_as(
            r#"
            SELECT instance.id,
                   COALESCE(jsonb_agg(jsonb_build_object(
                       'from_status', lifecycle.from_status,
                       'to_status', lifecycle.to_status,
                       'action_description', lifecycle.action_description,
                       'actor_name', lifecycle.actor_name,
                       'occurred_at', lifecycle.occurred_at
                   ) ORDER BY lifecycle.occurred_at, lifecycle.event_sequence)
                   FILTER (WHERE lifecycle.id IS NOT NULL), '[]'::jsonb) AS lifecycle_events
              FROM alert_instances instance
              JOIN alert_definitions definition ON definition.id = instance.alert_definition_id
              LEFT JOIN alert_lifecycle_events lifecycle
                ON lifecycle.alert_instance_id = instance.id
             WHERE instance.owner_id = $1 AND definition.is_gsp_forced = TRUE
               AND instance.triggered_at BETWEEN $2 AND $3
               AND ($4::UUID IS NULL OR instance.warehouse_id = $4)
             GROUP BY instance.id
             ORDER BY MAX(instance.triggered_at) DESC
             LIMIT $5
            "#,
        )
        .bind(ctx.owner_id)
        .bind(from)
        .bind(to)
        .bind(query.warehouse_id)
        .bind(query.limit.unwrap_or(100).clamp(1, 1000))
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut data = Vec::with_capacity(ids.len());
        for (id, lifecycle_events) in ids {
            let alert = self
                .instances
                .get(ctx.owner_id, id)
                .await
                .map_err(repository_error)?;
            data.push(GspAlertLifecycleRecord {
                alert,
                lifecycle_events,
            });
        }
        self.audit_query(ctx, "alert.gsp_report.queried", &query, now)
            .await?;
        Ok(GspAlertLifecycleReport {
            generated_at: now,
            page: PageMeta {
                next_cursor: None,
                count: data.len().min(u32::MAX as usize) as u32,
            },
            data,
        })
    }

    pub async fn create_export(
        &self,
        ctx: &AuthContext,
        request: CreateAlertExportRequest,
        now: DateTime<Utc>,
    ) -> Result<AlertExportJob, AlertDashboardError> {
        if !matches!(request.format.as_str(), "excel" | "pdf") {
            return Err(AlertDashboardError::InvalidExportFormat);
        }
        let query = self.authorized_query(ctx, request.filters).await?;
        let row_count = count_rows(&self.pool, ctx.owner_id, &query).await?;
        let status = export_mode(row_count);
        let id = Uuid::new_v4();
        let token = Uuid::new_v4();
        let filters = serde_json::to_value(&query)
            .map_err(|error| AlertDashboardError::Serialize(error.to_string()))?;
        let generated = if status == "ready" {
            Some(generate_export(&self.pool, ctx.owner_id, &request.format, &query).await?)
        } else {
            None
        };
        let completed_at = generated.as_ref().map(|_| now);
        let email_status = if status == "queued" && request.recipient_email.is_some() {
            Some("pending")
        } else {
            None
        };
        let (content, content_type, filename) = generated
            .map(|value| (Some(value.0), Some(value.1), Some(value.2)))
            .unwrap_or((None, None, None));
        sqlx::query(
            r#"
            INSERT INTO alert_report_exports (
                id, owner_id, requested_by, format, status, filters, row_count,
                content, content_type, filename, download_token, recipient_email,
                email_notification_status, created_at, updated_at, completed_at, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                      $14, $14, $15, $14 + INTERVAL '7 days')
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(ctx.user_id)
        .bind(&request.format)
        .bind(status)
        .bind(filters)
        .bind(row_count)
        .bind(content)
        .bind(content_type)
        .bind(filename)
        .bind(token)
        .bind(request.recipient_email)
        .bind(email_status)
        .bind(now)
        .bind(completed_at)
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        self.audit_query(ctx, "alert.export.requested", &query, now)
            .await?;
        Ok(AlertExportJob {
            id,
            status: status.to_string(),
            format: request.format,
            row_count,
            download_url: (status == "ready")
                .then(|| format!("/api/v1/alerts/exports/{token}/download")),
            email_notification_status: email_status.map(str::to_string),
            created_at: now,
            completed_at,
        })
    }

    pub async fn download(
        &self,
        owner_id: Uuid,
        token: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(Vec<u8>, String, String), AlertDashboardError> {
        sqlx::query_as::<_, (Vec<u8>, String, String)>(
            "SELECT content, content_type, filename FROM alert_report_exports WHERE owner_id = $1 AND download_token = $2 AND status = 'ready' AND expires_at > $3",
        )
        .bind(owner_id)
        .bind(token)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?
            .ok_or(AlertDashboardError::NotFound)
    }

    pub async fn get_export(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<AlertExportJob, AlertDashboardError> {
        let row: (String, String, i64, Uuid, Option<String>, DateTime<Utc>, Option<DateTime<Utc>>) =
            sqlx::query_as(
                "SELECT status, format, row_count, download_token, email_notification_status, created_at, completed_at FROM alert_report_exports WHERE owner_id = $1 AND id = $2",
            )
            .bind(owner_id)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(AlertDashboardError::NotFound)?;
        Ok(AlertExportJob {
            id,
            status: row.0.clone(),
            format: row.1,
            row_count: row.2,
            download_url: (row.0 == "ready")
                .then(|| format!("/api/v1/alerts/exports/{}/download", row.3)),
            email_notification_status: row.4,
            created_at: row.5,
            completed_at: row.6,
        })
    }

    pub async fn changes_since(
        &self,
        ctx: &AuthContext,
        since: DateTime<Utc>,
    ) -> Result<Vec<AlertChangeEvent>, AlertDashboardError> {
        let rows: Vec<(Uuid, Uuid, String, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT lifecycle.id, lifecycle.alert_instance_id, lifecycle.to_status,
                   lifecycle.occurred_at
              FROM alert_lifecycle_events lifecycle
             WHERE lifecycle.owner_id = $1 AND lifecycle.occurred_at > $2
             ORDER BY lifecycle.occurred_at, lifecycle.event_sequence
             LIMIT 1000
            "#,
        )
        .bind(ctx.owner_id)
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, alert_instance_id, to_status, occurred_at)| AlertChangeEvent {
                    id,
                    alert_instance_id,
                    to_status,
                    occurred_at,
                },
            )
            .collect())
    }

    async fn authorized_query(
        &self,
        ctx: &AuthContext,
        mut query: AlertInstanceListQuery,
    ) -> Result<AlertInstanceListQuery, AlertDashboardError> {
        if ctx
            .permissions
            .iter()
            .any(|value| value == "hal.alert.read.all")
        {
            return Ok(query);
        }
        let scopes: Vec<Uuid> = sqlx::query_scalar(
            "SELECT warehouse_id FROM auth_user_warehouse_scopes WHERE user_id = $1 AND owner_id = $2 ORDER BY warehouse_id",
        )
        .bind(ctx.user_id)
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        match query.warehouse_id {
            Some(warehouse_id) if scopes.contains(&warehouse_id) => {}
            Some(_) => return Err(AlertDashboardError::WarehouseScopeDenied),
            None if scopes.len() == 1 => query.warehouse_id = scopes.first().copied(),
            None => return Err(AlertDashboardError::WarehouseScopeRequired),
        }
        Ok(query)
    }

    async fn audit_query<T: serde::Serialize>(
        &self,
        ctx: &AuthContext,
        action: &str,
        filters: &T,
        now: DateTime<Utc>,
    ) -> Result<(), AlertDashboardError> {
        let after = serde_json::to_value(filters)
            .map_err(|error| AlertDashboardError::Serialize(error.to_string()))?;
        append_event(
            &self.pool,
            &AuditWriteRequest {
                occurred_at: now,
                actor_id: ctx.user_id,
                actor_name: ctx.actor_name.clone(),
                owner_id: ctx.owner_id,
                jti: ctx.jti.clone(),
                action: action.to_string(),
                module: "H-AL".to_string(),
                resource_type: "alert_report".to_string(),
                resource_id: format!("{}:{now}", ctx.user_id),
                diff: Some(AuditDiff {
                    before: serde_json::json!({}),
                    after,
                    changed_keys: vec!["filters".to_string()],
                }),
                request_id: None,
                ip: None,
                user_agent: Some("wms-hal-dashboard".to_string()),
            },
        )
        .await
        .map_err(|error| AlertDashboardError::Audit(format!("{error:?}")))?;
        Ok(())
    }
}

fn map_monthly(row: MonthlyRow) -> AlertMonthlyMetric {
    AlertMonthlyMetric {
        month: row.month,
        triggered_count: row.triggered_count,
        acknowledgement_rate: row.acknowledgement_rate,
        average_response_seconds: row.average_response_seconds,
        escalation_rate: row.escalation_rate,
    }
}

fn map_ranking(row: RankingRow) -> AlertRankingItem {
    AlertRankingItem {
        key: row.key,
        count: row.count,
        average_response_seconds: row.average_response_seconds,
        unacknowledged_count: row.unacknowledged_count,
    }
}

fn repository_error(error: AlertInstanceRepositoryError) -> AlertDashboardError {
    match error {
        AlertInstanceRepositoryError::NotFound => AlertDashboardError::NotFound,
        AlertInstanceRepositoryError::Database(message) => AlertDashboardError::Database(message),
    }
}

fn db_error(error: sqlx::Error) -> AlertDashboardError {
    AlertDashboardError::Database(error.to_string())
}
