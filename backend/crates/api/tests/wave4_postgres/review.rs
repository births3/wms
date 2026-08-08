#[sqlx::test(migrations = "../../migrations")]
async fn outbound_complete_pick_review_ship_replays_and_deducts_inventory(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let picker_ctx = ctx(owner_id);
    let reviewer_ctx = ctx(owner_id);
    let second_reviewer_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let shipping_service = Wave4ShippingService::new(Arc::new(repo.clone()));
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 8, 0, 0)
        .single()
        .expect("valid time");
    seed_outbound_inventory(&pool, owner_id, "P-OUT-001", "B-OUT-001", 10, now).await;
    sqlx::query(
        "INSERT INTO products (id, owner_id, erp_goods_id, product_code, product_name, specification, storage_condition, special_drug_category, status) VALUES ($1, $2, 1001, 'P-OUT-001', '出库复核策略商品', '1 unit', 'normal', 'none', 'active')",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("seed outbound review product");
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '出库复核策略测试货主') ON CONFLICT (id) DO UPDATE SET owner_name = EXCLUDED.owner_name",
    )
    .bind(owner_id)
    .bind(format!("M4-REVIEW-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed outbound review owner");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '第一复核员', 'test-hash', 'active')",
    )
    .bind(reviewer_ctx.user_id)
    .bind(format!("m4-reviewer-{}", &reviewer_ctx.user_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed first reviewer");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(reviewer_ctx.user_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("bind first reviewer");
    sqlx::query(
        "INSERT INTO auth_users (id, username, display_name, password_hash, status) VALUES ($1, $2, '第二复核员', 'test-hash', 'active')",
    )
    .bind(second_reviewer_id)
    .bind(format!("m4-reviewer-{}", &second_reviewer_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("seed second reviewer");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary) VALUES ($1, $2, TRUE, TRUE)",
    )
    .bind(second_reviewer_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("bind second reviewer");
    sqlx::query(
        "INSERT INTO auth_user_roles (user_id, owner_id, role_id) SELECT $1, $2, id FROM auth_roles WHERE owner_id = $2 AND role_code = 'custodian'",
    )
    .bind(second_reviewer_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("assign custodian role to second reviewer");
    let customer_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let delivery_address_id =
        seed_outbound_route_binding(&pool, owner_id, warehouse_id, customer_id, now).await;

    let order = repo
        .create_outbound_order(
            &ctx,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "WMS-R-20260605-001".to_string(),
                erp_order_no: Some("ERP-SO-001".to_string()),
                invoice_no: None,
                transport_mode_code: None,
                department_code: None,
                sales_group_code: None,
                order_group_no: None,
                business_type_code: None,
                customer_id,
                warehouse_id,
                delivery_address_id,
                required_ship_at: None,
                lines: vec![CreateOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-OUT-001".to_string(),
                    batch_no: "B-OUT-001".to_string(),
                    planned_qty: 10.into(),
                }],
            },
            now,
            "outbound-create-1",
            None,
        )
        .await
        .expect("outbound order should be created")
        .value;
    assert_eq!(order.status, "confirmed");
    repo.attach_erp_outbound_identity(
        owner_id,
        order.id,
        9101,
        "ERP-SO-001",
        1,
        1,
        Some(1),
        "corr-m4-001",
    )
    .await
    .expect("attach ERP outbound identity");

    let wave = repo
        .create_outbound_wave(
            &ctx,
            CreateOutboundWaveRequest {
                wave_no: "WAVE-20260605-001".to_string(),
                order_ids: vec![order.id],
            },
            now,
            "outbound-wave-1",
            None,
        )
        .await
        .expect("wave should be created")
        .value;
    assert_eq!(wave.order_ids, vec![order.id]);

    let mte_pick_tasks: Vec<(String, String, String, i64)> = sqlx::query_as(
        r#"
        SELECT task_type_code, status, product_code, planned_qty::BIGINT
          FROM warehouse_tasks
         WHERE owner_id = $1
           AND source_doc_type = 'outbound_order'
           AND source_doc_id = $2
           AND task_type_code = 'pick'
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_all(&pool)
    .await
    .expect("wave release should create M-TE pick tasks");
    assert_eq!(
        mte_pick_tasks,
        vec![(
            "pick".to_string(),
            "pending_assignment".to_string(),
            "P-OUT-001".to_string(),
            10,
        )]
    );

    let locked: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT qty_locked::BIGINT FROM inventory_batches
              WHERE owner_id = $1 AND product_code = 'P-OUT-001' AND batch_no = 'B-OUT-001'),
            (SELECT COALESCE(SUM(allocated_qty), 0)::BIGINT FROM inventory_allocations
              WHERE owner_id = $1 AND outbound_order_id = $2 AND status = 'locked'),
            (SELECT COUNT(*) FROM inventory_allocations
              WHERE owner_id = $1 AND outbound_order_id = $2 AND status = 'consumed')
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("locked inventory counts");
    assert_eq!(locked, (10, 10, 0));

    let short = repo
        .complete_pick_task(
            &picker_ctx,
            order.id,
            CompletePickTaskRequest {
                line_no: 1,
                picked_qty: 8.into(),
                exception_code: Some("SHORT_PICK".to_string()),
                exception_note: Some("零拣位不足，等待补拣".to_string()),
            },
            now,
            "outbound-pick-short-1",
            None,
        )
        .await
        .expect("short pick can continue")
        .value;
    assert_eq!(short.status, "picked_short");
    assert!(short.short_pick);

    let same_operator = repo
        .review_outbound_order(
            &picker_ctx,
            order.id,
            ReviewOutboundOrderRequest {
                reviewer_id: picker_ctx.user_id,
                review_mode: "pda_loose".to_string(),
                second_reviewer_id: None,
                lines: vec![ReviewOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-OUT-001".to_string(),
                    reviewed_qty: 8.into(),
                }],
            },
            now,
            "outbound-review-same-operator-1",
            None,
        )
        .await
        .expect_err("picker must not review the same outbound order");
    assert!(matches!(
        same_operator,
        Wave4RepositoryError::ReviewValidation(wms_domain::ReviewValidationError::SameOperator)
    ));

    let mismatch = repo
        .review_outbound_order(
            &reviewer_ctx,
            order.id,
            ReviewOutboundOrderRequest {
                reviewer_id: reviewer_ctx.user_id,
                review_mode: "pda_loose".to_string(),
                second_reviewer_id: None,
                lines: vec![ReviewOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-OUT-001".to_string(),
                    reviewed_qty: 7.into(),
                }],
            },
            now,
            "outbound-review-mismatch-1",
            None,
        )
        .await
        .expect_err("mismatched scanned quantity should be rejected");
    assert!(matches!(
        mismatch,
        Wave4RepositoryError::ReviewValidation(
            wms_domain::ReviewValidationError::QuantityMismatch {
                line_no: 1,
                expected,
                actual,
            }
        ) if expected == 8.into() && actual == 7.into()
    ));
    let unchanged = repo
        .get_outbound_order(&reviewer_ctx, order.id)
        .await
        .expect("rejected review should leave the order readable");
    assert_eq!(unchanged.lines[0].reviewed_qty, 0.into());

    let missing_second = repo
        .review_outbound_order(
            &reviewer_ctx,
            order.id,
            ReviewOutboundOrderRequest {
                reviewer_id: reviewer_ctx.user_id,
                review_mode: "pda_loose".to_string(),
                second_reviewer_id: None,
                lines: vec![ReviewOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-OUT-001".to_string(),
                    reviewed_qty: 8.into(),
                }],
            },
            now,
            "outbound-review-missing-second-1",
            None,
        )
        .await
        .expect_err("M-VR dual_scan policy must require a second reviewer");
    assert!(matches!(
        missing_second,
        Wave4RepositoryError::MissingSecondReviewer
    ));

    let reviewed_short = repo
        .review_outbound_order(
            &reviewer_ctx,
            order.id,
            ReviewOutboundOrderRequest {
                reviewer_id: reviewer_ctx.user_id,
                review_mode: "pda_loose".to_string(),
                second_reviewer_id: Some(second_reviewer_id),
                lines: vec![ReviewOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-OUT-001".to_string(),
                    reviewed_qty: 8.into(),
                }],
            },
            now,
            "outbound-review-short-1",
            None,
        )
        .await
        .expect("short pick can be reviewed with marker")
        .value;
    assert_eq!(reviewed_short.status, "reviewed_short");
    let loading_task_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM warehouse_tasks WHERE owner_id = $1 AND source_doc_id = $2 AND task_type_code = 'loading'",
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("short review loading task count should query");
    assert_eq!(loading_task_count, 0, "短拣未补齐时不得提前创建装车任务");

    let blocked_ship = repo
        .ship_outbound_order(
            &ctx,
            order.id,
            ShipOutboundOrderRequest {
                delivery_provider_type: "own_fleet".to_string(),
                vehicle_no: Some("VEHICLE-001".to_string()),
                plate_no: "沪A12345".to_string(),
                driver_user_id: Some(second_reviewer_id),
                courier_name: None,
                courier_phone: None,
                signature_attachment_id: None,
                loading_temperature_celsius: None,
                cold_chain_packages: Vec::new(),
                package_count: 1,
            },
            now,
            "outbound-ship-blocked-1",
            None,
        )
        .await
        .expect_err("short pick must be replenished before ship");
    assert!(matches!(
        blocked_ship,
        Wave4RepositoryError::ShortPickNotReplenished
    ));

    let replenished_request = CompletePickTaskRequest {
        line_no: 1,
        picked_qty: 10.into(),
        exception_code: None,
        exception_note: Some("补拣补齐".to_string()),
    };
    let replenished = repo
        .complete_pick_task(
            &picker_ctx,
            order.id,
            replenished_request.clone(),
            now,
            "outbound-pick-replenished-1",
            None,
        )
        .await
        .expect("replenishment pick should clear short pick");
    let pick_replay = repo
        .complete_pick_task(
            &picker_ctx,
            order.id,
            replenished_request,
            now,
            "outbound-pick-replenished-1",
            None,
        )
        .await
        .expect("same-key complete pick should replay");
    assert!(pick_replay.replayed);
    assert_eq!(pick_replay.value.id, replenished.value.id);

    sqlx::query(
        "UPDATE products SET special_drug_category = 'narcotic', storage_condition = 'cold' WHERE owner_id = $1 AND product_code = 'P-OUT-001'",
    )
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("switch product to approval-required category");

    let review_request = ReviewOutboundOrderRequest {
        reviewer_id: reviewer_ctx.user_id,
        review_mode: "pda_loose".to_string(),
        second_reviewer_id: Some(second_reviewer_id),
        lines: vec![ReviewOutboundOrderLineRequest {
            line_no: 1,
            product_code: "P-OUT-001".to_string(),
            reviewed_qty: 10.into(),
        }],
    };
    let missing_approval = repo
        .review_outbound_order(
            &reviewer_ctx,
            order.id,
            review_request.clone(),
            now,
            "outbound-review-missing-approval-1",
            None,
        )
        .await
        .expect_err("dual_scan_with_approval must block before H4 approval");
    assert!(matches!(
        missing_approval,
        Wave4RepositoryError::DualPersonApprovalRequired
    ));
    let approval_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO h4_approval_records (
            id, owner_id, scenario, business_ref, dedupe_key, approver_user,
            process_id, callback_path, summary, status, external_approval_id,
            approved_by, approved_at, created_at, updated_at
        )
        VALUES ($1, $2, 'mvr.dual_person', $3, 'm4-review-approval', $4,
                'mvr-dual-person', '/api/v1/wechat-notify/approvals/callback',
                '特殊药品出库复核', 'approved', 'WX-M4-APPROVED', $4, $5, $5, $5)
        "#,
    )
    .bind(approval_id)
    .bind(owner_id)
    .bind(order.id.to_string())
    .bind(second_reviewer_id.to_string())
    .bind(now)
    .execute(&pool)
    .await
    .expect("seed approved H4 dual-person approval");
    let reviewed = repo
        .review_outbound_order(
            &reviewer_ctx,
            order.id,
            review_request.clone(),
            now,
            "outbound-review-replenished-1",
            Some(AuditWriteRequest::from_auth_context(
                &reviewer_ctx,
                "review_outbound_order",
                "M4",
                "outbound_order",
                order.id.to_string(),
                None,
            )),
        )
        .await
        .expect("replenished order should be reviewed again");
    let review_replay = repo
        .review_outbound_order(
            &reviewer_ctx,
            order.id,
            review_request,
            now,
            "outbound-review-replenished-1",
            None,
        )
        .await
        .expect("same-key outbound review should replay");
    assert!(review_replay.replayed);
    assert_eq!(review_replay.value.id, reviewed.value.id);
    let loading_task: (String, String, String, i64) = sqlx::query_as(
        r#"
        SELECT task_type_code, status, product_code, planned_qty::BIGINT
          FROM warehouse_tasks
         WHERE owner_id = $1
           AND source_doc_type = 'outbound_order'
           AND source_doc_id = $2
           AND task_type_code = 'loading'
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("complete review should create M-TE loading task");
    assert_eq!(
        loading_task,
        (
            "loading".to_string(),
            "pending_assignment".to_string(),
            "P-OUT-001".to_string(),
            10,
        )
    );

    let signature_attachment_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO attachments (
            id, owner_id, module, entity_type, entity_id, file_name, content_type,
            size_bytes, storage_key, sha256, uploaded_by
        )
        VALUES ($1, $2, 'M4', 'outbound_handover_signature', $3,
                'driver-signature.png', 'image/png', 128, $4, $5, $6)
        "#,
    )
    .bind(signature_attachment_id)
    .bind(owner_id)
    .bind(order.id)
    .bind(format!("m4/{owner_id}/{signature_attachment_id}.png"))
    .bind("a".repeat(64))
    .bind(second_reviewer_id)
    .execute(&pool)
    .await
    .expect("handover signature attachment should seed");

    let ship_request = ShipOutboundOrderRequest {
        delivery_provider_type: "own_fleet".to_string(),
        vehicle_no: Some("VEHICLE-001".to_string()),
        plate_no: "沪A12345".to_string(),
        driver_user_id: Some(second_reviewer_id),
        courier_name: None,
        courier_phone: None,
        signature_attachment_id: Some(signature_attachment_id),
        loading_temperature_celsius: Some(4.2),
        cold_chain_packages: vec![OutboundColdChainPackage {
            insulated_container_no: "BOX-COLD-001".to_string(),
            ice_pack_count: 4,
        }],
        package_count: 1,
    };
    let mut invalid_driver_request = ship_request.clone();
    invalid_driver_request.driver_user_id = Some(Uuid::new_v4());
    let invalid_driver = repo
        .ship_outbound_order(
            &reviewer_ctx,
            order.id,
            invalid_driver_request,
            now,
            "outbound-ship-invalid-driver-1",
            None,
        )
        .await
        .expect_err("driver must be an active user bound to the owner");
    assert!(matches!(invalid_driver, Wave4RepositoryError::InvalidDriver));

    let mut invalid_signature_request = ship_request.clone();
    invalid_signature_request.signature_attachment_id = Some(Uuid::new_v4());
    let invalid_signature = repo
        .ship_outbound_order(
            &reviewer_ctx,
            order.id,
            invalid_signature_request,
            now,
            "outbound-ship-invalid-signature-1",
            None,
        )
        .await
        .expect_err("signature attachment must belong to this handover");
    assert!(matches!(
        invalid_signature,
        Wave4RepositoryError::InvalidSignatureAttachment
    ));

    let shipped = shipping_service
        .ship_outbound_order(
            &reviewer_ctx,
            order.id,
            ship_request.clone(),
            now,
            "outbound-ship-1",
        )
        .await
        .expect("replenished order can ship")
        .value;
    assert_eq!(shipped.status, "shipped");
    assert_eq!(shipped.lines[0].shipped_qty, 10.into());
    let shipment = shipped
        .shipment
        .as_ref()
        .expect("shipping response should include persisted handover");
    assert_eq!(shipment.delivery_provider_type, "own_fleet");
    assert_eq!(shipment.driver_user_id, Some(second_reviewer_id));
    assert_eq!(shipment.driver_name.as_deref(), Some("第二复核员"));
    assert_eq!(shipment.signature_attachment_id, Some(signature_attachment_id));
    assert!(shipment.cold_chain);
    assert_eq!(shipment.loading_temperature_celsius, Some(4.2));
    assert_eq!(shipment.handover_by, reviewer_ctx.user_id);
    let ship_replay = shipping_service
        .ship_outbound_order(&ctx, order.id, ship_request, now, "outbound-ship-1")
        .await
        .expect("same-key outbound ship should replay");
    assert!(ship_replay.replayed);
    assert_eq!(ship_replay.value.id, shipped.id);

    type PersistedShipment = (
        String,
        Option<String>,
        String,
        Option<Uuid>,
        Option<String>,
        Option<Uuid>,
        bool,
        Option<f64>,
        serde_json::Value,
        Uuid,
    );
    let persisted_shipment: PersistedShipment = sqlx::query_as(
        r#"
        SELECT delivery_provider_type, vehicle_no, plate_no, driver_user_id,
               driver_name, signature_attachment_id, cold_chain,
               loading_temperature_celsius, cold_chain_packages, handover_by
          FROM outbound_shipments
         WHERE owner_id = $1 AND outbound_order_id = $2
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("shipment handover fields should persist");
    assert_eq!(persisted_shipment.0, "own_fleet");
    assert_eq!(persisted_shipment.1.as_deref(), Some("VEHICLE-001"));
    assert_eq!(persisted_shipment.2, "沪A12345");
    assert_eq!(persisted_shipment.3, Some(second_reviewer_id));
    assert_eq!(persisted_shipment.4.as_deref(), Some("第二复核员"));
    assert_eq!(persisted_shipment.5, Some(signature_attachment_id));
    assert!(persisted_shipment.6);
    assert_eq!(persisted_shipment.7, Some(4.2));
    assert_eq!(
        persisted_shipment.8,
        serde_json::json!([{
            "insulated_container_no": "BOX-COLD-001",
            "ice_pack_count": 4
        }])
    );
    assert_eq!(persisted_shipment.9, reviewer_ctx.user_id);

    let shipment_outbox: (i64, serde_json::Value) = sqlx::query_as(
        r#"
        SELECT COUNT(*) OVER (), payload
          FROM shipment_confirm_erp_feedback_outbox
         WHERE owner_id = $1 AND outbound_order_id = $2
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("shipment confirmation outbox");
    assert_eq!(shipment_outbox.0, 1);
    assert_eq!(
        shipment_outbox.1["warehouse_id"],
        order.warehouse_id.to_string()
    );
    assert_eq!(shipment_outbox.1["outbound_order_id"], order.id.to_string());
    assert_eq!(shipment_outbox.1["wms_order_no"], order.wms_order_no);
    assert_eq!(shipment_outbox.1["package_count"], 1);
    assert_eq!(shipment_outbox.1["erp_bill_code"], "ERP-SO-001");
    assert_eq!(shipment_outbox.1["revision"], 1);
    assert_eq!(shipment_outbox.1["correlation_id"], "corr-m4-001");
    assert_eq!(shipment_outbox.1["line_count"], 1);
    assert_eq!(shipment_outbox.1["lines"][0]["goods_id"], 1001);
    assert_eq!(shipment_outbox.1["lines"][0]["expected_amount"], "10.0000");
    assert_eq!(shipment_outbox.1["lines"][0]["shipped_amount"], "10.0000");

    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT qty_on_hand::BIGINT FROM inventory_batches
              WHERE owner_id = $1 AND product_code = 'P-OUT-001' AND batch_no = 'B-OUT-001'),
            (SELECT COALESCE(SUM(qty_delta), 0)::BIGINT FROM inventory_movements
              WHERE owner_id = $1 AND source_document_type = 'outbound_order'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'review_outbound_order'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'ship_outbound_order')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (0, -10, 2, 1));

    let review_diff: serde_json::Value = sqlx::query_scalar(
        r#"
        SELECT diff
          FROM audit_event
         WHERE owner_id = $1 AND action = 'review_outbound_order'
         ORDER BY id DESC
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("review audit diff should be persisted");
    assert_eq!(
        review_diff["after"]["reviewer_id"],
        serde_json::json!(reviewer_ctx.user_id)
    );
    assert_eq!(
        review_diff["after"]["lines"][0]["reviewed_qty"],
        serde_json::json!("10")
    );
    assert!(review_diff["after"]["strategy_rule_id"].is_string());
    assert_eq!(
        review_diff["after"]["approval_record_id"],
        serde_json::json!(approval_id)
    );
    let ship_diff: serde_json::Value = sqlx::query_scalar(
        r#"
        SELECT diff
          FROM audit_event
         WHERE owner_id = $1 AND action = 'ship_outbound_order'
         ORDER BY id DESC
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("shipment audit diff should be persisted");
    assert_eq!(
        ship_diff["after"]["driver_user_id"],
        serde_json::json!(second_reviewer_id)
    );
    assert_eq!(ship_diff["after"]["cold_chain"], serde_json::json!(true));

    let persisted_review: (Uuid, Uuid, Uuid) = sqlx::query_as(
        r#"
        SELECT strategy_rule_id, second_reviewer_id, approval_record_id
         FROM outbound_review_records
         WHERE owner_id = $1 AND outbound_order_id = $2
           AND approval_record_id IS NOT NULL
         ORDER BY reviewed_at DESC, id DESC
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("outbound review evidence should be persisted");
    assert_eq!(persisted_review.1, second_reviewer_id);
    assert_eq!(persisted_review.2, approval_id);

    let consumed: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT qty_locked::BIGINT FROM inventory_batches
              WHERE owner_id = $1 AND product_code = 'P-OUT-001' AND batch_no = 'B-OUT-001'),
            (SELECT COUNT(*) FROM inventory_allocations
              WHERE owner_id = $1 AND outbound_order_id = $2 AND status = 'consumed')
        "#,
    )
    .bind(owner_id)
    .bind(order.id)
    .fetch_one(&pool)
    .await
    .expect("consumed inventory counts");
    assert_eq!(consumed, (0, 1));
}
