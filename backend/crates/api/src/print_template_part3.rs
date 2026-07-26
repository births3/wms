async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), PrintTemplateError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind(owner_id.to_string())
        .bind(idempotency_key)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, PrintTemplateError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;

    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(PrintTemplateError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| PrintTemplateError::Serialize(error.to_string()))
}

async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), PrintTemplateError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?;
    let resource_id = response_body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(resource_type)
        .to_string();
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES (
            $1, $2, $3, $4, $5, $6,
            200, $7, $8, $9, $10, $11
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

async fn append_h9_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
    now: DateTime<Utc>,
    diff: Option<AuditDiff>,
) -> Result<(), PrintTemplateError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H9",
        resource_type,
        resource_id.to_string(),
        diff,
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| PrintTemplateError::Audit(format!("{error:?}")))?;
    Ok(())
}

fn json_request_hash(value: &Value) -> Result<String, PrintTemplateError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

#[derive(Debug, FromRow)]
struct PrintTemplateVersionRow {
    id: Uuid,
    template_id: Uuid,
    template_code: String,
    template_name: String,
    template_type_code: String,
    owner_id: Uuid,
    scope: String,
    enabled: bool,
    is_default: bool,
    remark: Option<String>,
    field_library_version_id: Uuid,
    version_no: i32,
    status: String,
    hiprint_json: Value,
    field_bindings: Value,
    paper: Value,
    designer_version: String,
    created_at: DateTime<Utc>,
    created_by: Uuid,
    published_at: Option<DateTime<Utc>>,
    published_by: Option<Uuid>,
}

impl TryFrom<PrintTemplateVersionRow> for PrintTemplateVersion {
    type Error = PrintTemplateError;

    fn try_from(row: PrintTemplateVersionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            template_id: row.template_id,
            template_code: row.template_code,
            template_name: row.template_name,
            template_type_code: row.template_type_code,
            owner_id: row.owner_id,
            scope: PrintTemplateScope::from_db(row.scope)?,
            enabled: row.enabled,
            is_default: row.is_default,
            remark: row.remark,
            field_library_version_id: row.field_library_version_id,
            version_no: row.version_no,
            status: row.status,
            hiprint_json: row.hiprint_json,
            field_bindings: serde_json::from_value(row.field_bindings)
                .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?,
            paper: row.paper,
            designer_version: row.designer_version,
            created_at: row.created_at,
            created_by: row.created_by,
            published_at: row.published_at,
            published_by: row.published_by,
        })
    }
}

#[derive(Debug, FromRow)]
struct PrintTemplateSummaryRow {
    id: Uuid,
    template_code: String,
    template_name: String,
    template_type_code: String,
    owner_id: Uuid,
    scope: String,
    enabled: bool,
    is_default: bool,
    remark: Option<String>,
    latest_version_id: Uuid,
    latest_version_no: i32,
    latest_version_status: String,
    field_library_version_id: Uuid,
    designer_version: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
}

impl TryFrom<PrintTemplateSummaryRow> for PrintTemplateSummary {
    type Error = PrintTemplateError;

    fn try_from(row: PrintTemplateSummaryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            template_code: row.template_code,
            template_name: row.template_name,
            template_type_code: row.template_type_code,
            owner_id: row.owner_id,
            scope: PrintTemplateScope::from_db(row.scope)?,
            enabled: row.enabled,
            is_default: row.is_default,
            remark: row.remark,
            latest_version_id: row.latest_version_id,
            latest_version_no: row.latest_version_no,
            latest_version_status: row.latest_version_status,
            field_library_version_id: row.field_library_version_id,
            designer_version: row.designer_version,
            created_at: row.created_at,
            updated_at: row.updated_at,
            published_at: row.published_at,
        })
    }
}

fn map_db_error(error: sqlx::Error) -> PrintTemplateError {
    PrintTemplateError::Database(error.to_string())
}
