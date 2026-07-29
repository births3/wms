#[sqlx::test(migrations = "../../migrations")]
async fn required_not_ready_applies_frozen_policy_and_completeness(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let sample_order = seed_order(&pool, &scope, "SO-H9-008-P00", Some("INV-H9-008-P00")).await;
    let sample_group = cutoff(&service, &scope, sample_order, "h9-suite-pol-g0").await;

    // Suite 1 (customer scope): invoice required with wait_hold_instance.
    let hold_suite = publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "挂起当前实例组套",
            PrintSuiteScope::Customer,
            vec![
                rendered_item(scope.template_version_id, 1),
                external_item("invoice", 2, true, PrintSuiteReadyPolicy::WaitHoldInstance),
                external_item(
                    "drug_inspection_report",
                    3,
                    true,
                    PrintSuiteReadyPolicy::WaitHoldInstance,
                ),
            ],
        ),
        sample_group,
        "h9-suite-pol-hold",
    )
    .await;

    // Order without any ingested invoice/drug files: instance must hold.
    let order = seed_order(&pool, &scope, "SO-H9-008-P01", Some("INV-H9-008-P01")).await;
    seed_order_line(&pool, &scope, order, "PROD-H9-008", "BATCH-H9-008").await;
    let group = cutoff(&service, &scope, order, "h9-suite-pol-g1").await;
    let instance = single_instance(&service, &scope, group).await;
    assert_eq!(instance.status, "waiting_documents");
    assert_eq!(instance.hold_scope.as_deref(), Some("instance"));
    let invoice_item = instance
        .items
        .iter()
        .find(|item| item.category_code == "invoice")
        .expect("invoice item should exist");
    assert!(!invoice_item.ready);
    assert!(invoice_item.missing[0].contains("INV-H9-008-P01"));
    let drug_item = instance
        .items
        .iter()
        .find(|item| item.category_code == "drug_inspection_report")
        .expect("drug item should exist");
    assert!(!drug_item.ready);
    assert!(drug_item.missing[0].contains("PROD-H9-008"));

    // AC5: seed authoritative files, next instance is complete and queued
    // with frozen file bindings (AC6/AC8).
    seed_invoice_file(
        &pool,
        &scope,
        "INV-H9-008-P02",
        "HFILE-INV-P02",
        "hash-inv-p02",
    )
    .await;
    seed_drug_file(
        &pool,
        &scope,
        "PROD-H9-008",
        "BATCH-H9-008",
        "HFILE-DIR-P02",
        "hash-dir-p02",
    )
    .await;
    let order = seed_order(&pool, &scope, "SO-H9-008-P02", Some("INV-H9-008-P02")).await;
    seed_order_line(&pool, &scope, order, "PROD-H9-008", "BATCH-H9-008").await;
    let group = cutoff(&service, &scope, order, "h9-suite-pol-g2").await;
    let instance = single_instance(&service, &scope, group).await;
    assert_eq!(instance.status, "waiting_documents");
    assert_eq!(instance.hold_scope.as_deref(), Some("instance"));
    let invoice_item = instance
        .items
        .iter()
        .find(|item| item.category_code == "invoice")
        .expect("invoice item should exist");
    assert!(invoice_item.ready);
    assert_eq!(invoice_item.file_bindings.len(), 1);
    assert!(invoice_item.file_bindings[0]
        .file_ref
        .starts_with("h-file:"));
    assert_eq!(
        invoice_item.file_bindings[0].content_hash,
        test_hash("hash-inv-p02")
    );
    let drug_item = instance
        .items
        .iter()
        .find(|item| item.category_code == "drug_inspection_report")
        .expect("drug item should exist");
    assert!(drug_item.ready);
    assert!(drug_item.file_bindings[0]
        .file_ref
        .starts_with("h-file:"));

    // Suite 2: pause_agent_queue freezes the queue-level policy instead.
    service
        .disable_print_suite(&scope.actor, hold_suite, test_now(), "h9-suite-pol-disable")
        .await
        .expect("hold suite should disable");
    publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "暂停队列组套",
            PrintSuiteScope::Customer,
            vec![
                rendered_item(scope.template_version_id, 1),
                external_item("invoice", 2, true, PrintSuiteReadyPolicy::PauseAgentQueue),
            ],
        ),
        sample_group,
        "h9-suite-pol-queue",
    )
    .await;
    let order = seed_order(&pool, &scope, "SO-H9-008-P03", Some("INV-H9-008-P03")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-pol-g3").await;
    let instance = single_instance(&service, &scope, group).await;
    assert_eq!(instance.status, "waiting_documents");
    assert_eq!(instance.hold_scope.as_deref(), Some("agent_queue"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn instance_snapshot_and_policies_are_frozen(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let sample_order = seed_order(&pool, &scope, "SO-H9-008-F00", Some("INV-H9-008-F00")).await;
    let sample_group = cutoff(&service, &scope, sample_order, "h9-suite-frz-g0").await;
    let suite_id = publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "快照冻结组套",
            PrintSuiteScope::Customer,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        sample_group,
        "h9-suite-frz",
    )
    .await;
    let order = seed_order(&pool, &scope, "SO-H9-008-F01", Some("INV-H9-008-F01")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-frz-g1").await;
    let instance = single_instance(&service, &scope, group).await;
    assert_eq!(instance.suite_version_id, suite_id);
    assert_eq!(instance.suite_snapshot["name"], "快照冻结组套");
    assert_eq!(
        instance.source_documents[0]["wms_order_no"],
        "SO-H9-008-F01"
    );

    // AC8: the frozen snapshot cannot be rewritten.
    let rewrite = sqlx::query(
        "UPDATE h9_print_suite_instances SET suite_snapshot = $3 WHERE owner_id = $1 AND id = $2",
    )
    .bind(scope.owner_id)
    .bind(instance.id)
    .bind(json!({"name": "被篡改"}))
    .execute(&pool)
    .await;
    let error = format!("{:?}", rewrite.expect_err("snapshot must be immutable"));
    assert!(error.contains("immutable"), "unexpected: {error}");

    // AC7: frozen per-item policies cannot be relaxed afterwards.
    let policy_rewrite = sqlx::query(
        "UPDATE h9_print_suite_instance_items SET ready_policy = 'pause_agent_queue' WHERE owner_id = $1 AND instance_id = $2",
    )
    .bind(scope.owner_id)
    .bind(instance.id)
    .execute(&pool)
    .await;
    let error = format!("{:?}", policy_rewrite.expect_err("policies must be frozen"));
    assert!(error.contains("frozen"), "unexpected: {error}");

    // Disabling the suite later never mutates the frozen instance snapshot.
    service
        .disable_print_suite(&scope.actor, suite_id, test_now(), "h9-suite-frz-disable")
        .await
        .expect("suite should disable");
    let after = single_instance(&service, &scope, group).await;
    assert_eq!(after.suite_snapshot, instance.suite_snapshot);
    assert_eq!(after.suite_version_no, instance.suite_version_no);
}

async fn cutoff(
    service: &PrintOrchestrationService,
    scope: &Scope,
    order_id: Uuid,
    key: &str,
) -> Uuid {
    service
        .manual_cutoff(
            &scope.actor,
            ManualDeliveryNoteCutoffRequest {
                warehouse_id: scope.warehouse_id,
                delivery_address_id: scope.address_id,
                order_ids: vec![order_id],
                reason: "US-H9-008 测试截单".to_string(),
            },
            test_now(),
            key,
        )
        .await
        .expect("cutoff should create a group")
        .value
        .id
}

async fn instance_suite(
    service: &PrintOrchestrationService,
    scope: &Scope,
    group_id: Uuid,
) -> Uuid {
    single_instance(service, scope, group_id)
        .await
        .suite_version_id
}

async fn single_instance(
    service: &PrintOrchestrationService,
    scope: &Scope,
    group_id: Uuid,
) -> wms_domain::PrintSuiteInstance {
    let instances = service
        .list_print_suite_instances(&scope.actor, Some(group_id))
        .await
        .expect("instance query should work");
    assert_eq!(instances.data.len(), 1, "expected exactly one instance");
    instances.data.into_iter().next().expect("one instance")
}

async fn seed_scope(pool: &PgPool) -> Scope {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let address_id = Uuid::new_v4();
    let actor = ctx(owner_id);
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'H9008', 'H9 组套测试货主')",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner should insert");
    sqlx::query(
        "INSERT INTO auth_users (
            id, username, display_name, password_hash, status
         )
         VALUES ($1, $2, 'H9 组套测试用户', 'not-used-in-test', 'active')",
    )
    .bind(actor.user_id)
    .bind(format!("h9-suite-{}", &actor.user_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("actor should insert");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (
            user_id, owner_id, is_active, is_primary
         )
         VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(actor.user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("actor owner binding should insert");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type) VALUES ($1, $2, 'WH-H9-008', 'H9 组套测试仓', 'distribution')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("warehouse should insert");
    sqlx::query(
        "INSERT INTO customers (id, owner_id, customer_code, customer_name, customer_type) VALUES ($1, $2, 'CUS-H9-008', 'H9 组套测试客户', 'customer')",
    )
    .bind(customer_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("customer should insert");
    sqlx::query(
        "INSERT INTO customer_addresses (id, owner_id, customer_id, province, city, district, detail_address, contact_name, contact_phone) VALUES ($1, $2, $3, '浙江省', '杭州市', '西湖区', '真实数据路 008 号', '组套测试人', '13800000008')",
    )
    .bind(address_id)
    .bind(owner_id)
    .bind(customer_id)
    .execute(pool)
    .await
    .expect("address should insert");
    sqlx::query(
        "INSERT INTO document_number_rules (id, owner_id, document_type, rule_code, rule_name, template, reset_policy, sequence_width, enabled, effective_from) VALUES ($1, $2, 'print_document_category:delivery_note', 'h9-suite-008', '随货同行单号', 'SHTX-{OWNER}-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, TRUE, '2026-07-01T00:00:00Z')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("numbering rule should insert");
    let template_version_id = seed_template_version(pool, actor.user_id).await;
    Scope {
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        actor,
        template_version_id,
    }
}

async fn seed_template_version(pool: &PgPool, actor_id: Uuid) -> Uuid {
    let library_id = Uuid::new_v4();
    let library_version_id = Uuid::new_v4();
    let template_id = Uuid::new_v4();
    let template_version_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO print_field_libraries (id, library_code, library_name, business_module, source_schema) VALUES ($1, 'h9_suite_test', 'H9 组套字段库', 'M4', 'OutboundOrder')",
    )
    .bind(library_id)
    .execute(pool)
    .await
    .expect("field library should insert");
    sqlx::query(
        "INSERT INTO print_field_library_versions (id, library_id, version_no, status, source_schema, business_module, request_hash, created_by, published_at, published_by) VALUES ($1, $2, 1, 'published', 'OutboundOrder', 'M4', 'h9-suite-lib-v1', $3, now(), $3)",
    )
    .bind(library_version_id)
    .bind(library_id)
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("library version should insert");
    sqlx::query(
        "INSERT INTO print_templates (id, owner_id, template_code, template_name, template_type_code, scope, enabled, is_default, created_by, updated_by) VALUES ($1, '00000000-0000-0000-0000-000000000000', 'h9_suite_delivery_note', 'H9 组套随货同行单模板', 'delivery_note', 'global', TRUE, TRUE, $2, $2)",
    )
    .bind(template_id)
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("template should insert");
    sqlx::query(
        r#"
        INSERT INTO print_template_versions (
            id, template_id, field_library_version_id, template_name,
            template_type_code, scope, is_default, version_no, status,
            hiprint_json, field_bindings, paper, designer_version,
            request_hash, created_by, published_at, published_by
        )
        VALUES ($1, $2, $3, 'H9 组套随货同行单模板', 'delivery_note', 'global', TRUE,
                1, 'published', '{"panels":[]}', '[]', '{}', 'hiprint@0.4.0',
                'h9-suite-template-v1', $4, now(), $4)
        "#,
    )
    .bind(template_version_id)
    .bind(template_id)
    .bind(library_version_id)
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("template version should insert");
    template_version_id
}

async fn seed_scoped_template_version(
    pool: &PgPool,
    actor_id: Uuid,
    owner_id: Uuid,
    template_type_code: &str,
    enabled: bool,
) -> Uuid {
    let field_library_version_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM print_field_library_versions WHERE status = 'published' ORDER BY created_at LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("published field library version should exist");
    let template_id = Uuid::new_v4();
    let template_version_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO print_templates (
            id, owner_id, template_code, template_name, template_type_code,
            scope, enabled, is_default, created_by, updated_by
        )
        VALUES ($1, $2, $3, $4, $5, 'owner', $6, FALSE, $7, $7)
        "#,
    )
    .bind(template_id)
    .bind(owner_id)
    .bind(format!("h9_suite_{}", &template_id.to_string()[..8]))
    .bind(format!("H9 {template_type_code} 测试模板"))
    .bind(template_type_code)
    .bind(enabled)
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("scoped template should insert");
    sqlx::query(
        r#"
        INSERT INTO print_template_versions (
            id, template_id, field_library_version_id, template_name,
            template_type_code, scope, is_default, version_no, status,
            hiprint_json, field_bindings, paper, designer_version,
            request_hash, created_by, published_at, published_by
        )
        VALUES (
            $1, $2, $3, $4, $5, 'owner', FALSE, 1, 'published',
            '{"panels":[]}', '[]', '{}', 'hiprint@0.4.0',
            $6, $7, now(), $7
        )
        "#,
    )
    .bind(template_version_id)
    .bind(template_id)
    .bind(field_library_version_id)
    .bind(format!("H9 {template_type_code} 测试模板"))
    .bind(template_type_code)
    .bind(format!("h9-suite-template-{template_version_id}"))
    .bind(actor_id)
    .execute(pool)
    .await
    .expect("scoped template version should insert");
    template_version_id
}

async fn seed_order(
    pool: &PgPool,
    scope: &Scope,
    order_no: &str,
    invoice_no: Option<&str>,
) -> Uuid {
    let order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, erp_order_no, invoice_no,
            customer_id, warehouse_id, delivery_address_id,
            delivery_address_snapshot, status, created_at
         )
         VALUES (
            $1, $2, 'sales_outbound', $3, $3, $4, $5, $6, $7, $8,
            'confirmed', '2026-07-27T08:00:00Z'
         )",
    )
    .bind(order_id)
    .bind(scope.owner_id)
    .bind(order_no)
    .bind(invoice_no)
    .bind(scope.customer_id)
    .bind(scope.warehouse_id)
    .bind(scope.address_id)
    .bind(json!({
        "province": "浙江省",
        "city": "杭州市",
        "district": "西湖区",
        "detail_address": "真实数据路 008 号",
        "contact_name": "组套测试人",
        "contact_phone": "13800000008"
    }))
    .execute(pool)
    .await
    .expect("order should insert");
    sqlx::query(
        "INSERT INTO h9_outbound_route_snapshots (outbound_order_id, owner_id, warehouse_id, customer_id, delivery_address_id, route_code, frozen_at) VALUES ($1, $2, $3, $4, $5, 'LINE-H9-008', '2026-07-27T08:00:00Z')",
    )
    .bind(order_id)
    .bind(scope.owner_id)
    .bind(scope.warehouse_id)
    .bind(scope.customer_id)
    .bind(scope.address_id)
    .execute(pool)
    .await
    .expect("route snapshot should insert");
    order_id
}

async fn seed_order_line(
    pool: &PgPool,
    scope: &Scope,
    order_id: Uuid,
    product_code: &str,
    batch_no: &str,
) {
    sqlx::query(
        "INSERT INTO outbound_order_lines (id, outbound_order_id, owner_id, line_no, product_code, batch_no, planned_qty) VALUES ($1, $2, $3, 1, $4, $5, 10)",
    )
    .bind(Uuid::new_v4())
    .bind(order_id)
    .bind(scope.owner_id)
    .bind(product_code)
    .bind(batch_no)
    .execute(pool)
    .await
    .expect("order line should insert");
}

async fn seed_invoice_file(
    pool: &PgPool,
    scope: &Scope,
    invoice_no: &str,
    file_ref: &str,
    content_hash: &str,
) {
    let attachment_id = Uuid::new_v4();
    let content_hash = test_hash(content_hash);
    sqlx::query(
        r#"
        INSERT INTO attachments (
            id, owner_id, module, entity_type, entity_id, bucket, storage_key,
            file_name, content_type, size_bytes, content_hash, sha256, file_version,
            status, retention_policy, retain_until, created_by, uploaded_by,
            confirmed_at
        )
        VALUES (
            $1, $2, 'H9', 'authoritative_invoice', $1, 'wms-attachments', $3,
            'invoice.pdf', 'application/pdf', 100, $4, $4, 1, 'ready',
            'gsp_5_year', now() + interval '5 years', $5, $5, now()
        )
        "#,
    )
    .bind(attachment_id)
    .bind(scope.owner_id)
    .bind(file_ref)
    .bind(&content_hash)
    .bind(scope.actor.user_id)
    .execute(pool)
    .await
    .expect("invoice attachment should insert");
    sqlx::query(
        "INSERT INTO h9_document_file_bindings (id, owner_id, category_code, attachment_id, invoice_no) VALUES ($1, $2, 'invoice', $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(scope.owner_id)
    .bind(attachment_id)
    .bind(invoice_no)
    .execute(pool)
    .await
    .expect("invoice binding should insert");
}

async fn seed_drug_file(
    pool: &PgPool,
    scope: &Scope,
    product_code: &str,
    batch_no: &str,
    file_ref: &str,
    content_hash: &str,
) {
    let attachment_id = Uuid::new_v4();
    let content_hash = test_hash(content_hash);
    sqlx::query(
        r#"
        INSERT INTO attachments (
            id, owner_id, module, entity_type, entity_id, bucket, storage_key,
            file_name, content_type, size_bytes, content_hash, sha256, file_version,
            status, retention_policy, retain_until, created_by, uploaded_by,
            confirmed_at
        )
        VALUES (
            $1, $2, 'H9', 'authoritative_drug_report', $1, 'wms-attachments', $3,
            'drug-report.pdf', 'application/pdf', 100, $4, $4, 1, 'ready',
            'gsp_5_year', now() + interval '5 years', $5, $5, now()
        )
        "#,
    )
    .bind(attachment_id)
    .bind(scope.owner_id)
    .bind(file_ref)
    .bind(&content_hash)
    .bind(scope.actor.user_id)
    .execute(pool)
    .await
    .expect("drug-report attachment should insert");
    sqlx::query(
        "INSERT INTO h9_document_file_bindings (id, owner_id, category_code, attachment_id, product_code, batch_no) VALUES ($1, $2, 'drug_inspection_report', $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(scope.owner_id)
    .bind(attachment_id)
    .bind(product_code)
    .bind(batch_no)
    .execute(pool)
    .await
    .expect("drug-report binding should insert");
}

fn test_hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
