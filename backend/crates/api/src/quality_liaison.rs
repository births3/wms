use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{
    CreateQualityLiaisonRequest, QualityLiaisonApprovalCallbackRequest, QualityLiaisonOrder,
    QualityLiaisonTypeConfig, UpsertQualityLiaisonTypeRequest,
};

use crate::{
    auth::AuthContext,
    document_numbering::{GenerateDocumentNumberRequest, PgDocumentNumberingService},
};

mod actions;
mod persistence;

use actions::*;
use persistence::*;

const DOCUMENT_TYPE: &str = "quality_liaison";

#[derive(Clone, Debug)]
pub struct PgQualityLiaisonRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QualityLiaisonError {
    NotFound,
    TypeNotFound,
    InvalidRequest,
    ApprovalOpinionRequired,
    UnauthorizedApprover,
    AlreadyClosed,
    IdempotencyConflict,
    BusinessActionInvalid,
    BusinessAction(String),
    DocumentNumbering(String),
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentQualityLiaisonMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, FromRow)]
struct QualityLiaisonTypeRow {
    id: Uuid,
    owner_id: Uuid,
    type_code: String,
    type_name: String,
    approval_template_id: String,
    approver_user_id: Uuid,
    timeout_seconds: i32,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

#[derive(Clone, Debug, FromRow)]
struct QualityLiaisonOrderRow {
    id: Uuid,
    owner_id: Uuid,
    liaison_no: String,
    type_code: String,
    related_document_type: String,
    related_document_no: String,
    problem_description: String,
    disposition_suggestion: String,
    trigger_source: String,
    business_payload: serde_json::Value,
    status: String,
    approval_record_id: Option<Uuid>,
    approved_by: Option<Uuid>,
    approval_opinion: Option<String>,
    approved_at: Option<DateTime<Utc>>,
    created_by: Uuid,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

impl PgQualityLiaisonRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_type(
        &self,
        ctx: &AuthContext,
        request: UpsertQualityLiaisonTypeRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentQualityLiaisonMutation<QualityLiaisonTypeConfig>, QualityLiaisonError>
    {
        validate_type_request(&request)?;
        let request = normalize_type_request(request);
        let hash = request_hash(&serde_json::json!({"action":"upsert_type","request":request}))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentQualityLiaisonMutation {
                value,
                replayed: true,
            });
        }
        let approver_exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                  FROM auth_users user_account
                  JOIN auth_user_owner_bindings binding
                    ON binding.user_id = user_account.id
                   AND binding.owner_id = $1
                   AND binding.is_active = TRUE
                 WHERE user_account.id = $2 AND user_account.status = 'active'
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(request.approver_user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        if !approver_exists {
            return Err(QualityLiaisonError::InvalidRequest);
        }
        let row = sqlx::query_as::<_, QualityLiaisonTypeRow>(
            r#"
            INSERT INTO quality_liaison_types (
                id, owner_id, type_code, type_name, approval_template_id,
                approver_user_id, timeout_seconds, enabled, created_by, created_at, updated_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$10)
            ON CONFLICT (owner_id, type_code) DO UPDATE SET
                type_name = EXCLUDED.type_name,
                approval_template_id = EXCLUDED.approval_template_id,
                approver_user_id = EXCLUDED.approver_user_id,
                timeout_seconds = EXCLUDED.timeout_seconds,
                enabled = EXCLUDED.enabled,
                updated_at = EXCLUDED.updated_at,
                version = quality_liaison_types.version + 1
            RETURNING id, owner_id, type_code, type_name, approval_template_id,
                      approver_user_id, timeout_seconds, enabled, created_at, updated_at, version
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(&request.type_code)
        .bind(&request.type_name)
        .bind(&request.approval_template_id)
        .bind(request.approver_user_id)
        .bind(request.timeout_seconds)
        .bind(request.enabled)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let value: QualityLiaisonTypeConfig = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "PUT",
            "/api/v1/quality-liaisons/types/{type_code}",
            "quality_liaison_type",
            value.id,
            &value,
            "upsert_quality_liaison_type",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentQualityLiaisonMutation {
            value,
            replayed: false,
        })
    }

    pub async fn create(
        &self,
        ctx: &AuthContext,
        request: CreateQualityLiaisonRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentQualityLiaisonMutation<QualityLiaisonOrder>, QualityLiaisonError> {
        validate_create_request(&request)?;
        let request = normalize_create_request(request);
        let hash = request_hash(&serde_json::json!({"action":"create","request":request}))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentQualityLiaisonMutation {
                value,
                replayed: true,
            });
        }
        let type_config = sqlx::query_as::<_, QualityLiaisonTypeRow>(
            r#"
            SELECT id, owner_id, type_code, type_name, approval_template_id,
                   approver_user_id, timeout_seconds, enabled, created_at, updated_at, version
              FROM quality_liaison_types
             WHERE owner_id = $1 AND type_code = $2 AND enabled = TRUE
             FOR SHARE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(&request.type_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_database_error)?
        .ok_or(QualityLiaisonError::TypeNotFound)?;
        let order_id = Uuid::new_v4();
        let number = PgDocumentNumberingService::new()
            .generate_in_tx(
                &mut tx,
                ctx,
                GenerateDocumentNumberRequest {
                    document_type: DOCUMENT_TYPE.to_string(),
                    idempotency_key: format!("{idempotency_key}:number"),
                    source_module: "M-QL".to_string(),
                    source_document_id: Some(order_id),
                },
                now,
            )
            .await
            .map_err(|error| QualityLiaisonError::DocumentNumbering(format!("{error:?}")))?;
        let approval_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO h4_approval_records (
                id, owner_id, scenario, business_ref, dedupe_key, approver_user,
                process_id, callback_path, summary, status, created_at, updated_at
            ) VALUES ($1,$2,'quality_liaison',$3,$4,$5,$6,$7,$8,'pending',$9,$9)
            "#,
        )
        .bind(approval_id)
        .bind(ctx.owner_id)
        .bind(order_id.to_string())
        .bind(format!("quality-liaison:{order_id}"))
        .bind(type_config.approver_user_id.to_string())
        .bind(&type_config.approval_template_id)
        .bind(format!(
            "/api/v1/quality-liaisons/{order_id}/approval-callback"
        ))
        .bind(format!(
            "{}：{} / {}",
            type_config.type_name, request.related_document_no, request.problem_description
        ))
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let row = sqlx::query_as::<_, QualityLiaisonOrderRow>(&format!(
            r#"
                INSERT INTO quality_liaison_orders (
                    id, owner_id, liaison_no, type_code, related_document_type,
                    related_document_no, problem_description, disposition_suggestion,
                    trigger_source, business_payload, status, approval_record_id,
                    created_by, created_at, updated_at
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'pending_approval',$11,$12,$13,$13)
                RETURNING {}
                "#,
            order_columns()
        ))
        .bind(order_id)
        .bind(ctx.owner_id)
        .bind(number.value.generated_no)
        .bind(&request.type_code)
        .bind(&request.related_document_type)
        .bind(&request.related_document_no)
        .bind(&request.problem_description)
        .bind(&request.disposition_suggestion)
        .bind(&request.trigger_source)
        .bind(request.business_payload)
        .bind(approval_id)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let value: QualityLiaisonOrder = row.into();
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/quality-liaisons",
            "quality_liaison_order",
            value.id,
            &value,
            "create_quality_liaison",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentQualityLiaisonMutation {
            value,
            replayed: false,
        })
    }

    pub async fn apply_approval_callback(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        request: QualityLiaisonApprovalCallbackRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentQualityLiaisonMutation<QualityLiaisonOrder>, QualityLiaisonError> {
        let request = normalize_approval_request(request)?;
        let status = match request.conclusion.as_str() {
            "approved" | "同意" => "approved",
            "rejected" | "拒绝" => "rejected",
            _ => return Err(QualityLiaisonError::InvalidRequest),
        };
        let hash = request_hash(&serde_json::json!({
            "action":"approval_callback",
            "order_id":order_id,
            "request":request,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentQualityLiaisonMutation {
                value,
                replayed: true,
            });
        }
        let current = load_order_for_update(&mut tx, ctx.owner_id, order_id).await?;
        if current.status != "pending_approval" {
            return Err(QualityLiaisonError::AlreadyClosed);
        }
        let approval_id = current
            .approval_record_id
            .ok_or(QualityLiaisonError::InvalidRequest)?;
        let approver_user: String = sqlx::query_scalar(
            "SELECT approver_user FROM h4_approval_records WHERE owner_id = $1 AND id = $2 AND status = 'pending' FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(approval_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_database_error)?
        .ok_or(QualityLiaisonError::AlreadyClosed)?;
        if approver_user != ctx.user_id.to_string() {
            return Err(QualityLiaisonError::UnauthorizedApprover);
        }
        sqlx::query(
            r#"
            UPDATE h4_approval_records
               SET status = $3, opinion = $4, external_approval_id = $5,
                   approved_by = $6, approved_at = $7, updated_at = $7
             WHERE owner_id = $1 AND id = $2 AND status = 'pending'
            "#,
        )
        .bind(ctx.owner_id)
        .bind(approval_id)
        .bind(status)
        .bind(&request.opinion)
        .bind(&request.external_approval_id)
        .bind(ctx.user_id.to_string())
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let row = sqlx::query_as::<_, QualityLiaisonOrderRow>(&format!(
            r#"
            UPDATE quality_liaison_orders
               SET status = $3, approved_by = $4, approval_opinion = $5,
                   approved_at = $6, updated_at = $6, version = version + 1
             WHERE owner_id = $1 AND id = $2 AND status = 'pending_approval'
             RETURNING {}
            "#,
            order_columns()
        ))
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(status)
        .bind(ctx.user_id)
        .bind(&request.opinion)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_database_error)?
        .ok_or(QualityLiaisonError::AlreadyClosed)?;
        let value: QualityLiaisonOrder = row.into();
        if status == "approved" {
            apply_approved_action_in_tx(&mut tx, ctx, &current, now).await?;
        }
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/quality-liaisons/{id}/approval-callback",
            "quality_liaison_order",
            value.id,
            &value,
            "apply_quality_liaison_approval",
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentQualityLiaisonMutation {
            value,
            replayed: false,
        })
    }

    pub async fn get(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
    ) -> Result<QualityLiaisonOrder, QualityLiaisonError> {
        let row = sqlx::query_as::<_, QualityLiaisonOrderRow>(&format!(
            "SELECT {} FROM quality_liaison_orders WHERE owner_id = $1 AND id = $2",
            order_columns()
        ))
        .bind(ctx.owner_id)
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_database_error)?
        .ok_or(QualityLiaisonError::NotFound)?;
        Ok(row.into())
    }
}

impl From<QualityLiaisonTypeRow> for QualityLiaisonTypeConfig {
    fn from(row: QualityLiaisonTypeRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            type_code: row.type_code,
            type_name: row.type_name,
            approval_template_id: row.approval_template_id,
            approver_user_id: row.approver_user_id,
            timeout_seconds: row.timeout_seconds,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

impl From<QualityLiaisonOrderRow> for QualityLiaisonOrder {
    fn from(row: QualityLiaisonOrderRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            liaison_no: row.liaison_no,
            type_code: row.type_code,
            related_document_type: row.related_document_type,
            related_document_no: row.related_document_no,
            problem_description: row.problem_description,
            disposition_suggestion: row.disposition_suggestion,
            trigger_source: row.trigger_source,
            business_payload: row.business_payload,
            status: row.status,
            approval_record_id: row.approval_record_id,
            approved_by: row.approved_by,
            approval_opinion: row.approval_opinion,
            approved_at: row.approved_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        }
    }
}

fn normalize_type_request(
    request: UpsertQualityLiaisonTypeRequest,
) -> UpsertQualityLiaisonTypeRequest {
    UpsertQualityLiaisonTypeRequest {
        type_code: request.type_code.trim().to_ascii_lowercase(),
        type_name: request.type_name.trim().to_string(),
        approval_template_id: request.approval_template_id.trim().to_string(),
        ..request
    }
}

fn validate_type_request(
    request: &UpsertQualityLiaisonTypeRequest,
) -> Result<(), QualityLiaisonError> {
    let code = request.type_code.trim();
    let mut code_bytes = code.bytes();
    let valid_code = code_bytes
        .next()
        .is_some_and(|value| value.is_ascii_lowercase())
        && code_bytes
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'_');
    if code.len() < 2
        || code.len() > 64
        || !valid_code
        || request.type_name.trim().is_empty()
        || request.approval_template_id.trim().is_empty()
        || request.timeout_seconds <= 0
    {
        return Err(QualityLiaisonError::InvalidRequest);
    }
    Ok(())
}

fn normalize_create_request(request: CreateQualityLiaisonRequest) -> CreateQualityLiaisonRequest {
    CreateQualityLiaisonRequest {
        type_code: request.type_code.trim().to_ascii_lowercase(),
        related_document_type: request.related_document_type.trim().to_string(),
        related_document_no: request.related_document_no.trim().to_string(),
        problem_description: request.problem_description.trim().to_string(),
        disposition_suggestion: request.disposition_suggestion.trim().to_string(),
        trigger_source: request.trigger_source.trim().to_string(),
        business_payload: request.business_payload,
    }
}

fn validate_create_request(
    request: &CreateQualityLiaisonRequest,
) -> Result<(), QualityLiaisonError> {
    if [
        request.type_code.as_str(),
        request.related_document_type.as_str(),
        request.related_document_no.as_str(),
        request.problem_description.as_str(),
        request.disposition_suggestion.as_str(),
        request.trigger_source.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
        || !request.business_payload.is_object()
    {
        return Err(QualityLiaisonError::InvalidRequest);
    }
    Ok(())
}

fn normalize_approval_request(
    request: QualityLiaisonApprovalCallbackRequest,
) -> Result<QualityLiaisonApprovalCallbackRequest, QualityLiaisonError> {
    let opinion = request.opinion.trim();
    if opinion.is_empty() {
        return Err(QualityLiaisonError::ApprovalOpinionRequired);
    }
    let external_approval_id = request.external_approval_id.trim();
    if external_approval_id.is_empty() {
        return Err(QualityLiaisonError::InvalidRequest);
    }
    Ok(QualityLiaisonApprovalCallbackRequest {
        conclusion: request.conclusion.trim().to_ascii_lowercase(),
        opinion: opinion.to_string(),
        external_approval_id: external_approval_id.to_string(),
    })
}
