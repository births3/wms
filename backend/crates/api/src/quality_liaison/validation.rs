use wms_domain::{
    CreateQualityLiaisonRequest, QualityLiaisonApprovalCallbackRequest,
    UpsertQualityLiaisonTypeRequest,
};

use super::QualityLiaisonError;

pub(super) fn normalize_type_request(
    request: UpsertQualityLiaisonTypeRequest,
) -> UpsertQualityLiaisonTypeRequest {
    UpsertQualityLiaisonTypeRequest {
        type_code: request.type_code.trim().to_ascii_lowercase(),
        type_name: request.type_name.trim().to_string(),
        approval_template_id: request.approval_template_id.trim().to_string(),
        ..request
    }
}

pub(super) fn validate_type_request(
    request: &UpsertQualityLiaisonTypeRequest,
) -> Result<(), QualityLiaisonError> {
    let code = request.type_code.trim();
    let mut code_bytes = code.bytes();
    let valid_code = code_bytes
        .next()
        .is_some_and(|value| value.is_ascii_lowercase())
        && code_bytes
            .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'_');
    if code.len() < 2
        || code.len() > 64
        || !valid_code
        || request.type_name.trim().is_empty()
        || request.approval_template_id.trim().is_empty()
        || request.timeout_seconds <= 0
    {
        return Err(QualityLiaisonError::InvalidRequest);
    }
    Ok(())
}

pub(super) fn normalize_create_request(
    request: CreateQualityLiaisonRequest,
) -> CreateQualityLiaisonRequest {
    CreateQualityLiaisonRequest {
        type_code: request.type_code.trim().to_ascii_lowercase(),
        related_document_type: request.related_document_type.trim().to_string(),
        related_document_no: request.related_document_no.trim().to_string(),
        problem_description: request.problem_description.trim().to_string(),
        disposition_suggestion: request.disposition_suggestion.trim().to_string(),
        trigger_source: request.trigger_source.trim().to_string(),
        business_payload: request.business_payload,
    }
}

pub(super) fn validate_create_request(
    request: &CreateQualityLiaisonRequest,
) -> Result<(), QualityLiaisonError> {
    if [
        request.type_code.as_str(),
        request.related_document_type.as_str(),
        request.related_document_no.as_str(),
        request.problem_description.as_str(),
        request.disposition_suggestion.as_str(),
        request.trigger_source.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
        || !request.business_payload.is_object()
    {
        return Err(QualityLiaisonError::InvalidRequest);
    }
    Ok(())
}

pub(super) fn normalize_approval_request(
    request: QualityLiaisonApprovalCallbackRequest,
) -> Result<QualityLiaisonApprovalCallbackRequest, QualityLiaisonError> {
    let opinion = request.opinion.trim();
    if opinion.is_empty() {
        return Err(QualityLiaisonError::ApprovalOpinionRequired);
    }
    let external_approval_id = request.external_approval_id.trim();
    if external_approval_id.is_empty() {
        return Err(QualityLiaisonError::InvalidRequest);
    }
    Ok(QualityLiaisonApprovalCallbackRequest {
        conclusion: request.conclusion.trim().to_ascii_lowercase(),
        opinion: opinion.to_string(),
        external_approval_id: external_approval_id.to_string(),
    })
}
