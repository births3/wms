use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sqlx::{types::Json, FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{
    AlertDefinition, AlertDefinitionChangeOperation, AlertDefinitionDraft,
    AlertDefinitionListQuery, CreateAlertDefinitionRequest, SubmitAlertDefinitionChangeRequest,
};

mod workflow;

pub(crate) use workflow::apply_approved_change_in_tx;

#[derive(Clone, Debug)]
pub struct PgAlertDefinitionRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertDefinitionRepositoryError {
    Database(String),
    Audit(String),
    DuplicateCode,
    GspForcedCannotDisable,
    GspForcedCannotDelete,
    DisableNotAllowed,
    ConditionInvalid,
    ChannelNotFound,
    EscalationRuleNotFound,
    InUse,
    Invalid(String),
    NotFound,
    StaleVersion,
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
    enabled: bool,
    message_template: String,
    message_templates: Json<BTreeMap<String, String>>,
    is_gsp_forced: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

const COLUMNS: &str = "id, owner_id, alert_code, name, event_type, condition_expression, default_severity, recipient_roles, escalation_ref, silence_period_seconds, is_disable_allowed, enabled, message_template, message_templates, is_gsp_forced, created_at, updated_at, version";

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
                   message_templates, is_gsp_forced, created_at, updated_at
               ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$15)
               RETURNING id, owner_id, alert_code, name, event_type, condition_expression,
                         default_severity, recipient_roles, escalation_ref,
                         silence_period_seconds, is_disable_allowed, enabled, message_template,
                         message_templates, is_gsp_forced, created_at, updated_at, version"#,
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
        .bind(Json(BTreeMap::from([(
            "zh-CN".to_string(),
            request.message_template.trim().to_string(),
        )])))
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
                  SET is_disable_allowed = $3, updated_at = $4, version = version + 1
                WHERE id = $1 AND owner_id = $2
                RETURNING id, owner_id, alert_code, name, event_type, condition_expression,
                          default_severity, recipient_roles, escalation_ref,
                          silence_period_seconds, is_disable_allowed, enabled, message_template,
                          message_templates, is_gsp_forced, created_at, updated_at, version"#,
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
        let forced: Option<bool> = sqlx::query_scalar(
            "SELECT is_gsp_forced FROM alert_definitions WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?;
        match forced {
            None => return Err(AlertDefinitionRepositoryError::NotFound),
            Some(true) => return Err(AlertDefinitionRepositoryError::GspForcedCannotDelete),
            Some(false) => {}
        }
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

    pub async fn list(
        &self,
        owner_id: Uuid,
        query: &AlertDefinitionListQuery,
    ) -> Result<Vec<AlertDefinition>, AlertDefinitionRepositoryError> {
        let keyword = query.keyword.as_deref().map(str::trim).unwrap_or("");
        let severity = query
            .severity
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(value) = severity {
            validate_severity(value)?;
        }
        let limit = query.limit.unwrap_or(100).clamp(1, 500);
        sqlx::query_as::<_, AlertDefinitionRow>(&format!(
            r#"
            SELECT {COLUMNS}
              FROM alert_definitions
             WHERE owner_id = $1
               AND ($2 = '' OR alert_code ILIKE '%' || $2 || '%'
                            OR name ILIKE '%' || $2 || '%'
                            OR event_type ILIKE '%' || $2 || '%')
               AND ($3::TEXT IS NULL OR default_severity = $3)
               AND ($4::BOOLEAN IS NULL OR enabled = $4)
             ORDER BY is_gsp_forced DESC, updated_at DESC, alert_code
             LIMIT $5
            "#,
        ))
        .bind(owner_id)
        .bind(keyword)
        .bind(severity)
        .bind(query.enabled)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)
        .map(|rows| {
            rows.into_iter()
                .map(AlertDefinitionRow::into_domain)
                .collect()
        })
    }

    pub async fn get(
        &self,
        owner_id: Uuid,
        id: Uuid,
    ) -> Result<AlertDefinition, AlertDefinitionRepositoryError> {
        sqlx::query_as::<_, AlertDefinitionRow>(&format!(
            "SELECT {COLUMNS} FROM alert_definitions WHERE owner_id = $1 AND id = $2"
        ))
        .bind(owner_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .map(AlertDefinitionRow::into_domain)
        .ok_or(AlertDefinitionRepositoryError::NotFound)
    }

    pub fn validate_change(
        &self,
        request: &SubmitAlertDefinitionChangeRequest,
    ) -> Result<(), AlertDefinitionRepositoryError> {
        validate_change(request)
    }

    pub async fn ensure_notification_channel(
        &self,
        owner_id: Uuid,
        event_type: &str,
    ) -> Result<(), AlertDefinitionRepositoryError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM h4_notification_configs
                 WHERE owner_id = $1
                   AND event_type = $2
                   AND enabled
                   AND cardinality(channels) > 0
            )
            "#,
        )
        .bind(owner_id)
        .bind(event_type.trim())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        exists
            .then_some(())
            .ok_or(AlertDefinitionRepositoryError::ChannelNotFound)
    }

    pub async fn ensure_escalation_rule(
        &self,
        owner_id: Uuid,
        rule_code: &str,
    ) -> Result<(), AlertDefinitionRepositoryError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM alert_escalation_rules WHERE owner_id = $1 AND rule_code = $2 AND enabled)",
        )
        .bind(owner_id)
        .bind(rule_code.trim())
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        exists
            .then_some(())
            .ok_or(AlertDefinitionRepositoryError::EscalationRuleNotFound)
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

fn validate_change(
    request: &SubmitAlertDefinitionChangeRequest,
) -> Result<(), AlertDefinitionRepositoryError> {
    match request.operation {
        AlertDefinitionChangeOperation::Upsert => {
            let draft = request.definition.as_ref().ok_or_else(invalid_shape)?;
            validate_draft(draft)?;
            if request.definition_id.is_some() != request.expected_version.is_some()
                || request.enabled.is_some()
            {
                return Err(invalid_shape());
            }
        }
        AlertDefinitionChangeOperation::SetEnabled => {
            if request.definition_id.is_none()
                || request.expected_version.is_none()
                || request.enabled.is_none()
                || request.definition.is_some()
            {
                return Err(invalid_shape());
            }
        }
        AlertDefinitionChangeOperation::Delete => {
            if request.definition_id.is_none()
                || request.expected_version.is_none()
                || request.definition.is_some()
                || request.enabled.is_some()
            {
                return Err(invalid_shape());
            }
        }
    }
    Ok(())
}

fn validate_draft(draft: &AlertDefinitionDraft) -> Result<(), AlertDefinitionRepositoryError> {
    let code = draft.alert_code.trim();
    let event_type = draft.event_type.trim();
    let valid_identifier = |value: &str| {
        (2..=128).contains(&value.len())
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    };
    if !valid_identifier(code) || !valid_identifier(event_type) {
        return Err(AlertDefinitionRepositoryError::Invalid(
            "告警编码或事件类型格式非法".to_string(),
        ));
    }
    if draft.name.trim().is_empty() || draft.message_template.trim().is_empty() {
        return Err(AlertDefinitionRepositoryError::Invalid(
            "告警名称和中文模板不能为空".to_string(),
        ));
    }
    let templates = normalized_templates(&draft.message_templates);
    if templates.get("zh-CN").map(String::as_str) != Some(draft.message_template.trim()) {
        return Err(AlertDefinitionRepositoryError::Invalid(
            "多语言模板必须包含与中文模板一致的 zh-CN".to_string(),
        ));
    }
    validate_severity(draft.default_severity.trim())?;
    let condition = draft.condition_expression.trim();
    if !condition.is_empty() && serde_json::from_str::<serde_json::Value>(condition).is_err() {
        return Err(AlertDefinitionRepositoryError::ConditionInvalid);
    }
    let roles = normalized_roles(&draft.recipient_roles);
    const ALLOWED_ROLES: &[&str] = &[
        "warehouse_manager",
        "maintenance_operator",
        "system_admin",
        "owner_contact",
    ];
    if roles.is_empty()
        || roles
            .iter()
            .any(|role| !ALLOWED_ROLES.contains(&role.as_str()))
    {
        return Err(AlertDefinitionRepositoryError::Invalid(
            "接收人角色不在受控范围".to_string(),
        ));
    }
    if draft.silence_period_seconds < 0 {
        return Err(AlertDefinitionRepositoryError::Invalid(
            "静默期不能为负数".to_string(),
        ));
    }
    Ok(())
}

fn validate_severity(value: &str) -> Result<(), AlertDefinitionRepositoryError> {
    if matches!(value, "info" | "warning" | "critical") {
        Ok(())
    } else {
        Err(AlertDefinitionRepositoryError::Invalid(
            "默认级别必须是 info、warning 或 critical".to_string(),
        ))
    }
}

fn normalized_roles(values: &[String]) -> Vec<String> {
    let mut roles = values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    roles.sort();
    roles.dedup();
    roles
}

fn normalized_templates(values: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    values
        .iter()
        .filter_map(|(locale, template)| {
            let locale = locale.trim();
            let template = template.trim();
            (!locale.is_empty() && !template.is_empty())
                .then(|| (locale.to_string(), template.to_string()))
        })
        .collect()
}

fn normalize_condition(value: &str) -> &str {
    let value = value.trim();
    if value.is_empty() {
        "{}"
    } else {
        value
    }
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn invalid_shape() -> AlertDefinitionRepositoryError {
    AlertDefinitionRepositoryError::Invalid("告警定义变更请求结构非法".to_string())
}

fn serialize_error(error: serde_json::Error) -> AlertDefinitionRepositoryError {
    AlertDefinitionRepositoryError::Database(error.to_string())
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
            enabled: self.enabled,
            message_template: self.message_template,
            message_templates: self.message_templates.0,
            is_gsp_forced: self.is_gsp_forced,
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
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
        match database.code().as_deref() {
            Some("23503") => return AlertDefinitionRepositoryError::InUse,
            Some("23514") => return AlertDefinitionRepositoryError::GspForcedCannotDelete,
            _ => {}
        }
    }
    map_db_error(error)
}

fn map_db_error(error: sqlx::Error) -> AlertDefinitionRepositoryError {
    AlertDefinitionRepositoryError::Database(error.to_string())
}
