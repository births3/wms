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
