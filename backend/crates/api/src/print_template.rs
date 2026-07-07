//! H9 print template repository.

use std::collections::{BTreeSet, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::PageMeta;

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    auth::AuthContext,
};

#[derive(Clone, Debug)]
pub struct PgPrintTemplateRepository;

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrintTemplateError {
    InvalidRequest(String),
    IdempotencyConflict,
    FieldLibraryNotPublished,
    TemplateDisabled,
    TemplateFieldMismatch(Vec<String>),
    TemplateFieldMissing(Vec<String>),
    TemplateJsonInvalid,
    TemplateNotFound,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrintTemplateScope {
    Global,
    Owner,
}

impl PrintTemplateScope {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Owner => "owner",
        }
    }

    fn from_db(value: String) -> Result<Self, PrintTemplateError> {
        match value.as_str() {
            "global" => Ok(Self::Global),
            "owner" => Ok(Self::Owner),
            _ => Err(PrintTemplateError::Database(format!(
                "unknown print template scope: {value}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PrintFieldDefinitionInput {
    pub field_path: String,
    pub field_type: String,
    pub source_schema: String,
    pub display_name: String,
    pub group_code: String,
    pub group_name: String,
    pub metadata: Value,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PublishPrintFieldLibraryRequest {
    pub library_code: String,
    pub library_name: String,
    pub source_schema: String,
    pub fields: Vec<PrintFieldDefinitionInput>,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize, ToSchema)]
pub struct PrintFieldLibraryVersion {
    pub id: Uuid,
    pub library_id: Uuid,
    pub library_code: String,
    pub library_name: String,
    pub source_schema: String,
    pub version_no: i32,
    pub published_at: DateTime<Utc>,
    pub published_by: Uuid,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize, ToSchema)]
pub struct PrintFieldDefinition {
    pub id: Uuid,
    pub library_version_id: Uuid,
    pub field_path: String,
    pub field_type: String,
    pub source_schema: String,
    pub display_name: String,
    pub group_code: String,
    pub group_name: String,
    pub metadata: Value,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize, ToSchema)]
pub struct PrintFieldLibrarySummary {
    pub id: Uuid,
    pub library_code: String,
    pub library_name: String,
    pub source_schema: String,
    pub latest_version_id: Uuid,
    pub version_no: i32,
    pub field_count: i64,
    pub created_at: DateTime<Utc>,
    pub published_at: DateTime<Utc>,
    pub published_by: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PrintFieldLibraryListResponse {
    pub data: Vec<PrintFieldLibrarySummary>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PrintFieldDefinitionListResponse {
    pub data: Vec<PrintFieldDefinition>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct PrintTemplateBinding {
    pub field_path: String,
    pub required: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct SavePrintTemplateRequest {
    pub template_code: String,
    pub template_name: String,
    pub template_type_code: String,
    pub scope: PrintTemplateScope,
    pub enabled: bool,
    pub is_default: bool,
    pub remark: Option<String>,
    pub field_library_version_id: Uuid,
    pub hiprint_json: Value,
    pub field_bindings: Vec<PrintTemplateBinding>,
    pub paper: Value,
    pub designer_version: String,
    pub publish: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct PrintTemplateVersion {
    pub id: Uuid,
    pub template_id: Uuid,
    pub template_code: String,
    pub template_name: String,
    pub template_type_code: String,
    pub owner_id: Uuid,
    pub scope: PrintTemplateScope,
    pub enabled: bool,
    pub is_default: bool,
    pub remark: Option<String>,
    pub field_library_version_id: Uuid,
    pub version_no: i32,
    pub status: String,
    pub hiprint_json: Value,
    pub field_bindings: Vec<PrintTemplateBinding>,
    pub paper: Value,
    pub designer_version: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub published_at: Option<DateTime<Utc>>,
    pub published_by: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct PrintTemplateSummary {
    pub id: Uuid,
    pub template_code: String,
    pub template_name: String,
    pub template_type_code: String,
    pub owner_id: Uuid,
    pub scope: PrintTemplateScope,
    pub enabled: bool,
    pub is_default: bool,
    pub remark: Option<String>,
    pub latest_version_id: Uuid,
    pub latest_version_no: i32,
    pub latest_version_status: String,
    pub field_library_version_id: Uuid,
    pub designer_version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PrintTemplateListResponse {
    pub data: Vec<PrintTemplateSummary>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct PrintTemplatePreviewRequest {
    pub template_code: Option<String>,
    pub template_type_code: String,
    pub business_document_id: String,
    pub data: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct PrintTemplatePreviewResponse {
    pub template_id: Uuid,
    pub template_version_id: Uuid,
    pub template_code: String,
    pub template_name: String,
    pub template_type_code: String,
    pub version_no: i32,
    pub hiprint_json: Value,
    pub field_bindings: Vec<PrintTemplateBinding>,
    pub paper: Value,
    pub data: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct PrintTemplatePrintRequest {
    pub template_code: Option<String>,
    pub template_type_code: String,
    pub business_module: String,
    pub business_document_type: String,
    pub business_document_id: String,
    pub data: Value,
    pub status: String,
    pub failure_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize, ToSchema)]
pub struct PrintRecord {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub template_version_id: Uuid,
    pub business_module: String,
    pub business_document_type: String,
    pub business_document_id: String,
    pub status: String,
    pub failure_reason: Option<String>,
    pub retry_count: i32,
    pub printed_at: DateTime<Utc>,
    pub operator_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct ResolvePrintTemplateRequest {
    pub template_code: Option<String>,
    pub template_type_code: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct ResolvePrintTemplateResponse {
    pub template: PrintTemplateSummary,
    pub version: PrintTemplateVersion,
}

impl PgPrintTemplateRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn publish_field_library(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: PublishPrintFieldLibraryRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintFieldLibraryVersion>, PrintTemplateError> {
        validate_publish_request(&req)?;
        let request_hash = json_request_hash(&serde_json::json!({
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

        let library_id = upsert_library_for_update(&mut tx, &req, now).await?;
        let version_no = next_version_no(&mut tx, library_id).await?;
        let version_id = Uuid::new_v4();
        let version = sqlx::query_as::<_, PrintFieldLibraryVersion>(
            r#"
            INSERT INTO print_field_library_versions (
                id, library_id, version_no, status, published_at, published_by, request_hash, created_at
            )
            VALUES ($1, $2, $3, 'published', $4, $5, $6, $4)
            RETURNING
                id,
                library_id,
                $7::TEXT AS library_code,
                $8::TEXT AS library_name,
                $9::TEXT AS source_schema,
                version_no,
                published_at,
                published_by
            "#,
        )
        .bind(version_id)
        .bind(library_id)
        .bind(version_no)
        .bind(now)
        .bind(ctx.user_id)
        .bind(&request_hash)
        .bind(&req.library_code)
        .bind(&req.library_name)
        .bind(&req.source_schema)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        for field in &req.fields {
            sqlx::query(
                r#"
                INSERT INTO print_field_definitions (
                    id, library_version_id, field_path, field_type, source_schema,
                    display_name, group_code, group_name, metadata, sort_order, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(version.id)
            .bind(&field.field_path)
            .bind(&field.field_type)
            .bind(&field.source_schema)
            .bind(&field.display_name)
            .bind(&field.group_code)
            .bind(&field.group_name)
            .bind(&field.metadata)
            .bind(field.sort_order)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/internal/h9/field-libraries/publish",
            "print_field_library",
            &version,
            now,
        )
        .await?;
        append_publish_audit(&mut tx, ctx, &version, now).await?;
        tx.commit().await.map_err(map_db_error)?;

        Ok(IdempotentMutation {
            value: version,
            replayed: false,
        })
    }

    pub async fn list_field_version_fields(
        &self,
        pool: &PgPool,
        library_version_id: Uuid,
    ) -> Result<Vec<PrintFieldDefinition>, PrintTemplateError> {
        sqlx::query_as::<_, PrintFieldDefinition>(
            r#"
            SELECT id, library_version_id, field_path, field_type, source_schema,
                   display_name, group_code, group_name, metadata, sort_order
              FROM print_field_definitions
             WHERE library_version_id = $1
             ORDER BY sort_order ASC, field_path ASC
            "#,
        )
        .bind(library_version_id)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)
    }

    pub async fn list_field_libraries(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<PrintFieldLibrarySummary>, PrintTemplateError> {
        sqlx::query_as::<_, PrintFieldLibrarySummary>(
            r#"
            SELECT
                libraries.id,
                libraries.library_code,
                libraries.library_name,
                libraries.source_schema,
                latest_versions.id AS latest_version_id,
                latest_versions.version_no,
                COUNT(fields.id)::BIGINT AS field_count,
                libraries.created_at,
                latest_versions.published_at,
                latest_versions.published_by
              FROM print_field_libraries libraries
              JOIN LATERAL (
                SELECT id, version_no, published_at, published_by
                  FROM print_field_library_versions
                 WHERE library_id = libraries.id
                 ORDER BY version_no DESC
                 LIMIT 1
              ) latest_versions ON TRUE
              LEFT JOIN print_field_definitions fields
                ON fields.library_version_id = latest_versions.id
             GROUP BY
                libraries.id,
                libraries.library_code,
                libraries.library_name,
                libraries.source_schema,
                latest_versions.id,
                latest_versions.version_no,
                libraries.created_at,
                latest_versions.published_at,
                latest_versions.published_by
             ORDER BY libraries.library_code ASC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(map_db_error)
    }

    pub async fn list_templates(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
    ) -> Result<Vec<PrintTemplateSummary>, PrintTemplateError> {
        let rows = sqlx::query_as::<_, PrintTemplateSummaryRow>(
            r#"
            SELECT
                templates.id,
                templates.template_code,
                templates.template_name,
                templates.template_type_code,
                templates.owner_id,
                templates.scope,
                templates.enabled,
                templates.is_default,
                templates.remark,
                latest_versions.id AS latest_version_id,
                latest_versions.version_no AS latest_version_no,
                latest_versions.status AS latest_version_status,
                latest_versions.field_library_version_id,
                latest_versions.designer_version,
                templates.created_at,
                templates.updated_at,
                latest_versions.published_at
              FROM print_templates templates
              JOIN LATERAL (
                SELECT id, version_no, status, field_library_version_id, designer_version, published_at
                  FROM print_template_versions
                 WHERE template_id = templates.id
                 ORDER BY version_no DESC
                 LIMIT 1
              ) latest_versions ON TRUE
             WHERE templates.owner_id = $1
             ORDER BY templates.template_type_code ASC, templates.template_code ASC
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter().map(PrintTemplateSummary::try_from).collect()
    }

    pub async fn save_template(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: SavePrintTemplateRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintTemplateVersion>, PrintTemplateError> {
        validate_template_request(&req)?;
        validate_field_library_and_bindings(pool, &req).await?;
        let request_hash = json_request_hash(&serde_json::json!({
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

        let template_id = upsert_template_for_update(&mut tx, ctx, &req, now).await?;
        let version_no = next_template_version_no(&mut tx, template_id).await?;
        let version_id = Uuid::new_v4();
        let status = if req.publish { "published" } else { "draft" };
        let row = sqlx::query_as::<_, PrintTemplateVersionRow>(
            r#"
            INSERT INTO print_template_versions (
                id, template_id, field_library_version_id, version_no, status,
                hiprint_json, field_bindings, paper, designer_version, request_hash,
                created_at, created_by, published_at, published_by
            )
            VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12, CASE WHEN $5 = 'published' THEN $11 ELSE NULL END,
                CASE WHEN $5 = 'published' THEN $12 ELSE NULL END
            )
            RETURNING
                id,
                template_id,
                $13::TEXT AS template_code,
                $14::TEXT AS template_name,
                $15::TEXT AS template_type_code,
                $16::UUID AS owner_id,
                $17::TEXT AS scope,
                $18::BOOLEAN AS enabled,
                $19::BOOLEAN AS is_default,
                $20::TEXT AS remark,
                field_library_version_id,
                version_no,
                status,
                hiprint_json,
                field_bindings,
                paper,
                designer_version,
                created_at,
                created_by,
                published_at,
                published_by
            "#,
        )
        .bind(version_id)
        .bind(template_id)
        .bind(req.field_library_version_id)
        .bind(version_no)
        .bind(status)
        .bind(&req.hiprint_json)
        .bind(
            serde_json::to_value(&req.field_bindings)
                .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
        )
        .bind(&req.paper)
        .bind(&req.designer_version)
        .bind(&request_hash)
        .bind(now)
        .bind(ctx.user_id)
        .bind(&req.template_code)
        .bind(&req.template_name)
        .bind(&req.template_type_code)
        .bind(ctx.owner_id)
        .bind(req.scope.as_str())
        .bind(req.enabled)
        .bind(req.is_default)
        .bind(req.remark.as_deref())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let version = PrintTemplateVersion::try_from(row)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/print-templates/templates",
            "print_template",
            &version,
            now,
        )
        .await?;
        append_h9_audit(&mut tx, ctx, "save_print_template", "print_template", version.id, now)
            .await?;
        tx.commit().await.map_err(map_db_error)?;

        Ok(IdempotentMutation {
            value: version,
            replayed: false,
        })
    }

    pub async fn resolve_template(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: ResolvePrintTemplateRequest,
    ) -> Result<ResolvePrintTemplateResponse, PrintTemplateError> {
        let version = resolve_template_version(pool, ctx, &req.template_code, &req.template_type_code)
            .await?;
        let summary = PrintTemplateSummary {
            id: version.template_id,
            template_code: version.template_code.clone(),
            template_name: version.template_name.clone(),
            template_type_code: version.template_type_code.clone(),
            owner_id: version.owner_id,
            scope: version.scope.clone(),
            enabled: version.enabled,
            is_default: version.is_default,
            remark: version.remark.clone(),
            latest_version_id: version.id,
            latest_version_no: version.version_no,
            latest_version_status: version.status.clone(),
            field_library_version_id: version.field_library_version_id,
            designer_version: version.designer_version.clone(),
            created_at: version.created_at,
            updated_at: version.created_at,
            published_at: version.published_at,
        };
        Ok(ResolvePrintTemplateResponse {
            template: summary,
            version,
        })
    }

    pub async fn preview_template(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: PrintTemplatePreviewRequest,
    ) -> Result<PrintTemplatePreviewResponse, PrintTemplateError> {
        let version =
            resolve_template_version(pool, ctx, &req.template_code, &req.template_type_code).await?;
        validate_required_fields(&version.field_bindings, &req.data)?;
        Ok(PrintTemplatePreviewResponse {
            template_id: version.template_id,
            template_version_id: version.id,
            template_code: version.template_code,
            template_name: version.template_name,
            template_type_code: version.template_type_code,
            version_no: version.version_no,
            hiprint_json: version.hiprint_json,
            field_bindings: version.field_bindings,
            paper: version.paper,
            data: req.data,
        })
    }

    pub async fn record_print(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: PrintTemplatePrintRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintRecord>, PrintTemplateError> {
        validate_print_request(&req)?;
        let version =
            resolve_template_version(pool, ctx, &req.template_code, &req.template_type_code).await?;
        validate_required_fields(&version.field_bindings, &req.data)?;
        let request_hash = json_request_hash(&serde_json::json!({
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

        let record = sqlx::query_as::<_, PrintRecord>(
            r#"
            INSERT INTO print_records (
                id, owner_id, template_version_id, business_module, business_document_type,
                business_document_id, status, failure_reason, retry_count, printed_at,
                operator_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9, $10, $9)
            RETURNING id, owner_id, template_version_id, business_module, business_document_type,
                      business_document_id, status, failure_reason, retry_count, printed_at,
                      operator_id, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(version.id)
        .bind(&req.business_module)
        .bind(&req.business_document_type)
        .bind(&req.business_document_id)
        .bind(&req.status)
        .bind(&req.failure_reason)
        .bind(now)
        .bind(ctx.user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/print-templates/print",
            "print_record",
            &record,
            now,
        )
        .await?;
        append_h9_audit(&mut tx, ctx, "print_template", "print_record", record.id, now).await?;
        tx.commit().await.map_err(map_db_error)?;

        Ok(IdempotentMutation {
            value: record,
            replayed: false,
        })
    }
}

fn validate_publish_request(
    req: &PublishPrintFieldLibraryRequest,
) -> Result<(), PrintTemplateError> {
    if req.library_code.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "library_code is required".to_string(),
        ));
    }
    if req.library_name.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "library_name is required".to_string(),
        ));
    }
    if req.source_schema.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "source_schema is required".to_string(),
        ));
    }
    if req.fields.is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "fields are required".to_string(),
        ));
    }
    let mut paths = BTreeSet::new();
    for field in &req.fields {
        if field.field_path.trim().is_empty() {
            return Err(PrintTemplateError::InvalidRequest(
                "field_path is required".to_string(),
            ));
        }
        if !paths.insert(field.field_path.as_str()) {
            return Err(PrintTemplateError::InvalidRequest(format!(
                "duplicate field_path: {}",
                field.field_path
            )));
        }
    }
    Ok(())
}

fn validate_template_request(req: &SavePrintTemplateRequest) -> Result<(), PrintTemplateError> {
    if req.template_code.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "template_code is required".to_string(),
        ));
    }
    if req.template_name.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "template_name is required".to_string(),
        ));
    }
    if req.template_type_code.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "template_type_code is required".to_string(),
        ));
    }
    if req.designer_version.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "designer_version is required".to_string(),
        ));
    }
    if !req.hiprint_json.is_object()
        || !req
            .hiprint_json
            .get("panels")
            .is_some_and(serde_json::Value::is_array)
    {
        return Err(PrintTemplateError::TemplateJsonInvalid);
    }
    let mut paths = BTreeSet::new();
    for binding in &req.field_bindings {
        if binding.field_path.trim().is_empty() {
            return Err(PrintTemplateError::InvalidRequest(
                "field binding path is required".to_string(),
            ));
        }
        if !paths.insert(binding.field_path.as_str()) {
            return Err(PrintTemplateError::InvalidRequest(format!(
                "duplicate field binding: {}",
                binding.field_path
            )));
        }
    }
    Ok(())
}

fn validate_print_request(req: &PrintTemplatePrintRequest) -> Result<(), PrintTemplateError> {
    if req.template_type_code.trim().is_empty()
        || req.business_module.trim().is_empty()
        || req.business_document_type.trim().is_empty()
        || req.business_document_id.trim().is_empty()
    {
        return Err(PrintTemplateError::InvalidRequest(
            "print template business fields are required".to_string(),
        ));
    }
    if !matches!(req.status.as_str(), "printed" | "cancelled" | "failed") {
        return Err(PrintTemplateError::InvalidRequest(
            "print status must be printed, cancelled or failed".to_string(),
        ));
    }
    Ok(())
}

async fn validate_field_library_and_bindings(
    pool: &PgPool,
    req: &SavePrintTemplateRequest,
) -> Result<(), PrintTemplateError> {
    let status: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT status
          FROM print_field_library_versions
         WHERE id = $1
        "#,
    )
    .bind(req.field_library_version_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    if status.as_ref().map(|row| row.0.as_str()) != Some("published") {
        return Err(PrintTemplateError::FieldLibraryNotPublished);
    }

    let field_paths: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT field_path
          FROM print_field_definitions
         WHERE library_version_id = $1
        "#,
    )
    .bind(req.field_library_version_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    let available: HashSet<&str> = field_paths.iter().map(|row| row.0.as_str()).collect();
    let missing: Vec<String> = req
        .field_bindings
        .iter()
        .filter(|binding| !available.contains(binding.field_path.as_str()))
        .map(|binding| binding.field_path.clone())
        .collect();
    if !missing.is_empty() {
        return Err(PrintTemplateError::TemplateFieldMismatch(missing));
    }
    Ok(())
}

fn validate_required_fields(
    bindings: &[PrintTemplateBinding],
    data: &Value,
) -> Result<(), PrintTemplateError> {
    let missing: Vec<String> = bindings
        .iter()
        .filter(|binding| binding.required && value_at_path(data, &binding.field_path).is_none())
        .map(|binding| binding.field_path.clone())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(PrintTemplateError::TemplateFieldMissing(missing))
    }
}

fn value_at_path<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = data;
    for part in path.split('.') {
        if part.is_empty() {
            return None;
        }
        current = current.get(part)?;
    }
    if current.is_null() {
        None
    } else {
        Some(current)
    }
}

async fn upsert_library_for_update(
    tx: &mut Transaction<'_, Postgres>,
    req: &PublishPrintFieldLibraryRequest,
    now: DateTime<Utc>,
) -> Result<Uuid, PrintTemplateError> {
    let existing: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id
          FROM print_field_libraries
         WHERE library_code = $1
         FOR UPDATE
        "#,
    )
    .bind(&req.library_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if let Some((id,)) = existing {
        sqlx::query(
            r#"
            UPDATE print_field_libraries
               SET library_name = $1,
                   source_schema = $2,
                   updated_at = $3,
                   version = version + 1
             WHERE id = $4
            "#,
        )
        .bind(&req.library_name)
        .bind(&req.source_schema)
        .bind(now)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        return Ok(id);
    }

    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO print_field_libraries (
            id, library_code, library_name, source_schema, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $5)
        "#,
    )
    .bind(id)
    .bind(&req.library_code)
    .bind(&req.library_name)
    .bind(&req.source_schema)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(id)
}

async fn next_version_no(
    tx: &mut Transaction<'_, Postgres>,
    library_id: Uuid,
) -> Result<i32, PrintTemplateError> {
    let max_version: Option<i32> = sqlx::query_scalar(
        r#"
        SELECT MAX(version_no)
          FROM print_field_library_versions
         WHERE library_id = $1
        "#,
    )
    .bind(library_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(max_version.unwrap_or(0) + 1)
}

async fn upsert_template_for_update(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    req: &SavePrintTemplateRequest,
    now: DateTime<Utc>,
) -> Result<Uuid, PrintTemplateError> {
    let existing: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id
          FROM print_templates
         WHERE owner_id = $1 AND template_code = $2
         FOR UPDATE
        "#,
    )
    .bind(ctx.owner_id)
    .bind(&req.template_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if let Some((id,)) = existing {
        sqlx::query(
            r#"
            UPDATE print_templates
               SET template_name = $1,
                   template_type_code = $2,
                   scope = $3,
                   enabled = $4,
                   is_default = $5,
                   remark = $6,
                   updated_at = $7,
                   updated_by = $8,
                   version = version + 1
             WHERE id = $9
            "#,
        )
        .bind(&req.template_name)
        .bind(&req.template_type_code)
        .bind(req.scope.as_str())
        .bind(req.enabled)
        .bind(req.is_default)
        .bind(&req.remark)
        .bind(now)
        .bind(ctx.user_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        return Ok(id);
    }

    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code,
            scope, enabled, is_default, remark, created_at, updated_at,
            created_by, updated_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10, $11, $11)
        "#,
    )
    .bind(id)
    .bind(ctx.owner_id)
    .bind(&req.template_code)
    .bind(&req.template_name)
    .bind(&req.template_type_code)
    .bind(req.scope.as_str())
    .bind(req.enabled)
    .bind(req.is_default)
    .bind(&req.remark)
    .bind(now)
    .bind(ctx.user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(id)
}

async fn next_template_version_no(
    tx: &mut Transaction<'_, Postgres>,
    template_id: Uuid,
) -> Result<i32, PrintTemplateError> {
    let max_version: Option<i32> = sqlx::query_scalar(
        r#"
        SELECT MAX(version_no)
          FROM print_template_versions
         WHERE template_id = $1
        "#,
    )
    .bind(template_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(max_version.unwrap_or(0) + 1)
}

async fn resolve_template_version(
    pool: &PgPool,
    ctx: &AuthContext,
    template_code: &Option<String>,
    template_type_code: &str,
) -> Result<PrintTemplateVersion, PrintTemplateError> {
    if let Some(code) = template_code.as_ref().filter(|value| !value.trim().is_empty()) {
        let row = sqlx::query_as::<_, PrintTemplateVersionRow>(
            r#"
            SELECT
                versions.id,
                templates.id AS template_id,
                templates.template_code,
                templates.template_name,
                templates.template_type_code,
                templates.owner_id,
                templates.scope,
                templates.enabled,
                templates.is_default,
                templates.remark,
                versions.field_library_version_id,
                versions.version_no,
                versions.status,
                versions.hiprint_json,
                versions.field_bindings,
                versions.paper,
                versions.designer_version,
                versions.created_at,
                versions.created_by,
                versions.published_at,
                versions.published_by
              FROM print_templates templates
              JOIN LATERAL (
                SELECT *
                  FROM print_template_versions
                 WHERE template_id = templates.id AND status = 'published'
                 ORDER BY version_no DESC
                 LIMIT 1
              ) versions ON TRUE
             WHERE templates.owner_id = $1
               AND templates.template_code = $2
               AND templates.template_type_code = $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(code)
        .bind(template_type_code)
        .fetch_optional(pool)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintTemplateError::TemplateNotFound)?;
        let version = PrintTemplateVersion::try_from(row)?;
        if !version.enabled {
            return Err(PrintTemplateError::TemplateDisabled);
        }
        return Ok(version);
    }

    let row = sqlx::query_as::<_, PrintTemplateVersionRow>(
        r#"
        SELECT
            versions.id,
            templates.id AS template_id,
            templates.template_code,
            templates.template_name,
            templates.template_type_code,
            templates.owner_id,
            templates.scope,
            templates.enabled,
            templates.is_default,
            templates.remark,
            versions.field_library_version_id,
            versions.version_no,
            versions.status,
            versions.hiprint_json,
            versions.field_bindings,
            versions.paper,
            versions.designer_version,
            versions.created_at,
            versions.created_by,
            versions.published_at,
            versions.published_by
          FROM print_templates templates
          JOIN LATERAL (
            SELECT *
              FROM print_template_versions
             WHERE template_id = templates.id AND status = 'published'
             ORDER BY version_no DESC
             LIMIT 1
          ) versions ON TRUE
         WHERE templates.owner_id = $1
           AND templates.template_type_code = $2
           AND templates.enabled = TRUE
         ORDER BY
           CASE templates.scope WHEN 'owner' THEN 0 ELSE 1 END,
           templates.is_default DESC,
           templates.updated_at DESC
         LIMIT 1
        "#,
    )
    .bind(ctx.owner_id)
    .bind(template_type_code)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintTemplateError::TemplateNotFound)?;
    PrintTemplateVersion::try_from(row)
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), PrintTemplateError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind(owner_id.to_string())
        .bind(idempotency_key)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, PrintTemplateError> {
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
        return Err(PrintTemplateError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| PrintTemplateError::Serialize(error.to_string()))
}

async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), PrintTemplateError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?;
    let resource_id = response_body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(resource_type)
        .to_string();
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            200, $7, $8, $9, $10, $11
        )
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

async fn append_publish_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    version: &PrintFieldLibraryVersion,
    now: DateTime<Utc>,
) -> Result<(), PrintTemplateError> {
    append_h9_audit(
        tx,
        ctx,
        "publish_field_library",
        "print_field_library",
        version.id,
        now,
    )
    .await
}

async fn append_h9_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), PrintTemplateError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H9",
        resource_type,
        resource_id.to_string(),
        None,
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| PrintTemplateError::Audit(format!("{error:?}")))?;
    Ok(())
}

fn json_request_hash(value: &Value) -> Result<String, PrintTemplateError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

#[derive(Debug, FromRow)]
struct PrintTemplateVersionRow {
    id: Uuid,
    template_id: Uuid,
    template_code: String,
    template_name: String,
    template_type_code: String,
    owner_id: Uuid,
    scope: String,
    enabled: bool,
    is_default: bool,
    remark: Option<String>,
    field_library_version_id: Uuid,
    version_no: i32,
    status: String,
    hiprint_json: Value,
    field_bindings: Value,
    paper: Value,
    designer_version: String,
    created_at: DateTime<Utc>,
    created_by: Uuid,
    published_at: Option<DateTime<Utc>>,
    published_by: Option<Uuid>,
}

impl TryFrom<PrintTemplateVersionRow> for PrintTemplateVersion {
    type Error = PrintTemplateError;

    fn try_from(row: PrintTemplateVersionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            template_id: row.template_id,
            template_code: row.template_code,
            template_name: row.template_name,
            template_type_code: row.template_type_code,
            owner_id: row.owner_id,
            scope: PrintTemplateScope::from_db(row.scope)?,
            enabled: row.enabled,
            is_default: row.is_default,
            remark: row.remark,
            field_library_version_id: row.field_library_version_id,
            version_no: row.version_no,
            status: row.status,
            hiprint_json: row.hiprint_json,
            field_bindings: serde_json::from_value(row.field_bindings)
                .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
            paper: row.paper,
            designer_version: row.designer_version,
            created_at: row.created_at,
            created_by: row.created_by,
            published_at: row.published_at,
            published_by: row.published_by,
        })
    }
}

#[derive(Debug, FromRow)]
struct PrintTemplateSummaryRow {
    id: Uuid,
    template_code: String,
    template_name: String,
    template_type_code: String,
    owner_id: Uuid,
    scope: String,
    enabled: bool,
    is_default: bool,
    remark: Option<String>,
    latest_version_id: Uuid,
    latest_version_no: i32,
    latest_version_status: String,
    field_library_version_id: Uuid,
    designer_version: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
}

impl TryFrom<PrintTemplateSummaryRow> for PrintTemplateSummary {
    type Error = PrintTemplateError;

    fn try_from(row: PrintTemplateSummaryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            template_code: row.template_code,
            template_name: row.template_name,
            template_type_code: row.template_type_code,
            owner_id: row.owner_id,
            scope: PrintTemplateScope::from_db(row.scope)?,
            enabled: row.enabled,
            is_default: row.is_default,
            remark: row.remark,
            latest_version_id: row.latest_version_id,
            latest_version_no: row.latest_version_no,
            latest_version_status: row.latest_version_status,
            field_library_version_id: row.field_library_version_id,
            designer_version: row.designer_version,
            created_at: row.created_at,
            updated_at: row.updated_at,
            published_at: row.published_at,
        })
    }
}

fn map_db_error(error: sqlx::Error) -> PrintTemplateError {
    PrintTemplateError::Database(error.to_string())
}
