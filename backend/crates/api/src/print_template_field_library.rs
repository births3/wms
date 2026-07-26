impl PgPrintTemplateRepository {
    pub async fn generate_field_library_draft(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: GeneratePrintFieldLibraryDraftRequest,
        openapi: &Value,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintFieldLibraryVersion>, PrintTemplateError> {
        validate_draft_request(&req)?;
        let fields = generate_openapi_fields(openapi, &req.source_schema)?;
        let request_hash = json_request_hash(&serde_json::json!({
            "request": &req,
            "source_schema": schema_by_name(openapi, &req.source_schema),
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

        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('h9-field-library'), hashtext($1))")
            .bind(&req.library_code)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        let library_id = upsert_draft_library(&mut tx, &req, now).await?;
        let version_no = next_version_no(&mut tx, library_id).await?;
        let version_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO print_field_library_versions (
                id, library_id, version_no, status, source_schema, business_module, request_hash,
                created_at, created_by
            )
            VALUES ($1, $2, $3, 'draft', $4, $5, $6, $7, $8)
            "#,
        )
        .bind(version_id)
        .bind(library_id)
        .bind(version_no)
        .bind(&req.source_schema)
        .bind(&req.business_module)
        .bind(&request_hash)
        .bind(now)
        .bind(ctx.user_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        for field in fields {
            sqlx::query(
                r#"
                INSERT INTO print_field_definitions (
                    id, library_version_id, field_path, field_type, source_schema,
                    display_name, group_code, group_name, description, example_value,
                    printable, sensitive, supports_barcode, supports_qrcode,
                    is_table_detail, sort_order, created_at
                )
                VALUES (
                    $1, $2, $3, $4, $5,
                    $6, $7, $8, $9, $10,
                    TRUE, FALSE, FALSE, FALSE,
                    $11, $12, $13
                )
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(version_id)
            .bind(field.field_path)
            .bind(field.field_type)
            .bind(field.source_schema)
            .bind(field.display_name)
            .bind(field.group_code)
            .bind(field.group_name)
            .bind(field.description)
            .bind(field.example_value)
            .bind(field.is_table_detail)
            .bind(field.sort_order)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        let version = field_library_version(&mut tx, version_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/print-templates/field-libraries/drafts",
            "print_field_library",
            &version,
            now,
        )
        .await?;
        append_h9_audit(
            &mut tx,
            ctx,
            "generate_print_field_library_draft",
            "print_field_library",
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

    #[allow(clippy::too_many_arguments)]
    pub async fn update_field_definition(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        library_version_id: Uuid,
        field_id: Uuid,
        req: UpdatePrintFieldDefinitionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintFieldDefinition>, PrintTemplateError> {
        validate_field_update(&req)?;
        let request_hash = json_request_hash(&serde_json::json!({
            "library_version_id": library_version_id,
            "field_id": field_id,
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

        let status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT versions.status
              FROM print_field_definitions fields
              JOIN print_field_library_versions versions
                ON versions.id = fields.library_version_id
             WHERE fields.id = $1
               AND fields.library_version_id = $2
             FOR UPDATE OF fields, versions
            "#,
        )
        .bind(field_id)
        .bind(library_version_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some(status) = status else {
            return Err(PrintTemplateError::FieldLibraryVersionNotFound);
        };
        if status != "draft" {
            return Err(PrintTemplateError::PublishedFieldLibraryImmutable);
        }
        let before = field_definition(&mut tx, library_version_id, field_id).await?;

        let field = sqlx::query_as::<_, PrintFieldDefinition>(
            r#"
            UPDATE print_field_definitions
               SET display_name = $1,
                   group_code = $2,
                   group_name = $3,
                   description = $4,
                   example_value = $5,
                   printable = $6,
                   sensitive = $7,
                   masking_rule = $8,
                   formatting_rule = $9,
                   supports_barcode = $10,
                   supports_qrcode = $11,
                   is_table_detail = $12,
                   sort_order = $13
             WHERE id = $14
               AND library_version_id = $15
             RETURNING
                id, library_version_id, field_path, field_type, source_schema,
                display_name, group_code, group_name, description, example_value,
                printable, sensitive, masking_rule, formatting_rule,
                supports_barcode, supports_qrcode, is_table_detail, sort_order
            "#,
        )
        .bind(req.display_name.trim())
        .bind(req.group_code.trim())
        .bind(req.group_name.trim())
        .bind(req.description.trim())
        .bind(req.example_value)
        .bind(req.printable)
        .bind(req.sensitive)
        .bind(trimmed_option(req.masking_rule))
        .bind(trimmed_option(req.formatting_rule))
        .bind(req.supports_barcode)
        .bind(req.supports_qrcode)
        .bind(req.is_table_detail)
        .bind(req.sort_order)
        .bind(field_id)
        .bind(library_version_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "PATCH",
            &format!(
                "/api/v1/print-templates/field-libraries/{library_version_id}/fields/{field_id}"
            ),
            "print_field_definition",
            &field,
            now,
        )
        .await?;
        append_h9_audit(
            &mut tx,
            ctx,
            "update_print_field_definition",
            "print_field_library",
            library_version_id,
            now,
            Some(AuditDiff::compute(
                serde_json::to_value(&before)
                    .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
                serde_json::to_value(&field)
                    .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
            )),
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: field,
            replayed: false,
        })
    }

    pub async fn publish_field_library_draft(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        library_version_id: Uuid,
        openapi: &Value,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintFieldLibraryVersion>, PrintTemplateError> {
        let request_hash = json_request_hash(&serde_json::json!({
            "library_version_id": library_version_id,
            "openapi_components": openapi.get("components"),
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

        let version: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT status, source_schema
              FROM print_field_library_versions
             WHERE id = $1
             FOR UPDATE
            "#,
        )
        .bind(library_version_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let Some((status, source_schema)) = version else {
            return Err(PrintTemplateError::FieldLibraryVersionNotFound);
        };
        if status != "draft" {
            return Err(PrintTemplateError::PublishedFieldLibraryImmutable);
        }
        let before = field_library_version(&mut tx, library_version_id).await?;

        let available: HashSet<String> = generate_openapi_fields(openapi, &source_schema)?
            .into_iter()
            .map(|field| field.field_path)
            .collect();
        let mut invalid: Vec<String> = sqlx::query_scalar(
            "SELECT field_path FROM print_field_definitions WHERE library_version_id = $1",
        )
        .bind(library_version_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .filter(|path| !available.contains(path))
        .collect();
        if !invalid.is_empty() {
            invalid.sort();
            return Err(PrintTemplateError::FieldPathInvalid(invalid));
        }

        sqlx::query(
            r#"
            UPDATE print_field_library_versions
               SET status = 'published',
                   published_at = $1,
                   published_by = $2
             WHERE id = $3
            "#,
        )
        .bind(now)
        .bind(ctx.user_id)
        .bind(library_version_id)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let version = field_library_version(&mut tx, library_version_id).await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            &format!("/api/v1/print-templates/field-libraries/{library_version_id}/publish"),
            "print_field_library",
            &version,
            now,
        )
        .await?;
        append_h9_audit(
            &mut tx,
            ctx,
            "publish_print_field_library",
            "print_field_library",
            library_version_id,
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
}

async fn field_library_version(
    tx: &mut Transaction<'_, Postgres>,
    version_id: Uuid,
) -> Result<PrintFieldLibraryVersion, PrintTemplateError> {
    sqlx::query_as::<_, PrintFieldLibraryVersion>(
        r#"
        SELECT
            versions.id,
            versions.library_id,
            libraries.library_code,
            libraries.library_name,
            versions.business_module,
            versions.source_schema,
            versions.version_no,
            versions.status,
            versions.created_at,
            versions.created_by,
            versions.published_at,
            versions.published_by
          FROM print_field_library_versions versions
          JOIN print_field_libraries libraries ON libraries.id = versions.library_id
         WHERE versions.id = $1
        "#,
    )
    .bind(version_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn field_definition(
    tx: &mut Transaction<'_, Postgres>,
    library_version_id: Uuid,
    field_id: Uuid,
) -> Result<PrintFieldDefinition, PrintTemplateError> {
    sqlx::query_as::<_, PrintFieldDefinition>(
        r#"
        SELECT
            id, library_version_id, field_path, field_type, source_schema,
            display_name, group_code, group_name, description, example_value,
            printable, sensitive, masking_rule, formatting_rule,
            supports_barcode, supports_qrcode, is_table_detail, sort_order
          FROM print_field_definitions
         WHERE id = $1
           AND library_version_id = $2
        "#,
    )
    .bind(field_id)
    .bind(library_version_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn upsert_draft_library(
    tx: &mut Transaction<'_, Postgres>,
    req: &GeneratePrintFieldLibraryDraftRequest,
    now: DateTime<Utc>,
) -> Result<Uuid, PrintTemplateError> {
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM print_field_libraries WHERE library_code = $1 FOR UPDATE",
    )
    .bind(req.library_code.trim())
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if let Some(id) = existing {
        sqlx::query(
            r#"
            UPDATE print_field_libraries
               SET library_name = $1,
                   business_module = $2,
                   source_schema = $3,
                   updated_at = $4,
                   version = version + 1
             WHERE id = $5
            "#,
        )
        .bind(req.library_name.trim())
        .bind(req.business_module.trim())
        .bind(req.source_schema.trim())
        .bind(now)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        return Ok(id);
    }

    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO print_field_libraries (
            id, library_code, library_name, business_module, source_schema,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        "#,
    )
    .bind(id)
    .bind(req.library_code.trim())
    .bind(req.library_name.trim())
    .bind(req.business_module.trim())
    .bind(req.source_schema.trim())
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(id)
}

fn validate_draft_request(
    req: &GeneratePrintFieldLibraryDraftRequest,
) -> Result<(), PrintTemplateError> {
    for (name, value) in [
        ("library_code", req.library_code.as_str()),
        ("library_name", req.library_name.as_str()),
        ("business_module", req.business_module.as_str()),
        ("source_schema", req.source_schema.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(PrintTemplateError::InvalidRequest(format!(
                "{name} is required"
            )));
        }
    }
    Ok(())
}

fn validate_field_update(
    req: &UpdatePrintFieldDefinitionRequest,
) -> Result<(), PrintTemplateError> {
    if req.display_name.trim().is_empty()
        || req.group_code.trim().is_empty()
        || req.group_name.trim().is_empty()
        || req.sort_order < 0
    {
        return Err(PrintTemplateError::InvalidRequest(
            "display_name, group_code, group_name and non-negative sort_order are required"
                .to_string(),
        ));
    }
    validate_metadata_rule(req.masking_rule.as_deref())?;
    validate_metadata_rule(req.formatting_rule.as_deref())?;
    Ok(())
}

fn validate_metadata_rule(rule: Option<&str>) -> Result<(), PrintTemplateError> {
    let Some(rule) = rule.map(str::trim).filter(|rule| !rule.is_empty()) else {
        return Ok(());
    };
    let valid = rule.len() <= 128
        && rule
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
        && rule.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(
                    character,
                    '_' | '-' | ':' | '.' | '/' | ',' | ' ' | '[' | ']'
                )
        });
    if !valid {
        return Err(PrintTemplateError::FieldFormatInvalid(rule.to_string()));
    }
    Ok(())
}

fn trimmed_option(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let item = item.trim();
        (!item.is_empty()).then(|| item.to_string())
    })
}
