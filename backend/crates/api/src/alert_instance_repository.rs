use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{AlertInstance, AlertInstanceListQuery};

#[derive(Clone, Debug)]
pub struct PgAlertInstanceRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertInstanceRepositoryError {
    NotFound,
    Database(String),
}

#[derive(Debug, FromRow)]
struct AlertInstanceRow {
    id: Uuid,
    owner_id: Uuid,
    alert_definition_id: Uuid,
    alert_code: String,
    alert_name: String,
    severity: String,
    event_type: String,
    resource_type: String,
    resource_id: String,
    resource_path: Option<String>,
    warehouse_id: Option<Uuid>,
    event_payload: serde_json::Value,
    recipients: Vec<String>,
    status: String,
    escalation_level: i32,
    action_description: Option<String>,
    ignored_reason: Option<String>,
    close_reason: Option<String>,
    triggered_at: chrono::DateTime<chrono::Utc>,
    notified_at: Option<chrono::DateTime<chrono::Utc>>,
    acknowledged_at: Option<chrono::DateTime<chrono::Utc>>,
    handled_at: Option<chrono::DateTime<chrono::Utc>>,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

const COLUMNS: &str = r#"
    instance.id, instance.owner_id, instance.alert_definition_id,
    instance.alert_code, definition.name AS alert_name, instance.severity,
    instance.event_type, instance.resource_type, instance.resource_id,
    instance.resource_path, instance.warehouse_id, instance.event_payload,
    instance.recipients, instance.status, instance.escalation_level,
    instance.action_description, instance.ignored_reason, instance.close_reason,
    instance.triggered_at, instance.notified_at, instance.acknowledged_at,
    instance.handled_at, instance.closed_at, instance.created_at, instance.updated_at
"#;

impl PgAlertInstanceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        owner_id: Uuid,
        query: &AlertInstanceListQuery,
    ) -> Result<Vec<AlertInstance>, AlertInstanceRepositoryError> {
        let sql = format!(
            r#"
            SELECT {COLUMNS}
              FROM alert_instances instance
              JOIN alert_definitions definition ON definition.id = instance.alert_definition_id
             WHERE instance.owner_id = $1
               AND ($2::UUID IS NULL OR instance.warehouse_id = $2)
               AND ($3::TEXT IS NULL OR instance.severity = $3)
               AND ($4::TEXT IS NULL OR instance.status = $4)
               AND ($5::TEXT IS NULL OR instance.alert_code = $5)
               AND ($6::TIMESTAMPTZ IS NULL OR instance.triggered_at >= $6)
               AND ($7::TIMESTAMPTZ IS NULL OR instance.triggered_at <= $7)
               AND (NOT $8 OR instance.status NOT IN ('closed', 'ignored'))
             ORDER BY CASE instance.severity
                        WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
                      instance.triggered_at DESC, instance.id DESC
             LIMIT $9
            "#,
        );
        let rows: Vec<AlertInstanceRow> = sqlx::query_as(&sql)
            .bind(owner_id)
            .bind(query.warehouse_id)
            .bind(query.severity.as_deref())
            .bind(query.status.as_deref())
            .bind(query.alert_code.as_deref())
            .bind(query.from)
            .bind(query.to)
            .bind(query.active_only.unwrap_or(false))
            .bind(query.limit.unwrap_or(100).clamp(1, 1000))
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<AlertInstance, AlertInstanceRepositoryError> {
        let sql = format!(
            "SELECT {COLUMNS} FROM alert_instances instance JOIN alert_definitions definition ON definition.id = instance.alert_definition_id WHERE instance.owner_id = $1 AND instance.id = $2"
        );
        sqlx::query_as::<_, AlertInstanceRow>(&sql)
            .bind(owner_id)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .map(Into::into)
            .ok_or(AlertInstanceRepositoryError::NotFound)
    }
}

impl From<AlertInstanceRow> for AlertInstance {
    fn from(row: AlertInstanceRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            alert_definition_id: row.alert_definition_id,
            alert_code: row.alert_code,
            alert_name: row.alert_name,
            severity: row.severity,
            event_type: row.event_type,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            resource_path: row.resource_path,
            warehouse_id: row.warehouse_id,
            event_payload: row.event_payload,
            recipients: row.recipients,
            status: row.status,
            escalation_level: row.escalation_level,
            action_description: row.action_description,
            ignored_reason: row.ignored_reason,
            close_reason: row.close_reason,
            triggered_at: row.triggered_at,
            notified_at: row.notified_at,
            acknowledged_at: row.acknowledged_at,
            handled_at: row.handled_at,
            closed_at: row.closed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn db_error(error: sqlx::Error) -> AlertInstanceRepositoryError {
    AlertInstanceRepositoryError::Database(error.to_string())
}
