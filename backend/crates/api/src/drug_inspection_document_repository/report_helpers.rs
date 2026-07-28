use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::DrugInspectionReportVersion;

use crate::auth::AuthContext;

use super::{
    helpers::store_idempotency,
    map_db_error,
    report_audit::{append_audit, version_snapshot},
    DrugInspectionDocumentRepositoryError,
};

#[derive(Clone, FromRow)]
pub(super) struct DrugInspectionVersionRow {
    pub(super) id: Uuid,
    report_id: Uuid,
    owner_id: Uuid,
    pub(super) version_number: i32,
    report_no: String,
    original_file_id: Uuid,
    original_file_hash: String,
    source: String,
    processing_mode: String,
    qualified: bool,
    pub(super) status: String,
    replaces_version_id: Option<Uuid>,
    modification_reason: Option<String>,
    uploaded_by: Uuid,
    submitted_at: Option<DateTime<Utc>>,
    reviewed_by: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    review_result: Option<String>,
    review_comment: Option<String>,
    customer_copy_status: String,
    customer_copy_file_id: Option<Uuid>,
    customer_copy_hash: Option<String>,
    stamp_version_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<DrugInspectionVersionRow> for DrugInspectionReportVersion {
    fn from(row: DrugInspectionVersionRow) -> Self {
        Self {
            id: row.id,
            report_id: row.report_id,
            owner_id: row.owner_id,
            version_number: row.version_number,
            report_no: row.report_no,
            original_file_id: row.original_file_id,
            original_file_hash: row.original_file_hash,
            source: row.source,
            processing_mode: row.processing_mode,
            qualified: row.qualified,
            status: row.status,
            replaces_version_id: row.replaces_version_id,
            modification_reason: row.modification_reason,
            uploaded_by: row.uploaded_by,
            submitted_at: row.submitted_at,
            reviewed_by: row.reviewed_by,
            reviewed_at: row.reviewed_at,
            review_result: row.review_result,
            review_comment: row.review_comment,
            customer_copy_status: row.customer_copy_status,
            customer_copy_file_id: row.customer_copy_file_id,
            customer_copy_hash: row.customer_copy_hash,
            stamp_version_id: row.stamp_version_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub(super) async fn validate_asn_batch(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    asn_id: Uuid,
    product_id: Uuid,
    batch_no: &str,
) -> Result<(), DrugInspectionDocumentRepositoryError> {
    let value: Option<(i64, Option<bool>, Option<bool>, Option<bool>)> = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT product_id),
               BOOL_OR(product_id = $3 AND batch_no = $4),
               BOOL_OR(product_id = $3 AND batch_no IS NULL),
               BOOL_OR(product_id = $3 AND batch_no IS NOT NULL)
          FROM receiving_order_lines
         WHERE owner_id = $1 AND receiving_order_id = $2
        "#,
    )
    .bind(owner_id)
    .bind(asn_id)
    .bind(product_id)
    .bind(batch_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    // 批号在验收时才写入行；验收前（该商品全部行批号仍为 NULL）允许按商品级先登记药检单。
    // 若同商品已有部分行写入了其它批号，禁止用「存在 NULL 行」放行任意批号，避免串批。
    let matched = matches!(
        value,
        Some((1, Some(true), _, _)) | Some((1, _, Some(true), Some(false)))
    );
    if !matched {
        return Err(DrugInspectionDocumentRepositoryError::Conflict(
            "asn_product_batch_mismatch",
        ));
    }
    Ok(())
}

pub(super) async fn attachment_hash(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    attachment_id: Uuid,
) -> Result<String, DrugInspectionDocumentRepositoryError> {
    sqlx::query_scalar(
        r#"
        SELECT sha256
          FROM attachments
         WHERE owner_id = $1 AND id = $2
           AND module = 'M-DI'
           AND content_type IN ('image/jpeg', 'image/png', 'application/pdf')
        "#,
    )
    .bind(owner_id)
    .bind(attachment_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(DrugInspectionDocumentRepositoryError::NotFound)
}

pub(super) async fn report_by_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    product_id: Uuid,
    batch_no: &str,
) -> Result<Option<Uuid>, DrugInspectionDocumentRepositoryError> {
    sqlx::query_scalar(
        "SELECT id FROM drug_inspection_reports WHERE owner_id = $1 AND product_id = $2 AND batch_no = $3 FOR UPDATE",
    )
    .bind(owner_id)
    .bind(product_id)
    .bind(batch_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn insert_version(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    report_id: Uuid,
    version_number: i32,
    report_no: &str,
    original_file_id: Uuid,
    original_file_hash: &str,
    source: &str,
    processing_mode: &str,
    qualified: bool,
    replaces_version_id: Option<Uuid>,
    modification_reason: Option<&str>,
    now: DateTime<Utc>,
) -> Result<DrugInspectionReportVersion, DrugInspectionDocumentRepositoryError> {
    sqlx::query_as::<_, DrugInspectionVersionRow>(
        r#"
        INSERT INTO drug_inspection_report_versions (
            id, report_id, owner_id, version_number, report_no, original_file_id,
            original_file_hash, source, processing_mode, qualified, status,
            replaces_version_id, modification_reason, uploaded_by, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'draft', $11, $12, $13, $14, $14)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(report_id)
    .bind(ctx.owner_id)
    .bind(version_number)
    .bind(report_no)
    .bind(original_file_id)
    .bind(original_file_hash)
    .bind(source)
    .bind(processing_mode)
    .bind(qualified)
    .bind(replaces_version_id)
    .bind(modification_reason)
    .bind(ctx.user_id)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map(Into::into)
    .map_err(map_db_error)
}

pub(super) async fn fetch_version_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    version_id: Uuid,
) -> Result<Option<DrugInspectionReportVersion>, DrugInspectionDocumentRepositoryError> {
    sqlx::query_as::<_, DrugInspectionVersionRow>(
        "SELECT * FROM drug_inspection_report_versions WHERE owner_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(owner_id)
    .bind(version_id)
    .fetch_optional(&mut **tx)
    .await
    .map(|row| row.map(Into::into))
    .map_err(map_db_error)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn finish_mutation(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    action: &str,
    value: &DrugInspectionReportVersion,
    before: Value,
    now: DateTime<Utc>,
) -> Result<(), DrugInspectionDocumentRepositoryError> {
    store_idempotency(
        tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        "drug_inspection_report_version",
        value.id,
        value,
        now,
    )
    .await?;
    append_audit(
        tx,
        ctx,
        action,
        "drug_inspection_report_version",
        value.id,
        before,
        version_snapshot(value),
        now,
    )
    .await
}
