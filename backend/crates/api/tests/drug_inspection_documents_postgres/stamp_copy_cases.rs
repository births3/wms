#[sqlx::test(migrations = "../../migrations")]
async fn stamp_publish_requires_two_people_and_freezes_the_published_version(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    let repository = PgDrugInspectionStampRepository::new(pool.clone());
    let configurer = context(fixture.owner_id, fixture.uploader_id, &all_permissions());
    let reviewer = context(fixture.owner_id, fixture.reviewer_id, &all_permissions());

    let draft = repository
        .create_version(
            &configurer,
            CreateDrugInspectionStampVersionRequest {
                png_attachment_id: fixture.stamp_attachment_id,
                relative_x: 0.7,
                relative_y: 0.75,
                relative_width: 0.2,
            },
            "stamp-create-v1",
        )
        .await
        .expect("stamp draft should create");
    assert_eq!(draft.status, "draft");

    let pending = repository
        .submit_version(&configurer, draft.id, "stamp-submit-v1")
        .await
        .expect("stamp should submit");
    assert_eq!(pending.status, "pending_review");

    let self_review = repository
        .review_version(
            &configurer,
            draft.id,
            ReviewDrugInspectionStampVersionRequest {
                decision: "published".to_string(),
                comment: None,
            },
            "stamp-self-review",
        )
        .await;
    assert!(self_review.is_err());

    let published = repository
        .review_version(
            &reviewer,
            draft.id,
            ReviewDrugInspectionStampVersionRequest {
                decision: "published".to_string(),
                comment: Some("位置与透明背景已核对".to_string()),
            },
            "stamp-publish-v1",
        )
        .await
        .expect("second user should publish");
    assert_eq!(published.status, "published");
    assert_eq!(published.reviewed_by, Some(fixture.reviewer_id));

    let next = repository
        .create_version(
            &configurer,
            CreateDrugInspectionStampVersionRequest {
                png_attachment_id: fixture.stamp_attachment_id,
                relative_x: 0.65,
                relative_y: 0.72,
                relative_width: 0.22,
            },
            "stamp-create-v2",
        )
        .await
        .expect("new immutable version should create");
    assert_eq!(next.version_number, 2);
    let still_published = repository
        .published_version(fixture.owner_id)
        .await
        .expect("published lookup should succeed")
        .expect("published version should exist");
    assert_eq!(still_published.id, draft.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn stamp_version_rejects_png_outside_the_validated_upload_channel(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    sqlx::query("UPDATE attachments SET module = 'H-FILE' WHERE id = $1")
        .bind(fixture.stamp_attachment_id)
        .execute(&pool)
        .await
        .expect("stamp fixture module should update");

    let error = PgDrugInspectionStampRepository::new(pool)
        .create_version(
            &context(fixture.owner_id, fixture.uploader_id, &all_permissions()),
            CreateDrugInspectionStampVersionRequest {
                png_attachment_id: fixture.stamp_attachment_id,
                relative_x: 0.7,
                relative_y: 0.75,
                relative_width: 0.2,
            },
            "stamp-invalid-upload-channel",
        )
        .await
        .expect_err("stamp must come from the validated M-DI upload channel");
    assert!(matches!(
        error,
        wms_api::drug_inspection_document_repository::DrugInspectionDocumentRepositoryError::Conflict(
            "stamp_attachment_entity_mismatch"
        )
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn publishing_first_stamp_requeues_copy_that_failed_without_a_stamp(pool: PgPool) {
    let fixture = seed_fixture(&pool).await;
    let uploader = context(fixture.owner_id, fixture.uploader_id, &all_permissions());
    let reviewer = context(fixture.owner_id, fixture.reviewer_id, &all_permissions());
    let reports = PgDrugInspectionDocumentRepository::new(pool.clone());
    let draft = reports
        .create_version(
            &uploader,
            CreateDrugInspectionVersionRequest {
                asn_id: fixture.first_asn_id,
                product_id: fixture.product_id,
                batch_no: "BATCH-A".to_string(),
                report_no: "REPORT-MISSING-STAMP".to_string(),
                original_file_id: fixture.first_attachment_id,
                source: "manual_upload".to_string(),
                processing_mode: "none".to_string(),
                qualified: true,
            },
            "missing-stamp-report-create",
        )
        .await
        .expect("report should create");
    reports
        .submit_version(&uploader, draft.id, "missing-stamp-report-submit")
        .await
        .expect("report should submit");
    let confirmed = reports
        .review_version(
            &reviewer,
            draft.id,
            ReviewDrugInspectionVersionRequest {
                decision: "confirmed".to_string(),
                comment: None,
            },
            "missing-stamp-report-confirm",
        )
        .await
        .expect("report confirmation must not wait for stamp configuration");
    assert_eq!(confirmed.stamp_version_id, None);

    let service = DrugInspectionCopyService::new(pool.clone(), std::env::temp_dir());
    assert!(matches!(
        service.process_next().await,
        Err(DrugInspectionCopyServiceError::Conflict(
            "published_stamp_missing"
        ))
    ));

    let stamps = PgDrugInspectionStampRepository::new(pool.clone());
    let stamp = stamps
        .create_version(
            &uploader,
            CreateDrugInspectionStampVersionRequest {
                png_attachment_id: fixture.stamp_attachment_id,
                relative_x: 0.7,
                relative_y: 0.75,
                relative_width: 0.2,
            },
            "missing-stamp-create",
        )
        .await
        .expect("stamp should create");
    stamps
        .submit_version(&uploader, stamp.id, "missing-stamp-submit")
        .await
        .expect("stamp should submit");
    stamps
        .review_version(
            &reviewer,
            stamp.id,
            ReviewDrugInspectionStampVersionRequest {
                decision: "published".to_string(),
                comment: None,
            },
            "missing-stamp-publish",
        )
        .await
        .expect("stamp should publish");

    let state: (Option<Uuid>, String, i32, Option<String>) = sqlx::query_as(
        "SELECT version.stamp_version_id, job.status, job.attempt_count, job.last_error
           FROM drug_inspection_report_versions AS version
           JOIN drug_inspection_customer_copy_jobs AS job
             ON job.report_version_id = version.id
            AND job.owner_id = version.owner_id
          WHERE version.owner_id = $1 AND version.id = $2",
    )
    .bind(fixture.owner_id)
    .bind(draft.id)
    .fetch_one(&pool)
    .await
    .expect("requeued copy state should query");
    assert_eq!(state, (Some(stamp.id), "queued".to_string(), 0, None));
}

#[sqlx::test(migrations = "../../migrations")]
async fn review_drug_inspection_stamp_version_and_approve_drug_inspection_copy_oversize_generate_a_real_pdf(
    pool: PgPool,
) {
    let fixture = seed_fixture(&pool).await;
    let storage_root =
        std::env::temp_dir().join(format!("wms-di-copy-{}", Uuid::new_v4().simple()));
    write_attachment_bytes(
        &pool,
        &storage_root,
        fixture.first_attachment_id,
        &png(600, 900, 255),
    )
    .await;
    write_attachment_bytes(
        &pool,
        &storage_root,
        fixture.stamp_attachment_id,
        &png(160, 100, 100),
    )
    .await;
    let uploader = context(fixture.owner_id, fixture.uploader_id, &all_permissions());
    let reviewer = context(fixture.owner_id, fixture.reviewer_id, &all_permissions());
    let stamps = PgDrugInspectionStampRepository::new(pool.clone());
    let stamp = stamps
        .create_version(
            &uploader,
            CreateDrugInspectionStampVersionRequest {
                png_attachment_id: fixture.stamp_attachment_id,
                relative_x: 0.68,
                relative_y: 0.72,
                relative_width: 0.2,
            },
            "copy-stamp-create",
        )
        .await
        .expect("stamp should create");
    stamps
        .submit_version(&uploader, stamp.id, "copy-stamp-submit")
        .await
        .expect("stamp should submit");
    stamps
        .review_version(
            &reviewer,
            stamp.id,
            ReviewDrugInspectionStampVersionRequest {
                decision: "published".to_string(),
                comment: Some("真实副本测试发布".to_string()),
            },
            "copy-stamp-publish",
        )
        .await
        .expect("stamp should publish");

    let reports = PgDrugInspectionDocumentRepository::new(pool.clone());
    let draft = reports
        .create_version(
            &uploader,
            CreateDrugInspectionVersionRequest {
                asn_id: fixture.first_asn_id,
                product_id: fixture.product_id,
                batch_no: "BATCH-A".to_string(),
                report_no: "REPORT-COPY-001".to_string(),
                original_file_id: fixture.first_attachment_id,
                source: "manual_upload".to_string(),
                processing_mode: "color_enhance".to_string(),
                qualified: true,
            },
            "copy-report-create",
        )
        .await
        .expect("report should create");
    reports
        .submit_version(&uploader, draft.id, "copy-report-submit")
        .await
        .expect("report should submit");
    let confirmed = reports
        .review_version(
            &reviewer,
            draft.id,
            ReviewDrugInspectionVersionRequest {
                decision: "confirmed".to_string(),
                comment: None,
            },
            "copy-report-confirm",
        )
        .await
        .expect("report should confirm");
    assert_eq!(confirmed.stamp_version_id, Some(stamp.id));
    assert_eq!(confirmed.customer_copy_status, "queued");

    sqlx::query(
        "UPDATE drug_inspection_customer_copy_jobs
            SET status = 'processing', attempt_count = 3,
                started_at = now() - interval '11 minutes'
          WHERE owner_id = $1 AND report_version_id = $2",
    )
    .bind(fixture.owner_id)
    .bind(draft.id)
    .execute(&pool)
    .await
    .expect("expired third claim should seed");

    let service = DrugInspectionCopyService::new(pool.clone(), storage_root.clone());
    let job = service
        .process_next()
        .await
        .expect("copy processing should succeed")
        .expect("copy job should exist");
    assert_eq!(job.status, "succeeded");
    assert_eq!(job.attempt_count, 3);
    let (status, file_id): (String, Option<Uuid>) = sqlx::query_as(
        "SELECT customer_copy_status, customer_copy_file_id FROM drug_inspection_report_versions WHERE id = $1",
    )
    .bind(draft.id)
    .fetch_one(&pool)
    .await
    .expect("copy state should query");
    assert_eq!(status, "available");
    let portal_projection: (i64, i64) = sqlx::query_as(
        "SELECT
            COUNT(DISTINCT event.id)::BIGINT,
            COUNT(DISTINCT delivery.id)::BIGINT
         FROM event_bus_event AS event
         LEFT JOIN event_bus_delivery AS delivery
           ON delivery.owner_id = event.owner_id
          AND delivery.event_id = event.id
         LEFT JOIN event_bus_subscription AS subscription
           ON subscription.owner_id = delivery.owner_id
          AND subscription.id = delivery.subscription_id
          AND subscription.subscriber_key = 'mdi-customer-portal'
         WHERE event.owner_id = $1
           AND event.event_type = 'portal.drug_inspection_report.upsert'
           AND event.resource_id = $2",
    )
    .bind(fixture.owner_id)
    .bind(draft.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("customer copy portal projection should query");
    assert_eq!(portal_projection, (1, 1));
    let storage_key: String =
        sqlx::query_scalar("SELECT storage_key FROM attachments WHERE id = $1")
            .bind(file_id.expect("customer copy file should exist"))
            .fetch_one(&pool)
            .await
            .expect("copy attachment should query");
    let bytes = tokio::fs::read(storage_root.join(storage_key))
        .await
        .expect("customer PDF should read");
    let mut warnings = Vec::new();
    let pdf = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut warnings)
        .expect("customer PDF should parse");
    assert_eq!(pdf.page_count(), 1);
    let original_copy_id = file_id.expect("customer copy file should exist");
    let rule = stamps
        .publish_processing_rule(
            &uploader,
            PublishDrugInspectionProcessingRuleRequest {
                apply_scope: "reprocess_current".to_string(),
            },
            "copy-rule-reprocess",
        )
        .await
        .expect("processing rule should publish");
    assert_eq!(rule.reprocess_job_count, 1);
    let app_can_update_rule_versions: bool = sqlx::query_scalar(
        "SELECT has_table_privilege(
            'wms_app',
            'drug_inspection_processing_rule_versions',
            'UPDATE'
        )",
    )
    .fetch_one(&pool)
    .await
    .expect("processing rule privileges should query");
    assert!(
        !app_can_update_rule_versions,
        "published processing rule versions must stay insert-only"
    );
    let replayed = stamps
        .publish_processing_rule(
            &uploader,
            PublishDrugInspectionProcessingRuleRequest {
                apply_scope: "reprocess_current".to_string(),
            },
            "copy-rule-reprocess",
        )
        .await
        .expect("processing rule idempotency should replay");
    assert_eq!(replayed.id, rule.id);
    let before_reprocess: (String, Option<Uuid>) = sqlx::query_as(
        "SELECT customer_copy_status, customer_copy_file_id
         FROM drug_inspection_report_versions
         WHERE id = $1",
    )
    .bind(draft.id)
    .fetch_one(&pool)
    .await
    .expect("current customer copy should remain queryable");
    assert_eq!(
        before_reprocess,
        ("available".to_string(), Some(original_copy_id))
    );
    let reprocessed = service
        .process_next()
        .await
        .expect("reprocess should succeed")
        .expect("reprocess job should exist");
    assert_eq!(reprocessed.status, "succeeded");
    let replacement_copy_id: Uuid = sqlx::query_scalar(
        "SELECT customer_copy_file_id
         FROM drug_inspection_report_versions
         WHERE id = $1",
    )
    .bind(draft.id)
    .fetch_one(&pool)
    .await
    .expect("replacement copy should query");
    assert_ne!(replacement_copy_id, original_copy_id);

    sqlx::query(
        r#"
        UPDATE drug_inspection_customer_copy_jobs
           SET status = 'oversize_review',
               candidate_file_id = $3,
               candidate_hash = attachment.sha256,
               candidate_size = 52428801,
               finished_at = now(),
               updated_at = now()
          FROM attachments AS attachment
         WHERE drug_inspection_customer_copy_jobs.owner_id = $1
           AND drug_inspection_customer_copy_jobs.id = $2
           AND attachment.owner_id = $1
           AND attachment.id = $3
        "#,
    )
    .bind(fixture.owner_id)
    .bind(reprocessed.id)
    .bind(replacement_copy_id)
    .execute(&pool)
    .await
    .expect("oversize review fixture should update");
    let approval = ApproveDrugInspectionCopyOversizeRequest {
        reason: "业务确认该批客户副本允许超过 50MB".to_string(),
    };
    let approved = service
        .approve_oversize(
            &reviewer,
            reprocessed.id,
            approval.clone(),
            "copy-oversize-approval",
        )
        .await
        .expect("oversize approval should succeed");
    let replayed = service
        .approve_oversize(
            &reviewer,
            reprocessed.id,
            approval,
            "copy-oversize-approval",
        )
        .await
        .expect("oversize approval should replay");
    assert_eq!(approved.id, replayed.id);
    assert_eq!(approved.updated_at, replayed.updated_at);
    let conflict = service
        .approve_oversize(
            &reviewer,
            reprocessed.id,
            ApproveDrugInspectionCopyOversizeRequest {
                reason: "同一幂等键不能换一个原因".to_string(),
            },
            "copy-oversize-approval",
        )
        .await
        .expect_err("same key with another request should conflict");
    assert!(matches!(
        conflict,
        DrugInspectionCopyServiceError::IdempotencyConflict
    ));
    let approval_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event
         WHERE owner_id = $1
           AND action = 'di.customer_copy.oversize_approved'
           AND resource_id = $2",
    )
    .bind(fixture.owner_id)
    .bind(reprocessed.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("oversize approval audit should query");
    assert_eq!(approval_audit_count, 1);
    tokio::fs::remove_dir_all(&storage_root)
        .await
        .expect("test attachment root should remove");
}
