use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CreateDrugInspectionStampVersionRequest, DrugInspectionProcessingRuleVersion,
    DrugInspectionStampVersion, PublishDrugInspectionProcessingRuleRequest,
    ReviewDrugInspectionStampVersionRequest,
};

use crate::auth::AuthContext;

use super::{
    helpers::{lock_idempotency_key, replay_idempotency, request_hash, store_idempotency},
    map_db_error,
    report_audit::append_audit,
    DrugInspectionDocumentRepositoryError,
};

#[derive(Clone)]
pub struct PgDrugInspectionStampRepository {
    pool: PgPool,
}

impl PgDrugInspectionStampRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_version(
        &self,
        ctx: &AuthContext,
        request: CreateDrugInspectionStampVersionRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionStampVersion, DrugInspectionDocumentRepositoryError> {
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
        let attachment: Option<(bool, bool, bool)> = sqlx::query_as(
            "SELECT content_type = 'image/png', module = 'M-DI',
                    entity_type = 'drug_inspection_stamp'
             FROM attachments WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(request.png_attachment_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        match attachment {
            Some((true, true, true)) => {}
            Some((false, _, _)) | None => {
                return Err(DrugInspectionDocumentRepositoryError::Conflict(
                    "stamp_must_be_png",
                ));
            }
            // 非图章上传通道的 PNG 未经透明度校验，配置成图章会让副本任务批量失败。
            Some((true, _, _)) => {
                return Err(DrugInspectionDocumentRepositoryError::Conflict(
                    "stamp_attachment_entity_mismatch",
                ));
            }
        }
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), 4212)")
            .bind(ctx.owner_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        let version_number: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM drug_inspection_stamp_versions WHERE owner_id = $1",
        )
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value = sqlx::query_as::<_, StampVersionRow>(
            r#"
            INSERT INTO drug_inspection_stamp_versions (
                id, owner_id, version_number, png_attachment_id,
                relative_x, relative_y, relative_width, status,
                configured_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'draft', $8, $9, $9)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(version_number)
        .bind(request.png_attachment_id)
        .bind(request.relative_x)
        .bind(request.relative_y)
        .bind(request.relative_width)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map(Into::into)
        .map_err(map_db_error)?;
        finish_stamp_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/drug-inspection/stamp-versions",
            "di.stamp.created",
            &value,
            Value::Null,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn submit_version(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        idempotency_key: &str,
    ) -> Result<DrugInspectionStampVersion, DrugInspectionDocumentRepositoryError> {
        let hash = request_hash(&(version_id, "submit"))?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = stamp_for_update(&mut tx, ctx.owner_id, version_id)
            .await?
            .ok_or(DrugInspectionDocumentRepositoryError::NotFound)?;
        if before.status != "draft" || before.configured_by != ctx.user_id {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "stamp_not_editable",
            ));
        }
        let value = sqlx::query_as::<_, StampVersionRow>(
            r#"
            UPDATE drug_inspection_stamp_versions
               SET status = 'pending_review', submitted_at = $3,
                   reviewed_by = NULL, reviewed_at = NULL,
                   review_comment = NULL, updated_at = $3
             WHERE owner_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(ctx.owner_id)
        .bind(version_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map(Into::into)
        .map_err(map_db_error)?;
        finish_stamp_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            &format!("/api/v1/drug-inspection/stamp-versions/{version_id}/submit"),
            "di.stamp.submitted",
            &value,
            stamp_snapshot(&before),
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn review_version(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        request: ReviewDrugInspectionStampVersionRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionStampVersion, DrugInspectionDocumentRepositoryError> {
        request
            .validate()
            .map_err(DrugInspectionDocumentRepositoryError::Invalid)?;
        let hash = request_hash(&(version_id, &request))?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let before = stamp_for_update(&mut tx, ctx.owner_id, version_id)
            .await?
            .ok_or(DrugInspectionDocumentRepositoryError::NotFound)?;
        if before.status != "pending_review" {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "stamp_not_pending",
            ));
        }
        if before.configured_by == ctx.user_id {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "reviewer_is_configurer",
            ));
        }
        let published = request.decision.trim() == "published";
        if published {
            sqlx::query(
                "UPDATE drug_inspection_stamp_versions SET status = 'superseded', updated_at = $2 WHERE owner_id = $1 AND status = 'published'",
            )
            .bind(ctx.owner_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            // 确认时尚无图章的版本在首个图章发布后固定到该版本，失败副本才具备重试条件。
            sqlx::query(
                "UPDATE drug_inspection_report_versions AS version
                    SET stamp_version_id = $2, updated_at = $3
                  WHERE version.owner_id = $1
                    AND version.stamp_version_id IS NULL
                    AND EXISTS (
                        SELECT 1
                          FROM drug_inspection_customer_copy_jobs AS job
                         WHERE job.owner_id = version.owner_id
                           AND job.report_version_id = version.id
                           AND job.status = 'failed'
                           AND job.last_error LIKE '%published_stamp_missing%'
                    )",
            )
            .bind(ctx.owner_id)
            .bind(version_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            // 仅重排队因缺已发布图章失败的任务；其它失败原因（坏图、超限等）保持 failed，避免误重跑。
            sqlx::query(
                "UPDATE drug_inspection_customer_copy_jobs AS job
                    SET status = 'queued', attempt_count = 0, last_error = NULL, updated_at = $2
                  WHERE job.owner_id = $1
                    AND job.status = 'failed'
                    AND job.last_error LIKE '%published_stamp_missing%'
                    AND EXISTS (
                        SELECT 1
                          FROM drug_inspection_report_versions AS version
                         WHERE version.owner_id = job.owner_id
                           AND version.id = job.report_version_id
                           AND version.stamp_version_id = $3
                    )",
            )
            .bind(ctx.owner_id)
            .bind(now)
            .bind(version_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let value = sqlx::query_as::<_, StampVersionRow>(
            r#"
            UPDATE drug_inspection_stamp_versions
               SET status = $3, reviewed_by = $4, reviewed_at = $5,
                   review_comment = $6, updated_at = $5
             WHERE owner_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(ctx.owner_id)
        .bind(version_id)
        .bind(if published { "published" } else { "draft" })
        .bind(ctx.user_id)
        .bind(now)
        .bind(request.comment.as_deref().map(str::trim))
        .fetch_one(&mut *tx)
        .await
        .map(Into::into)
        .map_err(map_db_error)?;
        finish_stamp_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            &format!("/api/v1/drug-inspection/stamp-versions/{version_id}/review"),
            if published {
                "di.stamp.published"
            } else {
                "di.stamp.rejected"
            },
            &value,
            stamp_snapshot(&before),
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn list_versions(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<DrugInspectionStampVersion>, DrugInspectionDocumentRepositoryError> {
        sqlx::query_as::<_, StampVersionRow>(
            "SELECT * FROM drug_inspection_stamp_versions WHERE owner_id = $1 ORDER BY version_number DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_db_error)
    }

    pub async fn published_version(
        &self,
        owner_id: Uuid,
    ) -> Result<Option<DrugInspectionStampVersion>, DrugInspectionDocumentRepositoryError> {
        sqlx::query_as::<_, StampVersionRow>(
            "SELECT * FROM drug_inspection_stamp_versions WHERE owner_id = $1 AND status = 'published'",
        )
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map(|row| row.map(Into::into))
        .map_err(map_db_error)
    }

    pub async fn publish_processing_rule(
        &self,
        ctx: &AuthContext,
        request: PublishDrugInspectionProcessingRuleRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionProcessingRuleVersion, DrugInspectionDocumentRepositoryError> {
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
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), 4213)")
            .bind(ctx.owner_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        let version_number: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1
             FROM drug_inspection_processing_rule_versions
             WHERE owner_id = $1",
        )
        .bind(ctx.owner_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let id = Uuid::new_v4();
        let rule_code = format!("mdi-image-v{version_number}");
        let reprocess_job_count = if request.apply_scope.trim() == "reprocess_current" {
            let affected = sqlx::query(
                r#"
                INSERT INTO drug_inspection_customer_copy_jobs (
                    id, owner_id, report_version_id, status,
                    processing_rule, created_at, updated_at
                )
                SELECT gen_random_uuid(), version.owner_id, version.id, 'queued',
                       $2, $3, $3
                  FROM drug_inspection_reports AS report
                  JOIN drug_inspection_report_versions AS version
                    ON version.id = report.current_version_id
                   AND version.owner_id = report.owner_id
                 WHERE report.owner_id = $1
                   AND version.status = 'confirmed'
                   AND version.customer_copy_file_id IS NOT NULL
                   AND NOT EXISTS (
                       SELECT 1
                         FROM drug_inspection_customer_copy_jobs AS active
                        WHERE active.owner_id = version.owner_id
                          AND active.report_version_id = version.id
                          AND active.status IN ('queued', 'processing', 'oversize_review')
                   )
                "#,
            )
            .bind(ctx.owner_id)
            .bind(&rule_code)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?
            .rows_affected();
            i32::try_from(affected).map_err(|_| {
                DrugInspectionDocumentRepositoryError::Conflict("reprocess_job_count_overflow")
            })?
        } else {
            0
        };
        let value: DrugInspectionProcessingRuleVersion =
            sqlx::query_as::<_, ProcessingRuleVersionRow>(
                "INSERT INTO drug_inspection_processing_rule_versions (
                    id, owner_id, version_number, rule_code, apply_scope,
                    reprocess_job_count, published_by, published_at
                 )
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 RETURNING *",
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(version_number)
            .bind(&rule_code)
            .bind(request.apply_scope.trim())
            .bind(reprocess_job_count)
            .bind(ctx.user_id)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map(Into::into)
            .map_err(map_db_error)?;
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/drug-inspection/processing-rule-versions",
            "drug_inspection_processing_rule_version",
            value.id,
            &value,
            now,
        )
        .await?;
        append_audit(
            &mut tx,
            ctx,
            "di.processing_rule.published",
            "drug_inspection_processing_rule_version",
            value.id,
            Value::Null,
            processing_rule_snapshot(&value),
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn list_processing_rules(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<DrugInspectionProcessingRuleVersion>, DrugInspectionDocumentRepositoryError>
    {
        sqlx::query_as::<_, ProcessingRuleVersionRow>(
            "SELECT * FROM drug_inspection_processing_rule_versions
             WHERE owner_id = $1
             ORDER BY version_number DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(Into::into).collect())
        .map_err(map_db_error)
    }
}

#[derive(Clone, FromRow)]
struct StampVersionRow {
    id: Uuid,
    owner_id: Uuid,
    version_number: i32,
    png_attachment_id: Uuid,
    relative_x: f64,
    relative_y: f64,
    relative_width: f64,
    status: String,
    configured_by: Uuid,
    submitted_at: Option<DateTime<Utc>>,
    reviewed_by: Option<Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    review_comment: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<StampVersionRow> for DrugInspectionStampVersion {
    fn from(row: StampVersionRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            version_number: row.version_number,
            png_attachment_id: row.png_attachment_id,
            relative_x: row.relative_x,
            relative_y: row.relative_y,
            relative_width: row.relative_width,
            status: row.status,
            configured_by: row.configured_by,
            submitted_at: row.submitted_at,
            reviewed_by: row.reviewed_by,
            reviewed_at: row.reviewed_at,
            review_comment: row.review_comment,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Clone, FromRow)]
struct ProcessingRuleVersionRow {
    id: Uuid,
    owner_id: Uuid,
    version_number: i32,
    rule_code: String,
    apply_scope: String,
    reprocess_job_count: i32,
    published_by: Uuid,
    published_at: DateTime<Utc>,
}

impl From<ProcessingRuleVersionRow> for DrugInspectionProcessingRuleVersion {
    fn from(row: ProcessingRuleVersionRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            version_number: row.version_number,
            rule_code: row.rule_code,
            apply_scope: row.apply_scope,
            reprocess_job_count: row.reprocess_job_count,
            published_by: row.published_by,
            published_at: row.published_at,
        }
    }
}

async fn stamp_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<Option<DrugInspectionStampVersion>, DrugInspectionDocumentRepositoryError> {
    sqlx::query_as::<_, StampVersionRow>(
        "SELECT * FROM drug_inspection_stamp_versions WHERE owner_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(owner_id)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map(|row| row.map(Into::into))
    .map_err(map_db_error)
}

fn stamp_snapshot(value: &DrugInspectionStampVersion) -> Value {
    json!({
        "id": value.id,
        "version_number": value.version_number,
        "png_attachment_id": value.png_attachment_id,
        "relative_x": value.relative_x,
        "relative_y": value.relative_y,
        "relative_width": value.relative_width,
        "status": value.status,
        "configured_by": value.configured_by,
        "reviewed_by": value.reviewed_by,
    })
}

fn processing_rule_snapshot(value: &DrugInspectionProcessingRuleVersion) -> Value {
    json!({
        "id": value.id,
        "version_number": value.version_number,
        "rule_code": value.rule_code,
        "apply_scope": value.apply_scope,
        "reprocess_job_count": value.reprocess_job_count,
        "published_by": value.published_by,
        "published_at": value.published_at,
    })
}

#[allow(clippy::too_many_arguments)]
async fn finish_stamp_mutation(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    hash: &str,
    method: &str,
    path: &str,
    action: &str,
    value: &DrugInspectionStampVersion,
    before: Value,
    now: DateTime<Utc>,
) -> Result<(), DrugInspectionDocumentRepositoryError> {
    store_idempotency(
        tx,
        ctx.owner_id,
        idempotency_key,
        hash,
        method,
        path,
        "drug_inspection_stamp_version",
        value.id,
        value,
        now,
    )
    .await?;
    append_audit(
        tx,
        ctx,
        action,
        "drug_inspection_stamp_version",
        value.id,
        before,
        stamp_snapshot(value),
        now,
    )
    .await
}
