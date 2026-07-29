use chrono::{TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    print_orchestration::{PrintOrchestrationError, PrintOrchestrationService},
    wave4_repository::PgWave4Repository,
};
use wms_domain::{
    AggregationDimension, AggregationFieldCode, AggregationMethod,
    CreateAggregationRuleDraftRequest, CreateOutboundOrderRequest, ManualDeliveryNoteCutoffRequest,
    TestAggregationRuleRequest,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "h9-rule-test".to_string(),
        permissions: vec!["h9.print_orchestration.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn published_rule_tests_real_orders_and_freezes_cutoff_snapshot(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let address_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 16, 0, 0)
        .single()
        .expect("valid test time");
    seed_scope(&pool, owner_id, warehouse_id, customer_id, address_id).await;
    let first = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-007-01",
        "INV-H9-007-A",
    )
    .await;
    let second = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-007-02",
        "INV-H9-007-A",
    )
    .await;
    let third = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-007-03",
        "INV-H9-007-B",
    )
    .await;
    let auth = ctx(owner_id);
    let service = PrintOrchestrationService::with_postgres(pool.clone());

    let draft = service
        .create_aggregation_rule_draft(
            &auth,
            CreateAggregationRuleDraftRequest {
                name: "按发票号归集".to_string(),
                dimensions: vec![AggregationDimension {
                    field_code: AggregationFieldCode::InvoiceNo,
                    method: AggregationMethod::Equals,
                    order: 1,
                }],
            },
            now,
            "h9-rule-draft-007",
        )
        .await
        .expect("rule draft should be created");
    assert_eq!(draft.value.status, "draft");
    assert_eq!(draft.value.version_no, 1);

    let tested = service
        .test_aggregation_rule(
            &auth,
            draft.value.id,
            TestAggregationRuleRequest {
                order_ids: vec![first, second, third],
            },
            now,
            "h9-rule-test-007",
        )
        .await
        .expect("real sample orders should be grouped");
    assert_eq!(tested.value.rule.status, "tested");
    assert_eq!(tested.value.groups.len(), 2);
    assert_eq!(tested.value.groups[0].group_key[0].field_code, "invoice_no");
    assert_eq!(tested.value.groups[0].group_key[0].value, "INV-H9-007-A");
    assert_eq!(tested.value.groups[0].order_ids, vec![first, second]);
    assert_eq!(tested.value.groups[1].order_ids, vec![third]);

    let published = service
        .publish_aggregation_rule(&auth, draft.value.id, now, "h9-rule-publish-007")
        .await
        .expect("tested rule should publish");
    assert_eq!(published.value.status, "published");

    let mixed = service
        .manual_cutoff(
            &auth,
            ManualDeliveryNoteCutoffRequest {
                warehouse_id,
                delivery_address_id: address_id,
                order_ids: vec![first, third],
                reason: "不得跨规则分组".to_string(),
            },
            now,
            "h9-rule-cutoff-mixed-007",
        )
        .await;
    assert_eq!(mixed, Err(PrintOrchestrationError::AggregationRuleMismatch));

    let cutoff = service
        .manual_cutoff(
            &auth,
            ManualDeliveryNoteCutoffRequest {
                warehouse_id,
                delivery_address_id: address_id,
                order_ids: vec![first, second],
                reason: "按已发布归集规则截单".to_string(),
            },
            now,
            "h9-rule-cutoff-007",
        )
        .await
        .expect("same rule key should cut off");
    assert_eq!(
        cutoff.value.aggregation_rule_version_id,
        Some(draft.value.id)
    );
    assert_eq!(cutoff.value.aggregation_rule_version_no, Some(1));
    assert_eq!(
        cutoff.value.aggregation_group_key["invoice_no"],
        "INV-H9-007-A"
    );

    let stored_snapshot: serde_json::Value = sqlx::query_scalar(
        "SELECT aggregation_rule_snapshot FROM h9_delivery_note_groups WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(cutoff.value.id)
    .fetch_one(&pool)
    .await
    .expect("frozen rule snapshot should load");
    assert_eq!(stored_snapshot["version_no"], 1);
    assert_eq!(stored_snapshot["dimensions"][0]["field_code"], "invoice_no");
}

#[sqlx::test(migrations = "../../migrations")]
async fn scheduled_cutoff_splits_one_address_by_published_rule_key(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let address_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 16, 0, 0)
        .single()
        .expect("valid test time");
    seed_scope(&pool, owner_id, warehouse_id, customer_id, address_id).await;
    let first = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-007-S01",
        "INV-H9-007-A",
    )
    .await;
    let second = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-007-S02",
        "INV-H9-007-A",
    )
    .await;
    let third = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-007-S03",
        "INV-H9-007-B",
    )
    .await;
    let auth = ctx(owner_id);
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let draft = service
        .create_aggregation_rule_draft(
            &auth,
            invoice_rule_request(),
            now,
            "h9-rule-draft-scheduled-007",
        )
        .await
        .expect("rule draft should be created");
    service
        .test_aggregation_rule(
            &auth,
            draft.value.id,
            TestAggregationRuleRequest {
                order_ids: vec![first, second, third],
            },
            now,
            "h9-rule-test-scheduled-007",
        )
        .await
        .expect("rule should test");
    service
        .publish_aggregation_rule(&auth, draft.value.id, now, "h9-rule-publish-scheduled-007")
        .await
        .expect("rule should publish");
    sqlx::query(
        r#"
        INSERT INTO h9_cutoff_plans (
            id, owner_id, name, warehouse_id, scope_type, customer_id,
            utc_offset_minutes, weekly_schedule, exceptions, effective_from,
            status, created_by, published_by, published_at
        )
        VALUES (
            $1, $2, 'H9 规则定时截单', $3, 'customer', $4, 0,
            '[{"weekday":7,"cutoff_time":"15:00"}]'::jsonb, '[]'::jsonb,
            '2026-07-01T00:00:00Z', 'published', $5, $5, $6
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("cutoff plan should insert");

    let groups = service
        .run_scheduled_cutoffs(&auth, now)
        .await
        .expect("scheduled cutoff should split by rule key");
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].order_ids, vec![first, second]);
    assert_eq!(
        groups[0].aggregation_group_key["invoice_no"],
        "INV-H9-007-A"
    );
    assert_eq!(groups[1].order_ids, vec![third]);
    assert_eq!(
        groups[1].aggregation_group_key["invoice_no"],
        "INV-H9-007-B"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn rule_lifecycle_replays_audits_and_rejects_rewrite(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let address_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 27, 9, 0, 0)
        .single()
        .expect("valid test time");
    seed_scope(&pool, owner_id, warehouse_id, customer_id, address_id).await;
    let sample = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-007-L01",
        "INV-H9-007-L",
    )
    .await;
    let auth = ctx(owner_id);
    let service = PrintOrchestrationService::with_postgres(pool.clone());

    // 幂等重放：同一幂等键重复创建返回同一版本且标记 replayed
    let draft = service
        .create_aggregation_rule_draft(&auth, invoice_rule_request(), now, "h9-rule-life-draft")
        .await
        .expect("draft should be created");
    let replayed = service
        .create_aggregation_rule_draft(&auth, invoice_rule_request(), now, "h9-rule-life-draft")
        .await
        .expect("same idempotency key should replay");
    assert!(replayed.replayed);
    assert_eq!(replayed.value.id, draft.value.id);

    // 非法状态转换：草稿不得直接发布
    let premature = service
        .publish_aggregation_rule(&auth, draft.value.id, now, "h9-rule-life-premature")
        .await;
    assert_eq!(
        premature,
        Err(PrintOrchestrationError::AggregationRuleInvalidState)
    );

    service
        .test_aggregation_rule(
            &auth,
            draft.value.id,
            TestAggregationRuleRequest {
                order_ids: vec![sample],
            },
            now,
            "h9-rule-life-test",
        )
        .await
        .expect("rule should test");
    service
        .publish_aggregation_rule(&auth, draft.value.id, now, "h9-rule-life-publish")
        .await
        .expect("tested rule should publish");

    // AC3：已发布版本内容不可改写（数据库触发器兜底）
    let rewrite = sqlx::query(
        "UPDATE h9_aggregation_rule_versions SET name = '篡改名称' WHERE owner_id = $1 AND id = $2",
    )
    .bind(owner_id)
    .bind(draft.value.id)
    .execute(&pool)
    .await;
    let rewrite_error = format!(
        "{:?}",
        rewrite.expect_err("published content must be immutable")
    );
    assert!(
        rewrite_error.contains("immutable"),
        "unexpected: {rewrite_error}"
    );

    let disabled = service
        .disable_aggregation_rule(&auth, draft.value.id, now, "h9-rule-life-disable")
        .await
        .expect("published rule should disable");
    assert_eq!(disabled.value.status, "disabled");

    // AC6：全部规则动作写入 H2 审计
    for action in [
        "create_aggregation_rule_draft",
        "test_aggregation_rule",
        "publish_aggregation_rule",
        "disable_aggregation_rule",
    ] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H9' AND action = $2",
        )
        .bind(owner_id)
        .bind(action)
        .fetch_one(&pool)
        .await
        .expect("audit count should load");
        assert_eq!(count, 1, "audit missing for {action}");
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_creation_persists_all_configurable_aggregation_fields(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let address_id = Uuid::new_v4();
    let now = Utc
        .with_ymd_and_hms(2026, 7, 27, 14, 0, 0)
        .single()
        .expect("valid test time");
    seed_scope(&pool, owner_id, warehouse_id, customer_id, address_id).await;
    let auth = ctx(owner_id);
    sqlx::query(
        r#"
        INSERT INTO h9_route_bindings (
            id, owner_id, warehouse_id, customer_id, delivery_address_id,
            route_code, effective_from, created_by
        )
        VALUES ($1, $2, $3, $4, $5, 'LINE-H9-007', $6, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .bind(now - chrono::Duration::days(1))
    .bind(auth.user_id)
    .execute(&pool)
    .await
    .expect("route binding should insert");

    // 通过真实创建入口提交，而不是在测试里直写 outbound_orders。
    let request: CreateOutboundOrderRequest = serde_json::from_value(serde_json::json!({
        "document_type": "sales_outbound",
        "wms_order_no": "SO-H9-007-PROD",
        "erp_order_no": "ERP-H9-007-PROD",
        "invoice_no": "INV-H9-007-PROD",
        "transport_mode_code": "121",
        "department_code": "302",
        "sales_group_code": "SG-01",
        "order_group_no": "GRP-H9-007",
        "business_type_code": "THIRD_PARTY",
        "customer_id": customer_id,
        "warehouse_id": warehouse_id,
        "delivery_address_id": address_id,
        "required_ship_at": now,
        "lines": [{
            "line_no": 1,
            "product_code": "P-H9-007",
            "batch_no": "B-H9-007",
            "planned_qty": 1
        }]
    }))
    .expect("production request should deserialize");
    let order = PgWave4Repository::new(pool.clone())
        .create_outbound_order(&auth, request, now, "h9-rule-production-write-007", None)
        .await
        .expect("production outbound create should persist aggregation fields")
        .value;

    type StoredAggregationFields = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let stored: StoredAggregationFields = sqlx::query_as(
        r#"
        SELECT invoice_no, transport_mode_code, department_code,
               sales_group_code, order_group_no, business_type_code
          FROM outbound_orders
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("created order should load");
    assert_eq!(
        stored,
        (
            Some("INV-H9-007-PROD".to_string()),
            Some("121".to_string()),
            Some("302".to_string()),
            Some("SG-01".to_string()),
            Some("GRP-H9-007".to_string()),
            Some("THIRD_PARTY".to_string()),
        )
    );
}

fn invoice_rule_request() -> CreateAggregationRuleDraftRequest {
    CreateAggregationRuleDraftRequest {
        name: "按发票号归集".to_string(),
        dimensions: vec![AggregationDimension {
            field_code: AggregationFieldCode::InvoiceNo,
            method: AggregationMethod::Equals,
            order: 1,
        }],
    }
}

async fn seed_scope(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    customer_id: Uuid,
    address_id: Uuid,
) {
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'H9007', 'H9 规则测试货主')",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner should insert");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type) VALUES ($1, $2, 'WH-H9-007', 'H9 规则测试仓', 'distribution')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("warehouse should insert");
    sqlx::query(
        "INSERT INTO customers (id, owner_id, customer_code, customer_name, customer_type) VALUES ($1, $2, 'CUS-H9-007', 'H9 规则测试客户', 'customer')",
    )
    .bind(customer_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("customer should insert");
    sqlx::query(
        "INSERT INTO customer_addresses (id, owner_id, customer_id, province, city, district, detail_address, contact_name, contact_phone) VALUES ($1, $2, $3, '浙江省', '杭州市', '拱墅区', '真实数据路 007 号', '规则测试人', '13800000007')",
    )
    .bind(address_id)
    .bind(owner_id)
    .bind(customer_id)
    .execute(pool)
    .await
    .expect("address should insert");
    sqlx::query(
        "INSERT INTO document_number_rules (id, owner_id, document_type, rule_code, rule_name, template, reset_policy, sequence_width, enabled, effective_from) VALUES ($1, $2, 'print_document_category:delivery_note', 'h9-rule-007', '随货同行单号', 'SHTX-{OWNER}-{YYYY}{MM}{DD}-{SEQ}', 'daily', 4, TRUE, '2026-07-01T00:00:00Z')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("numbering rule should insert");
}

async fn seed_order(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    customer_id: Uuid,
    address_id: Uuid,
    order_no: &str,
    invoice_no: &str,
) -> Uuid {
    let order_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO outbound_orders (id, owner_id, document_type, wms_order_no, erp_order_no, invoice_no, customer_id, warehouse_id, status, created_at) VALUES ($1, $2, 'sales_outbound', $3, $3, $4, $5, $6, 'confirmed', '2026-07-26T08:00:00Z')",
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(order_no)
    .bind(invoice_no)
    .bind(customer_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("order should insert");
    sqlx::query(
        "INSERT INTO h9_outbound_route_snapshots (outbound_order_id, owner_id, warehouse_id, customer_id, delivery_address_id, route_code, frozen_at) VALUES ($1, $2, $3, $4, $5, 'LINE-H9-007', '2026-07-26T08:00:00Z')",
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .execute(pool)
    .await
    .expect("route snapshot should insert");
    order_id
}
