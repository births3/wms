//! H9 print template repository first slice.

use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    auth::AuthContext,
};

#[derive(Clone, Debug)]
pub struct PgPrintTemplateRepository;

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrintTemplateError {
    InvalidRequest(String),
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PrintFieldDefinitionInput {
    pub field_path: String,
    pub field_type: String,
    pub source_schema: String,
    pub display_name: String,
    pub group_code: String,
    pub group_name: String,
    pub metadata: Value,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PublishPrintFieldLibraryRequest {
    pub library_code: String,
    pub library_name: String,
    pub source_schema: String,
    pub fields: Vec<PrintFieldDefinitionInput>,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize)]
pub struct PrintFieldLibraryVersion {
    pub id: Uuid,
    pub library_id: Uuid,
    pub library_code: String,
    pub library_name: String,
    pub source_schema: String,
    pub version_no: i32,
    pub published_at: DateTime<Utc>,
    pub published_by: Uuid,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize)]
pub struct PrintFieldDefinition {
    pub id: Uuid,
    pub library_version_id: Uuid,
    pub field_path: String,
    pub field_type: String,
    pub source_schema: String,
    pub display_name: String,
    pub group_code: String,
    pub group_name: String,
    pub metadata: Value,
    pub sort_order: i32,
}

impl PgPrintTemplateRepository {
    pub fn new() -> Self {
        Self
    }

    pub async fn publish_field_library(
        &self,
        pool: &PgPool,
        ctx: &AuthContext,
        req: PublishPrintFieldLibraryRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<PrintFieldLibraryVersion>, PrintTemplateError> {
        validate_publish_request(&req)?;
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

        let library_id = upsert_library_for_update(&mut tx, &req, now).await?;
        let version_no = next_version_no(&mut tx, library_id).await?;
        let version_id = Uuid::new_v4();
        let version = sqlx::query_as::<_, PrintFieldLibraryVersion>(
            r#"
            INSERT INTO print_field_library_versions (
                id, library_id, version_no, status, published_at, published_by, request_hash, created_at
            )
            VALUES ($1, $2, $3, 'published', $4, $5, $6, $4)
            RETURNING
                id,
                library_id,
                $7::TEXT AS library_code,
                $8::TEXT AS library_name,
                $9::TEXT AS source_schema,
                version_no,
                published_at,
                published_by
            "#,
        )
        .bind(version_id)
        .bind(library_id)
        .bind(version_no)
        .bind(now)
        .bind(ctx.user_id)
        .bind(&request_hash)
        .bind(&req.library_code)
        .bind(&req.library_name)
        .bind(&req.source_schema)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        for field in &req.fields {
            sqlx::query(
                r#"
                INSERT INTO print_field_definitions (
                    id, library_version_id, field_path, field_type, source_schema,
                    display_name, group_code, group_name, metadata, sort_order, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(version.id)
            .bind(&field.field_path)
            .bind(&field.field_type)
            .bind(&field.source_schema)
            .bind(&field.display_name)
            .bind(&field.group_code)
            .bind(&field.group_name)
            .bind(&field.metadata)
            .bind(field.sort_order)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            &version,
            now,
        )
        .await?;
        append_publish_audit(&mut tx, ctx, &version, now).await?;
        tx.commit().await.map_err(map_db_error)?;

        Ok(IdempotentMutation {
            value: version,
            replayed: false,
        })
    }

    pub async fn list_field_version_fields(
        &self,
        pool: &PgPool,
        library_version_id: Uuid,
    ) -> Result<Vec<PrintFieldDefinition>, PrintTemplateError> {
        sqlx::query_as::<_, PrintFieldDefinition>(
            r#"
            SELECT id, library_version_id, field_path, field_type, source_schema,
                   display_name, group_code, group_name, metadata, sort_order
              FROM print_field_definitions
             WHERE library_version_id = $1
             ORDER BY sort_order ASC, field_path ASC
            "#,
        )
        .bind(library_version_id)
        .fetch_all(pool)
        .await
        .map_err(map_db_error)
    }
}

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
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), PrintTemplateError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| PrintTemplateError::Serialize(error.to_string()))?;
    let resource_id = response_body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("print_field_library")
        .to_string();
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES (
            $1, $2, $3, $4, 'POST', '/api/internal/h9/field-libraries/publish',
            200, $5, 'print_field_library', $6, $7, $8
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(response_body)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

async fn append_publish_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    version: &PrintFieldLibraryVersion,
    now: DateTime<Utc>,
) -> Result<(), PrintTemplateError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        "publish_field_library",
        "H9",
        "print_field_library",
        version.id.to_string(),
        None,
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

fn map_db_error(error: sqlx::Error) -> PrintTemplateError {
    PrintTemplateError::Database(error.to_string())
}
