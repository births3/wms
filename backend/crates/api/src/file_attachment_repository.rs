use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    ConfirmFileUploadRequest, CreateFileUploadRequest, FileAttachment,
    FileAttachmentDownloadUrlResponse, FileAttachmentValidationError, FileUploadSessionResponse,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

const UPLOAD_TTL_MINUTES: i64 = 5;
const DOWNLOAD_TTL_MINUTES: i64 = 15;

#[derive(Clone)]
pub struct PgFileAttachmentRepository {
    pool: PgPool,
}

#[derive(Clone, Debug)]
pub struct UploadTarget {
    pub storage_key: String,
    pub module: String,
    pub entity_type: String,
    pub content_type: String,
    pub expected_size: i64,
}

#[derive(Clone, Debug, FromRow)]
pub struct DownloadTarget {
    pub storage_key: String,
    pub content_type: String,
    pub file_name: String,
}

#[derive(Clone, Debug, FromRow)]
pub struct DrugInspectionImageTarget {
    pub storage_key: String,
    pub content_type: String,
}

#[derive(Debug)]
pub enum FileAttachmentRepositoryError {
    Invalid(FileAttachmentValidationError),
    NotFound,
    UnauthorizedToken,
    UploadExpired,
    UploadNotCompleted,
    UploadMetadataMismatch,
    IdempotencyConflict,
    Audit(String),
    Database(String),
    Serialize(String),
}

impl PgFileAttachmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_upload_session(
        &self,
        ctx: &AuthContext,
        request: CreateFileUploadRequest,
        idempotency_key: &str,
    ) -> Result<FileUploadSessionResponse, FileAttachmentRepositoryError> {
        request
            .validate()
            .map_err(FileAttachmentRepositoryError::Invalid)?;
        let request_hash = request_hash(&request)?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }

        let upload_id = Uuid::new_v4();
        let token = format!("{}.{}", ctx.owner_id, Uuid::new_v4());
        let expires_at = now + Duration::minutes(UPLOAD_TTL_MINUTES);
        let storage_key = storage_key(ctx.owner_id, &request, upload_id);
        let value = FileUploadSessionResponse {
            upload_id,
            upload_url: format!("/api/v1/attachments/uploads/{upload_id}/content?token={token}"),
            expires_at,
        };
        sqlx::query(
            r#"
            INSERT INTO h_file_upload_sessions (
                id, owner_id, module, entity_type, entity_id, file_name,
                content_type, expected_size, storage_key, token_hash, status,
                uploaded_by, expires_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'created', $11, $12, $13, $13)
            "#,
        )
        .bind(upload_id)
        .bind(ctx.owner_id)
        .bind(request.module.trim())
        .bind(request.entity_type.trim())
        .bind(request.entity_id)
        .bind(request.file_name.trim())
        .bind(request.content_type.trim())
        .bind(request.size_bytes)
        .bind(storage_key)
        .bind(hash_token(&token))
        .bind(ctx.user_id)
        .bind(expires_at)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/attachments/uploads",
            "h_file_upload_session",
            upload_id,
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn authorize_upload(
        &self,
        upload_id: Uuid,
        token: &str,
        content_type: &str,
        size_bytes: i64,
    ) -> Result<UploadTarget, FileAttachmentRepositoryError> {
        let owner_id = upload_token_owner(token)?;
        let row = sqlx::query_as::<_, UploadTargetRow>(
            r#"
            SELECT storage_key, module, entity_type, content_type, expected_size,
                   token_hash, status, expires_at
              FROM h_file_upload_sessions
             WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(upload_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(FileAttachmentRepositoryError::NotFound)?;
        if row.token_hash != hash_token(token) {
            return Err(FileAttachmentRepositoryError::UnauthorizedToken);
        }
        if row.expires_at <= Utc::now() {
            return Err(FileAttachmentRepositoryError::UploadExpired);
        }
        if !matches!(row.status.as_str(), "created" | "uploaded") {
            return Err(FileAttachmentRepositoryError::UploadNotCompleted);
        }
        if row.content_type != content_type || row.expected_size != size_bytes {
            return Err(FileAttachmentRepositoryError::UploadMetadataMismatch);
        }
        Ok(UploadTarget {
            storage_key: row.storage_key,
            module: row.module,
            entity_type: row.entity_type,
            content_type: row.content_type,
            expected_size: row.expected_size,
        })
    }

    pub async fn mark_uploaded(
        &self,
        upload_id: Uuid,
        token: &str,
        size_bytes: i64,
        sha256: &str,
    ) -> Result<(), FileAttachmentRepositoryError> {
        let owner_id = upload_token_owner(token)?;
        let result = sqlx::query(
            r#"
            UPDATE h_file_upload_sessions
               SET status = 'uploaded', uploaded_size = $4, sha256 = $5, updated_at = $6
             WHERE id = $1
               AND owner_id = $2
               AND token_hash = $3
               AND status IN ('created', 'uploaded')
               AND expires_at > $6
               AND expected_size = $4
            "#,
        )
        .bind(upload_id)
        .bind(owner_id)
        .bind(hash_token(token))
        .bind(size_bytes)
        .bind(sha256)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;
        if result.rows_affected() == 0 {
            return Err(FileAttachmentRepositoryError::UploadNotCompleted);
        }
        Ok(())
    }

    pub async fn confirm_upload(
        &self,
        ctx: &AuthContext,
        request: ConfirmFileUploadRequest,
        idempotency_key: &str,
    ) -> Result<FileAttachment, FileAttachmentRepositoryError> {
        let request_hash = request_hash(&request)?;
        let now = Utc::now();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            tx.commit().await.map_err(map_db_error)?;
            return Ok(value);
        }
        let session = sqlx::query_as::<_, UploadSessionRow>(
            r#"
            SELECT id, owner_id, module, entity_type, entity_id, file_name, content_type,
                   uploaded_size, storage_key, sha256, status, uploaded_by
              FROM h_file_upload_sessions
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE
            "#,
        )
        .bind(request.upload_id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(FileAttachmentRepositoryError::NotFound)?;

        let value = if session.status == "confirmed" {
            fetch_attachment(&mut tx, ctx.owner_id, session.id)
                .await?
                .ok_or(FileAttachmentRepositoryError::NotFound)?
        } else {
            if session.status != "uploaded" {
                return Err(FileAttachmentRepositoryError::UploadNotCompleted);
            }
            let size_bytes = session
                .uploaded_size
                .ok_or(FileAttachmentRepositoryError::UploadNotCompleted)?;
            let sha256 = session
                .sha256
                .clone()
                .ok_or(FileAttachmentRepositoryError::UploadNotCompleted)?;
            let row = sqlx::query_as::<_, FileAttachmentRow>(
                r#"
                INSERT INTO attachments (
                    id, owner_id, module, entity_type, entity_id, file_name,
                    content_type, size_bytes, storage_key, sha256, uploaded_by, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                RETURNING id, owner_id, module, entity_type, entity_id, file_name,
                          content_type, size_bytes, storage_key, sha256, uploaded_by, created_at
                "#,
            )
            .bind(session.id)
            .bind(session.owner_id)
            .bind(&session.module)
            .bind(&session.entity_type)
            .bind(session.entity_id)
            .bind(&session.file_name)
            .bind(&session.content_type)
            .bind(size_bytes)
            .bind(&session.storage_key)
            .bind(&sha256)
            .bind(session.uploaded_by)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            sqlx::query(
                "UPDATE h_file_upload_sessions SET status = 'confirmed', updated_at = $3 WHERE id = $1 AND owner_id = $2",
            )
            .bind(session.id)
            .bind(ctx.owner_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
            row.into()
        };

        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/attachments/confirm",
            "attachment",
            value.id,
            &value,
            now,
        )
        .await?;
        let audit = AuditWriteRequest::from_auth_context(
            ctx,
            "h_file.attachment.confirmed",
            "H-FILE",
            "attachment",
            value.id.to_string(),
            Some(AuditDiff::compute(Value::Null, attachment_snapshot(&value))),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| FileAttachmentRepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(value)
    }

    pub async fn create_download_url(
        &self,
        ctx: &AuthContext,
        attachment_id: Uuid,
    ) -> Result<FileAttachmentDownloadUrlResponse, FileAttachmentRepositoryError> {
        let now = Utc::now();
        let expires_at = now + Duration::minutes(DOWNLOAD_TTL_MINUTES);
        let token = Uuid::new_v4().to_string();
        let download_id = Uuid::new_v4();
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let attachment = fetch_attachment(&mut tx, ctx.owner_id, attachment_id)
            .await?
            .ok_or(FileAttachmentRepositoryError::NotFound)?;
        sqlx::query(
            r#"
            INSERT INTO h_file_download_sessions (
                id, owner_id, attachment_id, token_hash, created_by, expires_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(download_id)
        .bind(ctx.owner_id)
        .bind(attachment_id)
        .bind(hash_token(&token))
        .bind(ctx.user_id)
        .bind(expires_at)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let audit = AuditWriteRequest::from_auth_context(
            ctx,
            "h_file.attachment.download_authorized",
            "H-FILE",
            "attachment",
            attachment_id.to_string(),
            Some(AuditDiff::compute(
                Value::Null,
                serde_json::json!({
                    "attachment_id": attachment.id,
                    "entity_type": attachment.entity_type,
                    "entity_id": attachment.entity_id,
                    "expires_at": expires_at,
                }),
            )),
        );
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| FileAttachmentRepositoryError::Audit(format!("{error:?}")))?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(FileAttachmentDownloadUrlResponse {
            url: format!(
                "/api/v1/attachments/{attachment_id}/content?download_id={download_id}&token={token}"
            ),
            expires_at,
        })
    }

    pub async fn drug_inspection_image_target(
        &self,
        owner_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<DrugInspectionImageTarget, FileAttachmentRepositoryError> {
        sqlx::query_as::<_, DrugInspectionImageTarget>(
            r#"
            SELECT storage_key, content_type
              FROM attachments
             WHERE id = $1
               AND owner_id = $2
               AND module = 'M-DI'
               AND entity_type = 'drug_inspection_original'
               AND content_type IN ('image/jpeg', 'image/png')
            "#,
        )
        .bind(attachment_id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(FileAttachmentRepositoryError::NotFound)
    }

    pub async fn authorize_download(
        &self,
        attachment_id: Uuid,
        download_id: Uuid,
        token: &str,
    ) -> Result<DownloadTarget, FileAttachmentRepositoryError> {
        sqlx::query_as::<_, DownloadTarget>(
            r#"
            SELECT attachment.storage_key, attachment.content_type, attachment.file_name
              FROM h_file_download_sessions AS download
              JOIN attachments AS attachment
                ON attachment.id = download.attachment_id
               AND attachment.owner_id = download.owner_id
             WHERE download.id = $1
               AND download.attachment_id = $2
               AND download.token_hash = $3
               AND download.expires_at > $4
            "#,
        )
        .bind(download_id)
        .bind(attachment_id)
        .bind(hash_token(token))
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(FileAttachmentRepositoryError::UnauthorizedToken)
    }
}

#[derive(FromRow)]
struct UploadTargetRow {
    storage_key: String,
    module: String,
    entity_type: String,
    content_type: String,
    expected_size: i64,
    token_hash: String,
    status: String,
    expires_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct UploadSessionRow {
    id: Uuid,
    owner_id: Uuid,
    module: String,
    entity_type: String,
    entity_id: Uuid,
    file_name: String,
    content_type: String,
    uploaded_size: Option<i64>,
    storage_key: String,
    sha256: Option<String>,
    status: String,
    uploaded_by: Uuid,
}

#[derive(FromRow)]
struct FileAttachmentRow {
    id: Uuid,
    owner_id: Uuid,
    module: String,
    entity_type: String,
    entity_id: Uuid,
    file_name: String,
    content_type: String,
    size_bytes: i64,
    #[allow(dead_code)]
    storage_key: String,
    sha256: String,
    uploaded_by: Uuid,
    created_at: DateTime<Utc>,
}

impl From<FileAttachmentRow> for FileAttachment {
    fn from(row: FileAttachmentRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            module: row.module,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            file_name: row.file_name,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            sha256: row.sha256,
            uploaded_by: row.uploaded_by,
            created_at: row.created_at,
        }
    }
}

async fn fetch_attachment(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    attachment_id: Uuid,
) -> Result<Option<FileAttachment>, FileAttachmentRepositoryError> {
    sqlx::query_as::<_, FileAttachmentRow>(
        r#"
        SELECT id, owner_id, module, entity_type, entity_id, file_name, content_type,
               size_bytes, storage_key, sha256, uploaded_by, created_at
          FROM attachments
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(attachment_id)
    .fetch_optional(&mut **tx)
    .await
    .map(|row| row.map(Into::into))
    .map_err(map_db_error)
}

fn storage_key(owner_id: Uuid, request: &CreateFileUploadRequest, upload_id: Uuid) -> String {
    let extension = match request.content_type.trim() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "application/pdf" => "pdf",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "text/csv" => "csv",
        _ => "bin",
    };
    format!(
        "{}/{}/{}/{}.{}",
        owner_id,
        request.module.trim(),
        request.entity_id,
        upload_id,
        extension
    )
}

fn attachment_snapshot(value: &FileAttachment) -> Value {
    serde_json::json!({
        "id": value.id,
        "owner_id": value.owner_id,
        "module": value.module,
        "entity_type": value.entity_type,
        "entity_id": value.entity_id,
        "file_name": value.file_name,
        "content_type": value.content_type,
        "size_bytes": value.size_bytes,
        "sha256": value.sha256,
        "uploaded_by": value.uploaded_by,
    })
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), FileAttachmentRepositoryError> {
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
) -> Result<Option<T>, FileAttachmentRepositoryError> {
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
        return Err(FileAttachmentRepositoryError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| FileAttachmentRepositoryError::Serialize(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: Uuid,
    value: &T,
    now: DateTime<Utc>,
) -> Result<(), FileAttachmentRepositoryError> {
    let response_body = serde_json::to_value(value)
        .map_err(|error| FileAttachmentRepositoryError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
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
    .bind(resource_id.to_string())
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

fn request_hash<T: Serialize>(value: &T) -> Result<String, FileAttachmentRepositoryError> {
    serde_json::to_vec(value)
        .map(|bytes| hex::encode(Sha256::digest(bytes)))
        .map_err(|error| FileAttachmentRepositoryError::Serialize(error.to_string()))
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn upload_token_owner(token: &str) -> Result<Uuid, FileAttachmentRepositoryError> {
    let (owner_id, secret) = token
        .split_once('.')
        .ok_or(FileAttachmentRepositoryError::UnauthorizedToken)?;
    if secret.is_empty() {
        return Err(FileAttachmentRepositoryError::UnauthorizedToken);
    }
    Uuid::parse_str(owner_id).map_err(|_| FileAttachmentRepositoryError::UnauthorizedToken)
}

fn map_db_error(error: sqlx::Error) -> FileAttachmentRepositoryError {
    FileAttachmentRepositoryError::Database(error.to_string())
}
