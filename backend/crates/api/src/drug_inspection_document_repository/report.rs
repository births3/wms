use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;
use wms_domain::{
    CreateDrugInspectionCorrectionRequest, CreateDrugInspectionVersionRequest,
    DrugInspectionReportVersion, ReusableDrugInspectionReportResponse,
    ReuseDrugInspectionReportRequest, ReuseDrugInspectionReportResponse,
    ReviewDrugInspectionVersionRequest, UpdateDrugInspectionDraftRequest,
};

use crate::operation_context::OperationContext as AuthContext;

use super::{
    helpers::{lock_idempotency_key, replay_idempotency, request_hash, store_idempotency},
    map_db_error,
    report_audit::{append_audit, version_snapshot},
    report_helpers::{
        attachment_hash, fetch_version_for_update, finish_mutation, insert_version, report_by_key,
        validate_asn_batch, DrugInspectionVersionRow,
    },
    DrugInspectionDocumentRepositoryError, PgDrugInspectionDocumentRepository,
};

impl PgDrugInspectionDocumentRepository {
    pub async fn find_editable_version(
        &self,
        owner_id: Uuid,
        uploader_id: Uuid,
        asn_id: Uuid,
        product_id: Uuid,
        batch_no: &str,
    ) -> Result<DrugInspectionReportVersion, DrugInspectionDocumentRepositoryError> {
        sqlx::query_as::<_, DrugInspectionVersionRow>(
            r#"
            SELECT version.*
              FROM drug_inspection_reports AS report
              JOIN drug_inspection_report_versions AS version
                ON version.report_id = report.id
               AND version.status = 'draft'
               AND version.uploaded_by = $2
              JOIN drug_inspection_asn_links AS link
                ON link.owner_id = report.owner_id
               AND link.report_id = report.id
               AND link.asn_id = $3
               AND link.batch_no = report.batch_no
             WHERE report.owner_id = $1
               AND report.product_id = $4
               AND report.batch_no = $5
             ORDER BY version.version_number DESC
             LIMIT 1
            "#,
        )
        .bind(owner_id)
        .bind(uploader_id)
        .bind(asn_id)
        .bind(product_id)
        .bind(batch_no.trim())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .map(Into::into)
        .ok_or(DrugInspectionDocumentRepositoryError::NotFound)
    }

    pub async fn find_reusable_report(
        &self,
        owner_id: Uuid,
        product_id: Uuid,
        batch_no: &str,
        asn_id: Option<Uuid>,
    ) -> Result<ReusableDrugInspectionReportResponse, DrugInspectionDocumentRepositoryError> {
        sqlx::query_as::<_, (Uuid, Uuid, i32, String, bool)>(
            r#"
            SELECT report.id, version.id, version.version_number, version.report_no,
                   CASE WHEN $4::UUID IS NULL THEN FALSE ELSE EXISTS (
                       SELECT 1
                         FROM drug_inspection_asn_links AS link
                        WHERE link.owner_id = report.owner_id
                          AND link.report_id = report.id
                          AND link.asn_id = $4
                          AND link.batch_no = report.batch_no
                   ) END
              FROM drug_inspection_reports AS report
              JOIN drug_inspection_report_versions AS version
                ON version.id = report.current_version_id
               AND version.status = 'confirmed'
             WHERE report.owner_id = $1
               AND report.product_id = $2
               AND report.batch_no = $3
            "#,
        )
        .bind(owner_id)
        .bind(product_id)
        .bind(batch_no.trim())
        .bind(asn_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .map(
            |(report_id, current_version_id, version_number, report_no, linked_to_asn)| {
                ReusableDrugInspectionReportResponse {
                    report_id,
                    current_version_id,
                    version_number,
                    report_no,
                    linked_to_asn,
                }
            },
        )
        .ok_or(DrugInspectionDocumentRepositoryError::NotFound)
    }

    pub async fn create_version(
        &self,
        ctx: &AuthContext,
        request: CreateDrugInspectionVersionRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionReportVersion, DrugInspectionDocumentRepositoryError> {
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
        validate_asn_batch(
            &mut tx,
            ctx.owner_id,
            request.asn_id,
            request.product_id,
            request.batch_no.trim(),
        )
        .await?;
        let original_file_hash =
            attachment_hash(&mut tx, ctx.owner_id, request.original_file_id).await?;
        if report_by_key(
            &mut tx,
            ctx.owner_id,
            request.product_id,
            request.batch_no.trim(),
        )
        .await?
        .is_some()
        {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "report_exists",
            ));
        }

        let report_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO drug_inspection_reports (
                id, owner_id, product_id, batch_no, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            "#,
        )
        .bind(report_id)
        .bind(ctx.owner_id)
        .bind(request.product_id)
        .bind(request.batch_no.trim())
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value = insert_version(
            &mut tx,
            ctx,
            report_id,
            1,
            request.report_no.trim(),
            request.original_file_id,
            &original_file_hash,
            request.source.trim(),
            request.processing_mode.trim(),
            request.qualified,
            None,
            None,
            now,
        )
        .await?;
        sqlx::query(
            r#"
            INSERT INTO drug_inspection_asn_links (
                id, owner_id, asn_id, batch_no, report_id, source_version_id,
                source, linked_by, linked_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'uploaded', $7, $8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(request.asn_id)
        .bind(request.batch_no.trim())
        .bind(report_id)
        .bind(value.id)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/drug-inspection/report-versions",
            "di.report_version.created",
            &value,
            Value::Null,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn update_draft_version(
        &self,
        ctx: &AuthContext,
        version_id: Uuid,
        request: UpdateDrugInspectionDraftRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionReportVersion, DrugInspectionDocumentRepositoryError> {
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
        let before = fetch_version_for_update(&mut tx, ctx.owner_id, version_id)
            .await?
            .ok_or(DrugInspectionDocumentRepositoryError::NotFound)?;
        if before.uploaded_by != ctx.user_id || before.status != "draft" {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "version_not_editable",
            ));
        }
        let original_file_hash =
            attachment_hash(&mut tx, ctx.owner_id, request.original_file_id).await?;
        let value = sqlx::query_as::<_, DrugInspectionVersionRow>(
            r#"
            UPDATE drug_inspection_report_versions
               SET report_no = $3, original_file_id = $4, original_file_hash = $5,
                   processing_mode = $6, qualified = $7, submitted_at = NULL,
                   reviewed_by = NULL, reviewed_at = NULL, review_result = NULL,
                   review_comment = NULL, updated_at = $8
             WHERE owner_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(ctx.owner_id)
        .bind(version_id)
        .bind(request.report_no.trim())
        .bind(request.original_file_id)
        .bind(&original_file_hash)
        .bind(request.processing_mode.trim())
        .bind(request.qualified)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map(Into::into)
        .map_err(map_db_error)?;
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "PUT",
            &format!("/api/v1/drug-inspection/report-versions/{version_id}"),
            "di.report_version.draft_updated",
            &value,
            version_snapshot(&before),
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
    ) -> Result<DrugInspectionReportVersion, DrugInspectionDocumentRepositoryError> {
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
        let before = fetch_version_for_update(&mut tx, ctx.owner_id, version_id)
            .await?
            .ok_or(DrugInspectionDocumentRepositoryError::NotFound)?;
        if before.uploaded_by != ctx.user_id || before.status != "draft" {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "version_not_editable",
            ));
        }
        let value = sqlx::query_as::<_, DrugInspectionVersionRow>(
            r#"
            UPDATE drug_inspection_report_versions
               SET status = 'pending_confirmation', submitted_at = $3,
                   review_result = NULL, review_comment = NULL,
                   reviewed_by = NULL, reviewed_at = NULL, updated_at = $3
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
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            &format!("/api/v1/drug-inspection/report-versions/{version_id}/submit"),
            "di.report_version.submitted",
            &value,
            version_snapshot(&before),
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
        request: ReviewDrugInspectionVersionRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionReportVersion, DrugInspectionDocumentRepositoryError> {
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
        let before = fetch_version_for_update(&mut tx, ctx.owner_id, version_id)
            .await?
            .ok_or(DrugInspectionDocumentRepositoryError::NotFound)?;
        if before.status != "pending_confirmation" {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "version_not_pending",
            ));
        }
        if before.uploaded_by == ctx.user_id {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "reviewer_is_uploader",
            ));
        }
        let decision = request.decision.trim();
        let stamp_version_id: Option<Uuid> = if decision == "confirmed" {
            sqlx::query_scalar(
                "SELECT id FROM drug_inspection_stamp_versions WHERE owner_id = $1 AND status = 'published'",
            )
            .bind(ctx.owner_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?
        } else {
            None
        };
        if decision == "confirmed" {
            sqlx::query(
                r#"
                UPDATE drug_inspection_report_versions
                   SET status = 'superseded', updated_at = $3
                 WHERE owner_id = $1 AND report_id = $2 AND status = 'confirmed'
                "#,
            )
            .bind(ctx.owner_id)
            .bind(before.report_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        let next_status = if decision == "confirmed" {
            "confirmed"
        } else {
            "draft"
        };
        let value = sqlx::query_as::<_, DrugInspectionVersionRow>(
            r#"
            UPDATE drug_inspection_report_versions
               SET status = $3, reviewed_by = $4, reviewed_at = $5,
                   review_result = $6, review_comment = $7, updated_at = $5,
                   customer_copy_status = CASE
                       WHEN $6 = 'confirmed' THEN 'queued'
                       ELSE customer_copy_status
                   END,
                   stamp_version_id = CASE
                       WHEN $6 = 'confirmed' THEN $8
                       ELSE stamp_version_id
                   END
             WHERE owner_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(ctx.owner_id)
        .bind(version_id)
        .bind(next_status)
        .bind(ctx.user_id)
        .bind(now)
        .bind(decision)
        .bind(request.comment.as_deref().map(str::trim))
        .bind(stamp_version_id)
        .fetch_one(&mut *tx)
        .await
        .map(Into::into)
        .map_err(map_db_error)?;
        if decision == "confirmed" {
            sqlx::query(
                "UPDATE drug_inspection_reports SET current_version_id = $3, updated_at = $4 WHERE owner_id = $1 AND id = $2",
            )
            .bind(ctx.owner_id)
            .bind(before.report_id)
            .bind(version_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                r#"
                INSERT INTO drug_inspection_customer_copy_jobs (
                    id, owner_id, report_version_id, status, processing_rule,
                    created_at, updated_at
                )
                VALUES (
                    $1, $2, $3, 'queued',
                    COALESCE(
                        (
                            SELECT rule_code
                              FROM drug_inspection_processing_rule_versions
                             WHERE owner_id = $2
                             ORDER BY version_number DESC
                             LIMIT 1
                        ),
                        'mdi-image-v1'
                    ),
                    $4, $4
                )
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(version_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            &format!("/api/v1/drug-inspection/report-versions/{version_id}/review"),
            if decision == "confirmed" {
                "di.report_version.confirmed"
            } else {
                "di.report_version.rejected"
            },
            &value,
            version_snapshot(&before),
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn create_correction(
        &self,
        ctx: &AuthContext,
        report_id: Uuid,
        request: CreateDrugInspectionCorrectionRequest,
        idempotency_key: &str,
    ) -> Result<DrugInspectionReportVersion, DrugInspectionDocumentRepositoryError> {
        request
            .validate()
            .map_err(DrugInspectionDocumentRepositoryError::Invalid)?;
        let hash = request_hash(&(report_id, &request))?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let current = sqlx::query_as::<_, DrugInspectionVersionRow>(
            r#"
            SELECT version.*
              FROM drug_inspection_reports AS report
              JOIN drug_inspection_report_versions AS version
                ON version.id = report.current_version_id
             WHERE report.owner_id = $1 AND report.id = $2
             FOR UPDATE OF report, version
            "#,
        )
        .bind(ctx.owner_id)
        .bind(report_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(DrugInspectionDocumentRepositoryError::NotFound)?;
        if current.status != "confirmed" {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "report_not_confirmed",
            ));
        }
        let original_file_hash =
            attachment_hash(&mut tx, ctx.owner_id, request.original_file_id).await?;
        let value = insert_version(
            &mut tx,
            ctx,
            report_id,
            current.version_number + 1,
            request.report_no.trim(),
            request.original_file_id,
            &original_file_hash,
            "manual_upload",
            request.processing_mode.trim(),
            request.qualified,
            Some(current.id),
            Some(request.modification_reason.trim()),
            now,
        )
        .await?;
        finish_mutation(
            &mut tx,
            ctx,
            idempotency_key,
            &hash,
            "POST",
            &format!("/api/v1/drug-inspection/reports/{report_id}/corrections"),
            "di.report_version.correction_created",
            &value,
            version_snapshot(&current.clone().into()),
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn reuse_report(
        &self,
        ctx: &AuthContext,
        report_id: Uuid,
        request: ReuseDrugInspectionReportRequest,
        idempotency_key: &str,
    ) -> Result<ReuseDrugInspectionReportResponse, DrugInspectionDocumentRepositoryError> {
        let hash = request_hash(&(report_id, &request))?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let report: (Uuid, String, Uuid) = sqlx::query_as(
            r#"
            SELECT report.product_id, report.batch_no, version.id
              FROM drug_inspection_reports AS report
              JOIN drug_inspection_report_versions AS version
                ON version.id = report.current_version_id
               AND version.status = 'confirmed'
             WHERE report.owner_id = $1 AND report.id = $2
             FOR UPDATE OF report
            "#,
        )
        .bind(ctx.owner_id)
        .bind(report_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(DrugInspectionDocumentRepositoryError::NotFound)?;
        if report.1 != request.batch_no.trim() {
            return Err(DrugInspectionDocumentRepositoryError::Conflict(
                "batch_mismatch",
            ));
        }
        validate_asn_batch(
            &mut tx,
            ctx.owner_id,
            request.asn_id,
            report.0,
            request.batch_no.trim(),
        )
        .await?;
        sqlx::query(
            r#"
            INSERT INTO drug_inspection_asn_links (
                id, owner_id, asn_id, batch_no, report_id, source_version_id,
                source, linked_by, linked_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'reused', $7, $8)
            ON CONFLICT (owner_id, asn_id, batch_no) DO UPDATE
               SET report_id = EXCLUDED.report_id,
                   source_version_id = EXCLUDED.source_version_id,
                   source = EXCLUDED.source,
                   linked_by = EXCLUDED.linked_by,
                   linked_at = EXCLUDED.linked_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(request.asn_id)
        .bind(request.batch_no.trim())
        .bind(report_id)
        .bind(report.2)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let value = ReuseDrugInspectionReportResponse {
            report_id,
            asn_id: request.asn_id,
            batch_no: request.batch_no.trim().to_string(),
            source_version_id: report.2,
            linked_at: now,
        };
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &format!("/api/v1/drug-inspection/reports/{report_id}/reuse"),
            "drug_inspection_asn_link",
            report_id,
            &value,
            now,
        )
        .await?;
        append_audit(
            &mut tx,
            ctx,
            "di.report.reused",
            "drug_inspection_report",
            report_id,
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
}
