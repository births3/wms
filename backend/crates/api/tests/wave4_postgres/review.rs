#[sqlx::test(migrations = "../../migrations")]
async fn outbound_complete_pick_review_ship_replays_and_deducts_inventory(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let picker_ctx = ctx(owner_id);
    let reviewer_ctx = ctx(owner_id);
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 8, 0, 0)
        .single()
        .expect("valid time");
    seed_outbound_inventory(&pool, owner_id, "P-OUT-001", "B-OUT-001", 10, now).await;

    let order = repo
        .create_outbound_order(
            &ctx,
            CreateOutboundOrderRequest {
                document_type: "sales_outbound".to_string(),
                wms_order_no: "WMS-R-20260605-001".to_string(),
                erp_order_no: Some("ERP-SO-001".to_string()),
                customer_id: Uuid::new_v4(),
                warehouse_id: Uuid::new_v4(),
                required_ship_at: None,
                lines: vec![CreateOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-OUT-001".to_string(),
                    batch_no: "B-OUT-001".to_string(),
                    planned_qty: 10,
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

    let locked: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT qty_locked FROM inventory_batches
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
                picked_qty: 8,
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
                    reviewed_qty: 8,
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
                    reviewed_qty: 7,
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
                expected: 8,
                actual: 7,
            }
        )
    ));
    let unchanged = repo
        .get_outbound_order(&reviewer_ctx, order.id)
        .await
        .expect("rejected review should leave the order readable");
    assert_eq!(unchanged.lines[0].reviewed_qty, 0);

    let reviewed_short = repo
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
                    reviewed_qty: 8,
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

    let blocked_ship = repo
        .ship_outbound_order(
            &ctx,
            order.id,
            ShipOutboundOrderRequest {
                carrier_type: "own_fleet".to_string(),
                handover_to: "driver-001".to_string(),
                package_count: 1,
                shipped_at: Some(now),
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
        picked_qty: 10,
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

    let review_request = ReviewOutboundOrderRequest {
        reviewer_id: reviewer_ctx.user_id,
        review_mode: "pda_loose".to_string(),
        second_reviewer_id: None,
        lines: vec![ReviewOutboundOrderLineRequest {
            line_no: 1,
            product_code: "P-OUT-001".to_string(),
            reviewed_qty: 10,
        }],
    };
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

    let ship_request = ShipOutboundOrderRequest {
        carrier_type: "own_fleet".to_string(),
        handover_to: "driver-001".to_string(),
        package_count: 1,
        shipped_at: Some(now),
    };
    let shipped = repo
        .ship_outbound_order(
            &reviewer_ctx,
            order.id,
            ship_request.clone(),
            now,
            "outbound-ship-1",
            None,
        )
        .await
        .expect("replenished order can ship")
        .value;
    assert_eq!(shipped.status, "shipped");
    assert_eq!(shipped.lines[0].shipped_qty, 10);
    let ship_replay = repo
        .ship_outbound_order(&ctx, order.id, ship_request, now, "outbound-ship-1", None)
        .await
        .expect("same-key outbound ship should replay");
    assert!(ship_replay.replayed);
    assert_eq!(ship_replay.value.id, shipped.id);

    let counts: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT qty_on_hand FROM inventory_batches
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
        serde_json::json!(10)
    );

    let consumed: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT qty_locked FROM inventory_batches
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
