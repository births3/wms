//! US-H9-009 category PDF preparation through the shared H-FILE port.

use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use wms_domain::{
    CategoryPdfOutput, CategoryPdfOutputListResponse, CategoryPdfPreparation, PrintSuiteSourceMode,
};

use crate::{
    auth::AuthContext,
    file_attachment::{FileRetentionPolicy, StorePdfRequest},
    pdf_document::merge_pdfs,
};

use super::{
    category_pdf_repository::{
        ensure_instance, load_instance_item_sources, load_instance_source, load_outputs,
        InstanceSourceRow,
    },
    repository::map_db_error,
    IdempotentMutation, PrintOrchestrationError, PrintOrchestrationService,
};

use super::category_pdf_repository::PreparationRow;

impl PrintOrchestrationService {
    /// Runs the server-side Render Worker slice for one frozen suite instance.
    pub async fn prepare_category_pdfs(
        &self,
        ctx: &AuthContext,
        instance_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<CategoryPdfPreparation>, PrintOrchestrationError> {
        if instance_id.is_nil() || idempotency_key.trim().is_empty() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        let request_hash = hash_json(&json!({"instance_id": instance_id}))?;
        let (preparation_id, replayed) = self
            .start_preparation(ctx, instance_id, idempotency_key, &request_hash, now)
            .await?;
        if replayed {
            return Ok(IdempotentMutation {
                value: self
                    .load_preparation(ctx.owner_id, instance_id, preparation_id)
                    .await?,
                replayed: true,
            });
        }

        let pending = self
            .load_processable_outputs(ctx.owner_id, instance_id)
            .await?;
        let mut failure = None;
        let mut render_failure = None;
        for output in pending {
            if let Err(error) = self.process_output(ctx, &output, now).await {
                if let PrintOrchestrationError::RenderWorker(render_error) = &error {
                    render_failure.get_or_insert_with(|| render_error.clone());
                }
                let reason = format!("{error:?}");
                self.mark_output_failed(ctx.owner_id, output.id, &reason, now)
                    .await?;
                failure.get_or_insert(reason);
            }
        }
        if let Some(reason) = failure {
            let mut tx = self.repository.pool.begin().await.map_err(map_db_error)?;
            sqlx::query(
                r#"
                UPDATE h9_category_pdf_preparations
                   SET status = 'failed', last_error = $3, updated_at = $4
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(ctx.owner_id)
            .bind(preparation_id)
            .bind(&reason)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            self.audit_preparation_in_tx(
                &mut tx,
                ctx,
                "prepare_category_pdfs_failed",
                instance_id,
                preparation_id,
                Some(&reason),
                now,
            )
            .await?;
            tx.commit().await.map_err(map_db_error)?;
            if let Some(error) = render_failure {
                return Err(PrintOrchestrationError::RenderWorker(error));
            }
        } else {
            let mut tx = self.repository.pool.begin().await.map_err(map_db_error)?;
            sqlx::query(
                r#"
                UPDATE h9_category_pdf_preparations
                   SET status = 'completed', last_error = NULL,
                       updated_at = $3, completed_at = $3
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(ctx.owner_id)
            .bind(preparation_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                r#"
                UPDATE h9_print_suite_instances
                   SET status = 'queued', hold_scope = NULL
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(ctx.owner_id)
            .bind(instance_id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            self.audit_preparation_in_tx(
                &mut tx,
                ctx,
                "prepare_category_pdfs",
                instance_id,
                preparation_id,
                None,
                now,
            )
            .await?;
            tx.commit().await.map_err(map_db_error)?;
        }
        Ok(IdempotentMutation {
            value: self
                .load_preparation(ctx.owner_id, instance_id, preparation_id)
                .await?,
            replayed: false,
        })
    }

    /// Lists category PDF results without exposing an object-store URL.
    pub async fn list_category_pdfs(
        &self,
        ctx: &AuthContext,
        instance_id: Uuid,
    ) -> Result<CategoryPdfOutputListResponse, PrintOrchestrationError> {
        if instance_id.is_nil() {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        ensure_instance(&self.repository.pool, ctx.owner_id, instance_id).await?;
        let preparation = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT status, idempotency_key
              FROM h9_category_pdf_preparations
             WHERE owner_id = $1 AND instance_id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(instance_id)
        .fetch_optional(&self.repository.pool)
        .await
        .map_err(map_db_error)?;
        Ok(CategoryPdfOutputListResponse {
            data: load_outputs(&self.repository.pool, ctx.owner_id, instance_id).await?,
            preparation_status: preparation.as_ref().map(|row| row.0.clone()),
            retry_idempotency_key: preparation.map(|row| row.1),
        })
    }

    /// Returns one selected category or a temporary all/partial merge.
    pub async fn download_category_pdfs(
        &self,
        ctx: &AuthContext,
        instance_id: Uuid,
        category_pdf_ids: &[Uuid],
        emergency_print: bool,
        now: DateTime<Utc>,
    ) -> Result<Vec<u8>, PrintOrchestrationError> {
        if instance_id.is_nil()
            || category_pdf_ids.iter().any(Uuid::is_nil)
            || category_pdf_ids.iter().collect::<BTreeSet<_>>().len() != category_pdf_ids.len()
        {
            return Err(PrintOrchestrationError::InvalidRequest);
        }
        let mut outputs = load_outputs(&self.repository.pool, ctx.owner_id, instance_id).await?;
        if !category_pdf_ids.is_empty() {
            let selected = category_pdf_ids.iter().copied().collect::<BTreeSet<_>>();
            outputs.retain(|output| selected.contains(&output.id));
            if outputs.len() != selected.len() {
                return Err(PrintOrchestrationError::CategoryPdfNotFound);
            }
        }
        if outputs.is_empty()
            || outputs
                .iter()
                .any(|output| output.processing_status != "ready")
        {
            return Err(PrintOrchestrationError::CategoryPdfDocumentsNotReady);
        }
        let action = if emergency_print {
            "h_file.emergency_print"
        } else {
            "h_file.download"
        };
        let mut documents = Vec::with_capacity(outputs.len());
        for output in outputs {
            let attachment_id = output
                .attachment_id
                .ok_or(PrintOrchestrationError::CategoryPdfNotFound)?;
            documents.push(
                self.h_file
                    .read_and_audit(ctx, attachment_id, action, now)
                    .await
                    .map_err(PrintOrchestrationError::FileAttachment)?,
            );
        }
        merge_pdfs(&documents).map_err(PrintOrchestrationError::Serialize)
    }

    async fn start_preparation(
        &self,
        ctx: &AuthContext,
        instance_id: Uuid,
        idempotency_key: &str,
        request_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<(Uuid, bool), PrintOrchestrationError> {
        let mut tx = self.repository.pool.begin().await.map_err(map_db_error)?;
        sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtext('h9-category-pdf'), hashtext($1::text))",
        )
        .bind(instance_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let instance = load_instance_source(&mut tx, ctx.owner_id, instance_id).await?;
        let items = load_instance_item_sources(&mut tx, ctx.owner_id, instance_id).await?;
        if items.is_empty() || items.iter().any(|item| !item.ready) {
            return Err(PrintOrchestrationError::CategoryPdfDocumentsNotReady);
        }
        let existing = sqlx::query_as::<_, PreparationRow>(
            r#"
            SELECT id, idempotency_key, request_hash, status
              FROM h9_category_pdf_preparations
             WHERE owner_id = $1 AND instance_id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(instance_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if let Some(existing) = existing {
            if existing.idempotency_key != idempotency_key || existing.request_hash != request_hash
            {
                return Err(PrintOrchestrationError::IdempotencyConflict);
            }
            if existing.status == "completed" {
                tx.commit().await.map_err(map_db_error)?;
                return Ok((existing.id, true));
            }
            sqlx::query(
                r#"
                UPDATE h9_category_pdf_preparations
                   SET status = 'processing', last_error = NULL, updated_at = $3
                 WHERE owner_id = $1 AND id = $2
                "#,
            )
            .bind(ctx.owner_id)
            .bind(existing.id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                r#"
                UPDATE h9_category_pdf_outputs
                   SET processing_status = 'pending', failure_reason = NULL,
                       processed_at = NULL
                 WHERE owner_id = $1 AND preparation_id = $2
                   AND processing_status = 'failed'
                "#,
            )
            .bind(ctx.owner_id)
            .bind(existing.id)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            tx.commit().await.map_err(map_db_error)?;
            return Ok((existing.id, false));
        }

        let preparation_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO h9_category_pdf_preparations (
                id, owner_id, instance_id, idempotency_key, request_hash,
                status, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 'processing', $6, $7, $7)
            "#,
        )
        .bind(preparation_id)
        .bind(ctx.owner_id)
        .bind(instance_id)
        .bind(idempotency_key)
        .bind(request_hash)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let source_data_version = hash_json(&instance.source_documents)?;
        for item in items {
            let source_mode = parse_source_mode(&item.source_mode)?;
            let (data_version, bindings, retention_policy, cache_expires_at) = match source_mode {
                PrintSuiteSourceMode::Rendered => (
                    Some(source_data_version.clone()),
                    json!([]),
                    "gsp_5_year",
                    None,
                ),
                PrintSuiteSourceMode::ExternalFile => (
                    None,
                    item.file_bindings,
                    "short_cache",
                    Some(now + Duration::days(7)),
                ),
            };
            sqlx::query(
                r#"
                INSERT INTO h9_category_pdf_outputs (
                    id, owner_id, preparation_id, instance_id, instance_item_id,
                    category_code, source_mode, source_data_version,
                    source_file_bindings, template_version_id, processing_status,
                    retention_policy, cache_expires_at, created_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    'pending', $11, $12, $13
                )
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(preparation_id)
            .bind(instance_id)
            .bind(item.id)
            .bind(item.category_code)
            .bind(item.source_mode)
            .bind(data_version)
            .bind(bindings)
            .bind(item.template_version_id)
            .bind(retention_policy)
            .bind(cache_expires_at)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok((preparation_id, false))
    }

    async fn load_processable_outputs(
        &self,
        owner_id: Uuid,
        instance_id: Uuid,
    ) -> Result<Vec<CategoryPdfOutput>, PrintOrchestrationError> {
        let outputs = load_outputs(&self.repository.pool, owner_id, instance_id).await?;
        Ok(outputs
            .into_iter()
            .filter(|output| output.processing_status != "ready")
            .collect())
    }

    async fn process_output(
        &self,
        ctx: &AuthContext,
        output: &CategoryPdfOutput,
        now: DateTime<Utc>,
    ) -> Result<(), PrintOrchestrationError> {
        sqlx::query(
            r#"
            UPDATE h9_category_pdf_outputs
               SET processing_status = 'processing', attempt_count = attempt_count + 1
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(output.id)
        .execute(&self.repository.pool)
        .await
        .map_err(map_db_error)?;
        let (content, attachment_id, content_hash) = match output.source_mode {
            PrintSuiteSourceMode::Rendered => {
                let source = sqlx::query_as::<_, InstanceSourceRow>(
                    r#"
                    SELECT group_row.delivery_note_no, instance.source_documents
                      FROM h9_print_suite_instances instance
                      JOIN h9_delivery_note_groups group_row
                        ON group_row.owner_id = instance.owner_id
                       AND group_row.id = instance.group_id
                     WHERE instance.owner_id = $1 AND instance.id = $2
                    "#,
                )
                .bind(ctx.owner_id)
                .bind(output.instance_id)
                .fetch_one(&self.repository.pool)
                .await
                .map_err(map_db_error)?;
                let template_version_id = output
                    .template_version_id
                    .ok_or(PrintOrchestrationError::PrintSuiteBindingInvalid)?;
                let template: Value = sqlx::query_scalar(
                    r#"
                    SELECT hiprint_json
                      FROM print_template_versions
                     WHERE id = $1 AND status = 'published'
                    "#,
                )
                .bind(template_version_id)
                .fetch_optional(&self.repository.pool)
                .await
                .map_err(map_db_error)?
                .ok_or(PrintOrchestrationError::PrintSuiteBindingInvalid)?;
                let render_data = category_pdf_render_data(&source, output, template_version_id);
                let content = self
                    .category_pdf_renderer
                    .render(&template, &render_data)
                    .await
                    .map_err(PrintOrchestrationError::RenderWorker)?;
                let attachment = self
                    .h_file
                    .store_pdf(
                        ctx,
                        StorePdfRequest {
                            module: "H9".to_string(),
                            entity_type: "category_pdf_output".to_string(),
                            entity_id: output.id,
                            file_name: format!("{}-{}.pdf", output.category_code, output.id),
                            retention_policy: FileRetentionPolicy::GspFiveYear,
                        },
                        &content,
                        now,
                    )
                    .await
                    .map_err(PrintOrchestrationError::FileAttachment)?;
                (content, attachment.id, attachment.content_hash)
            }
            PrintSuiteSourceMode::ExternalFile => {
                let mut contents = Vec::with_capacity(output.source_file_bindings.len());
                for binding in &output.source_file_bindings {
                    let content = self
                        .h_file
                        .read_internal(ctx.owner_id, binding.file_id)
                        .await
                        .map_err(PrintOrchestrationError::FileAttachment)?;
                    let hash = hex::encode(Sha256::digest(&content));
                    if hash != binding.content_hash {
                        return Err(PrintOrchestrationError::FileAttachment(
                            crate::file_attachment::FileAttachmentError::ContentHashMismatch,
                        ));
                    }
                    contents.push(content);
                }
                let content = merge_pdfs(&contents).map_err(PrintOrchestrationError::Serialize)?;
                let hash = hex::encode(Sha256::digest(&content));
                if output.source_file_bindings.len() == 1 {
                    (content, output.source_file_bindings[0].file_id, hash)
                } else {
                    let attachment = self
                        .h_file
                        .store_pdf(
                            ctx,
                            StorePdfRequest {
                                module: "H9".to_string(),
                                entity_type: "category_pdf_cache".to_string(),
                                entity_id: output.id,
                                file_name: format!(
                                    "{}-{}-cache.pdf",
                                    output.category_code, output.id
                                ),
                                retention_policy: FileRetentionPolicy::ShortCache,
                            },
                            &content,
                            now,
                        )
                        .await
                        .map_err(PrintOrchestrationError::FileAttachment)?;
                    (content, attachment.id, attachment.content_hash)
                }
            }
        };
        debug_assert!(!content.is_empty());
        sqlx::query(
            r#"
            UPDATE h9_category_pdf_outputs
               SET attachment_id = $3, content_hash = $4,
                   processing_status = 'ready', failure_reason = NULL,
                   processed_at = $5
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(output.id)
        .bind(attachment_id)
        .bind(content_hash)
        .bind(now)
        .execute(&self.repository.pool)
        .await
        .map_err(map_db_error)?;
        Ok(())
    }

    async fn mark_output_failed(
        &self,
        owner_id: Uuid,
        output_id: Uuid,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<(), PrintOrchestrationError> {
        sqlx::query(
            r#"
            UPDATE h9_category_pdf_outputs
               SET attachment_id = NULL, content_hash = NULL,
                   processing_status = 'failed', failure_reason = $3,
                   processed_at = $4
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(output_id)
        .bind(reason)
        .bind(now)
        .execute(&self.repository.pool)
        .await
        .map_err(map_db_error)?;
        Ok(())
    }
}

fn parse_source_mode(value: &str) -> Result<PrintSuiteSourceMode, PrintOrchestrationError> {
    PrintSuiteSourceMode::try_from(value)
        .map_err(|()| PrintOrchestrationError::Serialize(format!("unknown source mode: {value}")))
}

fn hash_json(value: &Value) -> Result<String, PrintOrchestrationError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn category_pdf_render_data(
    source: &InstanceSourceRow,
    output: &CategoryPdfOutput,
    template_version_id: Uuid,
) -> Value {
    let documents = source
        .source_documents
        .as_array()
        .cloned()
        .unwrap_or_default();
    let values = |field: &str| {
        documents
            .iter()
            .filter_map(|document| document.get(field).and_then(Value::as_str))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join("、")
    };
    json!({
        "delivery_note_no": source.delivery_note_no,
        "category_code": output.category_code,
        "template_version_id": template_version_id,
        "source_version": output.source_data_version,
        "order_count": documents.len(),
        "wms_order_no": values("wms_order_no"),
        "erp_order_no": values("erp_order_no"),
        "invoice_no": values("invoice_no"),
        "orders": documents,
        "source_documents": source.source_documents,
    })
}
