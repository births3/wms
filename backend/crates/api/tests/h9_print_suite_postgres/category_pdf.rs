#[sqlx::test(migrations = "../../migrations")]
async fn rendered_category_pdf_is_written_to_h_file_before_instance_queues(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let h_file = wms_api::file_attachment::FileAttachmentService::with_memory(pool.clone());
    let service =
        PrintOrchestrationService::with_file_attachment_for_tests(pool.clone(), h_file.clone());
    let sample_order = seed_order(&pool, &scope, "SO-H9-009-R00", Some("INV-H9-009-R00")).await;
    let sample_group = cutoff(&service, &scope, sample_order, "h9-pdf-render-g0").await;
    publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "分类 PDF 渲染组套",
            PrintSuiteScope::Customer,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        sample_group,
        "h9-pdf-render-suite",
    )
    .await;

    let order = seed_order(&pool, &scope, "SO-H9-009-R01", Some("INV-H9-009-R01")).await;
    let group = cutoff(&service, &scope, order, "h9-pdf-render-g1").await;
    let instance = single_instance(&service, &scope, group).await;
    assert_eq!(
        instance.status, "waiting_documents",
        "source documents alone must not make an instance executable"
    );

    let prepared = service
        .prepare_category_pdfs(
            &scope.actor,
            instance.id,
            test_now(),
            "h9-pdf-render-prepare",
        )
        .await
        .expect("render worker should prepare the category PDF");
    assert!(!prepared.replayed);
    assert_eq!(prepared.value.instance_id, instance.id);
    assert_eq!(prepared.value.outputs.len(), 1);
    let output = &prepared.value.outputs[0];
    assert_eq!(output.category_code, "delivery_note");
    assert_eq!(output.processing_status, "ready");
    assert_eq!(output.source_mode, PrintSuiteSourceMode::Rendered);
    assert_eq!(output.template_version_id, Some(scope.template_version_id));
    assert!(output.source_data_version.is_some());
    assert!(output.source_file_bindings.is_empty());
    assert_eq!(output.retention_policy, "gsp_5_year");
    assert_eq!(output.content_hash.as_deref().map(str::len), Some(64));
    let attachment_id = output
        .attachment_id
        .expect("rendered PDF should have one H-FILE attachment");
    let bytes = h_file
        .read_internal(scope.owner_id, attachment_id)
        .await
        .expect("rendered H-FILE object should be readable");
    assert!(bytes.starts_with(b"%PDF-1.4"));
    assert!(bytes.ends_with(b"%%EOF\n"));
    let rendered_text = String::from_utf8_lossy(&bytes);
    assert!(
        rendered_text.contains("SO-H9-009-R01"),
        "rendered PDF must contain the frozen real source order"
    );
    assert!(
        rendered_text.contains(&scope.template_version_id.to_string()),
        "rendered PDF must identify the frozen published template version"
    );

    let after = single_instance(&service, &scope, group).await;
    assert_eq!(after.status, "queued");
    assert_eq!(after.hold_scope, None);
    let preparation_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event \
         WHERE owner_id = $1 AND module = 'H9' \
           AND action = 'prepare_category_pdfs' \
           AND resource_id = $2",
    )
    .bind(scope.owner_id)
    .bind(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM h9_category_pdf_preparations \
             WHERE owner_id = $1 AND instance_id = $2",
        )
        .bind(scope.owner_id)
        .bind(instance.id)
        .fetch_one(&pool)
        .await
        .expect("preparation id should load")
        .to_string(),
    )
    .fetch_one(&pool)
    .await
    .expect("preparation audit count should load");
    assert_eq!(
        preparation_audit_count, 1,
        "queued instance and preparation audit must commit together"
    );

    let replayed = service
        .prepare_category_pdfs(
            &scope.actor,
            instance.id,
            test_now(),
            "h9-pdf-render-prepare",
        )
        .await
        .expect("same idempotency key should replay");
    assert!(replayed.replayed);
    assert_eq!(replayed.value.outputs[0].id, output.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn external_pdf_is_referenced_without_rerender_and_selection_is_temporary(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let h_file = wms_api::file_attachment::FileAttachmentService::with_memory(pool.clone());
    let service =
        PrintOrchestrationService::with_file_attachment_for_tests(pool.clone(), h_file.clone());
    let sample_invoice =
        seed_real_invoice_attachment(&pool, &scope, &h_file, "INV-H9-009-E00", "sample invoice")
            .await;
    let sample_order = seed_order(&pool, &scope, "SO-H9-009-E00", Some("INV-H9-009-E00")).await;
    let sample_group = cutoff(&service, &scope, sample_order, "h9-pdf-external-g0").await;
    publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "外部 PDF 引用组套",
            PrintSuiteScope::Customer,
            vec![
                rendered_item(scope.template_version_id, 1),
                external_item("invoice", 2, true, PrintSuiteReadyPolicy::WaitHoldInstance),
            ],
        ),
        sample_group,
        "h9-pdf-external-suite",
    )
    .await;
    assert!(!sample_invoice.id.is_nil());

    let invoice = seed_real_invoice_attachment(
        &pool,
        &scope,
        &h_file,
        "INV-H9-009-E01",
        "authoritative invoice",
    )
    .await;
    let order = seed_order(&pool, &scope, "SO-H9-009-E01", Some("INV-H9-009-E01")).await;
    let group = cutoff(&service, &scope, order, "h9-pdf-external-g1").await;
    let instance = single_instance(&service, &scope, group).await;
    let prepared = service
        .prepare_category_pdfs(
            &scope.actor,
            instance.id,
            test_now(),
            "h9-pdf-external-prepare",
        )
        .await
        .expect("rendered and external categories should prepare");
    assert_eq!(prepared.value.status, "completed");
    assert_eq!(
        prepared
            .value
            .outputs
            .iter()
            .map(|output| output.category_code.as_str())
            .collect::<Vec<_>>(),
        ["delivery_note", "invoice"],
        "category outputs and temporary merge must preserve frozen suite order"
    );
    let external = prepared
        .value
        .outputs
        .iter()
        .find(|output| output.category_code == "invoice")
        .expect("invoice output should exist");
    assert_eq!(external.source_mode, PrintSuiteSourceMode::ExternalFile);
    assert_eq!(external.template_version_id, None);
    assert_eq!(external.source_data_version, None);
    assert_eq!(external.source_file_bindings.len(), 1);
    assert_eq!(external.source_file_bindings[0].file_id, invoice.id);
    assert_eq!(external.attachment_id, Some(invoice.id));
    assert_eq!(
        external.content_hash.as_deref(),
        Some(invoice.content_hash.as_str())
    );
    assert_eq!(external.retention_policy, "short_cache");
    assert!(external.cache_expires_at.is_some());

    // Selecting one category returns the authoritative PDF directly.
    let invoice_pdf = service
        .download_category_pdfs(&scope.actor, instance.id, &[external.id], false, test_now())
        .await
        .expect("selected invoice should download");
    assert_eq!(
        lopdf::Document::load_mem(&invoice_pdf)
            .expect("invoice should remain a valid PDF")
            .get_pages()
            .len(),
        1
    );

    // Empty selection means all ready categories and is merged only in memory.
    let attachment_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM attachments WHERE owner_id = $1")
            .bind(scope.owner_id)
            .fetch_one(&pool)
            .await
            .expect("attachment count should load");
    let all_pdf = service
        .download_category_pdfs(&scope.actor, instance.id, &[], false, test_now())
        .await
        .expect("all categories should merge temporarily");
    assert_eq!(
        lopdf::Document::load_mem(&all_pdf)
            .expect("temporary merge should be a valid PDF")
            .get_pages()
            .len(),
        2
    );
    let attachment_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM attachments WHERE owner_id = $1")
            .bind(scope.owner_id)
            .fetch_one(&pool)
            .await
            .expect("attachment count should reload");
    assert_eq!(
        attachment_count_after, attachment_count_before,
        "full-suite PDF must not become a persistent archive object"
    );

    service
        .download_category_pdfs(&scope.actor, instance.id, &[external.id], true, test_now())
        .await
        .expect("authorized emergency download should use the same stable source");
    for action in ["h_file.download", "h_file.emergency_print"] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H-FILE' AND action = $2",
        )
        .bind(scope.owner_id)
        .bind(action)
        .fetch_one(&pool)
        .await
        .expect("H-FILE audit should load");
        assert!(count > 0, "missing audit for {action}");
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn failed_render_retries_same_instance_output_and_idempotency_key(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let unavailable = PrintOrchestrationService::with_postgres(pool.clone());
    let sample_order = seed_order(&pool, &scope, "SO-H9-009-F00", Some("INV-H9-009-F00")).await;
    let sample_group = cutoff(&unavailable, &scope, sample_order, "h9-pdf-fail-g0").await;
    publish_flow(
        &unavailable,
        &scope,
        suite_request(
            &scope,
            "渲染失败重试组套",
            PrintSuiteScope::Customer,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        sample_group,
        "h9-pdf-fail-suite",
    )
    .await;
    let order = seed_order(&pool, &scope, "SO-H9-009-F01", Some("INV-H9-009-F01")).await;
    let group = cutoff(&unavailable, &scope, order, "h9-pdf-fail-g1").await;
    let instance = single_instance(&unavailable, &scope, group).await;
    let failed = unavailable
        .prepare_category_pdfs(&scope.actor, instance.id, test_now(), "h9-pdf-fail-prepare")
        .await
        .expect_err("worker failure must fail closed with a stable render error");
    assert!(matches!(
        failed,
        PrintOrchestrationError::RenderWorker(_)
    ));
    let failed_state = unavailable
        .list_category_pdfs(&scope.actor, instance.id)
        .await
        .expect("failed preparation should remain queryable");
    assert_eq!(failed_state.preparation_status.as_deref(), Some("failed"));
    assert_eq!(failed_state.data[0].processing_status, "failed");
    assert_eq!(failed_state.data[0].attempt_count, 1);
    let failed_output_id = failed_state.data[0].id;
    assert_eq!(
        single_instance(&unavailable, &scope, group).await.status,
        "waiting_documents"
    );

    let recovered_h_file =
        wms_api::file_attachment::FileAttachmentService::with_memory(pool.clone());
    let recovered =
        PrintOrchestrationService::with_file_attachment_for_tests(pool.clone(), recovered_h_file);
    let retried = recovered
        .prepare_category_pdfs(&scope.actor, instance.id, test_now(), "h9-pdf-fail-prepare")
        .await
        .expect("same key should retry the failed output");
    assert!(!retried.replayed);
    assert_eq!(retried.value.status, "completed");
    assert_eq!(retried.value.outputs[0].id, failed_output_id);
    assert_eq!(retried.value.outputs[0].attempt_count, 2);
    assert_eq!(
        single_instance(&recovered, &scope, group).await.status,
        "queued"
    );
    assert_eq!(
        recovered
            .prepare_category_pdfs(
                &scope.actor,
                instance.id,
                test_now(),
                "different-key-is-not-a-retry",
            )
            .await,
        Err(PrintOrchestrationError::IdempotencyConflict)
    );
}

async fn seed_real_invoice_attachment(
    pool: &PgPool,
    scope: &Scope,
    h_file: &wms_api::file_attachment::FileAttachmentService,
    invoice_no: &str,
    label: &str,
) -> wms_api::file_attachment::StoredAttachment {
    let content = wms_api::pdf_document::render_text_pdf(label);
    let attachment = h_file
        .store_pdf(
            &scope.actor,
            wms_api::file_attachment::StorePdfRequest {
                module: "H9".to_string(),
                entity_type: "authoritative_invoice".to_string(),
                entity_id: Uuid::new_v4(),
                file_name: format!("{invoice_no}.pdf"),
                retention_policy: wms_api::file_attachment::FileRetentionPolicy::GspFiveYear,
            },
            &content,
            test_now(),
        )
        .await
        .expect("authoritative invoice should enter H-FILE");
    sqlx::query(
        "INSERT INTO h9_document_file_bindings (id, owner_id, category_code, attachment_id, invoice_no) VALUES ($1, $2, 'invoice', $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(scope.owner_id)
    .bind(attachment.id)
    .bind(invoice_no)
    .execute(pool)
    .await
    .expect("invoice coverage binding should insert");
    attachment
}
