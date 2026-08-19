#![allow(dead_code, unused_imports)]

use super::*;

#[utoipa::path(
    post,
    path = "/api/v1/attachments/uploads",
    tag = "file-attachment",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateFileUploadRequest,
    responses(
        (status = 200, body = FileUploadSessionResponse),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn create_file_upload_session() {}

#[utoipa::path(
    put,
    path = "/api/v1/attachments/uploads/{upload_id}/content",
    tag = "file-attachment",
    params(
        ("upload_id" = uuid::Uuid, Path, description = "上传会话 ID"),
        ("token" = String, Query, description = "5 分钟上传令牌")
    ),
    request_body(content = Vec<u8>, content_type = "application/octet-stream"),
    responses(
        (status = 204, description = "附件字节已写入对象存储"),
        (status = 401, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 410, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn upload_file_content() {}

#[utoipa::path(
    post,
    path = "/api/v1/attachments/confirm",
    tag = "file-attachment",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = ConfirmFileUploadRequest,
    responses(
        (status = 200, body = FileAttachment),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse)
    )
)]
pub(crate) fn confirm_file_upload() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/image-previews",
    tag = "drug-inspection",
    request_body = CreateDrugInspectionImagePreviewRequest,
    responses(
        (status = 200, body = DrugInspectionImagePreviewResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn create_drug_inspection_image_preview() {}

#[utoipa::path(
    get,
    path = "/api/v1/attachments/{attachment_id}/url",
    tag = "file-attachment",
    params(("attachment_id" = uuid::Uuid, Path, description = "附件 ID")),
    responses(
        (status = 200, body = FileAttachmentDownloadUrlResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse)
    )
)]
pub(crate) fn create_file_download_url() {}

#[utoipa::path(
    get,
    path = "/api/v1/attachments/{attachment_id}/content",
    tag = "file-attachment",
    params(
        ("attachment_id" = uuid::Uuid, Path, description = "附件 ID"),
        ("download_id" = uuid::Uuid, Query, description = "下载授权 ID"),
        ("token" = String, Query, description = "15 分钟下载令牌")
    ),
    responses(
        (status = 200, description = "附件字节"),
        (status = 401, body = ErrorResponse),
        (status = 404, body = ErrorResponse)
    )
)]
pub(crate) fn download_file_content() {}
