// @governance: skip-page-size - 药检文档全生命周期集成测试聚合（上传/复核/复制任务/分页断言），拆分破坏场景链完整性。
use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, Request, StatusCode},
};
use chrono::{Duration, Utc};
use image::{DynamicImage, ImageBuffer, ImageEncoder, Rgba};
use printpdf::{PdfDocument, PdfParseOptions};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    drug_inspection_copy_service::{DrugInspectionCopyService, DrugInspectionCopyServiceError},
    drug_inspection_document_handlers::{
        drug_inspection_document_router, DrugInspectionDocumentAppState,
    },
    drug_inspection_document_repository::PgDrugInspectionDocumentRepository,
    drug_inspection_document_repository::PgDrugInspectionStampRepository,
    drug_inspection_stamp_handlers::{drug_inspection_stamp_router, DrugInspectionStampAppState},
};
use wms_domain::{
    ApproveDrugInspectionCopyOversizeRequest, CreateDrugInspectionStampVersionRequest,
    CreateDrugInspectionVersionRequest, PublishDrugInspectionProcessingRuleRequest,
    ReviewDrugInspectionStampVersionRequest, ReviewDrugInspectionVersionRequest,
};

fn context(owner_id: Uuid, user_id: Uuid, permissions: &[&str]) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: format!("di-test-{}", user_id.simple()),
        permissions: permissions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn json_request(
    method: &str,
    path: &str,
    ctx: AuthContext,
    key: Option<&str>,
    payload: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(CONTENT_TYPE, "application/json");
    if let Some(key) = key {
        builder = builder.header("Idempotency-Key", key);
    }
    let mut request = builder
        .body(Body::from(payload.to_string()))
        .expect("request should build");
    request.extensions_mut().insert(ctx);
    request
}

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response should read"),
    )
    .expect("response should be JSON")
}

struct Fixture {
    owner_id: Uuid,
    uploader_id: Uuid,
    reviewer_id: Uuid,
    product_id: Uuid,
    supplier_id: Uuid,
    first_asn_id: Uuid,
    second_asn_id: Uuid,
    first_attachment_id: Uuid,
    correction_attachment_id: Uuid,
    upstream_attachment_id: Uuid,
    stamp_attachment_id: Uuid,
}

async fn seed_fixture(pool: &PgPool) -> Fixture {
    let owner_id = Uuid::new_v4();
    let uploader_id = Uuid::new_v4();
    let reviewer_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    let supplier_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let first_asn_id = Uuid::new_v4();
    let second_asn_id = Uuid::new_v4();
    let first_attachment_id = Uuid::new_v4();
    let correction_attachment_id = Uuid::new_v4();
    let upstream_attachment_id = Uuid::new_v4();
    let stamp_attachment_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '药检测试货主')",
    )
    .bind(owner_id)
    .bind(format!("DI-{}", owner_id.simple()))
    .execute(pool)
    .await
    .expect("owner should seed");
    for user_id in [uploader_id, reviewer_id] {
        sqlx::query(
            "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, $3, 'test-hash', 'active')",
        )
        .bind(user_id)
        .bind(format!("di-{}", user_id.simple()))
        .bind(format!("DI user {}", user_id.simple()))
        .execute(pool)
        .await
        .expect("user should seed");
        sqlx::query(
            "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
        )
        .bind(user_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("binding should seed");
    }
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, status) VALUES ($1, $2, 'P-DI-001', '药检商品', '10mg', 'normal_10_30', 'active')",
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("product should seed");
    sqlx::query(
        "INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, status) VALUES ($1, $2, 'S-DI-001', '药检供应商', $3, 'active')",
    )
    .bind(supplier_id)
    .bind(owner_id)
    .bind(format!("USCC{}", &supplier_id.simple().to_string()[..14]))
    .execute(pool)
    .await
    .expect("supplier should seed");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, 'W-DI-001', '药检仓', 'pharma', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("warehouse should seed");

    for (asn_id, receipt_no, received_at, batches) in [
        (
            first_asn_id,
            "ASN-DI-001",
            now - Duration::days(1),
            vec!["BATCH-A", "BATCH-B"],
        ),
        (second_asn_id, "ASN-DI-002", now, vec!["BATCH-A"]),
    ] {
        sqlx::query(
            r#"
            INSERT INTO receiving_orders (
                id, owner_id, receipt_no, document_type, supplier_id, warehouse_id,
                external_ref, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'purchase_inbound', $4, $5, $6, 'received', $7, $7)
            "#,
        )
        .bind(asn_id)
        .bind(owner_id)
        .bind(receipt_no)
        .bind(supplier_id)
        .bind(warehouse_id)
        .bind(format!("PO-{receipt_no}"))
        .bind(received_at)
        .execute(pool)
        .await
        .expect("ASN should seed");
        for (index, batch_no) in batches.into_iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO receiving_order_lines (
                    id, receiving_order_id, owner_id, line_no, product_id,
                    product_code, expected_qty, batch_no, created_at
                )
                VALUES ($1, $2, $3, $4, $5, 'P-DI-001', 10, $6, $7)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(asn_id)
            .bind(owner_id)
            .bind(i32::try_from(index + 1).expect("line number should fit"))
            .bind(product_id)
            .bind(batch_no)
            .bind(received_at)
            .execute(pool)
            .await
            .expect("ASN batch should seed");
        }
        sqlx::query(
            r#"
            INSERT INTO receiving_order_receipts (
                id, receiving_order_id, owner_id, actual_qty, shortage_qty,
                rejected_qty, occurred_at, created_at
            )
            VALUES ($1, $2, $3, 10, 0, 0, $4, $4)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(asn_id)
        .bind(owner_id)
        .bind(received_at)
        .execute(pool)
        .await
        .expect("receipt should seed");
    }
    for (attachment_id, name, content_type) in [
        (first_attachment_id, "report-a.png", "image/png"),
        (
            correction_attachment_id,
            "report-a-v2.pdf",
            "application/pdf",
        ),
        (
            upstream_attachment_id,
            "upstream-delivery.pdf",
            "application/pdf",
        ),
        (stamp_attachment_id, "owner-stamp.png", "image/png"),
    ] {
        let entity_type = if attachment_id == stamp_attachment_id {
            "drug_inspection_stamp"
        } else {
            "drug_inspection_test"
        };
        sqlx::query(
            r#"
            INSERT INTO attachments (
                id, owner_id, module, entity_type, entity_id, file_name, content_type,
                size_bytes, storage_key, sha256, uploaded_by, created_at
            )
            VALUES ($1, $2, 'M-DI', $10, $3, $4, $5, 20, $6, $7, $8, $9)
            "#,
        )
        .bind(attachment_id)
        .bind(owner_id)
        .bind(Uuid::new_v4())
        .bind(name)
        .bind(content_type)
        .bind(format!("{owner_id}/M-DI/{attachment_id}"))
        .bind(format!("sha256-{attachment_id}"))
        .bind(uploader_id)
        .bind(now)
        .bind(entity_type)
        .execute(pool)
        .await
        .expect("attachment should seed");
    }

    Fixture {
        owner_id,
        uploader_id,
        reviewer_id,
        product_id,
        supplier_id,
        first_asn_id,
        second_asn_id,
        first_attachment_id,
        correction_attachment_id,
        upstream_attachment_id,
        stamp_attachment_id,
    }
}

fn png(width: u32, height: u32, alpha: u8) -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
        width,
        height,
        Rgba([180, 20, 40, alpha]),
    ))
    .to_rgba8();
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .expect("PNG should encode");
    bytes
}

async fn write_attachment_bytes(
    pool: &PgPool,
    storage_root: &std::path::Path,
    attachment_id: Uuid,
    bytes: &[u8],
) {
    let storage_key: String =
        sqlx::query_scalar("SELECT storage_key FROM attachments WHERE id = $1")
            .bind(attachment_id)
            .fetch_one(pool)
            .await
            .expect("storage key should query");
    let path = storage_root.join(storage_key);
    tokio::fs::create_dir_all(path.parent().expect("attachment parent should exist"))
        .await
        .expect("attachment parent should create");
    tokio::fs::write(path, bytes)
        .await
        .expect("attachment bytes should write");
}

include!("drug_inspection_documents_postgres/stamp_copy_cases.rs");
include!("drug_inspection_documents_postgres/requirement_rule.rs");

fn all_permissions() -> [&'static str; 3] {
    [
        "m-di.document.read",
        "m-di.document.write",
        "m-di.document.review",
    ]
}

#[sqlx::test(migrations = "../../migrations")]
async fn rejected_report_can_be_edited_by_its_uploader_and_resubmitted(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    let app = drug_inspection_document_router(DrugInspectionDocumentAppState::with_postgres(
        pool.clone(),
    ));
    let uploader = context(fixture.owner_id, fixture.uploader_id, &all_permissions());
    let reviewer = context(fixture.owner_id, fixture.reviewer_id, &all_permissions());

    let create = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/drug-inspection/report-versions",
            uploader.clone(),
            Some("di-reject-edit-create"),
            json!({
                "asn_id": fixture.first_asn_id,
                "product_id": fixture.product_id,
                "batch_no": "BATCH-A",
                "report_no": "REPORT-UNCLEAR",
                "original_file_id": fixture.first_attachment_id,
                "source": "manual_upload",
                "processing_mode": "none",
                "qualified": true
            }),
        ))
        .await
        .expect("create should respond");
    assert_eq!(create.status(), StatusCode::OK);
    let version_id = response_json(create).await["id"]
        .as_str()
        .expect("version id should exist")
        .to_string();

    let submit = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/drug-inspection/report-versions/{version_id}/submit"),
            uploader.clone(),
            Some("di-reject-edit-submit"),
            json!({}),
        ))
        .await
        .expect("submit should respond");
    assert_eq!(submit.status(), StatusCode::OK);
    let reject = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/drug-inspection/report-versions/{version_id}/review"),
            reviewer.clone(),
            Some("di-reject-edit-reject"),
            json!({ "decision": "rejected", "comment": "图片文字无法辨认" }),
        ))
        .await
        .expect("reject should respond");
    assert_eq!(reject.status(), StatusCode::OK);
    assert_eq!(response_json(reject).await["status"], "draft");

    let editable_path = format!(
        "/api/v1/drug-inspection/report-versions/editable?asn_id={}&product_id={}&batch_no=BATCH-A",
        fixture.first_asn_id, fixture.product_id
    );
    let other_user_lookup = app
        .clone()
        .oneshot(json_request(
            "GET",
            &editable_path,
            reviewer.clone(),
            None,
            json!({}),
        ))
        .await
        .expect("other user lookup should respond");
    assert_eq!(other_user_lookup.status(), StatusCode::NOT_FOUND);
    let editable = app
        .clone()
        .oneshot(json_request(
            "GET",
            &editable_path,
            uploader.clone(),
            None,
            json!({}),
        ))
        .await
        .expect("uploader lookup should respond");
    assert_eq!(editable.status(), StatusCode::OK);
    assert_eq!(response_json(editable).await["id"], version_id);

    let update_body = json!({
        "report_no": "REPORT-CLEAR",
        "original_file_id": fixture.correction_attachment_id,
        "processing_mode": "black_white_enhance",
        "qualified": true
    });
    for _ in 0..2 {
        let update = app
            .clone()
            .oneshot(json_request(
                "PUT",
                &format!("/api/v1/drug-inspection/report-versions/{version_id}"),
                uploader.clone(),
                Some("di-reject-edit-update"),
                update_body.clone(),
            ))
            .await
            .expect("update should respond");
        assert_eq!(update.status(), StatusCode::OK);
        let body = response_json(update).await;
        assert_eq!(body["id"], version_id);
        assert_eq!(body["version_number"], 1);
        assert_eq!(body["report_no"], "REPORT-CLEAR");
        assert_eq!(body["processing_mode"], "black_white_enhance");
        assert!(body["review_result"].is_null());
        assert!(body["review_comment"].is_null());
    }

    let resubmit = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/drug-inspection/report-versions/{version_id}/submit"),
            uploader,
            Some("di-reject-edit-resubmit"),
            json!({}),
        ))
        .await
        .expect("resubmit should respond");
    assert_eq!(resubmit.status(), StatusCode::OK);
    assert_eq!(
        response_json(resubmit).await["status"],
        "pending_confirmation"
    );

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'di.report_version.draft_updated' AND resource_id = $2",
    )
    .bind(fixture.owner_id)
    .bind(&version_id)
    .fetch_one(&pool)
    .await
    .expect("audit should query");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn report_upload_review_reuse_and_correction_preserve_version_chain(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    let app = drug_inspection_document_router(DrugInspectionDocumentAppState::with_postgres(
        pool.clone(),
    ));
    let uploader = context(fixture.owner_id, fixture.uploader_id, &all_permissions());
    let reviewer = context(fixture.owner_id, fixture.reviewer_id, &all_permissions());

    let create = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/drug-inspection/report-versions",
            uploader.clone(),
            Some("di-create-a"),
            json!({
                "asn_id": fixture.first_asn_id,
                "product_id": fixture.product_id,
                "batch_no": "BATCH-A",
                "report_no": "REPORT-A",
                "original_file_id": fixture.first_attachment_id,
                "source": "manual_upload",
                "processing_mode": "none",
                "qualified": true
            }),
        ))
        .await
        .expect("create should respond");
    assert_eq!(create.status(), StatusCode::OK);
    let draft = response_json(create).await;
    let report_id = draft["report_id"].as_str().expect("report id should exist");
    let first_version_id = draft["id"].as_str().expect("version id should exist");
    assert_eq!(draft["status"], "draft");

    let submit = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/drug-inspection/report-versions/{first_version_id}/submit"),
            uploader.clone(),
            Some("di-submit-a"),
            json!({}),
        ))
        .await
        .expect("submit should respond");
    assert_eq!(submit.status(), StatusCode::OK);
    assert_eq!(
        response_json(submit).await["status"],
        "pending_confirmation"
    );

    let queue = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/drug-inspection/review-queue",
            uploader.clone(),
            None,
            json!({}),
        ))
        .await
        .expect("review queue should respond");
    assert_eq!(queue.status(), StatusCode::OK);
    let queue_body = response_json(queue).await;
    assert_eq!(queue_body["page"]["count"], 1);
    assert_eq!(queue_body["page"]["total"], 1);
    assert_eq!(queue_body["data"][0]["version"]["id"], first_version_id);

    let self_review = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/drug-inspection/report-versions/{first_version_id}/review"),
            uploader.clone(),
            Some("di-self-review"),
            json!({ "decision": "confirmed", "comment": null }),
        ))
        .await
        .expect("self review should respond");
    assert_eq!(self_review.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let confirm = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/drug-inspection/report-versions/{first_version_id}/review"),
            reviewer.clone(),
            Some("di-confirm-a"),
            json!({ "decision": "confirmed", "comment": null }),
        ))
        .await
        .expect("confirm should respond");
    assert_eq!(confirm.status(), StatusCode::OK);
    assert_eq!(response_json(confirm).await["status"], "confirmed");

    let partial_list = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/drug-inspection/inbound-documents",
            uploader.clone(),
            None,
            json!({}),
        ))
        .await
        .expect("list should respond");
    assert_eq!(partial_list.status(), StatusCode::OK);
    let list_body = response_json(partial_list).await;
    assert_eq!(list_body["page"]["count"], 2);
    assert_eq!(list_body["page"]["total"], 2);
    let rows = list_body["data"]
        .as_array()
        .expect("list data should exist")
        .clone();
    let first_row = rows
        .iter()
        .find(|row| row["asn_id"] == fixture.first_asn_id.to_string())
        .expect("first ASN should be listed");
    assert_eq!(first_row["drug_inspection_status"], "partial");

    let reuse = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/drug-inspection/reports/{report_id}/reuse"),
            uploader.clone(),
            Some("di-reuse-a"),
            json!({
                "asn_id": fixture.second_asn_id,
                "batch_no": "BATCH-A"
            }),
        ))
        .await
        .expect("reuse should respond");
    assert_eq!(reuse.status(), StatusCode::OK);
    assert_eq!(
        response_json(reuse).await["source_version_id"],
        first_version_id
    );

    let correction = app
        .clone()
        .oneshot(json_request(
            "POST",
            &format!("/api/v1/drug-inspection/reports/{report_id}/corrections"),
            uploader.clone(),
            Some("di-correction-a"),
            json!({
                "report_no": "REPORT-A-2",
                "original_file_id": fixture.correction_attachment_id,
                "processing_mode": "none",
                "qualified": true,
                "modification_reason": "供应商补发清晰版"
            }),
        ))
        .await
        .expect("correction should respond");
    assert_eq!(correction.status(), StatusCode::OK);
    let correction_body = response_json(correction).await;
    let correction_id = correction_body["id"]
        .as_str()
        .expect("correction id should exist");
    assert_eq!(correction_body["version_number"], 2);

    let current_during_correction: Uuid =
        sqlx::query_scalar("SELECT current_version_id FROM drug_inspection_reports WHERE id = $1")
            .bind(Uuid::parse_str(report_id).expect("report id should parse"))
            .fetch_one(&pool)
            .await
            .expect("current version should query");
    assert_eq!(current_during_correction.to_string(), first_version_id);

    for (path, key) in [
        (
            format!("/api/v1/drug-inspection/report-versions/{correction_id}/submit"),
            "di-submit-correction",
        ),
        (
            format!("/api/v1/drug-inspection/report-versions/{correction_id}/review"),
            "di-confirm-correction",
        ),
    ] {
        let (ctx, payload) = if path.ends_with("/review") {
            (
                reviewer.clone(),
                json!({ "decision": "confirmed", "comment": null }),
            )
        } else {
            (uploader.clone(), json!({}))
        };
        let response = app
            .clone()
            .oneshot(json_request("POST", &path, ctx, Some(key), payload))
            .await
            .expect("correction transition should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let statuses: Vec<(i32, String)> = sqlx::query_as(
        "SELECT version_number, status FROM drug_inspection_report_versions WHERE report_id = $1 ORDER BY version_number",
    )
    .bind(Uuid::parse_str(report_id).expect("report id should parse"))
    .fetch_all(&pool)
    .await
    .expect("version chain should query");
    assert_eq!(
        statuses,
        vec![(1, "superseded".to_string()), (2, "confirmed".to_string())]
    );
    let evidence: (i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT COUNT(*) FROM audit_event
             WHERE owner_id = $1
               AND action = ANY($2::text[])),
           (SELECT COUNT(*) FROM idempotency_request
             WHERE owner_id = $1
               AND idempotency_key = ANY($3::text[]))",
    )
    .bind(fixture.owner_id)
    .bind(vec![
        "di.report_version.created",
        "di.report_version.submitted",
        "di.report_version.confirmed",
        "di.report.reused",
        "di.report_version.correction_created",
    ])
    .bind(vec![
        "di-create-a",
        "di-submit-a",
        "di-confirm-a",
        "di-reuse-a",
        "di-correction-a",
        "di-submit-correction",
        "di-confirm-correction",
    ])
    .fetch_one(&pool)
    .await
    .expect("report lifecycle audit and idempotency evidence should query");
    assert_eq!(evidence, (7, 7));

    let queue_after = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/drug-inspection/review-queue",
            uploader,
            None,
            json!({}),
        ))
        .await
        .expect("empty review queue should respond");
    assert_eq!(queue_after.status(), StatusCode::OK);
    let queue_after_body = response_json(queue_after).await;
    assert_eq!(queue_after_body["page"]["count"], 0);
    assert_eq!(queue_after_body["page"]["total"], 0);
    assert_eq!(
        queue_after_body["data"]
            .as_array()
            .expect("queue data should exist")
            .len(),
        0
    );

    let stamp_app = drug_inspection_stamp_router(DrugInspectionStampAppState::with_local_storage(
        pool.clone(),
        std::env::temp_dir(),
    ));
    let copy_reviewer = context(
        fixture.owner_id,
        fixture.reviewer_id,
        &["m-di.stamp.review"],
    );
    let copy_list = stamp_app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/drug-inspection/customer-copy-jobs?page_size=99999",
            copy_reviewer,
            None,
            json!({}),
        ))
        .await
        .expect("copy jobs list should respond");
    assert_eq!(copy_list.status(), StatusCode::OK);
    let copy_body = response_json(copy_list).await;
    assert_eq!(copy_body["page"]["count"], 2);
    assert_eq!(copy_body["page"]["total"], 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn upstream_delivery_upload_versions_multiple_asns_atomically(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    let app = drug_inspection_document_router(DrugInspectionDocumentAppState::with_postgres(
        pool.clone(),
    ));
    let uploader = context(fixture.owner_id, fixture.uploader_id, &all_permissions());
    let payload = json!({
        "supplier_id": fixture.supplier_id,
        "asn_ids": [fixture.first_asn_id, fixture.second_asn_id],
        "attachment_ids": [fixture.upstream_attachment_id],
        "modification_reason": null
    });
    let first = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/drug-inspection/upstream-delivery-document-versions",
            uploader.clone(),
            Some("upstream-first"),
            payload,
        ))
        .await
        .expect("first upstream version should respond");
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = response_json(first).await;
    let document_id = first_body["document_id"]
        .as_str()
        .expect("document id should exist");
    assert_eq!(first_body["version_number"], 1);
    assert_eq!(first_body["asn_ids"].as_array().expect("ASN ids").len(), 2);

    let without_reason = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/drug-inspection/upstream-delivery-document-versions",
            uploader.clone(),
            Some("upstream-no-reason"),
            json!({
                "document_id": document_id,
                "supplier_id": fixture.supplier_id,
                "asn_ids": [fixture.first_asn_id],
                "attachment_ids": [fixture.upstream_attachment_id],
                "modification_reason": null
            }),
        ))
        .await
        .expect("reupload without reason should respond");
    assert_eq!(without_reason.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let second = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/drug-inspection/upstream-delivery-document-versions",
            uploader,
            Some("upstream-second"),
            json!({
                "document_id": document_id,
                "supplier_id": fixture.supplier_id,
                "asn_ids": [fixture.first_asn_id, fixture.second_asn_id],
                "attachment_ids": [fixture.upstream_attachment_id],
                "modification_reason": "供应商补发新版"
            }),
        ))
        .await
        .expect("second upstream version should respond");
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(response_json(second).await["version_number"], 2);

    let current_links: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM upstream_delivery_asn_current WHERE owner_id = $1 AND version_id = (SELECT id FROM upstream_delivery_document_versions WHERE document_id = $2 AND version_number = 2)",
    )
    .bind(fixture.owner_id)
    .bind(Uuid::parse_str(document_id).expect("document id should parse"))
    .fetch_one(&pool)
    .await
    .expect("current ASN links should query");
    assert_eq!(current_links, 2);
    let history_links: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM upstream_delivery_document_asn_links WHERE owner_id = $1",
    )
    .bind(fixture.owner_id)
    .fetch_one(&pool)
    .await
    .expect("history links should query");
    assert_eq!(history_links, 4);
    let evidence: (i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT COUNT(*) FROM audit_event
             WHERE owner_id = $1
               AND action = 'di.upstream_delivery.version_created'),
           (SELECT COUNT(*) FROM idempotency_request
             WHERE owner_id = $1
               AND idempotency_key = ANY($2::text[]))",
    )
    .bind(fixture.owner_id)
    .bind(vec!["upstream-first", "upstream-second"])
    .fetch_one(&pool)
    .await
    .expect("upstream delivery audit and idempotency evidence should query");
    assert_eq!(evidence, (2, 2));
}
