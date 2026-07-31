use chrono::{DateTime, Utc};
use sqlx::{types::Json, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{AlertDefinitionChangeOperation, SubmitAlertDefinitionChangeRequest};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    operation_context::OperationContext as AuthContext,
};

use super::{
    invalid_shape, map_db_error, map_delete_error, map_write_error, normalize_condition,
    normalize_optional, normalized_roles, normalized_templates, serialize_error, validate_change,
    AlertDefinitionRepositoryError, AlertDefinitionRow, COLUMNS,
};

pub(crate) async fn apply_approved_change_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    request: &SubmitAlertDefinitionChangeRequest,
    now: DateTime<Utc>,
) -> Result<(), AlertDefinitionRepositoryError> {
    validate_change(request)?;
    match request.operation {
        AlertDefinitionChangeOperation::Upsert => {
            let draft = request.definition.as_ref().ok_or_else(invalid_shape)?;
            ensure_dependencies(tx, ctx.owner_id, draft).await?;
            if let Some(id) = request.definition_id {
                let expected_version = request.expected_version.ok_or_else(invalid_shape)?;
                let before = load_for_update(tx, ctx.owner_id, id).await?;
                if before.version != expected_version {
                    return Err(AlertDefinitionRepositoryError::StaleVersion);
                }
                if before.is_gsp_forced && draft.is_disable_allowed {
                    return Err(AlertDefinitionRepositoryError::GspForcedCannotDisable);
                }
                let after = sqlx::query_as::<_, AlertDefinitionRow>(&format!(
                    r#"
                    UPDATE alert_definitions
                       SET alert_code = $3, name = $4, event_type = $5,
                           condition_expression = $6, default_severity = $7,
                           recipient_roles = $8, escalation_ref = $9,
                           silence_period_seconds = $10, is_disable_allowed = $11,
                           message_template = $12, message_templates = $13,
                           updated_at = $14, version = version + 1
                     WHERE owner_id = $1 AND id = $2 AND version = $15
                     RETURNING {COLUMNS}
                    "#,
                ))
                .bind(ctx.owner_id)
                .bind(id)
                .bind(draft.alert_code.trim())
                .bind(draft.name.trim())
                .bind(draft.event_type.trim())
                .bind(normalize_condition(&draft.condition_expression))
                .bind(draft.default_severity.trim())
                .bind(normalized_roles(&draft.recipient_roles))
                .bind(normalize_optional(draft.escalation_ref.as_deref()))
                .bind(draft.silence_period_seconds)
                .bind(draft.is_disable_allowed)
                .bind(draft.message_template.trim())
                .bind(Json(normalized_templates(&draft.message_templates)))
                .bind(now)
                .bind(expected_version)
                .fetch_optional(&mut **tx)
                .await
                .map_err(map_write_error)?
                .ok_or(AlertDefinitionRepositoryError::StaleVersion)?;
                append_change_audit(
                    tx,
                    ctx,
                    "upsert_alert_definition",
                    id,
                    serde_json::to_value(before.into_domain()).map_err(serialize_error)?,
                    serde_json::to_value(after.into_domain()).map_err(serialize_error)?,
                    now,
                )
                .await?;
            } else {
                let id = Uuid::new_v4();
                let after = sqlx::query_as::<_, AlertDefinitionRow>(&format!(
                    r#"
                    INSERT INTO alert_definitions (
                        id, owner_id, alert_code, name, event_type, condition_expression,
                        default_severity, recipient_roles, escalation_ref,
                        silence_period_seconds, is_disable_allowed, enabled,
                        message_template, message_templates, is_gsp_forced, created_at, updated_at
                    ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,TRUE,$12,$13,FALSE,$14,$14)
                    RETURNING {COLUMNS}
                    "#,
                ))
                .bind(id)
                .bind(ctx.owner_id)
                .bind(draft.alert_code.trim())
                .bind(draft.name.trim())
                .bind(draft.event_type.trim())
                .bind(normalize_condition(&draft.condition_expression))
                .bind(draft.default_severity.trim())
                .bind(normalized_roles(&draft.recipient_roles))
                .bind(normalize_optional(draft.escalation_ref.as_deref()))
                .bind(draft.silence_period_seconds)
                .bind(draft.is_disable_allowed)
                .bind(draft.message_template.trim())
                .bind(Json(normalized_templates(&draft.message_templates)))
                .bind(now)
                .fetch_one(&mut **tx)
                .await
                .map_err(map_write_error)?;
                append_change_audit(
                    tx,
                    ctx,
                    "upsert_alert_definition",
                    id,
                    serde_json::json!({}),
                    serde_json::to_value(after.into_domain()).map_err(serialize_error)?,
                    now,
                )
                .await?;
            }
        }
        AlertDefinitionChangeOperation::SetEnabled => {
            let id = request.definition_id.ok_or_else(invalid_shape)?;
            let expected_version = request.expected_version.ok_or_else(invalid_shape)?;
            let enabled = request.enabled.ok_or_else(invalid_shape)?;
            let before = load_for_update(tx, ctx.owner_id, id).await?;
            if before.version != expected_version {
                return Err(AlertDefinitionRepositoryError::StaleVersion);
            }
            if !enabled && before.is_gsp_forced {
                return Err(AlertDefinitionRepositoryError::GspForcedCannotDisable);
            }
            if !enabled && !before.is_disable_allowed {
                return Err(AlertDefinitionRepositoryError::DisableNotAllowed);
            }
            let after = sqlx::query_as::<_, AlertDefinitionRow>(&format!(
                "UPDATE alert_definitions SET enabled = $3, updated_at = $4, version = version + 1 WHERE owner_id = $1 AND id = $2 AND version = $5 RETURNING {COLUMNS}"
            ))
            .bind(ctx.owner_id)
            .bind(id)
            .bind(enabled)
            .bind(now)
            .bind(expected_version)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_write_error)?
            .ok_or(AlertDefinitionRepositoryError::StaleVersion)?;
            append_change_audit(
                tx,
                ctx,
                "set_alert_definition_enabled",
                id,
                serde_json::to_value(before.into_domain()).map_err(serialize_error)?,
                serde_json::to_value(after.into_domain()).map_err(serialize_error)?,
                now,
            )
            .await?;
        }
        AlertDefinitionChangeOperation::Delete => {
            let id = request.definition_id.ok_or_else(invalid_shape)?;
            let expected_version = request.expected_version.ok_or_else(invalid_shape)?;
            let before = load_for_update(tx, ctx.owner_id, id).await?;
            if before.version != expected_version {
                return Err(AlertDefinitionRepositoryError::StaleVersion);
            }
            if before.is_gsp_forced {
                return Err(AlertDefinitionRepositoryError::GspForcedCannotDelete);
            }
            sqlx::query(
                "DELETE FROM alert_definitions WHERE owner_id = $1 AND id = $2 AND version = $3",
            )
            .bind(ctx.owner_id)
            .bind(id)
            .bind(expected_version)
            .execute(&mut **tx)
            .await
            .map_err(map_delete_error)?;
            append_change_audit(
                tx,
                ctx,
                "delete_alert_definition",
                id,
                serde_json::to_value(before.into_domain()).map_err(serialize_error)?,
                serde_json::json!({}),
                now,
            )
            .await?;
        }
    }
    Ok(())
}

async fn ensure_dependencies(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    draft: &wms_domain::AlertDefinitionDraft,
) -> Result<(), AlertDefinitionRepositoryError> {
    let channel_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM h4_notification_configs WHERE owner_id = $1 AND event_type = $2 AND enabled AND cardinality(channels) > 0)",
    )
    .bind(owner_id)
    .bind(draft.event_type.trim())
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if !channel_exists {
        return Err(AlertDefinitionRepositoryError::ChannelNotFound);
    }
    if let Some(rule_code) = normalize_optional(draft.escalation_ref.as_deref()) {
        let rule_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM alert_escalation_rules WHERE owner_id = $1 AND rule_code = $2 AND enabled)",
        )
        .bind(owner_id)
        .bind(rule_code)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if !rule_exists {
            return Err(AlertDefinitionRepositoryError::EscalationRuleNotFound);
        }
    }
    Ok(())
}

async fn load_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<AlertDefinitionRow, AlertDefinitionRepositoryError> {
    sqlx::query_as::<_, AlertDefinitionRow>(&format!(
        "SELECT {COLUMNS} FROM alert_definitions WHERE owner_id = $1 AND id = $2 FOR UPDATE"
    ))
    .bind(owner_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(AlertDefinitionRepositoryError::NotFound)
}

async fn append_change_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_id: Uuid,
    before: serde_json::Value,
    after: serde_json::Value,
    now: DateTime<Utc>,
) -> Result<(), AlertDefinitionRepositoryError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H-AL",
        "alert_definition",
        resource_id.to_string(),
        Some(AuditDiff::compute(before, after)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| AlertDefinitionRepositoryError::Audit(format!("{error:?}")))
}
