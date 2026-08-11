use std::{path::PathBuf, sync::Arc};

use chrono::{DateTime, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use wms_domain::{
    ApproveDrugInspectionCopyOversizeRequest, DrugInspectionCustomerCopyJob,
    DrugInspectionDocumentValidationError,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    drug_inspection_copy_processor::{
        generate_customer_pdf, DrugInspectionCopyError, StampPlacement, MDI_COPY_SOFT_LIMIT_BYTES,
    },
    drug_inspection_copy_projection::publish_report_projection_from_db,
    drug_inspection_document_repository::{
        helpers::{lock_idempotency_key, replay_idempotency, request_hash, store_idempotency},
        DrugInspectionDocumentRepositoryError,
    },
    operation_context::OperationContext as AuthContext,
};

#[derive(Clone)]
pub struct DrugInspectionCopyService {
    pool: PgPool,
    storage_root: Arc<PathBuf>,
}

#[derive(Debug)]
pub enum DrugInspectionCopyServiceError {
    Invalid(DrugInspectionDocumentValidationError),
    NotFound,
    Conflict(&'static str),
    IdempotencyConflict,
    Storage(String),
    Processing(DrugInspectionCopyError),
    Database(String),
    Audit(String),
    Serialize(String),
}

impl DrugInspectionCopyService {
    pub fn new(pool: PgPool, storage_root: PathBuf) -> Self {
        Self {
            pool,
            storage_root: Arc::new(storage_root),
        }
    }

    pub async fn process_next(
        &self,
    ) -> Result<Option<DrugInspectionCustomerCopyJob>, DrugInspectionCopyServiceError> {
        let Some(source) = self.claim_next().await? else {
            return Ok(None);
        };
        let result = self.generate_and_store(&source).await;
        if let Err(error) = result {
            self.mark_failed(&source, &error).await?;
            return Err(error);
        }
        self.get_job(source.job_id).await.map(Some)
    }

    pub async fn process_job(
        &self,
        owner_id: Uuid,
        job_id: Uuid,
    ) -> Result<DrugInspectionCustomerCopyJob, DrugInspectionCopyServiceError> {
        let source = self.claim_job(owner_id, job_id).await?;
        let result = self.generate_and_store(&source).await;
        if let Err(error) = result {
            self.mark_failed(&source, &error).await?;
            return Err(error);
        }
        self.get_job(job_id).await
    }

    pub async fn list_jobs(
        &self,
        owner_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<(Vec<DrugInspectionCustomerCopyJob>, i64), DrugInspectionCopyServiceError> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 200);
        let offset = ((page - 1) as i64) * (page_size as i64);
        let total: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM drug_inspection_customer_copy_jobs WHERE owner_id = $1",
        )
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        let rows = sqlx::query_as::<_, CopyJobRow>(
            "SELECT id, owner_id, report_version_id, status, attempt_count, last_error, created_at, started_at, finished_at, updated_at FROM drug_inspection_customer_copy_jobs WHERE owner_id = $1 ORDER BY created_at DESC, id LIMIT $2 OFFSET $3",
        )
        .bind(owner_id)
        .bind(page_size as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok((rows.into_iter().map(Into::into).collect(), total))
    }

    pub async fn approve_oversize(
        &self,
        ctx: &AuthContext,
        job_id: Uuid,
        request: ApproveDrugInspectionCopyOversizeRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionCustomerCopyJob, DrugInspectionCopyServiceError> {
        request
            .validate()
            .map_err(DrugInspectionCopyServiceError::Invalid)?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let request_fingerprint = request_hash(&json!({
            "job_id": job_id,
            "request": &request,
        }))
        .map_err(map_repository_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key)
            .await
            .map_err(map_repository_error)?;
        if let Some(replayed) = replay_idempotency::<DrugInspectionCustomerCopyJob>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_fingerprint,
            now,
        )
        .await
        .map_err(map_repository_error)?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(replayed);
        }
        type ClaimedJobRow = (
            Uuid,
            Uuid,
            Uuid,
            String,
            Option<Uuid>,
            Option<String>,
            Option<i64>,
        );
        let row: Option<ClaimedJobRow> = sqlx::query_as(
            r#"
                SELECT job.report_version_id, version.uploaded_by, version.id, job.status,
                       job.candidate_file_id, job.candidate_hash, job.candidate_size
                  FROM drug_inspection_customer_copy_jobs AS job
                  JOIN drug_inspection_report_versions AS version
                    ON version.id = job.report_version_id
                   AND version.owner_id = job.owner_id
                 WHERE job.owner_id = $1 AND job.id = $2
                 FOR UPDATE OF job, version
                "#,
        )
        .bind(ctx.owner_id)
        .bind(job_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some((
            version_id,
            uploaded_by,
            _,
            status,
            candidate_file_id,
            candidate_hash,
            candidate_size,
        )) = row
        else {
            return Err(DrugInspectionCopyServiceError::NotFound);
        };
        if status != "oversize_review" {
            return Err(DrugInspectionCopyServiceError::Conflict(
                "copy_not_awaiting_oversize_review",
            ));
        }
        if uploaded_by == ctx.user_id {
            return Err(DrugInspectionCopyServiceError::Conflict(
                "oversize_approver_is_uploader",
            ));
        }
        let file_id = candidate_file_id.ok_or(DrugInspectionCopyServiceError::Conflict(
            "copy_candidate_missing",
        ))?;
        let hash = candidate_hash.ok_or(DrugInspectionCopyServiceError::Conflict(
            "copy_candidate_missing",
        ))?;
        if candidate_size.is_none() {
            return Err(DrugInspectionCopyServiceError::Conflict(
                "copy_candidate_missing",
            ));
        }
        sqlx::query(
            r#"
            UPDATE drug_inspection_report_versions
               SET customer_copy_status = 'available',
                   customer_copy_file_id = $3,
                   customer_copy_hash = $4,
                   updated_at = $5
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(version_id)
        .bind(file_id)
        .bind(&hash)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            r#"
            UPDATE drug_inspection_customer_copy_jobs
               SET status = 'succeeded', oversize_reason = $3,
                   oversize_approved_by = $4, finished_at = $5, updated_at = $5
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(job_id)
        .bind(request.reason.trim())
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let audit = AuditWriteRequest::from_auth_context(
            ctx,
            "di.customer_copy.oversize_approved",
            "DI",
            "drug_inspection_customer_copy_job",
            job_id.to_string(),
            Some(AuditDiff::compute(
                json!({ "status": "oversize_review" }),
                json!({
                    "status": "succeeded",
                    "reason": request.reason.trim(),
                    "candidate_size": candidate_size,
                }),
            )),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| DrugInspectionCopyServiceError::Audit(format!("{error:?}")))?;
        publish_report_projection_from_db(&mut tx, ctx.owner_id, version_id, now).await?;
        let approved = sqlx::query_as::<_, CopyJobRow>(
            "SELECT id, owner_id, report_version_id, status, attempt_count, last_error, created_at, started_at, finished_at, updated_at FROM drug_inspection_customer_copy_jobs WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(job_id)
        .fetch_one(&mut *tx)
        .await
        .map(Into::into)
        .map_err(map_db_error)?;
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_fingerprint,
            "POST",
            &format!("/api/v1/drug-inspection/customer-copy-jobs/{job_id}/oversize-approval"),
            "drug_inspection_customer_copy_job",
            job_id,
            &approved,
            now,
        )
        .await
        .map_err(map_repository_error)?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(approved)
    }

    async fn claim_next(&self) -> Result<Option<CopySource>, DrugInspectionCopyServiceError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let job_id: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
              FROM drug_inspection_customer_copy_jobs
             WHERE (
                   (status IN ('queued', 'failed') AND attempt_count < 3)
                   -- 崩溃不等于一次已记录失败；即使第三次 claim 崩溃，超时租约仍须可回收。
                   OR (status = 'processing' AND started_at < now() - interval '10 minutes')
             )
             ORDER BY created_at
             FOR UPDATE SKIP LOCKED
             LIMIT 1
            "#,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some(job_id) = job_id else {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(None);
        };
        let source = claim_source(&mut tx, None, job_id).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(Some(source))
    }

    async fn claim_job(
        &self,
        owner_id: Uuid,
        job_id: Uuid,
    ) -> Result<CopySource, DrugInspectionCopyServiceError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let source = claim_source(&mut tx, Some(owner_id), job_id).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(source)
    }

    async fn generate_and_store(
        &self,
        source: &CopySource,
    ) -> Result<(), DrugInspectionCopyServiceError> {
        let stamp_key =
            source
                .stamp_storage_key
                .as_ref()
                .ok_or(DrugInspectionCopyServiceError::Conflict(
                    "published_stamp_missing",
                ))?;
        let original = tokio::fs::read(self.storage_root.join(&source.original_storage_key))
            .await
            .map_err(|error| DrugInspectionCopyServiceError::Storage(error.to_string()))?;
        let stamp = tokio::fs::read(self.storage_root.join(stamp_key))
            .await
            .map_err(|error| DrugInspectionCopyServiceError::Storage(error.to_string()))?;
        let content_type = source.original_content_type.clone();
        let digitally_signed_original = content_type == "application/pdf"
            && crate::drug_inspection_copy_processor::pdf_has_digital_signature_markers(&original);
        // 在副本生成前写入签名标记，失败任务也能投影出客户提示。
        {
            let now = Utc::now();
            sqlx::query(
                r#"
                UPDATE drug_inspection_report_versions
                   SET digitally_signed_original = $3, updated_at = $4
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(source.owner_id)
            .bind(source.report_version_id)
            .bind(digitally_signed_original)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;
        }
        let processing_mode = source.processing_mode.clone();
        let placement = StampPlacement {
            relative_x: source.relative_x.unwrap_or(0.7),
            relative_y: source.relative_y.unwrap_or(0.75),
            relative_width: source.relative_width.unwrap_or(0.2),
        };
        let bytes = tokio::task::spawn_blocking(move || {
            generate_customer_pdf(
                &original,
                &content_type,
                &processing_mode,
                &stamp,
                placement,
            )
        })
        .await
        .map_err(|error| DrugInspectionCopyServiceError::Storage(error.to_string()))?
        .map_err(DrugInspectionCopyServiceError::Processing)?;
        let attachment_id = Uuid::new_v4();
        let storage_key = format!(
            "{}/M-DI/customer-copy/{}/{}.pdf",
            source.owner_id, source.report_version_id, attachment_id
        );
        let path = self.storage_root.join(&storage_key);
        let parent = path
            .parent()
            .ok_or_else(|| DrugInspectionCopyServiceError::Storage("invalid path".to_string()))?;
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| DrugInspectionCopyServiceError::Storage(error.to_string()))?;
        let temporary = path.with_extension("pdf.part");
        tokio::fs::write(&temporary, &bytes)
            .await
            .map_err(|error| DrugInspectionCopyServiceError::Storage(error.to_string()))?;
        tokio::fs::rename(&temporary, &path)
            .await
            .map_err(|error| DrugInspectionCopyServiceError::Storage(error.to_string()))?;
        let hash = hex::encode(Sha256::digest(&bytes));
        let size = i64::try_from(bytes.len())
            .map_err(|error| DrugInspectionCopyServiceError::Storage(error.to_string()))?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        sqlx::query(
            r#"
            INSERT INTO attachments (
                id, owner_id, module, entity_type, entity_id, file_name,
                content_type, size_bytes, storage_key, sha256, uploaded_by, created_at
            )
            VALUES ($1, $2, 'M-DI', 'drug_inspection_customer_copy', $3, $4,
                    'application/pdf', $5, $6, $7, $8, $9)
            "#,
        )
        .bind(attachment_id)
        .bind(source.owner_id)
        .bind(source.report_version_id)
        .bind(format!(
            "{}-v{}-客户分发副本.pdf",
            source.report_no, source.version_number
        ))
        .bind(size)
        .bind(&storage_key)
        .bind(&hash)
        .bind(source.actor_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if bytes.len() > MDI_COPY_SOFT_LIMIT_BYTES {
            sqlx::query(
                r#"
                UPDATE drug_inspection_customer_copy_jobs
                   SET status = 'oversize_review', candidate_file_id = $3,
                       candidate_hash = $4, candidate_size = $5,
                       finished_at = $6, updated_at = $6
                 WHERE owner_id = $1 AND id = $2 AND status = 'processing'
                "#,
            )
            .bind(source.owner_id)
            .bind(source.job_id)
            .bind(attachment_id)
            .bind(&hash)
            .bind(size)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        } else {
            sqlx::query(
                r#"
                UPDATE drug_inspection_report_versions
                   SET customer_copy_status = 'available',
                       customer_copy_file_id = $3,
                       customer_copy_hash = $4,
                       updated_at = $5
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(source.owner_id)
            .bind(source.report_version_id)
            .bind(attachment_id)
            .bind(&hash)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                "UPDATE drug_inspection_customer_copy_jobs SET status = 'succeeded', finished_at = $3, updated_at = $3 WHERE owner_id = $1 AND id = $2 AND status = 'processing'",
            )
            .bind(source.owner_id)
            .bind(source.job_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let ctx = background_context(source);
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            if bytes.len() > MDI_COPY_SOFT_LIMIT_BYTES {
                "di.customer_copy.oversize_review_requested"
            } else {
                "di.customer_copy.generated"
            },
            "DI",
            "drug_inspection_customer_copy_job",
            source.job_id.to_string(),
            Some(AuditDiff::compute(
                json!({ "status": "processing" }),
                json!({
                    "status": if bytes.len() > MDI_COPY_SOFT_LIMIT_BYTES {
                        "oversize_review"
                    } else {
                        "succeeded"
                    },
                    "customer_copy_file_id": attachment_id,
                    "size_bytes": size,
                    "sha256": hash,
                }),
            )),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| DrugInspectionCopyServiceError::Audit(format!("{error:?}")))?;
        publish_report_projection_from_db(&mut tx, source.owner_id, source.report_version_id, now)
            .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(())
    }

    async fn mark_failed(
        &self,
        source: &CopySource,
        error: &DrugInspectionCopyServiceError,
    ) -> Result<(), DrugInspectionCopyServiceError> {
        let now = Utc::now();
        let message = format!("{error:?}");
        let message = message.chars().take(1000).collect::<String>();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        sqlx::query(
            "UPDATE drug_inspection_customer_copy_jobs SET status = 'failed', last_error = $3, finished_at = $4, updated_at = $4 WHERE owner_id = $1 AND id = $2",
        )
        .bind(source.owner_id)
        .bind(source.job_id)
        .bind(&message)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            "UPDATE drug_inspection_report_versions SET customer_copy_status = 'failed', updated_at = $3 WHERE owner_id = $1 AND id = $2",
        )
        .bind(source.owner_id)
        .bind(source.report_version_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let ctx = background_context(source);
        let audit = AuditWriteRequest::from_auth_context(
            &ctx,
            "di.customer_copy.failed",
            "DI",
            "drug_inspection_customer_copy_job",
            source.job_id.to_string(),
            Some(AuditDiff::compute(
                json!({ "status": "processing" }),
                json!({ "status": "failed", "error": message }),
            )),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|audit_error| {
                DrugInspectionCopyServiceError::Audit(format!("{audit_error:?}"))
            })?;
        publish_report_projection_from_db(&mut tx, source.owner_id, source.report_version_id, now)
            .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(())
    }

    async fn get_job(
        &self,
        job_id: Uuid,
    ) -> Result<DrugInspectionCustomerCopyJob, DrugInspectionCopyServiceError> {
        sqlx::query_as::<_, CopyJobRow>(
            "SELECT id, owner_id, report_version_id, status, attempt_count, last_error, created_at, started_at, finished_at, updated_at FROM drug_inspection_customer_copy_jobs WHERE id = $1",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .map(Into::into)
        .ok_or(DrugInspectionCopyServiceError::NotFound)
    }
}

pub fn spawn_drug_inspection_copy_worker(pool: PgPool, storage_root: PathBuf) {
    tokio::spawn(async move {
        let service = DrugInspectionCopyService::new(pool, storage_root);
        loop {
            match service.process_next().await {
                Ok(Some(_)) => continue,
                Ok(None) | Err(_) => tokio::time::sleep(std::time::Duration::from_secs(2)).await,
            }
        }
    });
}

#[derive(Clone, FromRow)]
struct CopySource {
    job_id: Uuid,
    owner_id: Uuid,
    report_version_id: Uuid,
    report_no: String,
    version_number: i32,
    original_storage_key: String,
    original_content_type: String,
    processing_mode: String,
    actor_id: Uuid,
    stamp_storage_key: Option<String>,
    relative_x: Option<f64>,
    relative_y: Option<f64>,
    relative_width: Option<f64>,
}

#[derive(FromRow)]
struct CopyJobRow {
    id: Uuid,
    owner_id: Uuid,
    report_version_id: Uuid,
    status: String,
    attempt_count: i32,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl From<CopyJobRow> for DrugInspectionCustomerCopyJob {
    fn from(row: CopyJobRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            report_version_id: row.report_version_id,
            status: row.status,
            attempt_count: row.attempt_count,
            last_error: row.last_error,
            created_at: row.created_at,
            started_at: row.started_at,
            finished_at: row.finished_at,
            updated_at: row.updated_at,
        }
    }
}

async fn claim_source(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    owner_id: Option<Uuid>,
    job_id: Uuid,
) -> Result<CopySource, DrugInspectionCopyServiceError> {
    let source = sqlx::query_as::<_, CopySource>(
        r#"
        SELECT job.id AS job_id, job.owner_id, job.report_version_id,
               version.report_no, version.version_number,
               original.storage_key AS original_storage_key,
               original.content_type AS original_content_type,
               version.processing_mode,
               COALESCE(version.reviewed_by, version.uploaded_by) AS actor_id,
               stamp_attachment.storage_key AS stamp_storage_key,
               stamp.relative_x, stamp.relative_y, stamp.relative_width
          FROM drug_inspection_customer_copy_jobs AS job
          JOIN drug_inspection_report_versions AS version
            ON version.id = job.report_version_id
           AND version.owner_id = job.owner_id
          JOIN attachments AS original
            ON original.id = version.original_file_id
           AND original.owner_id = version.owner_id
     LEFT JOIN drug_inspection_stamp_versions AS stamp
            ON stamp.id = version.stamp_version_id
           AND stamp.owner_id = version.owner_id
     LEFT JOIN attachments AS stamp_attachment
            ON stamp_attachment.id = stamp.png_attachment_id
           AND stamp_attachment.owner_id = stamp.owner_id
         WHERE job.id = $1
           AND ($2::UUID IS NULL OR job.owner_id = $2)
           AND (
               (job.status IN ('queued', 'failed') AND job.attempt_count < 3)
               OR (
                   job.status = 'processing'
                   AND job.started_at < now() - interval '10 minutes'
               )
           )
         FOR UPDATE OF job
        "#,
    )
    .bind(job_id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(DrugInspectionCopyServiceError::NotFound)?;
    let now = Utc::now();
    sqlx::query(
        "UPDATE drug_inspection_customer_copy_jobs
            SET status = 'processing', attempt_count = LEAST(attempt_count + 1, 3),
                started_at = $2, finished_at = NULL, last_error = NULL, updated_at = $2
          WHERE id = $1",
    )
    .bind(job_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    sqlx::query(
        "UPDATE drug_inspection_report_versions
         SET customer_copy_status = CASE
                 WHEN customer_copy_file_id IS NULL THEN 'processing'
                 ELSE customer_copy_status
             END,
             updated_at = $2
         WHERE id = $1",
    )
    .bind(source.report_version_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(source)
}

fn background_context(source: &CopySource) -> AuthContext {
    AuthContext {
        user_id: source.actor_id,
        owner_id: source.owner_id,
        actor_name: "药检客户副本后台任务".to_string(),
        permissions: Vec::new(),
        jti: format!("di-copy-job:{}", source.job_id),
        warehouse_scope: None,
    }
}

fn map_db_error(error: sqlx::Error) -> DrugInspectionCopyServiceError {
    DrugInspectionCopyServiceError::Database(error.to_string())
}

fn map_repository_error(
    error: DrugInspectionDocumentRepositoryError,
) -> DrugInspectionCopyServiceError {
    match error {
        DrugInspectionDocumentRepositoryError::IdempotencyConflict => {
            DrugInspectionCopyServiceError::IdempotencyConflict
        }
        DrugInspectionDocumentRepositoryError::Serialize(message) => {
            DrugInspectionCopyServiceError::Serialize(message)
        }
        DrugInspectionDocumentRepositoryError::Database(message) => {
            DrugInspectionCopyServiceError::Database(message)
        }
        other => DrugInspectionCopyServiceError::Database(format!("{other:?}")),
    }
}
