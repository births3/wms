impl PgPrintTemplateRepository {
    pub async fn preview_template(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: PrintTemplatePreviewRequest,
    ) -> Result<PrintTemplatePreviewResponse, PrintTemplateError> {
        let version =
            resolve_template_version(pool, ctx, &req.template_code, &req.template_type_code)
                .await?;
        validate_required_fields(&version.field_bindings, &req.data)?;
        let mut data = req.data;
        let fields = self
            .list_field_version_fields(pool, version.field_library_version_id)
            .await?;
        mask_sensitive_fields(&mut data, &fields);
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
            data,
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
            resolve_template_version(pool, ctx, &req.template_code, &req.template_type_code)
                .await?;
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
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                (
                    SELECT COUNT(*)::INT
                      FROM print_records
                     WHERE owner_id = $2
                       AND business_module = $4
                       AND business_document_type = $5
                       AND business_document_id = $6
                ),
                $9, $10, $9
            )
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
        append_h9_audit(
            &mut tx,
            ctx,
            "print_template",
            "print_record",
            record.id,
            now,
            None,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;

        Ok(IdempotentMutation {
            value: record,
            replayed: false,
        })
    }
}
