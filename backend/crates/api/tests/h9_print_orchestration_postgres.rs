use axum::{
    body::Body,
    http::{Request, StatusCode},
    Extension,
};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    print_orchestration::{PrintOrchestrationError, PrintOrchestrationService},
    print_orchestration_handlers::{print_orchestration_router, PrintOrchestrationAppState},
    print_orchestration_job,
    wave4_repository::PgWave4Repository,
};
use wms_domain::{
    CreateCutoffPlanRequest, CreateOutboundOrderLineRequest, CreateOutboundOrderRequest,
    CutoffDateException, CutoffPlanScope, ManualDeliveryNoteCutoffRequest,
    PublishRouteBindingRequest, WeeklyCutoffSlot,
};

const DELIVERY_NOTE_SUBJECT: &str = "print_document_category:delivery_note";

fn at(month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, month, day, hour, minute, 0)
        .single()
        .expect("valid test time")
}

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "h9-cutoff-test".to_string(),
        permissions: vec!["h9.print_orchestration.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_scope(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let customer_id = Uuid::new_v4();
    let address_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, 'H9006', 'H9 截单测试货主')",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("owner seed should insert");
    sqlx::query(
        r#"
        INSERT INTO warehouses (
            id, owner_id, warehouse_code, warehouse_name, warehouse_type
        )
        VALUES ($1, $2, 'WH-H9-006', 'H9 截单测试仓', 'distribution')
        "#,
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("warehouse seed should insert");
    sqlx::query(
        r#"
        INSERT INTO customers (
            id, owner_id, customer_code, customer_name, customer_type
        )
        VALUES ($1, $2, 'CUS-H9-006', 'H9 截单测试客户', 'customer')
        "#,
    )
    .bind(customer_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("customer seed should insert");
    sqlx::query(
        r#"
        INSERT INTO customer_addresses (
            id, owner_id, customer_id, province, city, district,
            detail_address, contact_name, contact_phone, is_default
        )
        VALUES (
            $1, $2, $3, '浙江省', '杭州市', '拱墅区',
            '真实数据路 006 号', '测试收货人', '13800000006', TRUE
        )
        "#,
    )
    .bind(address_id)
    .bind(owner_id)
    .bind(customer_id)
    .execute(pool)
    .await
    .expect("customer address seed should insert");
    sqlx::query(
        r#"
        INSERT INTO document_number_rules (
            id, owner_id, document_type, rule_code, rule_name, template,
            reset_policy, sequence_width, enabled, effective_from
        )
        VALUES (
            $1, $2, $3, 'h9-delivery-note', '随货同行单号',
            'SHTX-{OWNER}-{YYYY}{MM}{DD}-{SEQ}',
            'daily', 4, TRUE, '2026-07-01T00:00:00Z'
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(DELIVERY_NOTE_SUBJECT)
    .execute(pool)
    .await
    .expect("delivery-note numbering rule should insert");
    (owner_id, warehouse_id, customer_id, address_id)
}

async fn seed_order(
    pool: &PgPool,
    owner_id: Uuid,
    warehouse_id: Uuid,
    customer_id: Uuid,
    address_id: Uuid,
    order_no: &str,
) -> Uuid {
    let order_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, document_type, wms_order_no, customer_id,
            warehouse_id, status
        )
        VALUES ($1, $2, 'sales_outbound', $3, $4, $5, 'confirmed')
        "#,
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(order_no)
    .bind(customer_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("outbound order seed should insert");
    sqlx::query(
        r#"
        INSERT INTO h9_outbound_route_snapshots (
            outbound_order_id, owner_id, warehouse_id, customer_id,
            delivery_address_id, route_code, frozen_at
        )
        VALUES ($1, $2, $3, $4, $5, 'LINE-H9-006', '2026-07-26T08:00:00Z')
        "#,
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(address_id)
    .execute(pool)
    .await
    .expect("route snapshot seed should insert");
    order_id
}

#[sqlx::test(migrations = "../../migrations")]
async fn manual_cutoff_freezes_one_boundary_numbers_audits_and_replays(pool: PgPool) {
    let (owner_id, warehouse_id, customer_id, address_id) = seed_scope(&pool).await;
    let first = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-006-01",
    )
    .await;
    let second = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-006-02",
    )
    .await;
    let auth = ctx(owner_id);
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let now = at(7, 26, 9, 0);
    let request = ManualDeliveryNoteCutoffRequest {
        warehouse_id,
        delivery_address_id: address_id,
        order_ids: vec![first, second],
        reason: "客户截单时间已到".to_string(),
    };

    let created = service
        .manual_cutoff(&auth, request.clone(), now, "h9-cutoff-006")
        .await
        .expect("manual cutoff should succeed");
    assert!(!created.replayed);
    assert_eq!(created.value.order_ids, vec![first, second]);
    assert_eq!(created.value.route_code, "LINE-H9-006");
    assert_eq!(created.value.delivery_note_no, "SHTX-H9006-20260726-0001");

    let replay = service
        .manual_cutoff(&auth, request, now, "h9-cutoff-006")
        .await
        .expect("same cutoff request should replay");
    assert!(replay.replayed);
    assert_eq!(replay.value, created.value);

    let group_orders: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM h9_delivery_note_group_orders WHERE owner_id = $1 AND group_id = $2",
    )
    .bind(owner_id)
    .bind(created.value.id)
    .fetch_one(&pool)
    .await
    .expect("group order count should load");
    assert_eq!(group_orders, 2);
    let allocations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM document_number_allocations WHERE owner_id = $1 AND document_type = $2",
    )
    .bind(owner_id)
    .bind(DELIVERY_NOTE_SUBJECT)
    .fetch_one(&pool)
    .await
    .expect("number allocation count should load");
    assert_eq!(allocations, 1);
    let audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H9' AND action = 'manual_cutoff_delivery_note'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("cutoff audit count should load");
    assert_eq!(audits, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn delivery_note_workbench_lists_real_pending_orders_and_cutoff_results(pool: PgPool) {
    let (owner_id, warehouse_id, customer_id, address_id) = seed_scope(&pool).await;
    let first = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-WORKBENCH-01",
    )
    .await;
    let second = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-WORKBENCH-02",
    )
    .await;
    let auth = ctx(owner_id);
    let service = PrintOrchestrationService::with_postgres(pool.clone());

    let pending = service
        .list_delivery_note_candidates(&auth, Some(warehouse_id))
        .await
        .expect("pending cutoff orders should load");
    assert_eq!(pending.data.len(), 2);
    assert_eq!(pending.data[0].customer_code, "CUS-H9-006");
    assert_eq!(pending.data[0].customer_name, "H9 截单测试客户");
    assert_eq!(
        pending.data[0].delivery_address,
        "浙江省杭州市拱墅区真实数据路 006 号"
    );
    assert_eq!(pending.data[0].route_code, "LINE-H9-006");

    let cutoff_at = at(7, 26, 9, 0);
    let group = service
        .manual_cutoff(
            &auth,
            ManualDeliveryNoteCutoffRequest {
                warehouse_id,
                delivery_address_id: address_id,
                order_ids: vec![first, second],
                reason: "页面人工截单验收".to_string(),
            },
            cutoff_at,
            "h9-workbench-cutoff",
        )
        .await
        .expect("workbench cutoff should succeed")
        .value;

    assert!(service
        .list_delivery_note_candidates(&auth, Some(warehouse_id))
        .await
        .expect("cutoff orders must leave pending list")
        .data
        .is_empty());
    let groups = service
        .list_delivery_note_groups(&auth, Some(warehouse_id))
        .await
        .expect("cutoff result list should load");
    assert_eq!(groups.data.len(), 1);
    assert_eq!(groups.data[0].id, group.id);
    assert_eq!(
        groups.data[0].order_nos,
        vec!["SO-H9-WORKBENCH-01", "SO-H9-WORKBENCH-02"]
    );
    assert_eq!(groups.data[0].customer_name, "H9 截单测试客户");
}

#[sqlx::test(migrations = "../../migrations")]
async fn manual_cutoff_rejects_mixed_address_before_allocating_a_number(pool: PgPool) {
    let (owner_id, warehouse_id, customer_id, address_id) = seed_scope(&pool).await;
    let other_address_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO customer_addresses (
            id, owner_id, customer_id, province, city, district,
            detail_address, contact_name, contact_phone
        )
        VALUES (
            $1, $2, $3, '浙江省', '杭州市', '拱墅区',
            '真实数据路 007 号', '测试收货人', '13800000007'
        )
        "#,
    )
    .bind(other_address_id)
    .bind(owner_id)
    .bind(customer_id)
    .execute(&pool)
    .await
    .expect("second address seed should insert");
    let first = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-006-11",
    )
    .await;
    let second = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        other_address_id,
        "SO-H9-006-12",
    )
    .await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let error = service
        .manual_cutoff(
            &ctx(owner_id),
            ManualDeliveryNoteCutoffRequest {
                warehouse_id,
                delivery_address_id: address_id,
                order_ids: vec![first, second],
                reason: "边界反向测试".to_string(),
            },
            at(7, 26, 10, 0),
            "h9-cutoff-mixed-address",
        )
        .await
        .expect_err("mixed address cutoff must fail");
    assert_eq!(error, PrintOrchestrationError::AggregationBoundaryMismatch);
    sqlx::query("UPDATE outbound_orders SET status = 'cancelled' WHERE id = $1")
        .bind(first)
        .execute(&pool)
        .await
        .expect("order status should update");
    let error = service
        .manual_cutoff(
            &ctx(owner_id),
            ManualDeliveryNoteCutoffRequest {
                warehouse_id,
                delivery_address_id: address_id,
                order_ids: vec![first],
                reason: "订单状态反向测试".to_string(),
            },
            at(7, 26, 10, 1),
            "h9-cutoff-invalid-order-state",
        )
        .await
        .expect_err("cancelled order cutoff must fail");
    assert_eq!(error, PrintOrchestrationError::OrderNotEligibleForCutoff);

    let allocations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM document_number_allocations WHERE owner_id = $1 AND document_type = $2",
    )
    .bind(owner_id)
    .bind(DELIVERY_NOTE_SUBJECT)
    .fetch_one(&pool)
    .await
    .expect("number allocation count should load");
    assert_eq!(allocations, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn manual_cutoff_http_requires_orchestration_write_permission(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let app = print_orchestration_router(PrintOrchestrationAppState::with_postgres(pool)).layer(
        Extension(AuthContext {
            permissions: vec!["h9.print_orchestration.read".to_string()],
            ..ctx(owner_id)
        }),
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/print-orchestration/delivery-note-groups/manual-cutoff")
                .header("content-type", "application/json")
                .header("idempotency-key", "h9-cutoff-forbidden")
                .body(Body::from(
                    serde_json::json!({
                        "warehouse_id": Uuid::new_v4(),
                        "delivery_address_id": Uuid::new_v4(),
                        "order_ids": [Uuid::new_v4()],
                        "reason": "无权限反向测试"
                    })
                    .to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("cutoff route should respond");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../../migrations")]
async fn route_binding_publish_is_idempotent_and_rejects_address_time_overlap(pool: PgPool) {
    let (owner_id, warehouse_id, customer_id, address_id) = seed_scope(&pool).await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let auth = ctx(owner_id);
    let request = PublishRouteBindingRequest {
        warehouse_id,
        customer_id,
        delivery_address_id: address_id,
        route_code: "LINE-H9-A".to_string(),
        effective_from: Utc
            .with_ymd_and_hms(2026, 7, 26, 0, 0, 0)
            .single()
            .expect("valid effective time"),
        effective_to: None,
    };

    let created = service
        .publish_route_binding(
            &auth,
            request.clone(),
            request.effective_from,
            "h9-route-binding-a",
        )
        .await
        .expect("route binding should publish");
    assert!(!created.replayed);
    let replay = service
        .publish_route_binding(
            &auth,
            request.clone(),
            request.effective_from,
            "h9-route-binding-a",
        )
        .await
        .expect("same route binding should replay");
    assert!(replay.replayed);
    assert_eq!(replay.value, created.value);

    let conflict = service
        .publish_route_binding(
            &auth,
            PublishRouteBindingRequest {
                warehouse_id,
                customer_id,
                delivery_address_id: address_id,
                route_code: "LINE-H9-B".to_string(),
                effective_from: Utc
                    .with_ymd_and_hms(2026, 7, 27, 0, 0, 0)
                    .single()
                    .expect("valid overlapping time"),
                effective_to: None,
            },
            Utc.with_ymd_and_hms(2026, 7, 26, 1, 0, 0)
                .single()
                .expect("valid audit time"),
            "h9-route-binding-b",
        )
        .await
        .expect_err("same address cannot have overlapping route bindings");
    assert_eq!(conflict, PrintOrchestrationError::EffectivePeriodOverlap);

    let route_bindings: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM h9_route_bindings WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("route binding count should load");
    assert_eq!(route_bindings, 1);
    let audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND module = 'H9' AND action = 'publish_route_binding'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("route binding audit count should load");
    assert_eq!(audits, 1);
}

fn cutoff_plan_request(
    warehouse_id: Uuid,
    scope: CutoffPlanScope,
    customer_id: Option<Uuid>,
    route_code: Option<&str>,
    name: &str,
) -> CreateCutoffPlanRequest {
    CreateCutoffPlanRequest {
        name: name.to_string(),
        warehouse_id,
        scope,
        customer_id,
        route_code: route_code.map(str::to_string),
        utc_offset_minutes: 480,
        weekly_schedule: vec![WeeklyCutoffSlot {
            weekday: 1,
            cutoff_time: "09:00".to_string(),
        }],
        exceptions: vec![CutoffDateException {
            date: NaiveDate::from_ymd_opt(2026, 7, 27).expect("valid exception date"),
            cutoff_time: Some("10:30".to_string()),
        }],
        effective_from: Utc
            .with_ymd_and_hms(2026, 7, 1, 0, 0, 0)
            .single()
            .expect("valid effective time"),
        effective_to: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn cutoff_plan_publish_rejects_same_level_overlap_and_resolves_customer_first(pool: PgPool) {
    let (owner_id, warehouse_id, customer_id, _) = seed_scope(&pool).await;
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 8, 0, 0)
        .single()
        .expect("valid operation time");
    let plans = [
        cutoff_plan_request(
            warehouse_id,
            CutoffPlanScope::OwnerWarehouse,
            None,
            None,
            "货主仓默认截单",
        ),
        cutoff_plan_request(
            warehouse_id,
            CutoffPlanScope::Route,
            None,
            Some("LINE-H9-006"),
            "线路截单",
        ),
        cutoff_plan_request(
            warehouse_id,
            CutoffPlanScope::Customer,
            Some(customer_id),
            None,
            "客户截单",
        ),
    ];
    let mut published = Vec::new();
    for (index, request) in plans.into_iter().enumerate() {
        let draft = service
            .create_cutoff_plan(
                &auth,
                request,
                now,
                &format!("h9-cutoff-plan-draft-{index}"),
            )
            .await
            .expect("cutoff plan draft should create");
        let plan = service
            .publish_cutoff_plan(
                &auth,
                draft.value.id,
                now,
                &format!("h9-cutoff-plan-publish-{index}"),
            )
            .await
            .expect("cutoff plan should publish");
        published.push(plan.value);
    }

    let resolved = service
        .resolve_cutoff_plan(&auth, warehouse_id, customer_id, "LINE-H9-006", now)
        .await
        .expect("cutoff plan should resolve");
    assert_eq!(resolved.id, published[2].id);
    assert_eq!(resolved.scope, CutoffPlanScope::Customer);

    let overlapping = service
        .create_cutoff_plan(
            &auth,
            cutoff_plan_request(
                warehouse_id,
                CutoffPlanScope::Customer,
                Some(customer_id),
                None,
                "重复客户截单",
            ),
            now,
            "h9-cutoff-plan-overlap-draft",
        )
        .await
        .expect("overlapping plan can remain draft");
    let error = service
        .publish_cutoff_plan(
            &auth,
            overlapping.value.id,
            now,
            "h9-cutoff-plan-overlap-publish",
        )
        .await
        .expect_err("same-level effective period overlap must fail");
    assert_eq!(error, PrintOrchestrationError::EffectivePeriodOverlap);
}

#[sqlx::test(migrations = "../../migrations")]
async fn scheduled_cutoff_uses_exception_time_and_concurrent_runs_create_one_group(pool: PgPool) {
    let (owner_id, warehouse_id, customer_id, address_id) = seed_scope(&pool).await;
    let first = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-SCHEDULED-01",
    )
    .await;
    let unplanned_address_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO customer_addresses (id, owner_id, customer_id, province, city, district, detail_address, contact_name, contact_phone) VALUES ($1, $2, $3, '浙江省', '杭州市', '拱墅区', '未配置截单计划地址', '测试收货人', '13800000007')",
    )
    .bind(unplanned_address_id)
    .bind(owner_id)
    .bind(customer_id)
    .execute(&pool)
    .await
    .expect("unplanned address should insert");
    let unplanned_order = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        unplanned_address_id,
        "SO-H9-UNPLANNED-01",
    )
    .await;
    sqlx::query(
        "UPDATE h9_outbound_route_snapshots SET route_code = 'LINE-H9-UNPLANNED' WHERE outbound_order_id = $1",
    )
    .bind(unplanned_order)
    .execute(&pool)
    .await
    .expect("unplanned route should update");
    let second = seed_order(
        &pool,
        owner_id,
        warehouse_id,
        customer_id,
        address_id,
        "SO-H9-SCHEDULED-02",
    )
    .await;
    let auth = ctx(owner_id);
    let service = PrintOrchestrationService::with_postgres(pool.clone());
    let operation_at = Utc
        .with_ymd_and_hms(2026, 7, 26, 8, 0, 0)
        .single()
        .expect("valid operation time");
    let draft = service
        .create_cutoff_plan(
            &auth,
            cutoff_plan_request(
                warehouse_id,
                CutoffPlanScope::Route,
                None,
                Some("LINE-H9-006"),
                "线路例外日截单",
            ),
            operation_at,
            "h9-scheduled-plan-draft",
        )
        .await
        .expect("scheduled plan draft should create");
    let plan = service
        .publish_cutoff_plan(
            &auth,
            draft.value.id,
            operation_at,
            "h9-scheduled-plan-publish",
        )
        .await
        .expect("scheduled plan should publish")
        .value;

    let before_exception = Utc
        .with_ymd_and_hms(2026, 7, 27, 2, 29, 0)
        .single()
        .expect("valid time before UTC+8 exception");
    assert!(
        print_orchestration_job::run_once(&pool, before_exception)
            .await
            .expect("early scheduler run should succeed")
            .is_empty(),
        "weekly 09:00 must be overridden by the 10:30 exception"
    );

    let due_at = Utc
        .with_ymd_and_hms(2026, 7, 27, 2, 30, 0)
        .single()
        .expect("valid UTC+8 exception time");
    let (left, right) = tokio::join!(
        print_orchestration_job::run_once(&pool, due_at),
        service.run_scheduled_cutoffs(&auth, due_at)
    );
    let mut groups = left
        .expect("first concurrent scheduler run should succeed")
        .into_iter()
        .chain(
            right
                .expect("second concurrent scheduler run should succeed")
                .into_iter(),
        )
        .collect::<Vec<_>>();
    groups.dedup_by_key(|group| group.id);
    assert_eq!(groups.len(), 1);
    let group = &groups[0];
    assert_eq!(group.order_ids, vec![first, second]);
    assert_eq!(group.cutoff_mode, "scheduled");
    assert_eq!(group.cutoff_plan_id, Some(plan.id));
    assert_eq!(group.scheduled_cutoff_at, Some(due_at));

    let evidence: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM h9_delivery_note_groups WHERE owner_id = $1),
            (SELECT COUNT(*) FROM h9_delivery_note_group_orders WHERE owner_id = $1),
            (SELECT COUNT(*) FROM document_number_allocations
              WHERE owner_id = $1 AND document_type = $2),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND module = 'H9'
                AND action = 'scheduled_cutoff_delivery_note')
        "#,
    )
    .bind(owner_id)
    .bind(DELIVERY_NOTE_SUBJECT)
    .fetch_one(&pool)
    .await
    .expect("scheduled cutoff evidence should load");
    assert_eq!(evidence, (1, 2, 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_order_creation_freezes_the_effective_address_route(pool: PgPool) {
    let (owner_id, warehouse_id, customer_id, address_id) = seed_scope(&pool).await;
    let auth = ctx(owner_id);
    let now = Utc
        .with_ymd_and_hms(2026, 7, 26, 8, 0, 0)
        .single()
        .expect("valid order time");
    PrintOrchestrationService::with_postgres(pool.clone())
        .publish_route_binding(
            &auth,
            PublishRouteBindingRequest {
                warehouse_id,
                customer_id,
                delivery_address_id: address_id,
                route_code: "LINE-FROZEN".to_string(),
                effective_from: now,
                effective_to: None,
            },
            now,
            "h9-route-freeze-binding",
        )
        .await
        .expect("route binding should publish");

    let order = PgWave4Repository::new(pool.clone())
        .create_outbound_order(
            &auth,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "SO-H9-ROUTE-FREEZE".to_string(),
                erp_order_no: Some("ERP-H9-ROUTE-FREEZE".to_string()),
                customer_id,
                warehouse_id,
                delivery_address_id: address_id,
                required_ship_at: None,
                lines: vec![CreateOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-H9-ROUTE".to_string(),
                    batch_no: "B-H9-ROUTE".to_string(),
                    planned_qty: 1,
                }],
            },
            now,
            "h9-route-freeze-order",
            None,
        )
        .await
        .expect("outbound order should freeze route")
        .value;

    let frozen: (Uuid, String) = sqlx::query_as(
        "SELECT delivery_address_id, route_code FROM h9_outbound_route_snapshots WHERE owner_id = $1 AND outbound_order_id = $2",
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("route snapshot should load");
    assert_eq!(frozen, (address_id, "LINE-FROZEN".to_string()));
}
