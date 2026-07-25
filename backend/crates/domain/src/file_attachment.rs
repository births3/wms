use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub const H_FILE_MAX_SIZE_BYTES: i64 = 50 * 1024 * 1024;
pub const H_FILE_M_DI_IMAGE_MAX_SIZE_BYTES: i64 = 5 * 1024 * 1024;
pub const H_FILE_CONTENT_TYPES: [&str; 5] = [
    "image/jpeg",
    "image/png",
    "application/pdf",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "text/csv",
];

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateFileUploadRequest {
    pub module: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FileUploadSessionResponse {
    pub upload_id: Uuid,
    pub upload_url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ConfirmFileUploadRequest {
    pub upload_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FileAttachment {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub module: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct FileAttachmentDownloadUrlResponse {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateDrugInspectionImagePreviewRequest {
    pub attachment_id: Uuid,
    pub processing_mode: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionImagePreviewResponse {
    pub content_type: String,
    pub data_base64: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileAttachmentValidationError {
    FieldRequired(&'static str),
    FieldTooLong(&'static str),
    UnsupportedContentType,
    InvalidSize,
    MdiImageTooLarge,
    MdiUpstreamFileTooLarge,
}

impl CreateFileUploadRequest {
    pub fn validate(&self) -> Result<(), FileAttachmentValidationError> {
        validate_text(&self.module, "module", 32)?;
        validate_text(&self.entity_type, "entity_type", 64)?;
        validate_text(&self.file_name, "file_name", 255)?;
        if !H_FILE_CONTENT_TYPES.contains(&self.content_type.trim()) {
            return Err(FileAttachmentValidationError::UnsupportedContentType);
        }
        if !(1..=H_FILE_MAX_SIZE_BYTES).contains(&self.size_bytes) {
            return Err(FileAttachmentValidationError::InvalidSize);
        }
        if self.module.trim() == "M-DI"
            && matches!(self.content_type.trim(), "image/jpeg" | "image/png")
            && self.size_bytes > H_FILE_M_DI_IMAGE_MAX_SIZE_BYTES
        {
            return Err(FileAttachmentValidationError::MdiImageTooLarge);
        }
        if self.module.trim() == "M-DI"
            && self.entity_type.trim() == "upstream_delivery_document"
            && self.size_bytes > H_FILE_M_DI_IMAGE_MAX_SIZE_BYTES
        {
            return Err(FileAttachmentValidationError::MdiUpstreamFileTooLarge);
        }
        Ok(())
    }
}

fn validate_text(
    value: &str,
    field: &'static str,
    max_chars: usize,
) -> Result<(), FileAttachmentValidationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(FileAttachmentValidationError::FieldRequired(field));
    }
    if value.chars().count() > max_chars {
        return Err(FileAttachmentValidationError::FieldTooLong(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(content_type: &str, size_bytes: i64) -> CreateFileUploadRequest {
        CreateFileUploadRequest {
            module: "M-DI".to_string(),
            entity_type: "drug_inspection".to_string(),
            entity_id: Uuid::new_v4(),
            file_name: "report.pdf".to_string(),
            content_type: content_type.to_string(),
            size_bytes,
        }
    }

    #[test]
    fn accepts_pdf_up_to_generic_limit_and_limits_mdi_images_to_five_megabytes() {
        assert!(request("application/pdf", H_FILE_MAX_SIZE_BYTES)
            .validate()
            .is_ok());
        assert_eq!(
            request("image/jpeg", H_FILE_M_DI_IMAGE_MAX_SIZE_BYTES + 1).validate(),
            Err(FileAttachmentValidationError::MdiImageTooLarge)
        );
    }

    #[test]
    fn rejects_empty_metadata_unsupported_type_and_invalid_size() {
        let mut value = request("application/pdf", 1);
        value.file_name = " ".to_string();
        assert_eq!(
            value.validate(),
            Err(FileAttachmentValidationError::FieldRequired("file_name"))
        );
        assert_eq!(
            request("application/octet-stream", 1).validate(),
            Err(FileAttachmentValidationError::UnsupportedContentType)
        );
        assert_eq!(
            request("application/pdf", 0).validate(),
            Err(FileAttachmentValidationError::InvalidSize)
        );
    }

    #[test]
    fn limits_every_upstream_delivery_file_to_five_megabytes() {
        let mut value = request("application/pdf", H_FILE_M_DI_IMAGE_MAX_SIZE_BYTES + 1);
        value.entity_type = "upstream_delivery_document".to_string();
        assert_eq!(
            value.validate(),
            Err(FileAttachmentValidationError::MdiUpstreamFileTooLarge)
        );
    }
}
