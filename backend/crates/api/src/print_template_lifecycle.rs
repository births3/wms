impl PgPrintTemplateRepository {
    pub async fn publish_template_draft(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        template_id: Uuid,
        version_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintTemplateVersion>, PrintTemplateError> {
        let request_hash = json_request_hash(&serde_json::json!({
            "template_id": template_id,
            "version_id": version_id,
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

        let before = template_version_in_tx(&mut tx, ctx.owner_id, template_id, version_id).await?;
        if before.status != "draft" {
            return Err(PrintTemplateError::PublishedTemplateImmutable);
        }
        let latest_version_no: i32 = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(version_no) FROM print_template_versions WHERE template_id = $1",
        )
        .bind(template_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintTemplateError::TemplateVersionNotFound)?;
        if before.version_no != latest_version_no {
            return Err(PrintTemplateError::TemplateVersionNotLatest);
        }
        let Some(expected_library_code) = effective_template_type_field_library_code_in_tx(
            &mut tx,
            ctx.owner_id,
            &before.template_type_code,
            now,
        )
        .await?
        else {
            return Err(PrintTemplateError::TemplateDisabled);
        };
        let field_library_code: String = sqlx::query_scalar(
            r#"
            SELECT libraries.library_code
              FROM print_field_library_versions versions
              JOIN print_field_libraries libraries ON libraries.id = versions.library_id
             WHERE versions.id = $1
               AND versions.status = 'published'
            "#,
        )
        .bind(before.field_library_version_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintTemplateError::FieldLibraryNotPublished)?;
        if expected_library_code != field_library_code {
            return Err(PrintTemplateError::TemplateFieldMismatch(vec![format!(
                "field_library_code:{field_library_code}"
            )]));
        }

        sqlx::query(
            r#"
            UPDATE print_templates
               SET template_name = $1,
                   template_type_code = $2,
                   scope = $3,
                   is_default = $4,
                   remark = $5,
                   updated_at = $6,
                   updated_by = $7,
                   version = version + 1
             WHERE id = $8
               AND owner_id = $9
            "#,
        )
        .bind(&before.template_name)
        .bind(&before.template_type_code)
        .bind(before.scope.as_str())
        .bind(before.is_default)
        .bind(&before.remark)
        .bind(now)
        .bind(ctx.user_id)
        .bind(template_id)
        .bind(ctx.owner_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        sqlx::query(
            r#"
            UPDATE print_template_versions
               SET status = 'published',
                   published_at = $1,
                   published_by = $2
             WHERE id = $3
               AND template_id = $4
            "#,
        )
        .bind(now)
        .bind(ctx.user_id)
        .bind(version_id)
        .bind(template_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let version =
            template_version_in_tx(&mut tx, ctx.owner_id, template_id, version_id).await?;

        let path = format!(
            "/api/v1/print-templates/templates/{template_id}/versions/{version_id}/publish"
        );
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &path,
            "print_template",
            &version,
            now,
        )
        .await?;
        append_h9_audit(
            &mut tx,
            ctx,
            "publish_print_template",
            "print_template",
            template_id,
            now,
            Some(AuditDiff::compute(
                serde_json::to_value(&before)
                    .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
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

    pub async fn set_template_enabled(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        template_id: Uuid,
        enabled: bool,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintTemplateSummary>, PrintTemplateError> {
        let request_hash = json_request_hash(&serde_json::json!({
            "template_id": template_id,
            "enabled": enabled,
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

        let before = template_summary_in_tx(&mut tx, ctx.owner_id, template_id).await?;
        sqlx::query(
            r#"
            UPDATE print_templates
               SET enabled = $1,
                   updated_at = $2,
                   updated_by = $3,
                   version = version + 1
             WHERE id = $4
               AND owner_id = $5
            "#,
        )
        .bind(enabled)
        .bind(now)
        .bind(ctx.user_id)
        .bind(template_id)
        .bind(ctx.owner_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let template = template_summary_in_tx(&mut tx, ctx.owner_id, template_id).await?;

        let path = format!("/api/v1/print-templates/templates/{template_id}/enabled");
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &path,
            "print_template",
            &template,
            now,
        )
        .await?;
        append_h9_audit(
            &mut tx,
            ctx,
            "set_print_template_enabled",
            "print_template",
            template_id,
            now,
            Some(AuditDiff::compute(
                serde_json::to_value(&before)
                    .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
                serde_json::to_value(&template)
                    .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
            )),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: template,
            replayed: false,
        })
    }
}
