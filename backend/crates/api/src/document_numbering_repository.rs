//! US-CG-001 / US-CG-002 no-gap document numbering service.

use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::{DocumentNumberAllocation, PageMeta};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    auth::AuthContext,
};

pub const DEFAULT_DOCUMENT_NUMBER_ALLOCATION_LIMIT: u32 = 50;
pub const MAX_DOCUMENT_NUMBER_ALLOCATION_LIMIT: u32 = 100;

#[derive(Clone, Debug)]
pub struct PgDocumentNumberingService;

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentNumberingError {
    DocumentTypeInvalid,
    RuleNotFound,
    InvalidRule,
    InvalidEffectiveWindow,
    TemplateInvalid,
    SequenceOverflow,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct UpsertDocumentNumberRuleRequest {
    pub document_type: String,
    pub rule_name: String,
    pub template: String,
    pub reset_policy: String,
    pub sequence_width: i32,
    pub enabled: bool,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct SetDocumentNumberRuleEnabledRequest {
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub struct DocumentNumberRule {
    pub id: Uuid,
    pub owner_id: Option<Uuid>,
    pub document_type: String,
    pub rule_code: String,
    pub rule_name: String,
    pub template: String,
    pub reset_policy: String,
    pub sequence_width: i32,
    pub sequence_mode: String,
    pub enabled: bool,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DocumentNumberRuleListResponse {
    pub data: Vec<DocumentNumberRule>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GenerateDocumentNumberRequest {
    pub document_type: String,
    pub idempotency_key: String,
    pub source_module: String,
    pub source_document_id: Option<Uuid>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentNumberAllocationQuery {
    pub document_type: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: u32,
}

#[derive(Clone, Debug, FromRow)]
struct RuleRow {
    id: Uuid,
    document_type: String,
    template: String,
    reset_policy: String,
    sequence_width: i32,
}

#[derive(Clone, Debug, FromRow)]
struct DocumentNumberRuleRow {
    id: Uuid,
    owner_id: Option<Uuid>,
    document_type: String,
    rule_code: String,
    rule_name: String,
    template: String,
    reset_policy: String,
    sequence_width: i32,
    sequence_mode: String,
    enabled: bool,
    effective_from: Option<DateTime<Utc>>,
    effective_to: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

#[derive(Clone, Debug, FromRow)]
struct AllocationRow {
    id: Uuid,
    owner_id: Uuid,
    rule_id: Uuid,
    document_type: String,
    generated_no: String,
    sequence_value: i64,
    counter_key: String,
    source_module: String,
    source_document_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow)]
struct AllocationWithHashRow {
    id: Uuid,
    owner_id: Uuid,
    rule_id: Uuid,
    document_type: String,
    generated_no: String,
    sequence_value: i64,
    counter_key: String,
    source_module: String,
    source_document_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    request_hash: String,
}

impl PgDocumentNumberingService {
    pub fn new() -> Self {
        Self
    }

    pub async fn list_rules(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        document_type: Option<&str>,
    ) -> Result<Vec<DocumentNumberRule>, DocumentNumberingError> {
        let rows = sqlx::query_as::<_, DocumentNumberRuleRow>(
            r#"
            SELECT id, owner_id, document_type, rule_code, rule_name, template,
                   reset_policy, sequence_width, sequence_mode, enabled,
                   effective_from, effective_to, created_at, updated_at, version
              FROM document_number_rules
             WHERE (owner_id IS NULL OR owner_id = $1)
               AND ($2::TEXT IS NULL OR document_type = $2)
             ORDER BY document_type ASC,
                      CASE WHEN owner_id = $1 THEN 0 ELSE 1 END ASC,
                      updated_at DESC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(document_type)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn upsert_rule(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        rule_code: &str,
        req: UpsertDocumentNumberRuleRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<DocumentNumberRule>, DocumentNumberingError> {
        validate_rule_request(&req)?;
        let request_hash = json_request_hash(&serde_json::json!({
            "rule_code": rule_code,
            "request": &req,
        }))?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        ensure_document_type_valid(&mut tx, ctx.owner_id, &req.document_type, now).await?;

        let existing_id = load_rule_id_for_update(&mut tx, ctx.owner_id, rule_code).await?;
        let row = if let Some(id) = existing_id {
            sqlx::query_as::<_, DocumentNumberRuleRow>(
                r#"
                UPDATE document_number_rules
                   SET document_type = $1,
                       rule_name = $2,
                       template = $3,
                       reset_policy = $4,
                       sequence_width = $5,
                       enabled = $6,
                       effective_from = $7,
                       effective_to = $8,
                       updated_at = $9,
                       version = version + 1
                 WHERE id = $10 AND owner_id = $11
                 RETURNING id, owner_id, document_type, rule_code, rule_name, template,
                           reset_policy, sequence_width, sequence_mode, enabled,
                           effective_from, effective_to, created_at, updated_at, version
                "#,
            )
            .bind(&req.document_type)
            .bind(&req.rule_name)
            .bind(&req.template)
            .bind(&req.reset_policy)
            .bind(req.sequence_width)
            .bind(req.enabled)
            .bind(req.effective_from)
            .bind(req.effective_to)
            .bind(now)
            .bind(id)
            .bind(ctx.owner_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?
        } else {
            sqlx::query_as::<_, DocumentNumberRuleRow>(
                r#"
                INSERT INTO document_number_rules (
                    id, owner_id, document_type, rule_code, rule_name, template,
                    reset_policy, sequence_width, enabled, effective_from, effective_to,
                    created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
                RETURNING id, owner_id, document_type, rule_code, rule_name, template,
                          reset_policy, sequence_width, sequence_mode, enabled,
                          effective_from, effective_to, created_at, updated_at, version
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(&req.document_type)
            .bind(rule_code)
            .bind(&req.rule_name)
            .bind(&req.template)
            .bind(&req.reset_policy)
            .bind(req.sequence_width)
            .bind(req.enabled)
            .bind(req.effective_from)
            .bind(req.effective_to)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?
        };
        let rule = DocumentNumberRule::from(row);
        finish_rule_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "PUT",
            &format!("/api/v1/code-generator/document-number-rules/{rule_code}"),
            &rule,
            "upsert_document_number_rule",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: rule,
            replayed: false,
        })
    }

    pub async fn set_rule_enabled(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        rule_code: &str,
        req: SetDocumentNumberRuleEnabledRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<DocumentNumberRule>, DocumentNumberingError> {
        let request_hash = json_request_hash(&serde_json::json!({
            "rule_code": rule_code,
            "request": &req,
        }))?;
        let mut tx = pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let row = sqlx::query_as::<_, DocumentNumberRuleRow>(
            r#"
            UPDATE document_number_rules
               SET enabled = $1, updated_at = $2, version = version + 1
             WHERE owner_id = $3 AND rule_code = $4
             RETURNING id, owner_id, document_type, rule_code, rule_name, template,
                       reset_policy, sequence_width, sequence_mode, enabled,
                       effective_from, effective_to, created_at, updated_at, version
            "#,
        )
        .bind(req.enabled)
        .bind(now)
        .bind(ctx.owner_id)
        .bind(rule_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(DocumentNumberingError::RuleNotFound)?;
        let rule = DocumentNumberRule::from(row);
        finish_rule_mutation(
            tx,
            ctx,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!("/api/v1/code-generator/document-number-rules/{rule_code}/enabled"),
            &rule,
            "set_document_number_rule_enabled",
            now,
        )
        .await?;
        Ok(IdempotentMutation {
            value: rule,
            replayed: false,
        })
    }

    pub async fn generate_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        req: GenerateDocumentNumberRequest,
        now: DateTime<Utc>,
    ) -> Result<IdempotentMutation<DocumentNumberAllocation>, DocumentNumberingError> {
        let request_hash = document_number_request_hash(&req)?;
        lock_idempotency_key(tx, ctx.owner_id, &req.idempotency_key).await?;
        if let Some(allocation) =
            load_allocation_by_idempotency(tx, ctx.owner_id, &req.idempotency_key).await?
        {
            if allocation.1 != request_hash {
                return Err(DocumentNumberingError::IdempotencyConflict);
            }
            return Ok(IdempotentMutation {
                value: allocation.0,
                replayed: true,
            });
        }

        ensure_document_type_valid(tx, ctx.owner_id, &req.document_type, now).await?;
        let owner_code = load_owner_code(tx, ctx.owner_id).await?;
        let rule = load_effective_rule(tx, ctx.owner_id, &req.document_type, now).await?;
        let counter_key = counter_key(ctx.owner_id, &rule.document_type, &rule.reset_policy, now);
        let sequence_value =
            next_sequence_value(tx, rule.id, &counter_key, rule.sequence_width, now).await?;
        let generated_no = render_number(&rule, &owner_code, sequence_value, now)?;
        let allocation = insert_allocation(
            tx,
            ctx.owner_id,
            &req,
            &request_hash,
            &rule,
            &counter_key,
            sequence_value,
            &generated_no,
            now,
        )
        .await?;
        append_generation_audit(tx, ctx, &allocation, now).await?;

        Ok(IdempotentMutation {
            value: allocation,
            replayed: false,
        })
    }

    pub async fn list_allocations(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        query: DocumentNumberAllocationQuery,
    ) -> Result<Vec<DocumentNumberAllocation>, DocumentNumberingError> {
        let rows = sqlx::query_as::<_, AllocationRow>(
            r#"
            SELECT id, owner_id, rule_id, document_type, generated_no, sequence_value,
                   counter_key, source_module, source_document_id, created_at
              FROM document_number_allocations
             WHERE owner_id = $1
               AND ($2::TEXT IS NULL OR document_type = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR created_at <= $4)
             ORDER BY created_at DESC, id DESC
             LIMIT $5
            "#,
        )
        .bind(ctx.owner_id)
        .bind(query.document_type)
        .bind(query.from)
        .bind(query.to)
        .bind(i64::from(
            query.limit.clamp(1, MAX_DOCUMENT_NUMBER_ALLOCATION_LIMIT),
        ))
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

impl Default for PgDocumentNumberingService {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_rule_request(
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

fn validate_template(template: &str) -> Result<(), DocumentNumberingError> {
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
            "OWNER" | "DOCUMENT_TYPE" | "YYYY" | "MM" | "DD" | "SEQ"
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

async fn load_rule_id_for_update(
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

async fn ensure_document_type_valid(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    document_type: &str,
    effective_at: DateTime<Utc>,
) -> Result<(), DocumentNumberingError> {
    let params: Option<Value> = sqlx::query_scalar(
        r#"
        WITH scoped_items AS (
            SELECT
                params,
                enabled,
                ROW_NUMBER() OVER (
                    PARTITION BY item_code
                    ORDER BY CASE WHEN owner_id = $2 THEN 1 ELSE 0 END DESC, updated_at DESC
                ) AS scope_rank
              FROM system_dictionary_items
             WHERE dict_code = 'document_type'
               AND item_code = $1
               AND (owner_id IS NULL OR owner_id = $2)
               AND (effective_from IS NULL OR effective_from <= $3)
               AND (effective_to IS NULL OR effective_to > $3)
        )
        SELECT params
          FROM scoped_items
         WHERE scope_rank = 1 AND enabled = TRUE
        "#,
    )
    .bind(document_type)
    .bind(owner_id)
    .bind(effective_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let Some(params) = params else {
        return Err(DocumentNumberingError::DocumentTypeInvalid);
    };
    for key in ["direction", "workflow_template", "batch_policy"] {
        if !params.get(key).is_some_and(Value::is_string) {
            return Err(DocumentNumberingError::DocumentTypeInvalid);
        }
    }
    Ok(())
}

async fn load_owner_code(
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

async fn load_effective_rule(
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

async fn next_sequence_value(
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
async fn insert_allocation(
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

async fn load_allocation_by_idempotency(
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

async fn append_generation_audit(
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
async fn finish_rule_mutation<T: Serialize>(
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
    store_idempotency_success(
        &mut tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        "document_number_rule",
        resource_id.clone(),
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

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, DocumentNumberingError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(DocumentNumberingError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| DocumentNumberingError::Serialize(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), DocumentNumberingError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| DocumentNumberingError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

fn counter_key(
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

fn render_number(
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
        .replace("{MM}", &format!("{:02}", occurred_at.month()))
        .replace("{DD}", &format!("{:02}", occurred_at.day()))
        .replace("{SEQ}", &seq))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), DocumentNumberingError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(idempotency_lock_id(owner_id, idempotency_key))
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

fn document_number_request_hash(
    req: &GenerateDocumentNumberRequest,
) -> Result<String, DocumentNumberingError> {
    json_request_hash(
        &serde_json::to_value(req)
            .map_err(|error| DocumentNumberingError::Serialize(error.to_string()))?,
    )
}

fn json_request_hash(value: &Value) -> Result<String, DocumentNumberingError> {
    let text = serde_json::to_string(value)
        .map_err(|error| DocumentNumberingError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn idempotency_lock_id(owner_id: Uuid, idempotency_key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(owner_id.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

fn map_db_error(error: sqlx::Error) -> DocumentNumberingError {
    DocumentNumberingError::Database(error.to_string())
}

impl From<DocumentNumberRuleRow> for DocumentNumberRule {
    fn from(row: DocumentNumberRuleRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            document_type: row.document_type,
            rule_code: row.rule_code,
            rule_name: row.rule_name,
            template: row.template,
            reset_policy: row.reset_policy,
            sequence_width: row.sequence_width,
            sequence_mode: row.sequence_mode,
            enabled: row.enabled,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

impl From<AllocationRow> for DocumentNumberAllocation {
    fn from(row: AllocationRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            rule_id: row.rule_id,
            document_type: row.document_type,
            generated_no: row.generated_no,
            sequence_value: row.sequence_value,
            counter_key: row.counter_key,
            source_module: row.source_module,
            source_document_id: row.source_document_id,
            created_at: row.created_at,
        }
    }
}

impl From<AllocationWithHashRow> for DocumentNumberAllocation {
    fn from(row: AllocationWithHashRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            rule_id: row.rule_id,
            document_type: row.document_type,
            generated_no: row.generated_no,
            sequence_value: row.sequence_value,
            counter_key: row.counter_key,
            source_module: row.source_module,
            source_document_id: row.source_document_id,
            created_at: row.created_at,
        }
    }
}
