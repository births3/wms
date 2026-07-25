use super::*;

#[sqlx::test(migrations = "../../migrations")]
async fn approved_archive_revision_publishes_and_completed_callback_unlocks_asn(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let creator_id = Uuid::new_v4();
    let approver_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let asn_id = Uuid::new_v4();
    let receipt_record_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '档案补录测试货主')",
    )
    .bind(owner_id)
    .bind(format!("QL-AR-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("archive revision owner should seed");
    seed_actor(&pool, owner_id, creator_id, "archive-creator").await;
    seed_actor(&pool, owner_id, approver_id, "archive-approver").await;
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '档案补录测试仓', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("QL-AR-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("archive revision warehouse should seed");
    sqlx::query(
        "INSERT INTO products (id, owner_id, product_code, product_name, specification, storage_condition, approval_no, status) VALUES ($1, $2, 'P-ARCHIVE-001', '档案补录商品', '1 unit', 'normal', 'OLD-001', 'active')",
    )
    .bind(product_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("archive revision product should seed");
    sqlx::query(
        "INSERT INTO receiving_orders (id, owner_id, receipt_no, document_type, warehouse_id, status) VALUES ($1, $2, 'ASN-QL-ARCHIVE-001', 'purchase_inbound', $3, 'archive_replenishing')",
    )
    .bind(asn_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(&pool)
    .await
    .expect("archive revision ASN should seed");
    sqlx::query(
        "INSERT INTO receiving_order_lines (id, receiving_order_id, owner_id, line_no, product_code, expected_qty) VALUES ($1, $2, $3, 1, 'P-ARCHIVE-001', 1)",
    )
    .bind(Uuid::new_v4())
    .bind(asn_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("archive revision ASN line should seed");
    sqlx::query(
        "INSERT INTO receiving_order_receipts (id, receiving_order_id, owner_id, actual_qty, shortage_qty, rejected_qty, occurred_at) VALUES ($1, $2, $3, 1, 0, 0, $4)",
    )
    .bind(receipt_record_id)
    .bind(asn_id)
    .bind(owner_id)
    .bind(Utc::now())
    .execute(&pool)
    .await
    .expect("archive revision receipt should seed");

    let repository = PgQualityLiaisonRepository::new(pool.clone());
    let creator = ctx(owner_id, creator_id);
    let now = Utc::now();
    repository
        .upsert_type(
            &creator,
            UpsertQualityLiaisonTypeRequest {
                type_code: "archive_revision".to_string(),
                type_name: "档案补录".to_string(),
                approval_template_id: "ww-ql-archive-revision".to_string(),
                approver_user_id: approver_id,
                timeout_seconds: 4 * 60 * 60,
                enabled: true,
            },
            now,
            "ql-archive-type",
        )
        .await
        .expect("archive revision type should configure");
    let liaison = repository
        .create(
            &creator,
            CreateQualityLiaisonRequest {
                type_code: "archive_revision".to_string(),
                related_document_type: "asn".to_string(),
                related_document_no: "ASN-QL-ARCHIVE-001".to_string(),
                problem_description: "批准文号与实物不一致".to_string(),
                disposition_suggestion: "以 ERP 主数据变更结果为准".to_string(),
                trigger_source: "M2".to_string(),
                business_payload: serde_json::json!({
                    "action":"publish_archive_revision",
                    "warehouse_id":warehouse_id,
                    "asn_id":asn_id,
                    "receipt_record_id":receipt_record_id,
                    "product_code":"P-ARCHIVE-001",
                    "field_name":"approval_number",
                    "current_value":"OLD-001",
                    "new_value":"NEW-001",
                    "photo_evidence_urls":["https://files.example.test/archive-001.jpg"]
                }),
            },
            now,
            "ql-archive-create",
        )
        .await
        .expect("archive revision liaison should create");
    let callback = QualityLiaisonApprovalCallbackRequest {
        conclusion: "approved".to_string(),
        opinion: "同意推送 ERP 变更主数据".to_string(),
        external_approval_id: "ww-ql-archive-001".to_string(),
    };
    let approved = repository
        .apply_approval_callback(
            &ctx(owner_id, approver_id),
            liaison.value.id,
            callback.clone(),
            now,
            "ql-archive-approval",
        )
        .await
        .expect("approved archive revision should publish outbox");
    assert_eq!(approved.value.status, "pending_erp_sync");
    assert!(!approved.replayed);
    let replayed = repository
        .apply_approval_callback(
            &ctx(owner_id, approver_id),
            liaison.value.id,
            callback,
            now,
            "ql-archive-approval",
        )
        .await
        .expect("same archive approval should replay");
    assert!(replayed.replayed);
    assert_eq!(replayed.value.id, liaison.value.id);

    let outbox: (Uuid, Uuid, Uuid, String, String, serde_json::Value, String) = sqlx::query_as(
        r#"
            SELECT liaison_id, asn_id, receipt_record_id, product_code, field_name,
                   payload, status
              FROM archive_revision_erp_feedback_outbox
             WHERE owner_id = $1
            "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("archive revision outbox should query");
    assert_eq!(outbox.0, liaison.value.id);
    assert_eq!(outbox.1, asn_id);
    assert_eq!(outbox.2, receipt_record_id);
    assert_eq!(outbox.3, "P-ARCHIVE-001");
    assert_eq!(outbox.4, "approval_number");
    assert_eq!(outbox.5["warehouse_id"], warehouse_id.to_string());
    assert_eq!(outbox.5["new_value"], "NEW-001");
    assert_eq!(outbox.6, "pending");
    let outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM archive_revision_erp_feedback_outbox WHERE owner_id = $1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("archive revision outbox count should query");
    assert_eq!(outbox_count, 1, "approval replay must not duplicate outbox");
    let (audit_count, idempotency_count): (i64, i64) = sqlx::query_as(
        r#"
        SELECT
          (SELECT COUNT(*) FROM audit_event
            WHERE owner_id = $1 AND action = 'apply_quality_liaison_approval'
              AND resource_id = $2),
          (SELECT COUNT(*) FROM idempotency_request
            WHERE owner_id = $1 AND idempotency_key = 'ql-archive-approval')
        "#,
    )
    .bind(owner_id)
    .bind(liaison.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("archive revision replay evidence should query");
    assert_eq!((audit_count, idempotency_count), (1, 1));

    sqlx::query(
        "UPDATE archive_revision_erp_feedback_outbox SET status = 'succeeded', updated_at = $3 WHERE owner_id = $1 AND liaison_id = $2",
    )
    .bind(owner_id)
    .bind(liaison.value.id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("ERP archive publication should be marked succeeded");
    sqlx::query(
        "UPDATE products SET approval_no = 'NEW-001', updated_at = $3 WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(product_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("M1 product change should complete before archive closeout");
    let app = quality_liaison_router(QualityLiaisonAppState::with_postgres(pool.clone()));
    let completion_body = serde_json::json!({
        "asn_id": asn_id,
        "product_id": product_id,
        "product_code": "P-ARCHIVE-001",
        "field_name": "approval_number",
        "new_value": "NEW-001"
    });
    let forbidden = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/quality-liaisons/{}/archive-sync-callback",
                    liaison.value.id
                ))
                .header("content-type", "application/json")
                .header("idempotency-key", "ql-archive-sync-forbidden")
                .extension(creator.clone())
                .body(Body::from(completion_body.to_string()))
                .expect("forbidden archive completion request should build"),
        )
        .await
        .expect("forbidden archive completion should respond");
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let worker = AuthContext {
        permissions: vec!["h8.erp_connector.write".to_string()],
        ..creator.clone()
    };
    let mismatch = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/quality-liaisons/{}/archive-sync-callback",
                    liaison.value.id
                ))
                .header("content-type", "application/json")
                .header("idempotency-key", "ql-archive-sync-mismatch")
                .extension(worker.clone())
                .body(Body::from(
                    serde_json::json!({
                        "asn_id": Uuid::new_v4(),
                        "product_id": product_id,
                        "product_code": "P-ARCHIVE-001",
                        "field_name": "approval_number",
                        "new_value": "NEW-001"
                    })
                    .to_string(),
                ))
                .expect("mismatched archive completion request should build"),
        )
        .await
        .expect("mismatched archive completion should respond");
    assert_eq!(mismatch.status(), StatusCode::UNPROCESSABLE_ENTITY);

    for (key, scoped_worker, expected) in [
        (
            "ql-archive-sync-cross-owner",
            AuthContext {
                owner_id: Uuid::new_v4(),
                ..worker.clone()
            },
            StatusCode::NOT_FOUND,
        ),
        (
            "ql-archive-sync-cross-warehouse",
            AuthContext {
                warehouse_scope: Some(Uuid::new_v4()),
                ..worker.clone()
            },
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/v1/quality-liaisons/{}/archive-sync-callback",
                        liaison.value.id
                    ))
                    .header("content-type", "application/json")
                    .header("idempotency-key", key)
                    .extension(scoped_worker)
                    .body(Body::from(completion_body.to_string()))
                    .expect("scoped archive completion request should build"),
            )
            .await
            .expect("scoped archive completion should respond");
        assert_eq!(response.status(), expected);
    }

    let complete_request = || {
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/v1/quality-liaisons/{}/archive-sync-callback",
                liaison.value.id
            ))
            .header("content-type", "application/json")
            .header("idempotency-key", "ql-archive-sync-complete")
            .extension(worker.clone())
            .body(Body::from(completion_body.to_string()))
            .expect("archive completion request should build")
    };
    for replayed in [false, true] {
        let response = app
            .clone()
            .oneshot(complete_request())
            .await
            .expect("archive completion should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let completed: QualityLiaisonOrder = serde_json::from_slice(
            &to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("archive completion response should read"),
        )
        .expect("archive completion response should deserialize");
        assert_eq!(completed.status, "landed", "replayed={replayed}");
    }
    let closeout: (String, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT q.status, receiving_order.status,
               (SELECT COUNT(*) FROM audit_event
                 WHERE owner_id = $1 AND action = 'complete_archive_revision_sync'
                   AND resource_id = $2),
               (SELECT COUNT(*) FROM idempotency_request
                 WHERE owner_id = $1 AND idempotency_key = 'ql-archive-sync-complete')
          FROM quality_liaison_orders q
          JOIN receiving_orders receiving_order
            ON receiving_order.owner_id = q.owner_id AND receiving_order.id = $3
         WHERE q.owner_id = $1 AND q.id = $4
        "#,
    )
    .bind(owner_id)
    .bind(liaison.value.id.to_string())
    .bind(asn_id)
    .bind(liaison.value.id)
    .fetch_one(&pool)
    .await
    .expect("archive completion evidence should query");
    assert_eq!(
        closeout,
        ("landed".to_string(), "inspecting".to_string(), 1, 1)
    );

    let invalid_liaison = repository
        .create(
            &creator,
            CreateQualityLiaisonRequest {
                type_code: "archive_revision".to_string(),
                related_document_type: "asn".to_string(),
                related_document_no: "ASN-QL-ARCHIVE-001".to_string(),
                problem_description: "缺少实物包装照片".to_string(),
                disposition_suggestion: "补齐证据后再推送".to_string(),
                trigger_source: "M2".to_string(),
                business_payload: serde_json::json!({
                    "action":"publish_archive_revision",
                    "warehouse_id":warehouse_id,
                    "asn_id":asn_id,
                    "receipt_record_id":receipt_record_id,
                    "product_code":"P-ARCHIVE-001",
                    "field_name":"storage_condition",
                    "new_value":"cool"
                }),
            },
            now,
            "ql-archive-invalid-create",
        )
        .await
        .expect("invalid archive revision should still enter approval");
    let error = repository
        .apply_approval_callback(
            &ctx(owner_id, approver_id),
            invalid_liaison.value.id,
            QualityLiaisonApprovalCallbackRequest {
                conclusion: "approved".to_string(),
                opinion: "同意，但业务动作必须校验照片".to_string(),
                external_approval_id: "ww-ql-archive-002".to_string(),
            },
            now,
            "ql-archive-invalid-approval",
        )
        .await
        .expect_err("missing photo evidence must reject archive publication");
    assert_eq!(error, QualityLiaisonError::BusinessActionInvalid);
    let rolled_back: (String, String, i64) = sqlx::query_as(
        r#"
        SELECT q.status, h.status,
               (SELECT COUNT(*) FROM archive_revision_erp_feedback_outbox o
                 WHERE o.owner_id = q.owner_id AND o.liaison_id = q.id)
          FROM quality_liaison_orders q
          JOIN h4_approval_records h ON h.id = q.approval_record_id
         WHERE q.owner_id = $1 AND q.id = $2
        "#,
    )
    .bind(owner_id)
    .bind(invalid_liaison.value.id)
    .fetch_one(&pool)
    .await
    .expect("invalid archive approval states should query");
    assert_eq!(
        rolled_back,
        ("pending_approval".to_string(), "pending".to_string(), 0),
        "invalid outbox payload must roll back M-QL and H4 approval states"
    );
}
