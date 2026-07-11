fn validate_publish_request(
    req: &PublishPrintFieldLibraryRequest,
) -> Result<(), PrintTemplateError> {
    if req.library_code.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "library_code is required".to_string(),
        ));
    }
    if req.library_name.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "library_name is required".to_string(),
        ));
    }
    if req.source_schema.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "source_schema is required".to_string(),
        ));
    }
    if req.fields.is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "fields are required".to_string(),
        ));
    }
    let mut paths = BTreeSet::new();
    for field in &req.fields {
        if field.field_path.trim().is_empty() {
            return Err(PrintTemplateError::InvalidRequest(
                "field_path is required".to_string(),
            ));
        }
        if !paths.insert(field.field_path.as_str()) {
            return Err(PrintTemplateError::InvalidRequest(format!(
                "duplicate field_path: {}",
                field.field_path
            )));
        }
    }
    Ok(())
}
fn validate_template_request(req: &SavePrintTemplateRequest) -> Result<(), PrintTemplateError> {
    if req.template_code.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "template_code is required".to_string(),
        ));
    }
    if req.template_name.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "template_name is required".to_string(),
        ));
    }
    if req.template_type_code.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "template_type_code is required".to_string(),
        ));
    }
    if req.designer_version.trim().is_empty() {
        return Err(PrintTemplateError::InvalidRequest(
            "designer_version is required".to_string(),
        ));
    }
    if !req.hiprint_json.is_object()
        || !req
            .hiprint_json
            .get("panels")
            .is_some_and(serde_json::Value::is_array)
    {
        return Err(PrintTemplateError::TemplateJsonInvalid);
    }
    let mut paths = BTreeSet::new();
    for binding in &req.field_bindings {
        if binding.field_path.trim().is_empty() {
            return Err(PrintTemplateError::InvalidRequest(
                "field binding path is required".to_string(),
            ));
        }
        if !paths.insert(binding.field_path.as_str()) {
            return Err(PrintTemplateError::InvalidRequest(format!(
                "duplicate field binding: {}",
                binding.field_path
            )));
        }
    }
    Ok(())
}

fn validate_print_request(req: &PrintTemplatePrintRequest) -> Result<(), PrintTemplateError> {
    if req.template_type_code.trim().is_empty()
        || req.business_module.trim().is_empty()
        || req.business_document_type.trim().is_empty()
        || req.business_document_id.trim().is_empty()
    {
        return Err(PrintTemplateError::InvalidRequest(
            "print template business fields are required".to_string(),
        ));
    }
    if !matches!(req.status.as_str(), "printed" | "cancelled" | "failed") {
        return Err(PrintTemplateError::InvalidRequest(
            "print status must be printed, cancelled or failed".to_string(),
        ));
    }
    Ok(())
}

async fn validate_field_library_and_bindings(
    pool: &PgPool,
    req: &SavePrintTemplateRequest,
) -> Result<(), PrintTemplateError> {
    let status: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT status
          FROM print_field_library_versions
         WHERE id = $1
        "#,
    )
    .bind(req.field_library_version_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    if status.as_ref().map(|row| row.0.as_str()) != Some("published") {
        return Err(PrintTemplateError::FieldLibraryNotPublished);
    }

    let field_paths: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT field_path
          FROM print_field_definitions
         WHERE library_version_id = $1
        "#,
    )
    .bind(req.field_library_version_id)
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    let available: HashSet<&str> = field_paths.iter().map(|row| row.0.as_str()).collect();
    let missing: Vec<String> = req
        .field_bindings
        .iter()
        .filter(|binding| !available.contains(binding.field_path.as_str()))
        .map(|binding| binding.field_path.clone())
        .collect();
    if !missing.is_empty() {
        return Err(PrintTemplateError::TemplateFieldMismatch(missing));
    }
    Ok(())
}

fn validate_required_fields(
    bindings: &[PrintTemplateBinding],
    data: &Value,
) -> Result<(), PrintTemplateError> {
    let missing: Vec<String> = bindings
        .iter()
        .filter(|binding| binding.required && value_at_path(data, &binding.field_path).is_none())
        .map(|binding| binding.field_path.clone())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(PrintTemplateError::TemplateFieldMissing(missing))
    }
}

fn value_at_path<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = data;
    for part in path.split('.') {
        if part.is_empty() {
            return None;
        }
        current = current.get(part)?;
    }
    if current.is_null() {
        None
    } else {
        Some(current)
    }
}

async fn upsert_library_for_update(
    tx: &mut Transaction<'_, Postgres>,
    req: &PublishPrintFieldLibraryRequest,
    now: DateTime<Utc>,
) -> Result<Uuid, PrintTemplateError> {
    let existing: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id
          FROM print_field_libraries
         WHERE library_code = $1
         FOR UPDATE
        "#,
    )
    .bind(&req.library_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if let Some((id,)) = existing {
        sqlx::query(
            r#"
            UPDATE print_field_libraries
               SET library_name = $1,
                   source_schema = $2,
                   updated_at = $3,
                   version = version + 1
             WHERE id = $4
            "#,
        )
        .bind(&req.library_name)
        .bind(&req.source_schema)
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
            id, library_code, library_name, source_schema, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $5)
        "#,
    )
    .bind(id)
    .bind(&req.library_code)
    .bind(&req.library_name)
    .bind(&req.source_schema)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(id)
}

async fn next_version_no(
    tx: &mut Transaction<'_, Postgres>,
    library_id: Uuid,
) -> Result<i32, PrintTemplateError> {
    let max_version: Option<i32> = sqlx::query_scalar(
        r#"
        SELECT MAX(version_no)
          FROM print_field_library_versions
         WHERE library_id = $1
        "#,
    )
    .bind(library_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(max_version.unwrap_or(0) + 1)
}

async fn upsert_template_for_update(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    req: &SavePrintTemplateRequest,
    now: DateTime<Utc>,
) -> Result<Uuid, PrintTemplateError> {
    let existing: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id
          FROM print_templates
         WHERE owner_id = $1 AND template_code = $2
         FOR UPDATE
        "#,
    )
    .bind(ctx.owner_id)
    .bind(&req.template_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    if let Some((id,)) = existing {
        sqlx::query(
            r#"
            UPDATE print_templates
               SET template_name = $1,
                   template_type_code = $2,
                   scope = $3,
                   enabled = $4,
                   is_default = $5,
                   remark = $6,
                   updated_at = $7,
                   updated_by = $8,
                   version = version + 1
             WHERE id = $9
            "#,
        )
        .bind(&req.template_name)
        .bind(&req.template_type_code)
        .bind(req.scope.as_str())
        .bind(req.enabled)
        .bind(req.is_default)
        .bind(&req.remark)
        .bind(now)
        .bind(ctx.user_id)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        return Ok(id);
    }

    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code,
            scope, enabled, is_default, remark, created_at, updated_at,
            created_by, updated_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10, $11, $11)
        "#,
    )
    .bind(id)
    .bind(ctx.owner_id)
    .bind(&req.template_code)
    .bind(&req.template_name)
    .bind(&req.template_type_code)
    .bind(req.scope.as_str())
    .bind(req.enabled)
    .bind(req.is_default)
    .bind(&req.remark)
    .bind(now)
    .bind(ctx.user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(id)
}

async fn next_template_version_no(
    tx: &mut Transaction<'_, Postgres>,
    template_id: Uuid,
) -> Result<i32, PrintTemplateError> {
    let max_version: Option<i32> = sqlx::query_scalar(
        r#"
        SELECT MAX(version_no)
          FROM print_template_versions
         WHERE template_id = $1
        "#,
    )
    .bind(template_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(max_version.unwrap_or(0) + 1)
}

async fn resolve_template_version(
    pool: &PgPool,
    ctx: &AuthContext,
    template_code: &Option<String>,
    template_type_code: &str,
) -> Result<PrintTemplateVersion, PrintTemplateError> {
    if let Some(code) = template_code
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let row = sqlx::query_as::<_, PrintTemplateVersionRow>(
            r#"
            SELECT
                versions.id,
                templates.id AS template_id,
                templates.template_code,
                templates.template_name,
                templates.template_type_code,
                templates.owner_id,
                templates.scope,
                templates.enabled,
                templates.is_default,
                templates.remark,
                versions.field_library_version_id,
                versions.version_no,
                versions.status,
                versions.hiprint_json,
                versions.field_bindings,
                versions.paper,
                versions.designer_version,
                versions.created_at,
                versions.created_by,
                versions.published_at,
                versions.published_by
              FROM print_templates templates
              JOIN LATERAL (
                SELECT *
                  FROM print_template_versions
                 WHERE template_id = templates.id AND status = 'published'
                 ORDER BY version_no DESC
                 LIMIT 1
              ) versions ON TRUE
             WHERE templates.owner_id = $1
               AND templates.template_code = $2
               AND templates.template_type_code = $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(code)
        .bind(template_type_code)
        .fetch_optional(pool)
        .await
        .map_err(map_db_error)?
        .ok_or(PrintTemplateError::TemplateNotFound)?;
        let version = PrintTemplateVersion::try_from(row)?;
        if !version.enabled {
            return Err(PrintTemplateError::TemplateDisabled);
        }
        return Ok(version);
    }

    let row = sqlx::query_as::<_, PrintTemplateVersionRow>(
        r#"
        SELECT
            versions.id,
            templates.id AS template_id,
            templates.template_code,
            templates.template_name,
            templates.template_type_code,
            templates.owner_id,
            templates.scope,
            templates.enabled,
            templates.is_default,
            templates.remark,
            versions.field_library_version_id,
            versions.version_no,
            versions.status,
            versions.hiprint_json,
            versions.field_bindings,
            versions.paper,
            versions.designer_version,
            versions.created_at,
            versions.created_by,
            versions.published_at,
            versions.published_by
          FROM print_templates templates
          JOIN LATERAL (
            SELECT *
              FROM print_template_versions
             WHERE template_id = templates.id AND status = 'published'
             ORDER BY version_no DESC
             LIMIT 1
          ) versions ON TRUE
         WHERE templates.owner_id = $1
           AND templates.template_type_code = $2
           AND templates.enabled = TRUE
         ORDER BY
           CASE templates.scope WHEN 'owner' THEN 0 ELSE 1 END,
           templates.is_default DESC,
           templates.updated_at DESC
         LIMIT 1
        "#,
    )
    .bind(ctx.owner_id)
    .bind(template_type_code)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintTemplateError::TemplateNotFound)?;
    PrintTemplateVersion::try_from(row)
}
