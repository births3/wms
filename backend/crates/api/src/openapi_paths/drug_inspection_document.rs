#![allow(dead_code, unused_imports)]

use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/inbound-documents",
    tag = "drug-inspection",
    params(
        ("received_from" = Option<chrono::NaiveDate>, Query, description = "实际收货开始日期"),
        ("received_to" = Option<chrono::NaiveDate>, Query, description = "实际收货结束日期"),
        ("missing_drug_inspection" = Option<bool>, Query, description = "仅药检单不齐"),
        ("missing_upstream_delivery" = Option<bool>, Query, description = "仅上游随货同行单不齐"),
        ("page" = Option<u32>, Query, description = "页码，从 1 开始；缺省为 1"),
        ("page_size" = Option<u32>, Query, description = "每页条数；缺省为 20，上限 200")
    ),
    responses(
        (status = 200, body = InboundDocumentEntryListResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub(crate) fn list_inbound_documents() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/reports/reusable",
    tag = "drug-inspection",
    params(
        ("product_id" = uuid::Uuid, Query, description = "商品 ID"),
        ("batch_no" = String, Query, description = "批号"),
        ("asn_id" = Option<uuid::Uuid>, Query, description = "用于判断是否已关联的 ASN ID")
    ),
    responses(
        (status = 200, body = ReusableDrugInspectionReportResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse)
    )
)]
pub(crate) fn find_reusable_drug_inspection_report() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/review-queue",
    tag = "drug-inspection",
    params(
        ("page" = Option<u32>, Query, description = "页码，从 1 开始；缺省为 1"),
        ("page_size" = Option<u32>, Query, description = "每页条数；缺省为 20，上限 200")
    ),
    responses(
        (status = 200, body = ReviewQueueListResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub(crate) fn list_drug_inspection_review_queue() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/reports/{report_id}/versions",
    tag = "drug-inspection",
    params(("report_id" = uuid::Uuid, Path, description = "药检报告根 ID")),
    responses(
        (status = 200, body = [DrugInspectionReportVersion]),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub(crate) fn list_drug_inspection_report_versions() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/report-versions",
    tag = "drug-inspection",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateDrugInspectionVersionRequest,
    responses(
        (status = 200, body = DrugInspectionReportVersion),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn create_drug_inspection_version() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/report-versions/editable",
    tag = "drug-inspection",
    params(
        ("asn_id" = uuid::Uuid, Query, description = "当前 ASN ID"),
        ("product_id" = uuid::Uuid, Query, description = "商品 ID"),
        ("batch_no" = String, Query, description = "批号")
    ),
    responses(
        (status = 200, body = DrugInspectionReportVersion),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse)
    )
)]
pub(crate) fn find_editable_drug_inspection_version() {}

#[utoipa::path(
    put,
    path = "/api/v1/drug-inspection/report-versions/{version_id}",
    tag = "drug-inspection",
    params(
        ("version_id" = uuid::Uuid, Path, description = "可编辑的药检单草稿版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = UpdateDrugInspectionDraftRequest,
    responses(
        (status = 200, body = DrugInspectionReportVersion),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn update_drug_inspection_draft() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/report-versions/{version_id}/submit",
    tag = "drug-inspection",
    params(
        ("version_id" = uuid::Uuid, Path, description = "药检单版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, body = DrugInspectionReportVersion),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse)
    )
)]
pub(crate) fn submit_drug_inspection_version() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/report-versions/{version_id}/review",
    tag = "drug-inspection",
    params(
        ("version_id" = uuid::Uuid, Path, description = "药检单版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = ReviewDrugInspectionVersionRequest,
    responses(
        (status = 200, body = DrugInspectionReportVersion),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn review_drug_inspection_version() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/reports/{report_id}/corrections",
    tag = "drug-inspection",
    params(
        ("report_id" = uuid::Uuid, Path, description = "药检报告根 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = CreateDrugInspectionCorrectionRequest,
    responses(
        (status = 200, body = DrugInspectionReportVersion),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn create_drug_inspection_correction() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/reports/{report_id}/reuse",
    tag = "drug-inspection",
    params(
        ("report_id" = uuid::Uuid, Path, description = "药检报告根 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = ReuseDrugInspectionReportRequest,
    responses(
        (status = 200, body = ReuseDrugInspectionReportResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
        (status = 409, body = ErrorResponse)
    )
)]
pub(crate) fn reuse_drug_inspection_report() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/upstream-delivery-document-versions",
    tag = "drug-inspection",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateUpstreamDeliveryVersionRequest,
    responses(
        (status = 200, body = UpstreamDeliveryDocumentVersion),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn create_upstream_delivery_document_version() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/upstream-delivery-documents/{document_id}/versions",
    tag = "drug-inspection",
    params(("document_id" = uuid::Uuid, Path, description = "上游随货同行单 ID")),
    responses(
        (status = 200, body = [UpstreamDeliveryDocumentVersion]),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub(crate) fn list_upstream_delivery_document_versions() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/requirement-rules",
    tag = "drug-inspection",
    responses(
        (status = 200, body = [DrugInspectionRequirementRule]),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub(crate) fn list_drug_inspection_requirement_rules() {}

#[utoipa::path(
    put,
    path = "/api/v1/drug-inspection/requirement-rules/current",
    tag = "drug-inspection",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = UpsertDrugInspectionRequirementRuleRequest,
    responses(
        (status = 200, body = DrugInspectionRequirementRule),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn upsert_drug_inspection_requirement_rule() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/stamp-versions",
    tag = "drug-inspection",
    responses(
        (status = 200, body = [DrugInspectionStampVersion]),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub(crate) fn list_drug_inspection_stamp_versions() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/stamp-versions",
    tag = "drug-inspection",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = CreateDrugInspectionStampVersionRequest,
    responses(
        (status = 200, body = DrugInspectionStampVersion),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn create_drug_inspection_stamp_version() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/stamp-versions/{version_id}/submit",
    tag = "drug-inspection",
    params(
        ("version_id" = uuid::Uuid, Path, description = "图章版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    responses(
        (status = 200, body = DrugInspectionStampVersion),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse)
    )
)]
pub(crate) fn submit_drug_inspection_stamp_version() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/stamp-versions/{version_id}/review",
    tag = "drug-inspection",
    params(
        ("version_id" = uuid::Uuid, Path, description = "图章版本 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = ReviewDrugInspectionStampVersionRequest,
    responses(
        (status = 200, body = DrugInspectionStampVersion),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn review_drug_inspection_stamp_version() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/customer-copy-jobs",
    tag = "drug-inspection",
    params(
        ("page" = Option<u32>, Query, description = "页码，从 1 开始；缺省为 1"),
        ("page_size" = Option<u32>, Query, description = "每页条数；缺省为 20，上限 200")
    ),
    responses(
        (status = 200, body = CopyJobListResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub(crate) fn list_drug_inspection_copy_jobs() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/customer-copy-jobs/{job_id}/process",
    tag = "drug-inspection",
    params(("job_id" = uuid::Uuid, Path, description = "客户副本任务 ID")),
    responses(
        (status = 200, body = DrugInspectionCustomerCopyJob),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn process_drug_inspection_copy_job() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/customer-copy-jobs/{job_id}/oversize-approval",
    tag = "drug-inspection",
    params(
        ("job_id" = uuid::Uuid, Path, description = "客户副本任务 ID"),
        ("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")
    ),
    request_body = ApproveDrugInspectionCopyOversizeRequest,
    responses(
        (status = 200, body = DrugInspectionCustomerCopyJob),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 409, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn approve_drug_inspection_copy_oversize() {}

#[utoipa::path(
    get,
    path = "/api/v1/drug-inspection/processing-rule-versions",
    tag = "drug-inspection",
    responses(
        (status = 200, body = [DrugInspectionProcessingRuleVersion]),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse)
    )
)]
pub(crate) fn list_drug_inspection_processing_rule_versions() {}

#[utoipa::path(
    post,
    path = "/api/v1/drug-inspection/processing-rule-versions",
    tag = "drug-inspection",
    params(("Idempotency-Key" = String, Header, description = "客户端生成的幂等键")),
    request_body = PublishDrugInspectionProcessingRuleRequest,
    responses(
        (status = 200, body = DrugInspectionProcessingRuleVersion),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 422, body = ErrorResponse)
    )
)]
pub(crate) fn publish_drug_inspection_processing_rule_version() {}
