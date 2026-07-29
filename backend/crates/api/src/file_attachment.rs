//! H-FILE unified PDF storage port and S3/MinIO adapter (ADR-0031).

use std::{collections::HashMap, env, sync::Arc};

use aws_credential_types::Credentials;
use aws_sdk_s3::{primitives::ByteStream, types::ServerSideEncryption, Client};
use aws_types::region::Region;
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    audit::{append_event, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

const MAX_FILE_BYTES: usize = 50 * 1024 * 1024;
const DEFAULT_BUCKET: &str = "wms-attachments";
const DEFAULT_REGION: &str = "us-east-1";

/// H-FILE runtime configuration error or storage failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileAttachmentError {
    InvalidConfiguration(String),
    InvalidPdf,
    FileTooLarge,
    NotFound,
    NotReady,
    ContentHashMismatch,
    Storage(String),
    Database(String),
    Audit(String),
}

/// Controlled H-FILE retention class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileRetentionPolicy {
    GspFiveYear,
    ShortCache,
}

impl FileRetentionPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::GspFiveYear => "gsp_5_year",
            Self::ShortCache => "short_cache",
        }
    }
}

/// Metadata needed to persist a PDF through H-FILE.
#[derive(Clone, Debug)]
pub struct StorePdfRequest {
    pub module: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub file_name: String,
    pub retention_policy: FileRetentionPolicy,
}

/// Stable attachment fact returned to business modules.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StoredAttachment {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub file_version: i32,
    pub content_hash: String,
    pub size_bytes: i64,
    pub retention_policy: String,
    pub cache_expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
enum ObjectStore {
    S3 {
        client: Client,
        bucket: String,
        sse_aes256: bool,
    },
    Memory {
        bucket: String,
        objects: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    },
    Disabled,
}

impl ObjectStore {
    fn bucket(&self) -> Result<&str, FileAttachmentError> {
        match self {
            Self::S3 { bucket, .. } | Self::Memory { bucket, .. } => Ok(bucket),
            Self::Disabled => Err(FileAttachmentError::InvalidConfiguration(
                "H-FILE S3/MinIO 未配置".to_string(),
            )),
        }
    }

    async fn put(&self, key: &str, content: &[u8]) -> Result<(), FileAttachmentError> {
        match self {
            Self::S3 {
                client,
                bucket,
                sse_aes256,
            } => {
                let request = client
                    .put_object()
                    .bucket(bucket)
                    .key(key)
                    .content_type("application/pdf");
                let request = if *sse_aes256 {
                    request.server_side_encryption(ServerSideEncryption::Aes256)
                } else {
                    request
                };
                request
                    .body(ByteStream::from(content.to_vec()))
                    .send()
                    .await
                    .map(|_| ())
                    .map_err(|error| FileAttachmentError::Storage(error.to_string()))
            }
            Self::Memory { objects, .. } => {
                objects
                    .write()
                    .await
                    .insert(key.to_string(), content.to_vec());
                Ok(())
            }
            Self::Disabled => Err(FileAttachmentError::InvalidConfiguration(
                "H-FILE S3/MinIO 未配置".to_string(),
            )),
        }
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, FileAttachmentError> {
        match self {
            Self::S3 { client, bucket, .. } => {
                let output = client
                    .get_object()
                    .bucket(bucket)
                    .key(key)
                    .send()
                    .await
                    .map_err(|error| FileAttachmentError::Storage(error.to_string()))?;
                output
                    .body
                    .collect()
                    .await
                    .map(|body| body.into_bytes().to_vec())
                    .map_err(|error| FileAttachmentError::Storage(error.to_string()))
            }
            Self::Memory { objects, .. } => objects
                .read()
                .await
                .get(key)
                .cloned()
                .ok_or(FileAttachmentError::NotFound),
            Self::Disabled => Err(FileAttachmentError::InvalidConfiguration(
                "H-FILE S3/MinIO 未配置".to_string(),
            )),
        }
    }

    async fn delete(&self, key: &str) -> Result<(), FileAttachmentError> {
        match self {
            Self::S3 { client, bucket, .. } => client
                .delete_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .map(|_| ())
                .map_err(|error| FileAttachmentError::Storage(error.to_string())),
            Self::Memory { objects, .. } => {
                objects.write().await.remove(key);
                Ok(())
            }
            Self::Disabled => Err(FileAttachmentError::InvalidConfiguration(
                "H-FILE S3/MinIO 未配置".to_string(),
            )),
        }
    }
}

#[derive(Debug, FromRow)]
struct AttachmentRow {
    storage_key: String,
    size_bytes: i64,
    content_hash: String,
    status: String,
}

/// Shared H-FILE service. Business modules receive this port instead of an S3 client.
#[derive(Clone, Debug)]
pub struct FileAttachmentService {
    pool: PgPool,
    store: ObjectStore,
}

impl FileAttachmentService {
    /// Builds the production S3-compatible adapter from WMS_HFILE_* variables.
    pub fn from_env(pool: PgPool) -> Result<Self, FileAttachmentError> {
        let endpoint = required_env("WMS_HFILE_ENDPOINT")?;
        let access_key = required_env("WMS_HFILE_ACCESS_KEY")?;
        let secret_key = required_env("WMS_HFILE_SECRET_KEY")?;
        let region = env::var("WMS_HFILE_REGION").unwrap_or_else(|_| DEFAULT_REGION.to_string());
        let bucket = env::var("WMS_HFILE_BUCKET").unwrap_or_else(|_| DEFAULT_BUCKET.to_string());
        let sse_aes256 = parse_sse_mode(&required_env("WMS_HFILE_SSE_MODE")?)?;
        let credentials = Credentials::new(access_key, secret_key, None, None, "wms-h-file");
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new(region))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();
        Ok(Self {
            pool,
            store: ObjectStore::S3 {
                client: Client::from_conf(config),
                bucket,
                sse_aes256,
            },
        })
    }

    /// Builds an isolated object store for tests and the real-data E2E process.
    pub fn with_memory(pool: PgPool) -> Self {
        Self {
            pool,
            store: ObjectStore::Memory {
                bucket: DEFAULT_BUCKET.to_string(),
                objects: Arc::new(RwLock::new(HashMap::new())),
            },
        }
    }

    /// Keeps unrelated API use cases startable when H-FILE is intentionally absent.
    pub fn disabled(pool: PgPool) -> Self {
        Self {
            pool,
            store: ObjectStore::Disabled,
        }
    }

    /// Writes one validated PDF object and confirms its unified metadata.
    pub async fn store_pdf(
        &self,
        ctx: &AuthContext,
        request: StorePdfRequest,
        content: &[u8],
        now: DateTime<Utc>,
    ) -> Result<StoredAttachment, FileAttachmentError> {
        validate_pdf(content)?;
        let size_bytes =
            i64::try_from(content.len()).map_err(|_| FileAttachmentError::FileTooLarge)?;
        let content_hash = sha256_hex(content);
        let id = Uuid::new_v4();
        let bucket = self.store.bucket()?.to_string();
        let storage_key = format!(
            "{}/{}/{}/{}/{}.pdf",
            ctx.owner_id, request.module, request.entity_type, request.entity_id, id
        );
        let (retain_until, cache_expires_at) = retention_dates(request.retention_policy, now);
        sqlx::query(
            r#"
            INSERT INTO attachments (
                id, owner_id, module, entity_type, entity_id, bucket, storage_key,
                file_name, content_type, size_bytes, content_hash, sha256, file_version,
                status, retention_policy, retain_until, cache_expires_at,
                created_by, uploaded_by, created_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, 'application/pdf', $9, $10, $10,
                1, 'pending', $11, $12, $13, $14, $14, $15
            )
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(request.module.trim())
        .bind(request.entity_type.trim())
        .bind(request.entity_id)
        .bind(&bucket)
        .bind(&storage_key)
        .bind(request.file_name.trim())
        .bind(size_bytes)
        .bind(&content_hash)
        .bind(request.retention_policy.as_str())
        .bind(retain_until)
        .bind(cache_expires_at)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        if let Err(error) = self.store.put(&storage_key, content).await {
            let _ = sqlx::query(
                "UPDATE attachments SET status = 'failed' WHERE owner_id = $1 AND id = $2",
            )
            .bind(ctx.owner_id)
            .bind(id)
            .execute(&self.pool)
            .await;
            return Err(error);
        }
        if let Err(error) = sqlx::query(
            "UPDATE attachments SET status = 'ready', confirmed_at = $3 \
             WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(id)
        .bind(now)
        .execute(&self.pool)
        .await
        {
            let _ = self.store.delete(&storage_key).await;
            let _ = self.mark_failed(ctx.owner_id, id).await;
            return Err(database_error(error));
        }
        let attachment = StoredAttachment {
            id,
            owner_id: ctx.owner_id,
            file_version: 1,
            content_hash,
            size_bytes,
            retention_policy: request.retention_policy.as_str().to_string(),
            cache_expires_at,
        };
        if let Err(error) = self.audit(ctx, "h_file.upload", &attachment, now).await {
            let _ = self.store.delete(&storage_key).await;
            let _ = self.mark_failed(ctx.owner_id, id).await;
            return Err(error);
        }
        Ok(attachment)
    }

    /// Reads and verifies an object for an internal business service.
    pub async fn read_internal(
        &self,
        owner_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Vec<u8>, FileAttachmentError> {
        let row = self.load(owner_id, attachment_id).await?;
        if row.status != "ready" {
            return Err(FileAttachmentError::NotReady);
        }
        let content = self.store.get(&row.storage_key).await?;
        validate_pdf(&content)?;
        if i64::try_from(content.len()).ok() != Some(row.size_bytes)
            || sha256_hex(&content) != row.content_hash
        {
            return Err(FileAttachmentError::ContentHashMismatch);
        }
        Ok(content)
    }

    /// Reads one PDF for an authorized Web action and appends H2 audit without content.
    pub async fn read_and_audit(
        &self,
        ctx: &AuthContext,
        attachment_id: Uuid,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<u8>, FileAttachmentError> {
        let content = self.read_internal(ctx.owner_id, attachment_id).await?;
        let row = self.load(ctx.owner_id, attachment_id).await?;
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "H-FILE",
            "attachment",
            attachment_id.to_string(),
            Some(AuditDiff::compute(
                json!({}),
                json!({
                    "attachment_id": attachment_id,
                    "content_hash": row.content_hash,
                    "size_bytes": row.size_bytes,
                }),
            )),
        );
        audit.occurred_at = now;
        append_event(&self.pool, &audit)
            .await
            .map_err(|error| FileAttachmentError::Audit(format!("{error:?}")))?;
        Ok(content)
    }

    async fn load(
        &self,
        owner_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<AttachmentRow, FileAttachmentError> {
        sqlx::query_as(
            r#"
            SELECT storage_key, size_bytes, content_hash, status
              FROM attachments
             WHERE owner_id = $1 AND id = $2
            "#,
        )
        .bind(owner_id)
        .bind(attachment_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?
        .ok_or(FileAttachmentError::NotFound)
    }

    async fn audit(
        &self,
        ctx: &AuthContext,
        action: &str,
        attachment: &StoredAttachment,
        now: DateTime<Utc>,
    ) -> Result<(), FileAttachmentError> {
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "H-FILE",
            "attachment",
            attachment.id.to_string(),
            Some(AuditDiff::compute(
                json!({}),
                serde_json::to_value(attachment)
                    .map_err(|error| FileAttachmentError::Audit(error.to_string()))?,
            )),
        );
        audit.occurred_at = now;
        append_event(&self.pool, &audit)
            .await
            .map(|_| ())
            .map_err(|error| FileAttachmentError::Audit(format!("{error:?}")))
    }

    async fn mark_failed(
        &self,
        owner_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<(), FileAttachmentError> {
        sqlx::query(
            "UPDATE attachments SET status = 'failed', confirmed_at = NULL \
             WHERE owner_id = $1 AND id = $2",
        )
        .bind(owner_id)
        .bind(attachment_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(database_error)
    }
}

fn required_env(name: &str) -> Result<String, FileAttachmentError> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| FileAttachmentError::InvalidConfiguration(format!("{name} is required")))
}

fn parse_sse_mode(value: &str) -> Result<bool, FileAttachmentError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "aes256" => Ok(true),
        "none" => Ok(false),
        _ => Err(FileAttachmentError::InvalidConfiguration(
            "WMS_HFILE_SSE_MODE must be aes256 or none".to_string(),
        )),
    }
}

fn validate_pdf(content: &[u8]) -> Result<(), FileAttachmentError> {
    let logical_end = content
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(0, |index| index + 1);
    if content.is_empty()
        || !content.starts_with(b"%PDF-")
        || !content[..logical_end].ends_with(b"%%EOF")
    {
        return Err(FileAttachmentError::InvalidPdf);
    }
    if content.len() > MAX_FILE_BYTES {
        return Err(FileAttachmentError::FileTooLarge);
    }
    Ok(())
}

fn sha256_hex(content: &[u8]) -> String {
    hex::encode(Sha256::digest(content))
}

fn retention_dates(
    policy: FileRetentionPolicy,
    now: DateTime<Utc>,
) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    match policy {
        FileRetentionPolicy::GspFiveYear => {
            let retain_until = now
                .with_year(now.year() + 5)
                .unwrap_or(now + Duration::days(365 * 5));
            (Some(retain_until), None)
        }
        FileRetentionPolicy::ShortCache => (None, Some(now + Duration::days(7))),
    }
}

fn database_error(error: sqlx::Error) -> FileAttachmentError {
    FileAttachmentError::Database(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_mode_is_explicit_and_controlled() {
        assert_eq!(parse_sse_mode("aes256"), Ok(true));
        assert_eq!(parse_sse_mode("NONE"), Ok(false));
        assert!(matches!(
            parse_sse_mode(""),
            Err(FileAttachmentError::InvalidConfiguration(_))
        ));
    }

    #[test]
    fn accepts_standard_pdf_eof_with_or_without_trailing_whitespace() {
        assert_eq!(validate_pdf(b"%PDF-1.3\n%%EOF"), Ok(()));
        assert_eq!(validate_pdf(b"%PDF-1.7\n%%EOF\r\n"), Ok(()));
        assert_eq!(validate_pdf(b"%PDF-1.7\n%%EOF\n  \t"), Ok(()));
        assert_eq!(
            validate_pdf(b"%PDF-1.7\n%%EOF\nunexpected"),
            Err(FileAttachmentError::InvalidPdf)
        );
    }
}
