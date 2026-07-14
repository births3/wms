use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{AlertDefinition, CreateAlertDefinitionRequest};

#[derive(Clone, Debug)]
pub struct PgAlertDefinitionRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertDefinitionRepositoryError {
    Database(String),
    DuplicateCode,
    GspForcedCannotDisable,
    InUse,
    Invalid(String),
    NotFound,
}

#[derive(Clone, Debug, FromRow)]
struct AlertDefinitionRow {
    id: Uuid,
    owner_id: Uuid,
    alert_code: String,
    name: String,
    event_type: String,
    condition_expression: String,
    default_severity: String,
    recipient_roles: Vec<String>,
    escalation_ref: Option<String>,
    silence_period_seconds: i64,
    is_disable_allowed: bool,
    message_template: String,
    is_gsp_forced: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgAlertDefinitionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        owner_id: Uuid,
        request: &CreateAlertDefinitionRequest,
        now: DateTime<Utc>,
    ) -> Result<AlertDefinition, AlertDefinitionRepositoryError> {
        validate(request)?;
        sqlx::query_as::<_, AlertDefinitionRow>(
            r#"INSERT INTO alert_definitions (
                   id, owner_id, alert_code, name, event_type, condition_expression,
                   default_severity, recipient_roles, escalation_ref,
                   silence_period_seconds, is_disable_allowed, message_template,
                   is_gsp_forced, created_at, updated_at
               ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$14)
               RETURNING id, owner_id, alert_code, name, event_type, condition_expression,
                         default_severity, recipient_roles, escalation_ref,
                         silence_period_seconds, is_disable_allowed, message_template,
                         is_gsp_forced, created_at, updated_at"#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(request.alert_code.trim())
        .bind(request.name.trim())
        .bind(request.event_type.trim())
        .bind(request.condition_expression.trim())
        .bind(request.default_severity.trim())
        .bind(&request.recipient_roles)
        .bind(request.escalation_ref.as_deref().map(str::trim))
        .bind(request.silence_period_seconds)
        .bind(request.is_disable_allowed)
        .bind(request.message_template.trim())
        .bind(request.is_gsp_forced)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map(AlertDefinitionRow::into_domain)
        .map_err(map_write_error)
    }

    pub async fn set_disable_allowed(
        &self,
        owner_id: Uuid,
        id: Uuid,
        is_disable_allowed: bool,
        now: DateTime<Utc>,
    ) -> Result<AlertDefinition, AlertDefinitionRepositoryError> {
        sqlx::query_as::<_, AlertDefinitionRow>(
            r#"UPDATE alert_definitions
                  SET is_disable_allowed = $3, updated_at = $4
                WHERE id = $1 AND owner_id = $2
                RETURNING id, owner_id, alert_code, name, event_type, condition_expression,
                          default_severity, recipient_roles, escalation_ref,
                          silence_period_seconds, is_disable_allowed, message_template,
                          is_gsp_forced, created_at, updated_at"#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(is_disable_allowed)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_write_error)?
        .map(AlertDefinitionRow::into_domain)
        .ok_or(AlertDefinitionRepositoryError::NotFound)
    }

    pub async fn delete(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<(), AlertDefinitionRepositoryError> {
        let result = sqlx::query("DELETE FROM alert_definitions WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(map_delete_error)?;
        (result.rows_affected() == 1)
            .then_some(())
            .ok_or(AlertDefinitionRepositoryError::NotFound)
    }
}

fn validate(request: &CreateAlertDefinitionRequest) -> Result<(), AlertDefinitionRepositoryError> {
    let required = [
        ("alert_code", request.alert_code.trim()),
        ("name", request.name.trim()),
        ("event_type", request.event_type.trim()),
        ("condition_expression", request.condition_expression.trim()),
        ("default_severity", request.default_severity.trim()),
        ("message_template", request.message_template.trim()),
    ];
    required
        .iter()
        .find(|(_, value)| value.is_empty())
        .map(|(field, _)| AlertDefinitionRepositoryError::Invalid(format!("{field} 不能为空")))
        .map_or_else(
            || {
                if request.silence_period_seconds < 0 {
                    Err(AlertDefinitionRepositoryError::Invalid(
                        "silence_period_seconds 不能为负数".to_string(),
                    ))
                } else if request.is_gsp_forced && request.is_disable_allowed {
                    Err(AlertDefinitionRepositoryError::GspForcedCannotDisable)
                } else {
                    Ok(())
                }
            },
            Err,
        )
}

impl AlertDefinitionRow {
    fn into_domain(self) -> AlertDefinition {
        AlertDefinition {
            id: self.id,
            owner_id: self.owner_id,
            alert_code: self.alert_code,
            name: self.name,
            event_type: self.event_type,
            condition_expression: self.condition_expression,
            default_severity: self.default_severity,
            recipient_roles: self.recipient_roles,
            escalation_ref: self.escalation_ref,
            silence_period_seconds: self.silence_period_seconds,
            is_disable_allowed: self.is_disable_allowed,
            message_template: self.message_template,
            is_gsp_forced: self.is_gsp_forced,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn map_write_error(error: sqlx::Error) -> AlertDefinitionRepositoryError {
    if let sqlx::Error::Database(database) = &error {
        return match database.code().as_deref() {
            Some("23505") => AlertDefinitionRepositoryError::DuplicateCode,
            Some("23514") => AlertDefinitionRepositoryError::GspForcedCannotDisable,
            _ => map_db_error(error),
        };
    }
    map_db_error(error)
}

fn map_delete_error(error: sqlx::Error) -> AlertDefinitionRepositoryError {
    if let sqlx::Error::Database(database) = &error {
        if database.code().as_deref() == Some("23503") {
            return AlertDefinitionRepositoryError::InUse;
        }
    }
    map_db_error(error)
}

fn map_db_error(error: sqlx::Error) -> AlertDefinitionRepositoryError {
    AlertDefinitionRepositoryError::Database(error.to_string())
}
