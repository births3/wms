use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, Postgres, Transaction};
use std::collections::BTreeSet;
use uuid::Uuid;
use wms_domain::{
    CreateUpstreamDeliveryVersionRequest, DrugInspectionDocumentValidationError,
    UpstreamDeliveryDocumentVersion, H_FILE_M_DI_IMAGE_MAX_SIZE_BYTES,
};

use crate::operation_context::OperationContext as AuthContext;

use super::{
    helpers::{lock_idempotency_key, replay_idempotency, request_hash, store_idempotency},
    map_db_error,
    report_audit::append_audit,
    DrugInspectionDocumentRepositoryError, PgDrugInspectionDocumentRepository,
};

impl PgDrugInspectionDocumentRepository {
    pub async fn create_upstream_delivery_version(
        &self,
        ctx: &AuthContext,
        request: CreateUpstreamDeliveryVersionRequest,
        idempotency_key: &str,
    ) -> Result<UpstreamDeliveryDocumentVersion, DrugInspectionDocumentRepositoryError> {
        request
            .validate()
            .map_err(DrugInspectionDocumentRepositoryError::Invalid)?;
        let hash = request_hash(&request)?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }

        let asn_ids = request
            .asn_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        // 多页扫描件的页序由客户端提交顺序决定，去重必须保序，不能经 BTreeSet 重排。
        let mut seen_attachments = BTreeSet::new();
        let attachment_ids = request
            .attachment_ids
            .iter()
            .copied()
            .filter(|id| seen_attachments.insert(*id))
            .collect::<Vec<_>>();
        validate_asns(&mut tx, ctx.owner_id, request.supplier_id, &asn_ids).await?;
        validate_files(&mut tx, ctx.owner_id, &attachment_ids).await?;

        let (document_id, version_number) = if let Some(document_id) = request.document_id {
            let supplier_id: Uuid = sqlx::query_scalar(
                "SELECT supplier_id FROM upstream_delivery_documents WHERE owner_id = $1 AND id = $2 FOR UPDATE",
            )
            .bind(ctx.owner_id)
            .bind(document_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
            .ok_or(DrugInspectionDocumentRepositoryError::NotFound)?;
            if supplier_id != request.supplier_id {
                return Err(DrugInspectionDocumentRepositoryError::Conflict(
                    "supplier_mismatch",
                ));
            }
            let next: i32 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(version_number), 0) + 1 FROM upstream_delivery_document_versions WHERE document_id = $1",
            )
            .bind(document_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            (document_id, next)
        } else {
            let document_id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO upstream_delivery_documents (
                    id, owner_id, supplier_id, created_by, created_at
                )
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(document_id)
            .bind(ctx.owner_id)
            .bind(request.supplier_id)
            .bind(ctx.user_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            (document_id, 1)
        };

        let version_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO upstream_delivery_document_versions (
                id, document_id, owner_id, version_number, modification_reason,
                uploaded_by, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(version_id)
        .bind(document_id)
        .bind(ctx.owner_id)
        .bind(version_number)
        .bind(request.modification_reason.as_deref().map(str::trim))
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        for (index, attachment_id) in attachment_ids.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO upstream_delivery_document_files (
                    version_id, attachment_id, position
                )
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(version_id)
            .bind(attachment_id)
            .bind(i32::try_from(index + 1).map_err(|_| {
                DrugInspectionDocumentRepositoryError::Serialize(
                    "上游随货同行单文件过多".to_string(),
                )
            })?)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        for asn_id in &asn_ids {
            sqlx::query(
                r#"
                INSERT INTO upstream_delivery_document_asn_links (
                    id, owner_id, version_id, asn_id, linked_by, linked_at
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(version_id)
            .bind(asn_id)
            .bind(ctx.user_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                r#"
                INSERT INTO upstream_delivery_asn_current (
                    owner_id, asn_id, version_id, updated_at
                )
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (owner_id, asn_id) DO UPDATE
                   SET version_id = EXCLUDED.version_id,
                       updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(ctx.owner_id)
            .bind(asn_id)
            .bind(version_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let value = UpstreamDeliveryDocumentVersion {
            id: version_id,
            document_id,
            owner_id: ctx.owner_id,
            version_number,
            modification_reason: request
                .modification_reason
                .as_deref()
                .map(str::trim)
                .map(str::to_string),
            attachment_ids,
            asn_ids,
            uploaded_by: ctx.user_id,
            created_at: now,
        };
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/drug-inspection/upstream-delivery-document-versions",
            "upstream_delivery_document_version",
            version_id,
            &value,
            now,
        )
        .await?;
        append_audit(
            &mut tx,
            ctx,
            "di.upstream_delivery.version_created",
            "upstream_delivery_document_version",
            version_id,
            Value::Null,
            serde_json::to_value(&value).map_err(|error| {
                DrugInspectionDocumentRepositoryError::Serialize(error.to_string())
            })?,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn list_upstream_delivery_versions(
        &self,
        owner_id: Uuid,
        document_id: Uuid,
    ) -> Result<Vec<UpstreamDeliveryDocumentVersion>, DrugInspectionDocumentRepositoryError> {
        let rows = sqlx::query_as::<_, UpstreamVersionRow>(
            r#"
            SELECT version.id, version.document_id, version.owner_id, version.version_number,
                   version.modification_reason, version.uploaded_by, version.created_at,
                   COALESCE((
                       SELECT array_agg(file.attachment_id ORDER BY file.position)
                         FROM upstream_delivery_document_files AS file
                        WHERE file.version_id = version.id
                   ), ARRAY[]::UUID[]) AS attachment_ids,
                   COALESCE((
                       SELECT array_agg(DISTINCT link.asn_id)
                         FROM upstream_delivery_document_asn_links AS link
                        WHERE link.version_id = version.id
                   ), ARRAY[]::UUID[]) AS asn_ids
              FROM upstream_delivery_document_versions AS version
             WHERE version.owner_id = $1 AND version.document_id = $2
          ORDER BY version.version_number DESC
            "#,
        )
        .bind(owner_id)
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[derive(FromRow)]
struct UpstreamVersionRow {
    id: Uuid,
    document_id: Uuid,
    owner_id: Uuid,
    version_number: i32,
    modification_reason: Option<String>,
    uploaded_by: Uuid,
    created_at: DateTime<Utc>,
    attachment_ids: Vec<Uuid>,
    asn_ids: Vec<Uuid>,
}

impl From<UpstreamVersionRow> for UpstreamDeliveryDocumentVersion {
    fn from(row: UpstreamVersionRow) -> Self {
        Self {
            id: row.id,
            document_id: row.document_id,
            owner_id: row.owner_id,
            version_number: row.version_number,
            modification_reason: row.modification_reason,
            attachment_ids: row.attachment_ids,
            asn_ids: row.asn_ids,
            uploaded_by: row.uploaded_by,
            created_at: row.created_at,
        }
    }
}

async fn validate_asns(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    supplier_id: Uuid,
    asn_ids: &[Uuid],
) -> Result<(), DrugInspectionDocumentRepositoryError> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM receiving_orders
         WHERE owner_id = $1 AND supplier_id = $2
           AND document_type = 'purchase_inbound'
           AND id = ANY($3)
        "#,
    )
    .bind(owner_id)
    .bind(supplier_id)
    .bind(asn_ids)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if usize::try_from(count).ok() != Some(asn_ids.len()) {
        return Err(DrugInspectionDocumentRepositoryError::Conflict(
            "asn_supplier_mismatch",
        ));
    }
    Ok(())
}

async fn validate_files(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    attachment_ids: &[Uuid],
) -> Result<(), DrugInspectionDocumentRepositoryError> {
    let files = sqlx::query_as::<_, (Uuid, String, i64)>(
        r#"
        SELECT id, content_type, size_bytes
          FROM attachments
         WHERE owner_id = $1 AND id = ANY($2)
        "#,
    )
    .bind(owner_id)
    .bind(attachment_ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if files.len() != attachment_ids.len()
        || files
            .iter()
            .any(|(_, _, size)| *size > H_FILE_M_DI_IMAGE_MAX_SIZE_BYTES)
    {
        return Err(DrugInspectionDocumentRepositoryError::Conflict(
            "upstream_file_invalid",
        ));
    }
    let one_pdf = files.len() == 1 && files[0].1 == "application/pdf";
    let all_jpeg = files
        .iter()
        .all(|(_, content_type, _)| content_type == "image/jpeg");
    if !one_pdf && !all_jpeg {
        return Err(DrugInspectionDocumentRepositoryError::Invalid(
            DrugInspectionDocumentValidationError::EmptyAttachmentSelection,
        ));
    }
    Ok(())
}
