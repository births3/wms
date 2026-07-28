use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    print_orchestration::{PrintOrchestrationError, PrintOrchestrationService},
};
use wms_domain::{
    CreatePrintSuiteDraftRequest, ManualDeliveryNoteCutoffRequest, PrintSuiteFailurePolicy,
    PrintSuiteItemInput, PrintSuiteReadyPolicy, PrintSuiteScope, PrintSuiteSourceMode,
    TestPrintSuiteRequest,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "h9-suite-test".to_string(),
        permissions: vec!["h9.print_orchestration.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn test_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 27, 10, 0, 0)
        .single()
        .expect("valid test time")
}

struct Scope {
    owner_id: Uuid,
    warehouse_id: Uuid,
    customer_id: Uuid,
    address_id: Uuid,
    actor: AuthContext,
    template_version_id: Uuid,
}

fn rendered_item(template_version_id: Uuid, sort_order: i32) -> PrintSuiteItemInput {
    PrintSuiteItemInput {
        category_code: "delivery_note".to_string(),
        copies: 2,
        sort_order,
        output_slot: "tray-a4".to_string(),
        required: true,
        ready_policy: PrintSuiteReadyPolicy::WaitHoldInstance,
        failure_policy: PrintSuiteFailurePolicy::PauseSuite,
        source_mode: PrintSuiteSourceMode::Rendered,
        template_version_id: Some(template_version_id),
        external_file_ref: None,
    }
}

fn external_item(
    category_code: &str,
    sort_order: i32,
    required: bool,
    ready_policy: PrintSuiteReadyPolicy,
) -> PrintSuiteItemInput {
    PrintSuiteItemInput {
        category_code: category_code.to_string(),
        copies: 1,
        sort_order,
        output_slot: "tray-a4".to_string(),
        required,
        ready_policy,
        failure_policy: if required {
            PrintSuiteFailurePolicy::PauseSuite
        } else {
            PrintSuiteFailurePolicy::SkipAndContinue
        },
        source_mode: PrintSuiteSourceMode::ExternalFile,
        template_version_id: None,
        external_file_ref: Some(format!("h-file:{category_code}")),
    }
}

fn suite_request(
    scope: &Scope,
    name: &str,
    suite_scope: PrintSuiteScope,
    items: Vec<PrintSuiteItemInput>,
) -> CreatePrintSuiteDraftRequest {
    let (customer_id, delivery_address_id, route_code) = match suite_scope {
        PrintSuiteScope::DeliveryAddress => (Some(scope.customer_id), Some(scope.address_id), None),
        PrintSuiteScope::Customer => (Some(scope.customer_id), None, None),
        PrintSuiteScope::Route => (None, None, Some("LINE-H9-008".to_string())),
        PrintSuiteScope::WarehouseDefault => (None, None, None),
    };
    CreatePrintSuiteDraftRequest {
        name: name.to_string(),
        warehouse_id: scope.warehouse_id,
        scope: suite_scope,
        customer_id,
        delivery_address_id,
        route_code,
        effective_from: Utc
            .with_ymd_and_hms(2026, 7, 1, 0, 0, 0)
            .single()
            .expect("valid effective from"),
        effective_to: None,
        items,
    }
}

async fn publish_flow(
    service: &PrintOrchestrationService,
    scope: &Scope,
    request: CreatePrintSuiteDraftRequest,
    sample_group_id: Uuid,
    key_prefix: &str,
) -> Uuid {
    let draft = service
        .create_print_suite_draft(
            &scope.actor,
            request,
            test_now(),
            &format!("{key_prefix}-draft"),
        )
        .await
        .expect("suite draft should be created");
    service
        .test_print_suite(
            &scope.actor,
            draft.value.id,
            TestPrintSuiteRequest {
                group_ids: vec![sample_group_id],
            },
            test_now(),
            &format!("{key_prefix}-test"),
        )
        .await
        .expect("suite should test");
    service
        .publish_print_suite(
            &scope.actor,
            draft.value.id,
            test_now(),
            &format!("{key_prefix}-publish"),
        )
        .await
        .expect("tested suite should publish");
    draft.value.id
}

#[sqlx::test(migrations = "../../migrations")]
async fn suite_resolution_prefers_address_customer_route_then_default(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let sample_order = seed_order(&pool, &scope, "SO-H9-008-R00", Some("INV-H9-008-R00")).await;
    let sample_group = cutoff(&service, &scope, sample_order, "h9-suite-res-g0").await;

    // Publish one suite per scope level with distinguishable names.
    let address_suite = publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "地址级组套",
            PrintSuiteScope::DeliveryAddress,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        sample_group,
        "h9-suite-res-address",
    )
    .await;
    let customer_suite = publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "客户级组套",
            PrintSuiteScope::Customer,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        sample_group,
        "h9-suite-res-customer",
    )
    .await;
    let route_suite = publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "线路级组套",
            PrintSuiteScope::Route,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        sample_group,
        "h9-suite-res-route",
    )
    .await;
    let default_suite = publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "仓库默认组套",
            PrintSuiteScope::WarehouseDefault,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        sample_group,
        "h9-suite-res-default",
    )
    .await;

    // AC1: delivery address wins first.
    let order = seed_order(&pool, &scope, "SO-H9-008-R01", Some("INV-H9-008-R01")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-res-g1").await;
    assert_eq!(instance_suite(&service, &scope, group).await, address_suite);

    // Then customer.
    service
        .disable_print_suite(&scope.actor, address_suite, test_now(), "h9-suite-res-d1")
        .await
        .expect("address suite should disable");
    let order = seed_order(&pool, &scope, "SO-H9-008-R02", Some("INV-H9-008-R02")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-res-g2").await;
    assert_eq!(
        instance_suite(&service, &scope, group).await,
        customer_suite
    );

    // Then route.
    service
        .disable_print_suite(&scope.actor, customer_suite, test_now(), "h9-suite-res-d2")
        .await
        .expect("customer suite should disable");
    let order = seed_order(&pool, &scope, "SO-H9-008-R03", Some("INV-H9-008-R03")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-res-g3").await;
    assert_eq!(instance_suite(&service, &scope, group).await, route_suite);

    // Then owner-warehouse default.
    service
        .disable_print_suite(&scope.actor, route_suite, test_now(), "h9-suite-res-d3")
        .await
        .expect("route suite should disable");
    let order = seed_order(&pool, &scope, "SO-H9-008-R04", Some("INV-H9-008-R04")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-res-g4").await;
    assert_eq!(instance_suite(&service, &scope, group).await, default_suite);

    // Without any published suite the cutoff stays backward compatible.
    service
        .disable_print_suite(&scope.actor, default_suite, test_now(), "h9-suite-res-d4")
        .await
        .expect("default suite should disable");
    let order = seed_order(&pool, &scope, "SO-H9-008-R05", Some("INV-H9-008-R05")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-res-g5").await;
    let instances = service
        .list_print_suite_instances(&scope.actor, Some(group))
        .await
        .expect("instance query should work");
    assert!(instances.data.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_level_same_object_overlap_rejected_on_publish(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let order = seed_order(&pool, &scope, "SO-H9-008-O01", Some("INV-H9-008-O01")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-ovl-g1").await;

    publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "客户组套 A",
            PrintSuiteScope::Customer,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        group,
        "h9-suite-ovl-a",
    )
    .await;

    let overlapping = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(
                &scope,
                "客户组套 B（重叠）",
                PrintSuiteScope::Customer,
                vec![rendered_item(scope.template_version_id, 1)],
            ),
            test_now(),
            "h9-suite-ovl-b-draft",
        )
        .await
        .expect("overlapping draft should be created");
    service
        .test_print_suite(
            &scope.actor,
            overlapping.value.id,
            TestPrintSuiteRequest {
                group_ids: vec![group],
            },
            test_now(),
            "h9-suite-ovl-b-test",
        )
        .await
        .expect("overlapping draft should test");
    let publish = service
        .publish_print_suite(
            &scope.actor,
            overlapping.value.id,
            test_now(),
            "h9-suite-ovl-b-publish",
        )
        .await;
    assert_eq!(
        publish,
        Err(PrintOrchestrationError::EffectivePeriodOverlap)
    );

    // A different scope object at the same level is not an overlap.
    publish_flow(
        &service,
        &scope,
        suite_request(
            &scope,
            "线路组套（不同对象）",
            PrintSuiteScope::Route,
            vec![rendered_item(scope.template_version_id, 1)],
        ),
        group,
        "h9-suite-ovl-route",
    )
    .await;
}

#[sqlx::test(migrations = "../../migrations")]
async fn suite_lifecycle_replays_audits_and_rejects_rewrite(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let order = seed_order(&pool, &scope, "SO-H9-008-L01", Some("INV-H9-008-L01")).await;
    let group = cutoff(&service, &scope, order, "h9-suite-life-g1").await;

    let request = suite_request(
        &scope,
        "生命周期组套",
        PrintSuiteScope::Customer,
        vec![rendered_item(scope.template_version_id, 1)],
    );
    let draft = service
        .create_print_suite_draft(
            &scope.actor,
            request.clone(),
            test_now(),
            "h9-suite-life-draft",
        )
        .await
        .expect("draft should be created");
    assert_eq!(draft.value.status, "draft");
    assert_eq!(draft.value.items.len(), 1);
    assert_eq!(draft.value.items[0].category_name, "随货同行单");

    // AC9: idempotent replay returns the original version.
    let replayed = service
        .create_print_suite_draft(&scope.actor, request, test_now(), "h9-suite-life-draft")
        .await
        .expect("same idempotency key should replay");
    assert!(replayed.replayed);
    assert_eq!(replayed.value.id, draft.value.id);

    // AC2: a draft cannot be published before its readiness test.
    let premature = service
        .publish_print_suite(
            &scope.actor,
            draft.value.id,
            test_now(),
            "h9-suite-life-premature",
        )
        .await;
    assert_eq!(
        premature,
        Err(PrintOrchestrationError::PrintSuiteInvalidState)
    );

    let tested = service
        .test_print_suite(
            &scope.actor,
            draft.value.id,
            TestPrintSuiteRequest {
                group_ids: vec![group],
            },
            test_now(),
            "h9-suite-life-test",
        )
        .await
        .expect("suite should test");
    assert_eq!(tested.value.suite.status, "tested");
    assert_eq!(tested.value.samples.len(), 1);
    assert!(tested.value.samples[0].matches_this_version);
    assert_eq!(
        tested.value.samples[0].resolved_scope,
        Some(PrintSuiteScope::Customer)
    );
    assert!(tested.value.samples[0].item_readiness[0].ready);

    service
        .publish_print_suite(
            &scope.actor,
            draft.value.id,
            test_now(),
            "h9-suite-life-publish",
        )
        .await
        .expect("tested suite should publish");

    // AC2: published version content is immutable (database trigger).
    let rewrite = sqlx::query(
        "UPDATE h9_print_suite_versions SET name = '篡改名称' WHERE owner_id = $1 AND id = $2",
    )
    .bind(scope.owner_id)
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

    let item_rewrite = sqlx::query(
        "UPDATE h9_print_suite_items SET copies = 9 WHERE owner_id = $1 AND suite_version_id = $2",
    )
    .bind(scope.owner_id)
    .bind(draft.value.id)
    .execute(&pool)
    .await;
    let item_error = format!(
        "{:?}",
        item_rewrite.expect_err("published items must be immutable")
    );
    assert!(item_error.contains("immutable"), "unexpected: {item_error}");

    let disabled = service
        .disable_print_suite(
            &scope.actor,
            draft.value.id,
            test_now(),
            "h9-suite-life-disable",
        )
        .await
        .expect("published suite should disable");
    assert_eq!(disabled.value.status, "disabled");

    // AC9: every lifecycle action writes one H2 audit event.
    for action in [
        "create_print_suite_draft",
        "test_print_suite",
        "publish_print_suite",
        "disable_print_suite",
    ] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H9' AND action = $2",
        )
        .bind(scope.owner_id)
        .bind(action)
        .fetch_one(&pool)
        .await
        .expect("audit count should load");
        assert_eq!(count, 1, "audit missing for {action}");
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn binding_and_category_validation_is_mode_specific(pool: PgPool) {
    let scope = seed_scope(&pool).await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());

    // rendered without a template version is rejected before the database.
    let mut item = rendered_item(scope.template_version_id, 1);
    item.template_version_id = None;
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(&scope, "缺模板绑定", PrintSuiteScope::Customer, vec![item]),
            test_now(),
            "h9-suite-bind-1",
        )
        .await;
    assert_eq!(result, Err(PrintOrchestrationError::InvalidRequest));

    // rendered bound to an unknown template version is rejected.
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(
                &scope,
                "未知模板版本",
                PrintSuiteScope::Customer,
                vec![rendered_item(Uuid::new_v4(), 1)],
            ),
            test_now(),
            "h9-suite-bind-2",
        )
        .await;
    assert_eq!(
        result,
        Err(PrintOrchestrationError::PrintSuiteBindingInvalid)
    );

    // external_file must not carry a transient URL as its stable source.
    let mut item = external_item("invoice", 1, true, PrintSuiteReadyPolicy::WaitHoldInstance);
    item.external_file_ref = Some("https://temp.example.com/invoice.pdf".to_string());
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(
                &scope,
                "临时 URL 引用",
                PrintSuiteScope::Customer,
                vec![item],
            ),
            test_now(),
            "h9-suite-bind-3",
        )
        .await;
    assert_eq!(result, Err(PrintOrchestrationError::InvalidRequest));

    // Categories must come from the controlled M1 dictionary.
    let mut item = external_item("invoice", 1, true, PrintSuiteReadyPolicy::WaitHoldInstance);
    item.category_code = "unregistered_category".to_string();
    item.external_file_ref = Some("h-file:unregistered_category".to_string());
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(&scope, "未登记分类", PrintSuiteScope::Customer, vec![item]),
            test_now(),
            "h9-suite-bind-4",
        )
        .await;
    assert_eq!(
        result,
        Err(PrintOrchestrationError::PrintSuiteCategoryInvalid)
    );

    // source_mode must match the dictionary source_mode (invoice is external).
    let mut item = rendered_item(scope.template_version_id, 1);
    item.category_code = "invoice".to_string();
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(
                &scope,
                "来源模式不匹配",
                PrintSuiteScope::Customer,
                vec![item],
            ),
            test_now(),
            "h9-suite-bind-5",
        )
        .await;
    assert_eq!(
        result,
        Err(PrintOrchestrationError::PrintSuiteCategoryInvalid)
    );

    // 已发布版本还必须属于当前货主（或全局）、类型匹配且父模板启用。
    let other_owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '其他模板货主')",
    )
    .bind(other_owner_id)
    .bind(format!("H9O-{}", &other_owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("other owner should insert");
    let cross_owner = seed_scoped_template_version(
        &pool,
        scope.actor.user_id,
        other_owner_id,
        "delivery_note",
        true,
    )
    .await;
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(
                &scope,
                "跨货主模板",
                PrintSuiteScope::Customer,
                vec![rendered_item(cross_owner, 1)],
            ),
            test_now(),
            "h9-suite-bind-cross-owner",
        )
        .await;
    assert_eq!(
        result,
        Err(PrintOrchestrationError::PrintSuiteBindingInvalid)
    );

    let wrong_type =
        seed_scoped_template_version(&pool, scope.actor.user_id, scope.owner_id, "invoice", true)
            .await;
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(
                &scope,
                "模板类型不匹配",
                PrintSuiteScope::Customer,
                vec![rendered_item(wrong_type, 1)],
            ),
            test_now(),
            "h9-suite-bind-wrong-type",
        )
        .await;
    assert_eq!(
        result,
        Err(PrintOrchestrationError::PrintSuiteBindingInvalid)
    );

    let disabled = seed_scoped_template_version(
        &pool,
        scope.actor.user_id,
        scope.owner_id,
        "delivery_note",
        false,
    )
    .await;
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(
                &scope,
                "停用模板",
                PrintSuiteScope::Customer,
                vec![rendered_item(disabled, 1)],
            ),
            test_now(),
            "h9-suite-bind-disabled",
        )
        .await;
    assert_eq!(
        result,
        Err(PrintOrchestrationError::PrintSuiteBindingInvalid)
    );

    // ADR-0041: required items may never use skip_and_continue.
    let mut item = external_item("invoice", 1, true, PrintSuiteReadyPolicy::WaitHoldInstance);
    item.failure_policy = PrintSuiteFailurePolicy::SkipAndContinue;
    let result = service
        .create_print_suite_draft(
            &scope.actor,
            suite_request(&scope, "必需项跳过", PrintSuiteScope::Customer, vec![item]),
            test_now(),
            "h9-suite-bind-6",
        )
        .await;
    assert_eq!(result, Err(PrintOrchestrationError::InvalidRequest));

    // Database CHECK backstop: a rendered row with an external ref is invalid.
    let version_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO h9_print_suite_versions (
            id, owner_id, version_no, name, status, warehouse_id, scope_type,
            customer_id, effective_from, created_by
        )
        VALUES ($1, $2, 99, '直插版本', 'draft', $3, 'customer', $4, now(), $5)
        "#,
    )
    .bind(version_id)
    .bind(scope.owner_id)
    .bind(scope.warehouse_id)
    .bind(scope.customer_id)
    .bind(scope.actor.user_id)
    .execute(&pool)
    .await
    .expect("direct version insert should work");
    let bad_row = sqlx::query(
        r#"
        INSERT INTO h9_print_suite_items (
            id, owner_id, suite_version_id, category_code, copies, sort_order,
            output_slot, required, ready_policy, failure_policy, source_mode,
            template_version_id, external_file_ref
        )
        VALUES ($1, $2, $3, 'delivery_note', 1, 1, 'tray-a4', TRUE,
                'wait_hold_instance', 'pause_suite', 'rendered', $4, 'h-file:invoice')
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(scope.owner_id)
    .bind(version_id)
    .bind(scope.template_version_id)
    .execute(&pool)
    .await;
    assert!(
        bad_row.is_err(),
        "CHECK constraint must reject mixed binding"
    );
}

include!("h9_print_suite_postgres/part2.rs");
