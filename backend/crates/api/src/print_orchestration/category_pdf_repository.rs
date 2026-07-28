//! PostgreSQL reads for H9 category PDF preparation.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{CategoryPdfOutput, PrintSuiteSourceMode};

use super::{repository::map_db_error, PrintOrchestrationError};

#[derive(Debug, FromRow)]
pub(super) struct PreparationRow {
    pub(super) id: Uuid,
    pub(super) idempotency_key: String,
    pub(super) request_hash: String,
    pub(super) status: String,
}

#[derive(Debug, FromRow)]
pub(super) struct InstanceSourceRow {
    pub(super) delivery_note_no: String,
    pub(super) source_documents: Value,
}

#[derive(Debug, FromRow)]
pub(super) struct InstanceItemSourceRow {
    pub(super) id: Uuid,
    pub(super) category_code: String,
    pub(super) source_mode: String,
    pub(super) template_version_id: Option<Uuid>,
    pub(super) file_bindings: Value,
    pub(super) ready: bool,
}

#[derive(Debug, FromRow)]
struct CategoryPdfRow {
    id: Uuid,
    instance_id: Uuid,
    instance_item_id: Uuid,
    sort_order: i32,
    category_code: String,
    source_mode: String,
    source_data_version: Option<String>,
    source_file_bindings: Value,
    template_version_id: Option<Uuid>,
    attachment_id: Option<Uuid>,
    content_hash: Option<String>,
    processing_status: String,
    failure_reason: Option<String>,
    retention_policy: String,
    cache_expires_at: Option<DateTime<Utc>>,
    attempt_count: i32,
    created_at: DateTime<Utc>,
    processed_at: Option<DateTime<Utc>>,
}

pub(super) async fn ensure_instance(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    instance_id: Uuid,
) -> Result<(), PrintOrchestrationError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM h9_print_suite_instances WHERE owner_id = $1 AND id = $2)",
    )
    .bind(owner_id)
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(PrintOrchestrationError::PrintSuiteNotFound)
    }
}

pub(super) async fn load_instance_source(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    instance_id: Uuid,
) -> Result<InstanceSourceRow, PrintOrchestrationError> {
    sqlx::query_as(
        r#"
        SELECT group_row.delivery_note_no, instance.source_documents
          FROM h9_print_suite_instances instance
          JOIN h9_delivery_note_groups group_row
            ON group_row.owner_id = instance.owner_id
           AND group_row.id = instance.group_id
         WHERE instance.owner_id = $1 AND instance.id = $2
         FOR UPDATE OF instance
        "#,
    )
    .bind(owner_id)
    .bind(instance_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintOrchestrationError::PrintSuiteNotFound)
}

pub(super) async fn load_instance_item_sources(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    instance_id: Uuid,
) -> Result<Vec<InstanceItemSourceRow>, PrintOrchestrationError> {
    sqlx::query_as(
        r#"
        SELECT id, category_code, source_mode, template_version_id,
               file_bindings, ready
          FROM h9_print_suite_instance_items
         WHERE owner_id = $1 AND instance_id = $2
         ORDER BY sort_order
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(instance_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub(super) async fn load_outputs(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    instance_id: Uuid,
) -> Result<Vec<CategoryPdfOutput>, PrintOrchestrationError> {
    let rows = sqlx::query_as::<_, CategoryPdfRow>(
        r#"
        SELECT output.id, output.instance_id, output.instance_item_id,
               item.sort_order, output.category_code, output.source_mode,
               output.source_data_version, output.source_file_bindings,
               output.template_version_id, output.attachment_id,
               output.content_hash, output.processing_status,
               output.failure_reason, output.retention_policy,
               output.cache_expires_at, output.attempt_count,
               output.created_at, output.processed_at
          FROM h9_category_pdf_outputs output
          JOIN h9_print_suite_instance_items item
            ON item.owner_id = output.owner_id
           AND item.id = output.instance_item_id
         WHERE output.owner_id = $1 AND output.instance_id = $2
         ORDER BY item.sort_order, output.id
        "#,
    )
    .bind(owner_id)
    .bind(instance_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    rows.into_iter().map(map_output).collect()
}

fn map_output(row: CategoryPdfRow) -> Result<CategoryPdfOutput, PrintOrchestrationError> {
    Ok(CategoryPdfOutput {
        id: row.id,
        instance_id: row.instance_id,
        instance_item_id: row.instance_item_id,
        sort_order: row.sort_order,
        category_code: row.category_code,
        source_mode: PrintSuiteSourceMode::try_from(row.source_mode.as_str()).map_err(|()| {
            PrintOrchestrationError::Serialize(format!("unknown source mode: {}", row.source_mode))
        })?,
        source_data_version: row.source_data_version,
        source_file_bindings: serde_json::from_value(row.source_file_bindings)
            .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?,
        template_version_id: row.template_version_id,
        attachment_id: row.attachment_id,
        content_hash: row.content_hash,
        processing_status: row.processing_status,
        failure_reason: row.failure_reason,
        retention_policy: row.retention_policy,
        cache_expires_at: row.cache_expires_at,
        attempt_count: row.attempt_count,
        created_at: row.created_at,
        processed_at: row.processed_at,
    })
}
