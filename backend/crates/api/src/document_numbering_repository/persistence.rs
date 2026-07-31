use chrono::{DateTime, Datelike, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::DocumentNumberAllocation;

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    idempotency,
    operation_context::OperationContext as AuthContext,
};

use super::support::map_db_error;
use super::{
    AllocationRow, AllocationWithHashRow, DocumentNumberingError, GenerateDocumentNumberRequest,
    RuleRow, UpsertDocumentNumberRuleRequest,
};

pub(super) fn validate_rule_request(
    req: &UpsertDocumentNumberRuleRequest,
) -> Result<(), DocumentNumberingError> {
    if req.sequence_width <= 0 || req.sequence_width > 18 {
        return Err(DocumentNumberingError::InvalidRule);
    }
    if !matches!(req.reset_policy.as_str(), "daily" | "continuous") {
        return Err(DocumentNumberingError::InvalidRule);
    }
    if let (Some(from), Some(to)) = (req.effective_from, req.effective_to) {
        if to <= from {
            return Err(DocumentNumberingError::InvalidEffectiveWindow);
        }
    }
    validate_template(&req.template)
}

pub(super) fn validate_template(template: &str) -> Result<(), DocumentNumberingError> {
    if !template.contains("{SEQ}") {
        return Err(DocumentNumberingError::TemplateInvalid);
    }
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('}') else {
            return Err(DocumentNumberingError::TemplateInvalid);
        };
        let token = &after_start[..end];
        if !matches!(
            token,
            "OWNER" | "DOCUMENT_TYPE" | "YYYY" | "YY" | "MM" | "DD" | "SEQ"
        ) {
            return Err(DocumentNumberingError::TemplateInvalid);
        }
        rest = &after_start[end + 1..];
    }
    if rest.contains('}') {
        return Err(DocumentNumberingError::TemplateInvalid);
    }
    Ok(())
}
pub(super) async fn load_rule_id_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    rule_code: &str,
) -> Result<Option<Uuid>, DocumentNumberingError> {
    sqlx::query_scalar(
        r#"
        SELECT id
          FROM document_number_rules
         WHERE owner_id = $1 AND rule_code = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(rule_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub(super) async fn ensure_document_type_valid(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    document_type: &str,
    effective_at: DateTime<Utc>,
) -> Result<(), DocumentNumberingError> {
    let (dict_code, item_code) = match document_type.split_once(':') {
        Some((dict_code, item_code))
            if !dict_code.trim().is_empty() && !item_code.trim().is_empty() =>
        {
            (dict_code, item_code)
        }
        Some(_) => return Err(DocumentNumberingError::DocumentTypeInvalid),
        None => ("document_type", document_type),
    };
    if !matches!(dict_code, "document_type" | "print_document_category") {
        return Err(DocumentNumberingError::DocumentTypeInvalid);
    }
    let params: Option<Value> = sqlx::query_scalar(
        r#"
        WITH scoped_items AS (
            SELECT
                params,
                enabled,
                ROW_NUMBER() OVER (
                    PARTITION BY item_code
                    ORDER BY CASE WHEN owner_id = $3 THEN 1 ELSE 0 END DESC, updated_at DESC
                ) AS scope_rank
              FROM system_dictionary_items
             WHERE dict_code = $1
               AND item_code = $2
               AND (owner_id IS NULL OR owner_id = $3)
               AND (effective_from IS NULL OR effective_from <= $4)
               AND (effective_to IS NULL OR effective_to > $4)
        )
        SELECT params
          FROM scoped_items
         WHERE scope_rank = 1 AND enabled = TRUE
        "#,
    )
    .bind(dict_code)
    .bind(item_code)
    .bind(owner_id)
    .bind(effective_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let Some(params) = params else {
        return Err(DocumentNumberingError::DocumentTypeInvalid);
    };
    let required_keys: &[&str] = if dict_code == "document_type" {
        &["direction", "workflow_template", "batch_policy"]
    } else {
        &["source_mode"]
    };
    for key in required_keys {
        if !params.get(key).is_some_and(Value::is_string) {
            return Err(DocumentNumberingError::DocumentTypeInvalid);
        }
    }
    Ok(())
}

pub(super) async fn load_owner_code(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
) -> Result<String, DocumentNumberingError> {
    sqlx::query_scalar("SELECT owner_code FROM auth_owners WHERE id = $1")
        .bind(owner_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?
        .ok_or(DocumentNumberingError::RuleNotFound)
}

pub(super) async fn load_effective_rule(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    document_type: &str,
    effective_at: DateTime<Utc>,
) -> Result<RuleRow, DocumentNumberingError> {
    sqlx::query_as::<_, RuleRow>(
        r#"
        SELECT id, document_type, template, reset_policy, sequence_width
          FROM document_number_rules
         WHERE document_type = $1
           AND (owner_id IS NULL OR owner_id = $2)
           AND enabled = TRUE
           AND (effective_from IS NULL OR effective_from <= $3)
           AND (effective_to IS NULL OR effective_to > $3)
         ORDER BY CASE WHEN owner_id = $2 THEN 1 ELSE 0 END DESC, updated_at DESC
         LIMIT 1
        "#,
    )
    .bind(document_type)
    .bind(owner_id)
    .bind(effective_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(DocumentNumberingError::RuleNotFound)
}

pub(super) async fn next_sequence_value(
    tx: &mut Transaction<'_, Postgres>,
    rule_id: Uuid,
    counter_key: &str,
    width: i32,
    now: DateTime<Utc>,
) -> Result<i64, DocumentNumberingError> {
    sqlx::query(
        r#"
        INSERT INTO document_number_counters (
            id, rule_id, counter_key, current_value, created_at, updated_at
        )
        VALUES ($1, $2, $3, 0, $4, $4)
        ON CONFLICT (rule_id, counter_key) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(rule_id)
    .bind(counter_key)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let current_value: i64 = sqlx::query_scalar(
        r#"
        SELECT current_value
          FROM document_number_counters
         WHERE rule_id = $1 AND counter_key = $2
         FOR UPDATE
        "#,
    )
    .bind(rule_id)
    .bind(counter_key)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let next_value = current_value + 1;
    let max_value = 10_i64
        .checked_pow(width as u32)
        .and_then(|value| value.checked_sub(1))
        .ok_or(DocumentNumberingError::SequenceOverflow)?;
    if next_value > max_value {
        return Err(DocumentNumberingError::SequenceOverflow);
    }

    sqlx::query(
        r#"
        UPDATE document_number_counters
           SET current_value = $1, updated_at = $2, version = version + 1
         WHERE rule_id = $3 AND counter_key = $4
        "#,
    )
    .bind(next_value)
    .bind(now)
    .bind(rule_id)
    .bind(counter_key)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(next_value)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_allocation(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    req: &GenerateDocumentNumberRequest,
    request_hash: &str,
    rule: &RuleRow,
    counter_key: &str,
    sequence_value: i64,
    generated_no: &str,
    now: DateTime<Utc>,
) -> Result<DocumentNumberAllocation, DocumentNumberingError> {
    let row = sqlx::query_as::<_, AllocationRow>(
        r#"
        INSERT INTO document_number_allocations (
            id, owner_id, rule_id, document_type, idempotency_key, request_hash,
            generated_no, sequence_value, counter_key, source_module, source_document_id,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING id, owner_id, rule_id, document_type, generated_no, sequence_value,
                  counter_key, source_module, source_document_id, created_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(rule.id)
    .bind(&rule.document_type)
    .bind(&req.idempotency_key)
    .bind(request_hash)
    .bind(generated_no)
    .bind(sequence_value)
    .bind(counter_key)
    .bind(&req.source_module)
    .bind(req.source_document_id)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(row.into())
}

pub(super) async fn load_allocation_by_idempotency(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<(DocumentNumberAllocation, String)>, DocumentNumberingError> {
    let row: Option<AllocationWithHashRow> = sqlx::query_as(
        r#"
        SELECT
            id, owner_id, rule_id, document_type, generated_no, sequence_value,
            counter_key, source_module, source_document_id, created_at, request_hash
          FROM document_number_allocations
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(row.map(|row| {
        let hash = row.request_hash.clone();
        (row.into(), hash)
    }))
}

pub(super) async fn append_generation_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    allocation: &DocumentNumberAllocation,
    now: DateTime<Utc>,
) -> Result<(), DocumentNumberingError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        "generate_document_number",
        "M-CG",
        "document_number_allocation",
        allocation.id.to_string(),
        None,
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| DocumentNumberingError::Audit(format!("{error:?}")))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finish_rule_mutation<T: Serialize>(
    mut tx: Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    response: &T,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), DocumentNumberingError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| DocumentNumberingError::Serialize(error.to_string()))?;
    let resource_id = response_body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("document_number_rule")
        .to_string();
    idempotency::store_success(
        &mut tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        "document_number_rule",
        &resource_id,
        response,
        now,
    )
    .await?;
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M-CG",
        "document_number_rule",
        resource_id,
        None,
    );
    audit.occurred_at = now;
    append_event_in_tx(&mut tx, &audit)
        .await
        .map_err(|error| DocumentNumberingError::Audit(format!("{error:?}")))?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(())
}

pub(super) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, DocumentNumberingError> {
    idempotency::replay(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        now,
    )
    .await
    .map_err(Into::into)
}

pub(super) fn counter_key(
    owner_id: Uuid,
    document_type: &str,
    reset_policy: &str,
    occurred_at: DateTime<Utc>,
) -> String {
    match reset_policy {
        "daily" => format!(
            "{owner_id}:{document_type}:{}{:02}{:02}",
            occurred_at.year(),
            occurred_at.month(),
            occurred_at.day()
        ),
        _ => format!("{owner_id}:{document_type}:continuous"),
    }
}

pub(super) fn render_number(
    rule: &RuleRow,
    owner_code: &str,
    sequence_value: i64,
    occurred_at: DateTime<Utc>,
) -> Result<String, DocumentNumberingError> {
    let width = usize::try_from(rule.sequence_width)
        .map_err(|error| DocumentNumberingError::Database(error.to_string()))?;
    let seq = format!("{sequence_value:0width$}");
    Ok(rule
        .template
        .replace("{OWNER}", owner_code)
        .replace("{DOCUMENT_TYPE}", &rule.document_type)
        .replace("{YYYY}", &format!("{:04}", occurred_at.year()))
        .replace(
            "{YY}",
            &format!("{:02}", occurred_at.year().rem_euclid(100)),
        )
        .replace("{MM}", &format!("{:02}", occurred_at.month()))
        .replace("{DD}", &format!("{:02}", occurred_at.day()))
        .replace("{SEQ}", &seq))
}
