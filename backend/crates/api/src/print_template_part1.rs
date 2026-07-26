use std::collections::{BTreeSet, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::{PageMeta, SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

#[derive(Clone, Debug, Default)]
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
    FieldLibraryVersionNotFound,
    FieldLibraryNotPublished,
    FieldFormatInvalid(String),
    FieldPathInvalid(Vec<String>),
    PublishedFieldLibraryImmutable,
    TemplateDisabled,
    TemplateFieldMismatch(Vec<String>),
    TemplateFieldMissing(Vec<String>),
    TemplateJsonInvalid,
    TemplateDuplicate,
    TemplateNotFound,
    TemplateVersionNotFound,
    TemplateVersionNotLatest,
    PublishedTemplateImmutable,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct GeneratePrintFieldLibraryDraftRequest {
    pub library_code: String,
    pub library_name: String,
    pub business_module: String,
    pub source_schema: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct UpdatePrintFieldDefinitionRequest {
    pub display_name: String,
    pub group_code: String,
    pub group_name: String,
    pub description: String,
    pub example_value: Option<Value>,
    pub printable: bool,
    pub sensitive: bool,
    pub masking_rule: Option<String>,
    pub formatting_rule: Option<String>,
    pub supports_barcode: bool,
    pub supports_qrcode: bool,
    pub is_table_detail: bool,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize, ToSchema)]
pub struct PrintFieldLibraryVersion {
    pub id: Uuid,
    pub library_id: Uuid,
    pub library_code: String,
    pub library_name: String,
    pub business_module: String,
    pub source_schema: String,
    pub version_no: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub published_at: Option<DateTime<Utc>>,
    pub published_by: Option<Uuid>,
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
    pub description: String,
    pub example_value: Option<Value>,
    pub printable: bool,
    pub sensitive: bool,
    pub masking_rule: Option<String>,
    pub formatting_rule: Option<String>,
    pub supports_barcode: bool,
    pub supports_qrcode: bool,
    pub is_table_detail: bool,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize, ToSchema)]
pub struct PrintFieldLibrarySummary {
    pub id: Uuid,
    pub library_code: String,
    pub library_name: String,
    pub business_module: String,
    pub source_schema: String,
    pub latest_version_id: Uuid,
    pub version_no: i32,
    pub latest_version_status: String,
    pub field_count: i64,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub published_at: Option<DateTime<Utc>>,
    pub published_by: Option<Uuid>,
    pub latest_published_version_id: Option<Uuid>,
    pub latest_published_version_no: Option<i32>,
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
    pub template_id: Option<Uuid>,
    pub template_code: String,
    pub template_name: String,
    pub template_type_code: String,
    pub scope: PrintTemplateScope,
    pub is_default: bool,
    pub remark: Option<String>,
    pub field_library_version_id: Uuid,
    pub hiprint_json: Value,
    pub field_bindings: Vec<PrintTemplateBinding>,
    pub paper: Value,
    pub designer_version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct SetPrintTemplateEnabledRequest {
    pub enabled: bool,
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

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PrintTemplateVersionListResponse {
    pub data: Vec<PrintTemplateVersion>,
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

    pub async fn list_field_version_fields(
        &self,
        pool: &PgPool,
        library_version_id: Uuid,
    ) -> Result<Vec<PrintFieldDefinition>, PrintTemplateError> {
        sqlx::query_as::<_, PrintFieldDefinition>(
            r#"
            SELECT id, library_version_id, field_path, field_type, source_schema,
                   display_name, group_code, group_name, description, example_value,
                   printable, sensitive, masking_rule, formatting_rule,
                   supports_barcode, supports_qrcode, is_table_detail, sort_order
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
                latest_versions.business_module,
                latest_versions.source_schema,
                latest_versions.id AS latest_version_id,
                latest_versions.version_no,
                latest_versions.status AS latest_version_status,
                COUNT(fields.id)::BIGINT AS field_count,
                libraries.created_at,
                latest_versions.created_by,
                latest_versions.published_at,
                latest_versions.published_by,
                published_versions.id AS latest_published_version_id,
                published_versions.version_no AS latest_published_version_no
              FROM print_field_libraries libraries
              JOIN LATERAL (
                SELECT id, version_no, status, source_schema, business_module, created_by,
                       published_at, published_by
                  FROM print_field_library_versions
                 WHERE library_id = libraries.id
                 ORDER BY version_no DESC
                 LIMIT 1
              ) latest_versions ON TRUE
              LEFT JOIN LATERAL (
                SELECT id, version_no
                  FROM print_field_library_versions
                 WHERE library_id = libraries.id
                   AND status = 'published'
                 ORDER BY version_no DESC
                 LIMIT 1
              ) published_versions ON TRUE
              LEFT JOIN print_field_definitions fields
                ON fields.library_version_id = latest_versions.id
             GROUP BY
                libraries.id,
                libraries.library_code,
                libraries.library_name,
                latest_versions.business_module,
                latest_versions.source_schema,
                latest_versions.id,
                latest_versions.version_no,
                latest_versions.status,
                libraries.created_at,
                latest_versions.created_by,
                latest_versions.published_at,
                latest_versions.published_by,
                published_versions.id,
                published_versions.version_no
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
                latest_versions.template_name,
                latest_versions.template_type_code,
                templates.owner_id,
                latest_versions.scope,
                templates.enabled,
                latest_versions.is_default,
                latest_versions.remark,
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
                SELECT
                    id, template_name, template_type_code, scope, is_default, remark,
                    version_no, status, field_library_version_id, designer_version, published_at
                  FROM print_template_versions
                 WHERE template_id = templates.id
                 ORDER BY version_no DESC
                 LIMIT 1
              ) latest_versions ON TRUE
             WHERE templates.owner_id = $1
             ORDER BY latest_versions.template_type_code ASC, templates.template_code ASC
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter()
            .map(PrintTemplateSummary::try_from)
            .collect()
    }

    pub async fn list_template_versions(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        template_id: Uuid,
    ) -> Result<Vec<PrintTemplateVersion>, PrintTemplateError> {
        let rows = sqlx::query_as::<_, PrintTemplateVersionRow>(
            r#"
            SELECT
                versions.id,
                templates.id AS template_id,
                templates.template_code,
                versions.template_name,
                versions.template_type_code,
                templates.owner_id,
                versions.scope,
                templates.enabled,
                versions.is_default,
                versions.remark,
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
              FROM print_template_versions versions
              JOIN print_templates templates ON templates.id = versions.template_id
             WHERE templates.owner_id = $1
               AND templates.id = $2
             ORDER BY versions.version_no DESC
            "#,
        )
        .bind(ctx.owner_id)
        .bind(template_id)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter()
            .map(PrintTemplateVersion::try_from)
            .collect()
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
        let field_library_code = validate_field_library_and_bindings(pool, &req).await?;
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
        let Some(expected_library_code) = effective_template_type_field_library_code_in_tx(
            &mut tx,
            ctx.owner_id,
            &req.template_type_code,
            now,
        )
        .await?
        else {
            return Err(PrintTemplateError::TemplateDisabled);
        };
        if expected_library_code != field_library_code {
            return Err(PrintTemplateError::TemplateFieldMismatch(vec![format!(
                "field_library_code:{field_library_code}"
            )]));
        }

        let (template_id, enabled) = upsert_template_for_update(&mut tx, ctx, &req, now).await?;
        let version_no = next_template_version_no(&mut tx, template_id).await?;
        let version_id = Uuid::new_v4();
        let row = sqlx::query_as::<_, PrintTemplateVersionRow>(
            r#"
            INSERT INTO print_template_versions (
                id, template_id, field_library_version_id,
                template_name, template_type_code, scope, is_default, remark,
                version_no, status,
                hiprint_json, field_bindings, paper, designer_version, request_hash,
                created_at, created_by, published_at, published_by
            )
            VALUES (
                $1, $2, $3,
                $4, $5, $6, $7, $8,
                $9, 'draft',
                $10, $11, $12, $13, $14,
                $15, $16, NULL, NULL
            )
            RETURNING
                id,
                template_id,
                $17::TEXT AS template_code,
                template_name,
                template_type_code,
                $18::UUID AS owner_id,
                scope,
                $19::BOOLEAN AS enabled,
                is_default,
                remark,
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
        .bind(&req.template_name)
        .bind(&req.template_type_code)
        .bind(req.scope.as_str())
        .bind(req.is_default)
        .bind(&req.remark)
        .bind(version_no)
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
        .bind(ctx.owner_id)
        .bind(enabled)
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
        append_h9_audit(
            &mut tx,
            ctx,
            "save_print_template",
            "print_template",
            version.id,
            now,
            Some(AuditDiff::compute(
                Value::Null,
                serde_json::to_value(&version)
                    .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
            )),
        )
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
        let version =
            resolve_template_version(pool, ctx, &req.template_code, &req.template_type_code)
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
}
