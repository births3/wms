use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::PageMeta;

pub const DRUG_INSPECTION_PROCESSING_MODES: [&str; 3] =
    ["none", "color_enhance", "black_white_enhance"];

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InboundDocumentEntry {
    pub asn_id: Uuid,
    pub receipt_no: String,
    pub purchase_order_no: String,
    pub owner_id: Uuid,
    pub supplier_id: Uuid,
    pub supplier_name: String,
    pub product_id: Uuid,
    pub product_code: String,
    pub product_name: String,
    pub batch_nos: Vec<String>,
    pub actual_received_at: Option<DateTime<Utc>>,
    pub drug_inspection_status: String,
    pub drug_inspection_version: i32,
    pub upstream_delivery_status: String,
    pub upstream_version: i32,
    pub upstream_document_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct InboundDocumentEntryListResponse {
    pub data: Vec<InboundDocumentEntry>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateDrugInspectionVersionRequest {
    pub asn_id: Uuid,
    pub product_id: Uuid,
    pub batch_no: String,
    pub report_no: String,
    pub original_file_id: Uuid,
    pub source: String,
    pub processing_mode: String,
    pub qualified: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateDrugInspectionCorrectionRequest {
    pub report_no: String,
    pub original_file_id: Uuid,
    pub processing_mode: String,
    pub qualified: bool,
    pub modification_reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateDrugInspectionDraftRequest {
    pub report_no: String,
    pub original_file_id: Uuid,
    pub processing_mode: String,
    pub qualified: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReviewDrugInspectionVersionRequest {
    pub decision: String,
    pub comment: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReuseDrugInspectionReportRequest {
    pub asn_id: Uuid,
    pub batch_no: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReuseDrugInspectionReportResponse {
    pub report_id: Uuid,
    pub asn_id: Uuid,
    pub batch_no: String,
    pub source_version_id: Uuid,
    pub linked_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReusableDrugInspectionReportResponse {
    pub report_id: Uuid,
    pub current_version_id: Uuid,
    pub version_number: i32,
    pub report_no: String,
    pub linked_to_asn: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionReportVersion {
    pub id: Uuid,
    pub report_id: Uuid,
    pub owner_id: Uuid,
    pub version_number: i32,
    pub report_no: String,
    pub original_file_id: Uuid,
    pub original_file_hash: String,
    pub source: String,
    pub processing_mode: String,
    pub qualified: bool,
    pub status: String,
    pub replaces_version_id: Option<Uuid>,
    pub modification_reason: Option<String>,
    pub uploaded_by: Uuid,
    pub submitted_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_result: Option<String>,
    pub review_comment: Option<String>,
    pub customer_copy_status: String,
    pub customer_copy_file_id: Option<Uuid>,
    pub customer_copy_hash: Option<String>,
    pub stamp_version_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionReviewQueueEntry {
    pub version: DrugInspectionReportVersion,
    pub product_code: String,
    pub product_name: String,
    pub batch_no: String,
    pub uploader_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateDrugInspectionStampVersionRequest {
    pub png_attachment_id: Uuid,
    pub relative_x: f64,
    pub relative_y: f64,
    pub relative_width: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReviewDrugInspectionStampVersionRequest {
    pub decision: String,
    pub comment: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionStampVersion {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub version_number: i32,
    pub png_attachment_id: Uuid,
    pub relative_x: f64,
    pub relative_y: f64,
    pub relative_width: f64,
    pub status: String,
    pub configured_by: Uuid,
    pub submitted_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionCustomerCopyJob {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub report_version_id: Uuid,
    pub status: String,
    pub attempt_count: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApproveDrugInspectionCopyOversizeRequest {
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PublishDrugInspectionProcessingRuleRequest {
    pub apply_scope: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionProcessingRuleVersion {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub version_number: i32,
    pub rule_code: String,
    pub apply_scope: String,
    pub reprocess_job_count: i32,
    pub published_by: Uuid,
    pub published_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct DrugInspectionRequirementRule {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub special_drug_category: String,
    pub missing_behavior: String,
    pub enabled: bool,
    pub version: i64,
    pub updated_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpsertDrugInspectionRequirementRuleRequest {
    pub special_drug_category: String,
    pub missing_behavior: String,
    pub enabled: bool,
}

impl UpsertDrugInspectionRequirementRuleRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        if self.special_drug_category.trim().is_empty()
            || !matches!(self.missing_behavior.as_str(), "warning" | "block")
        {
            return Err(DrugInspectionDocumentValidationError::InvalidMissingBehavior);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateUpstreamDeliveryVersionRequest {
    pub document_id: Option<Uuid>,
    pub supplier_id: Uuid,
    pub asn_ids: Vec<Uuid>,
    pub attachment_ids: Vec<Uuid>,
    pub modification_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpstreamDeliveryDocumentVersion {
    pub id: Uuid,
    pub document_id: Uuid,
    pub owner_id: Uuid,
    pub version_number: i32,
    pub modification_reason: Option<String>,
    pub attachment_ids: Vec<Uuid>,
    pub asn_ids: Vec<Uuid>,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DrugInspectionDocumentValidationError {
    FieldRequired(&'static str),
    FieldTooLong(&'static str),
    InvalidSource,
    InvalidProcessingMode,
    InvalidDecision,
    ReviewCommentRequired,
    ModificationReasonRequired,
    EmptyAsnSelection,
    EmptyAttachmentSelection,
    InvalidStampGeometry,
    InvalidMissingBehavior,
}

impl CreateDrugInspectionVersionRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        validate_text(&self.batch_no, "batch_no", 128)?;
        validate_text(&self.report_no, "report_no", 128)?;
        if !matches!(self.source.trim(), "manual_upload" | "upstream_platform") {
            return Err(DrugInspectionDocumentValidationError::InvalidSource);
        }
        validate_processing_mode(&self.processing_mode)
    }
}

impl CreateDrugInspectionCorrectionRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        validate_text(&self.report_no, "report_no", 128)?;
        validate_text(&self.modification_reason, "modification_reason", 500)?;
        validate_processing_mode(&self.processing_mode)
    }
}

impl UpdateDrugInspectionDraftRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        validate_text(&self.report_no, "report_no", 128)?;
        validate_processing_mode(&self.processing_mode)
    }
}

impl ReviewDrugInspectionVersionRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        if !matches!(self.decision.trim(), "confirmed" | "rejected") {
            return Err(DrugInspectionDocumentValidationError::InvalidDecision);
        }
        if self.decision.trim() == "rejected"
            && self
                .comment
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(DrugInspectionDocumentValidationError::ReviewCommentRequired);
        }
        if self
            .comment
            .as_deref()
            .is_some_and(|value| value.chars().count() > 500)
        {
            return Err(DrugInspectionDocumentValidationError::FieldTooLong(
                "comment",
            ));
        }
        Ok(())
    }
}

impl CreateUpstreamDeliveryVersionRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        if self.asn_ids.is_empty() {
            return Err(DrugInspectionDocumentValidationError::EmptyAsnSelection);
        }
        if self.attachment_ids.is_empty() {
            return Err(DrugInspectionDocumentValidationError::EmptyAttachmentSelection);
        }
        if self.document_id.is_some()
            && self
                .modification_reason
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(DrugInspectionDocumentValidationError::ModificationReasonRequired);
        }
        if self
            .modification_reason
            .as_deref()
            .is_some_and(|value| value.chars().count() > 500)
        {
            return Err(DrugInspectionDocumentValidationError::FieldTooLong(
                "modification_reason",
            ));
        }
        Ok(())
    }
}

impl CreateDrugInspectionStampVersionRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        let finite = self.relative_x.is_finite()
            && self.relative_y.is_finite()
            && self.relative_width.is_finite();
        let in_bounds = (0.0..=1.0).contains(&self.relative_x)
            && (0.0..=1.0).contains(&self.relative_y)
            && self.relative_width > 0.0
            && self.relative_width <= 1.0
            && self.relative_x + self.relative_width <= 1.0;
        if finite && in_bounds {
            Ok(())
        } else {
            Err(DrugInspectionDocumentValidationError::InvalidStampGeometry)
        }
    }
}

impl ReviewDrugInspectionStampVersionRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        if !matches!(self.decision.trim(), "published" | "rejected") {
            return Err(DrugInspectionDocumentValidationError::InvalidDecision);
        }
        if self.decision.trim() == "rejected"
            && self
                .comment
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(DrugInspectionDocumentValidationError::ReviewCommentRequired);
        }
        Ok(())
    }
}

impl ApproveDrugInspectionCopyOversizeRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        validate_text(&self.reason, "reason", 500)
    }
}

impl PublishDrugInspectionProcessingRuleRequest {
    pub fn validate(&self) -> Result<(), DrugInspectionDocumentValidationError> {
        if !matches!(self.apply_scope.trim(), "future_only" | "reprocess_current") {
            return Err(DrugInspectionDocumentValidationError::InvalidProcessingMode);
        }
        Ok(())
    }
}

fn validate_processing_mode(value: &str) -> Result<(), DrugInspectionDocumentValidationError> {
    if DRUG_INSPECTION_PROCESSING_MODES.contains(&value.trim()) {
        Ok(())
    } else {
        Err(DrugInspectionDocumentValidationError::InvalidProcessingMode)
    }
}

fn validate_text(
    value: &str,
    field: &'static str,
    max_chars: usize,
) -> Result<(), DrugInspectionDocumentValidationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(DrugInspectionDocumentValidationError::FieldRequired(field));
    }
    if value.chars().count() > max_chars {
        return Err(DrugInspectionDocumentValidationError::FieldTooLong(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_geometry_must_stay_inside_the_page() {
        let mut request = CreateDrugInspectionStampVersionRequest {
            png_attachment_id: Uuid::new_v4(),
            relative_x: 0.7,
            relative_y: 0.75,
            relative_width: 0.2,
        };
        assert!(request.validate().is_ok());
        request.relative_width = 0.4;
        assert_eq!(
            request.validate(),
            Err(DrugInspectionDocumentValidationError::InvalidStampGeometry)
        );
    }

    #[test]
    fn stamp_rejection_and_oversize_approval_require_reasons() {
        assert_eq!(
            ReviewDrugInspectionStampVersionRequest {
                decision: "rejected".to_string(),
                comment: Some(" ".to_string()),
            }
            .validate(),
            Err(DrugInspectionDocumentValidationError::ReviewCommentRequired)
        );
        assert_eq!(
            ApproveDrugInspectionCopyOversizeRequest {
                reason: " ".to_string(),
            }
            .validate(),
            Err(DrugInspectionDocumentValidationError::FieldRequired(
                "reason"
            ))
        );
    }
}
