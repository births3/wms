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
        || contains_executable_hiprint_option(&req.hiprint_json)
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

fn contains_executable_hiprint_option(value: &Value) -> bool {
    const EXECUTABLE_OPTIONS: [&str; 9] = [
        "formatter",
        "styler",
        "rowsColumnsMerge",
        "rowStyler",
        "footerFormatter",
        "gridColumnsFooterFormatter",
        "styler2",
        "renderFormatter",
        "formatter2",
    ];

    match value {
        Value::Object(fields) => fields.iter().any(|(key, value)| {
            (EXECUTABLE_OPTIONS.contains(&key.as_str()) && !value.is_null())
                || contains_executable_hiprint_option(value)
        }),
        Value::Array(items) => items.iter().any(contains_executable_hiprint_option),
        _ => false,
    }
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
) -> Result<String, PrintTemplateError> {
    let version: Option<(String, String)> = sqlx::query_as(
        r#"
        SELECT versions.status, libraries.library_code
          FROM print_field_library_versions versions
          JOIN print_field_libraries libraries ON libraries.id = versions.library_id
         WHERE versions.id = $1
        "#,
    )
    .bind(req.field_library_version_id)
    .fetch_optional(pool)
    .await
    .map_err(map_db_error)?;
    let Some((status, library_code)) = version else {
        return Err(PrintTemplateError::FieldLibraryNotPublished);
    };
    if status != "published" {
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
    Ok(library_code)
}

async fn effective_template_type_field_library_code_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    template_type_code: &str,
    now: DateTime<Utc>,
) -> Result<Option<String>, PrintTemplateError> {
    sqlx::query_scalar(
        r#"
        WITH scoped_items AS (
            SELECT item.enabled,
                   item.params,
                   ROW_NUMBER() OVER (
                       ORDER BY
                           CASE WHEN item.owner_id = $1 THEN 1 ELSE 0 END DESC,
                           item.updated_at DESC
                   ) AS scope_rank
              FROM system_dictionary_items item
              JOIN system_dictionary_categories category
                ON category.dict_code = item.dict_code
               AND category.enabled = TRUE
             WHERE item.dict_code = $2
               AND item.item_code = $3
               AND (item.owner_id IS NULL OR item.owner_id = $1)
               AND (item.effective_from IS NULL OR item.effective_from <= $4)
               AND (item.effective_to IS NULL OR item.effective_to > $4)
        )
        SELECT params ->> 'field_library_code'
          FROM scoped_items
         WHERE scope_rank = 1
           AND enabled = TRUE
        "#,
    )
    .bind(owner_id)
    .bind(SYSTEM_DICTIONARY_PRINT_TEMPLATE_TYPE)
    .bind(template_type_code)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
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
) -> Result<(Uuid, bool), PrintTemplateError> {
    if let Some(template_id) = req.template_id {
        let existing: Option<(String, bool)> = sqlx::query_as(
            r#"
            SELECT template_code, enabled
              FROM print_templates
             WHERE owner_id = $1
               AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(template_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)?;
        let Some((template_code, enabled)) = existing else {
            return Err(PrintTemplateError::TemplateNotFound);
        };
        if template_code != req.template_code {
            return Err(PrintTemplateError::InvalidRequest(
                "template_code cannot be changed".to_string(),
            ));
        }
        return Ok((template_id, enabled));
    }

    let duplicate: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM print_templates
             WHERE owner_id = $1
               AND template_code = $2
        )
        "#,
    )
    .bind(ctx.owner_id)
    .bind(&req.template_code)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if duplicate {
        return Err(PrintTemplateError::TemplateDuplicate);
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
    .bind(true)
    .bind(req.is_default)
    .bind(&req.remark)
    .bind(now)
    .bind(ctx.user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok((id, true))
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
                versions.template_name,
                versions.template_type_code,
                templates.owner_id,
                versions.scope,
                templates.enabled,
                versions.is_default,
                versions.remark,
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
            versions.template_name,
            versions.template_type_code,
            templates.owner_id,
            versions.scope,
            templates.enabled,
            versions.is_default,
            versions.remark,
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

async fn template_version_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    template_id: Uuid,
    version_id: Uuid,
) -> Result<PrintTemplateVersion, PrintTemplateError> {
    let row = sqlx::query_as::<_, PrintTemplateVersionRow>(
        r#"
        SELECT
            versions.id,
            templates.id AS template_id,
            templates.template_code,
            versions.template_name,
            versions.template_type_code,
            templates.owner_id,
            versions.scope,
            templates.enabled,
            versions.is_default,
            versions.remark,
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
          FROM print_template_versions versions
          JOIN print_templates templates ON templates.id = versions.template_id
         WHERE templates.owner_id = $1
           AND templates.id = $2
           AND versions.id = $3
         FOR UPDATE OF templates, versions
        "#,
    )
    .bind(owner_id)
    .bind(template_id)
    .bind(version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintTemplateError::TemplateVersionNotFound)?;
    PrintTemplateVersion::try_from(row)
}

async fn template_summary_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    template_id: Uuid,
) -> Result<PrintTemplateSummary, PrintTemplateError> {
    let row = sqlx::query_as::<_, PrintTemplateSummaryRow>(
        r#"
        SELECT
            templates.id,
            templates.template_code,
            latest_versions.template_name,
            latest_versions.template_type_code,
            templates.owner_id,
            latest_versions.scope,
            templates.enabled,
            latest_versions.is_default,
            latest_versions.remark,
            latest_versions.id AS latest_version_id,
            latest_versions.version_no AS latest_version_no,
            latest_versions.status AS latest_version_status,
            latest_versions.field_library_version_id,
            latest_versions.designer_version,
            templates.created_at,
            templates.updated_at,
            latest_versions.published_at
          FROM print_templates templates
          JOIN LATERAL (
            SELECT
                id, template_name, template_type_code, scope, is_default, remark,
                version_no, status, field_library_version_id, designer_version, published_at
              FROM print_template_versions
             WHERE template_id = templates.id
             ORDER BY version_no DESC
             LIMIT 1
          ) latest_versions ON TRUE
         WHERE templates.owner_id = $1
           AND templates.id = $2
         FOR UPDATE OF templates
        "#,
    )
    .bind(owner_id)
    .bind(template_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(PrintTemplateError::TemplateNotFound)?;
    PrintTemplateSummary::try_from(row)
}
