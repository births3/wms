use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;
use wms_domain::{
    ConfirmFileUploadRequest, CreateDrugInspectionImagePreviewRequest, CreateFileUploadRequest,
    DrugInspectionImagePreviewResponse, ErrorResponse, FileAttachmentValidationError,
    H_FILE_MAX_SIZE_BYTES,
};

use crate::{
    auth::{AuthContext, AuthError},
    drug_inspection_copy_processor::{
        decode_mdi_image, generate_image_preview, validate_transparent_stamp,
    },
    file_attachment_repository::{FileAttachmentRepositoryError, PgFileAttachmentRepository},
};

pub const H_FILE_READ_PERMISSION: &str = "h-file.attachment.read";
pub const H_FILE_WRITE_PERMISSION: &str = "h-file.attachment.write";

#[derive(Clone)]
pub struct FileAttachmentAppState {
    repository: Arc<PgFileAttachmentRepository>,
    storage_root: Arc<PathBuf>,
}

impl FileAttachmentAppState {
    pub fn with_local_storage(pool: PgPool, storage_root: PathBuf) -> Self {
        Self {
            repository: Arc::new(PgFileAttachmentRepository::new(pool)),
            storage_root: Arc::new(storage_root),
        }
    }
}

pub fn file_attachment_router(state: FileAttachmentAppState) -> Router {
    Router::new()
        .route("/api/v1/attachments/uploads", post(create_upload_session))
        .route(
            "/api/v1/attachments/uploads/:upload_id/content",
            put(upload_content),
        )
        .route("/api/v1/attachments/confirm", post(confirm_upload))
        .route(
            "/api/v1/drug-inspection/image-previews",
            post(create_drug_inspection_image_preview),
        )
        .route(
            "/api/v1/attachments/:attachment_id/url",
            get(create_download_url),
        )
        .route(
            "/api/v1/attachments/:attachment_id/content",
            get(download_content),
        )
        .layer(DefaultBodyLimit::max(H_FILE_MAX_SIZE_BYTES as usize))
        .with_state(state)
}

async fn create_upload_session(
    ctx: AuthContext,
    State(state): State<FileAttachmentAppState>,
    headers: HeaderMap,
    Json(request): Json<CreateFileUploadRequest>,
) -> Result<Json<wms_domain::FileUploadSessionResponse>, FileAttachmentHandlerError> {
    ctx.require_permission(H_FILE_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    state
        .repository
        .create_upload_session(&ctx, request, &idempotency_key)
        .await
        .map(Json)
        .map_err(Into::into)
}

#[derive(Deserialize)]
struct UploadTokenQuery {
    token: String,
}

async fn upload_content(
    State(state): State<FileAttachmentAppState>,
    Path(upload_id): Path<Uuid>,
    Query(query): Query<UploadTokenQuery>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<StatusCode, FileAttachmentHandlerError> {
    let content_type = header_value(&headers, header::CONTENT_TYPE)?;
    let size_bytes =
        i64::try_from(bytes.len()).map_err(|_| FileAttachmentHandlerError::InvalidBody)?;
    let target = state
        .repository
        .authorize_upload(upload_id, &query.token, content_type, size_bytes)
        .await?;
    if target.module == "M-DI" {
        match target.content_type.as_str() {
            "image/jpeg" => {
                decode_mdi_image(&bytes, image::ImageFormat::Jpeg)
                    .map_err(|_| FileAttachmentHandlerError::InvalidImage)?;
            }
            "image/png" if target.entity_type == "drug_inspection_stamp" => {
                validate_transparent_stamp(&bytes)
                    .map_err(|_| FileAttachmentHandlerError::InvalidStamp)?;
            }
            "image/png" => {
                decode_mdi_image(&bytes, image::ImageFormat::Png)
                    .map_err(|_| FileAttachmentHandlerError::InvalidImage)?;
            }
            _ => {}
        }
    }
    let path = resolve_storage_path(&state.storage_root, &target.storage_key)?;
    let parent = path
        .parent()
        .ok_or(FileAttachmentHandlerError::StorageFailed)?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|_| FileAttachmentHandlerError::StorageFailed)?;
    let temporary_path = path.with_extension(format!(
        "{}.part",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("upload")
    ));
    tokio::fs::write(&temporary_path, &bytes)
        .await
        .map_err(|_| FileAttachmentHandlerError::StorageFailed)?;
    tokio::fs::rename(&temporary_path, &path)
        .await
        .map_err(|_| FileAttachmentHandlerError::StorageFailed)?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    if let Err(error) = state
        .repository
        .mark_uploaded(upload_id, &query.token, size_bytes, &sha256)
        .await
    {
        let _ = tokio::fs::remove_file(path).await;
        return Err(error.into());
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn confirm_upload(
    ctx: AuthContext,
    State(state): State<FileAttachmentAppState>,
    headers: HeaderMap,
    Json(request): Json<ConfirmFileUploadRequest>,
) -> Result<Json<wms_domain::FileAttachment>, FileAttachmentHandlerError> {
    ctx.require_permission(H_FILE_WRITE_PERMISSION)?;
    let idempotency_key = idempotency_key(&headers)?;
    state
        .repository
        .confirm_upload(&ctx, request, &idempotency_key)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn create_download_url(
    ctx: AuthContext,
    State(state): State<FileAttachmentAppState>,
    Path(attachment_id): Path<Uuid>,
) -> Result<Json<wms_domain::FileAttachmentDownloadUrlResponse>, FileAttachmentHandlerError> {
    ctx.require_permission(H_FILE_READ_PERMISSION)?;
    state
        .repository
        .create_download_url(&ctx, attachment_id)
        .await
        .map(Json)
        .map_err(Into::into)
}

async fn create_drug_inspection_image_preview(
    ctx: AuthContext,
    State(state): State<FileAttachmentAppState>,
    Json(request): Json<CreateDrugInspectionImagePreviewRequest>,
) -> Result<Json<DrugInspectionImagePreviewResponse>, FileAttachmentHandlerError> {
    ctx.require_permission("m-di.document.write")?;
    let target = state
        .repository
        .drug_inspection_image_target(ctx.owner_id, request.attachment_id)
        .await?;
    let bytes = tokio::fs::read(resolve_storage_path(
        &state.storage_root,
        &target.storage_key,
    )?)
    .await
    .map_err(|_| FileAttachmentHandlerError::StorageFailed)?;
    let content_type = target.content_type;
    let processing_mode = request.processing_mode;
    let (bytes, width, height) = tokio::task::spawn_blocking(move || {
        generate_image_preview(&bytes, &content_type, &processing_mode)
    })
    .await
    .map_err(|_| FileAttachmentHandlerError::PreviewFailed)?
    .map_err(|_| FileAttachmentHandlerError::PreviewFailed)?;
    Ok(Json(DrugInspectionImagePreviewResponse {
        content_type: "image/png".to_string(),
        data_base64: BASE64_STANDARD.encode(bytes),
        width,
        height,
    }))
}

#[derive(Deserialize)]
struct DownloadTokenQuery {
    download_id: Uuid,
    token: String,
}

async fn download_content(
    State(state): State<FileAttachmentAppState>,
    Path(attachment_id): Path<Uuid>,
    Query(query): Query<DownloadTokenQuery>,
) -> Result<Response, FileAttachmentHandlerError> {
    let target = state
        .repository
        .authorize_download(attachment_id, query.download_id, &query.token)
        .await?;
    let file = tokio::fs::File::open(resolve_storage_path(
        &state.storage_root,
        &target.storage_key,
    )?)
    .await
    .map_err(|_| FileAttachmentHandlerError::StorageFailed)?;
    let content_length = file
        .metadata()
        .await
        .map_err(|_| FileAttachmentHandlerError::StorageFailed)?
        .len();
    let content_type = HeaderValue::from_str(&target.content_type)
        .map_err(|_| FileAttachmentHandlerError::StorageFailed)?;
    let file_name = target
        .file_name
        .chars()
        .filter(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_'))
        .collect::<String>();
    let disposition = HeaderValue::from_str(&format!(
        "attachment; filename=\"{}\"",
        if file_name.is_empty() {
            "attachment"
        } else {
            &file_name
        }
    ))
    .map_err(|_| FileAttachmentHandlerError::StorageFailed)?;
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CONTENT_LENGTH, HeaderValue::from(content_length)),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        axum::body::Body::from_stream(tokio_util::io::ReaderStream::new(file)),
    )
        .into_response())
}

fn idempotency_key(headers: &HeaderMap) -> Result<String, FileAttachmentHandlerError> {
    headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 200)
        .map(str::to_string)
        .ok_or(FileAttachmentHandlerError::IdempotencyRequired)
}

/// 拒绝绝对路径与任何非普通分量（`..`、`.`、盘符等），storage_key 只能落在存储根内。
fn resolve_storage_path(
    root: &std::path::Path,
    storage_key: &str,
) -> Result<std::path::PathBuf, FileAttachmentHandlerError> {
    let relative = std::path::Path::new(storage_key);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(FileAttachmentHandlerError::StorageFailed);
    }
    Ok(root.join(relative))
}

#[cfg(test)]
mod resolve_storage_path_tests {
    use super::resolve_storage_path;
    use std::path::Path;

    #[test]
    fn accepts_nested_normal_components() {
        let path = resolve_storage_path(Path::new("/var/wms"), "M-DI/entity/file.pdf")
            .unwrap_or_else(|_| panic!("nested key should resolve"));
        assert_eq!(path, Path::new("/var/wms/M-DI/entity/file.pdf"));
    }

    #[test]
    fn rejects_empty_absolute_and_parent_components() {
        let root = Path::new("/var/wms");
        assert!(resolve_storage_path(root, "").is_err());
        assert!(resolve_storage_path(root, "/etc/passwd").is_err());
        assert!(resolve_storage_path(root, "../secret").is_err());
        assert!(resolve_storage_path(root, "a/../b").is_err());
        // Path 会折叠 `./`，故 `a/./b` 等价于安全相对路径，不单独拒绝。
        assert!(resolve_storage_path(root, "a/./b").is_ok());
    }
}

fn header_value(
    headers: &HeaderMap,
    name: header::HeaderName,
) -> Result<&str, FileAttachmentHandlerError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(FileAttachmentHandlerError::InvalidBody)
}

enum FileAttachmentHandlerError {
    Auth(AuthError),
    IdempotencyRequired,
    InvalidBody,
    InvalidImage,
    InvalidStamp,
    PreviewFailed,
    StorageFailed,
    Repository(FileAttachmentRepositoryError),
}

impl From<AuthError> for FileAttachmentHandlerError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<FileAttachmentRepositoryError> for FileAttachmentHandlerError {
    fn from(value: FileAttachmentRepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl IntoResponse for FileAttachmentHandlerError {
    fn into_response(self) -> Response {
        if let Self::Auth(error) = self {
            return error.into_response();
        }
        let (status, code, message) = match self {
            Self::IdempotencyRequired => (
                StatusCode::BAD_REQUEST,
                "H_FILE_IDEMPOTENCY_REQUIRED",
                "缺少或非法 Idempotency-Key",
            ),
            Self::InvalidBody => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H_FILE_BODY_INVALID",
                "附件正文或 Content-Type 非法",
            ),
            Self::InvalidImage => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H_FILE_MDI_IMAGE_INVALID",
                "药检图片无法解码，或超过 5000 万像素/单边 12000 像素",
            ),
            Self::InvalidStamp => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H_FILE_MDI_STAMP_INVALID",
                "药检图章必须是带透明通道的有效 PNG",
            ),
            Self::PreviewFailed => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "M_DI_IMAGE_PREVIEW_FAILED",
                "药检图片预览生成失败，请检查文件或处理方式",
            ),
            Self::StorageFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H_FILE_STORAGE_FAILED",
                "附件存储读写失败",
            ),
            Self::Repository(FileAttachmentRepositoryError::Invalid(error)) => {
                validation_error(error)
            }
            Self::Repository(FileAttachmentRepositoryError::NotFound) => (
                StatusCode::NOT_FOUND,
                "H_FILE_ATTACHMENT_NOT_FOUND",
                "附件或临时授权不存在",
            ),
            Self::Repository(FileAttachmentRepositoryError::UnauthorizedToken) => (
                StatusCode::UNAUTHORIZED,
                "H_FILE_TEMPORARY_TOKEN_INVALID",
                "附件临时授权令牌无效",
            ),
            Self::Repository(FileAttachmentRepositoryError::UploadExpired) => {
                (StatusCode::GONE, "H_FILE_UPLOAD_EXPIRED", "上传授权已过期")
            }
            Self::Repository(FileAttachmentRepositoryError::UploadNotCompleted) => (
                StatusCode::CONFLICT,
                "H_FILE_UPLOAD_NOT_COMPLETED",
                "附件尚未上传完成或已确认",
            ),
            Self::Repository(FileAttachmentRepositoryError::UploadMetadataMismatch) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "H_FILE_UPLOAD_METADATA_MISMATCH",
                "附件类型或大小与上传申请不一致",
            ),
            Self::Repository(FileAttachmentRepositoryError::IdempotencyConflict) => (
                StatusCode::CONFLICT,
                "H_FILE_IDEMPOTENCY_CONFLICT",
                "幂等键已用于不同请求",
            ),
            Self::Repository(FileAttachmentRepositoryError::Audit(_))
            | Self::Repository(FileAttachmentRepositoryError::Database(_))
            | Self::Repository(FileAttachmentRepositoryError::Serialize(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "H_FILE_PERSISTENCE_FAILED",
                "附件元数据或审计持久化失败",
            ),
            Self::Auth(_) => unreachable!("auth error returned above"),
        };
        (
            status,
            Json(ErrorResponse {
                code: code.to_string(),
                message: message.to_string(),
                severity: "error".to_string(),
                details: serde_json::json!({}),
                trace_id: "unavailable".to_string(),
                retry_hint: None,
            }),
        )
            .into_response()
    }
}

fn validation_error(
    error: FileAttachmentValidationError,
) -> (StatusCode, &'static str, &'static str) {
    match error {
        FileAttachmentValidationError::FieldRequired(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H_FILE_FIELD_REQUIRED",
            "附件元数据必填字段缺失",
        ),
        FileAttachmentValidationError::FieldTooLong(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H_FILE_FIELD_TOO_LONG",
            "附件元数据字段超长",
        ),
        FileAttachmentValidationError::FieldInvalidCharacters(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H_FILE_FIELD_INVALID_CHARACTERS",
            "附件模块与实体类型只允许字母数字、连字符与下划线",
        ),
        FileAttachmentValidationError::UnsupportedContentType => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H_FILE_CONTENT_TYPE_UNSUPPORTED",
            "附件类型不在白名单",
        ),
        FileAttachmentValidationError::InvalidSize => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H_FILE_SIZE_INVALID",
            "附件大小必须在 1 字节至 50MB 之间",
        ),
        FileAttachmentValidationError::MdiImageTooLarge => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H_FILE_M_DI_IMAGE_TOO_LARGE",
            "药检单 JPG/PNG 不得超过 5MB",
        ),
        FileAttachmentValidationError::MdiUpstreamFileTooLarge => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "H_FILE_M_DI_UPSTREAM_FILE_TOO_LARGE",
            "上游随货同行单每个文件不得超过 5MB",
        ),
    }
}
