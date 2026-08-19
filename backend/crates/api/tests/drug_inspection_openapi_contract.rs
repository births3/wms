use utoipa::OpenApi;
use wms_api::ApiDoc;

#[test]
fn drug_inspection_routes_publish_security_idempotency_and_error_contracts() {
    let document = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI should serialize");
    let paths = document["paths"]
        .as_object()
        .expect("OpenAPI paths should be an object");

    for path in [
        "/api/v1/drug-inspection/reports/reusable",
        "/api/v1/drug-inspection/review-queue",
        "/api/v1/drug-inspection/reports/{report_id}/versions",
        "/api/v1/drug-inspection/upstream-delivery-documents/{document_id}/versions",
        "/api/v1/drug-inspection/requirement-rules",
        "/api/v1/drug-inspection/requirement-rules/current",
        "/api/v1/drug-inspection/stamp-versions",
        "/api/v1/drug-inspection/stamp-versions/{version_id}/submit",
        "/api/v1/drug-inspection/stamp-versions/{version_id}/review",
        "/api/v1/drug-inspection/customer-copy-jobs",
        "/api/v1/drug-inspection/customer-copy-jobs/{job_id}/process",
        "/api/v1/drug-inspection/customer-copy-jobs/{job_id}/oversize-approval",
        "/api/v1/drug-inspection/processing-rule-versions",
        "/api/v1/attachments/uploads/{upload_id}/content",
        "/api/v1/drug-inspection/image-previews",
        "/api/v1/attachments/{attachment_id}/content",
    ] {
        assert!(paths.contains_key(path), "OpenAPI should publish {path}");
    }

    for (path, method) in [
        ("/api/v1/attachments/uploads/{upload_id}/content", "put"),
        ("/api/v1/attachments/{attachment_id}/content", "get"),
    ] {
        let operation = &document["paths"][path][method];
        assert_eq!(operation["security"], serde_json::json!([]));
        assert!(operation["x-auth-exempt-reason"].is_string());
        assert!(operation["responses"]["401"].is_object());
    }

    for (path, method) in [
        ("/api/v1/attachments/uploads/{upload_id}/content", "put"),
        ("/api/v1/drug-inspection/image-previews", "post"),
        (
            "/api/v1/drug-inspection/customer-copy-jobs/{job_id}/process",
            "post",
        ),
    ] {
        assert!(
            document["paths"][path][method]["x-idempotency-exempt-reason"].is_string(),
            "{method} {path} should explain its idempotency semantics",
        );
    }
}
