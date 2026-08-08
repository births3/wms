#[sqlx::test(migrations = "../../migrations")]
async fn outbound_wave_reads_are_owner_scoped_filterable_and_include_orders(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let owner_ctx = ctx(owner_id);
    let other_ctx = ctx(other_owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 9, 0, 0)
        .single()
        .expect("valid time");
    let owner_order = create_read_order(
        &pool,
        &repo,
        &owner_ctx,
        "WMS-WAVE-READ-001",
        "ERP-WAVE-READ-001",
        now,
    )
    .await;
    seed_outbound_inventory(
        &pool,
        owner_id,
        "P-WMS-WAVE-READ-001",
        "B-WMS-WAVE-READ-001",
        6,
        now,
    )
    .await;
    let owner_wave = repo
        .create_outbound_wave(
            &owner_ctx,
            CreateOutboundWaveRequest {
                wave_no: "WAVE-READ-OWNER".to_string(),
                order_ids: vec![owner_order.id],
            },
            now,
            "outbound-wave-read-owner",
            None,
        )
        .await
        .expect("owner wave should be created")
        .value;
    let other_order = create_read_order(
        &pool,
        &repo,
        &other_ctx,
        "WMS-WAVE-READ-002",
        "ERP-WAVE-READ-002",
        now,
    )
    .await;
    seed_outbound_inventory(
        &pool,
        other_owner_id,
        "P-WMS-WAVE-READ-002",
        "B-WMS-WAVE-READ-002",
        6,
        now,
    )
    .await;
    repo.create_outbound_wave(
        &other_ctx,
        CreateOutboundWaveRequest {
            wave_no: "WAVE-READ-OTHER".to_string(),
            order_ids: vec![other_order.id],
        },
        now,
        "outbound-wave-read-other",
        None,
    )
    .await
    .expect("other owner wave should be created");

    let waves = repo
        .list_outbound_waves(&owner_ctx, Some("released"), Some("OWNER"), Some(10))
        .await
        .expect("owner wave list should be filterable");
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].id, owner_wave.id);
    assert_eq!(waves[0].order_ids, vec![owner_order.id]);

    let detail = repo
        .get_outbound_wave(&owner_ctx, owner_wave.id)
        .await
        .expect("owner wave detail should load");
    assert_eq!(detail.order_ids, vec![owner_order.id]);

    let cross_owner = repo
        .get_outbound_wave(&other_ctx, owner_wave.id)
        .await
        .expect_err("other owner must not read wave detail");
    assert!(matches!(cross_owner, Wave4RepositoryError::NotFound));
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_wave_can_cancel_before_picking_and_release_inventory(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 10, 0, 0)
        .single()
        .expect("valid time");
    let order = create_read_order(
        &pool,
        &repo,
        &ctx,
        "WMS-WAVE-CANCEL-001",
        "ERP-CANCEL-001",
        now,
    )
    .await;
    seed_outbound_inventory(
        &pool,
        owner_id,
        "P-WMS-WAVE-CANCEL-001",
        "B-WMS-WAVE-CANCEL-001",
        6,
        now,
    )
    .await;
    let wave = repo
        .create_outbound_wave(
            &ctx,
            CreateOutboundWaveRequest {
                wave_no: "WAVE-CANCEL-001".to_string(),
                order_ids: vec![order.id],
            },
            now,
            "outbound-wave-cancel-001",
            None,
        )
        .await
        .expect("wave should be created")
        .value;

    let cancelled = repo
        .cancel_outbound_wave(&ctx, wave.id, now, "outbound-wave-cancel-002", None)
        .await
        .expect("wave should be cancellable before picking")
        .value;
    assert_eq!(cancelled.status, "cancelled");

    let replayed = repo
        .cancel_outbound_wave(&ctx, wave.id, now, "outbound-wave-cancel-002", None)
        .await
        .expect("cancellation should replay")
        .value;
    assert_eq!(replayed.id, cancelled.id);

    let state: (wms_domain::Quantity, String, String) = sqlx::query_as(
        "SELECT (SELECT qty_locked FROM inventory_batches WHERE owner_id = $1 AND product_code = $2), (SELECT status FROM outbound_orders WHERE owner_id = $1 AND id = $3), (SELECT status FROM outbound_waves WHERE owner_id = $1 AND id = $4)",
    )
    .bind(owner_id)
    .bind("P-WMS-WAVE-CANCEL-001")
    .bind(order.id)
    .bind(wave.id)
    .fetch_one(&pool)
    .await
    .expect("cancelled wave state should persist");
    assert_eq!(
        state,
        (
            wms_domain::Quantity::ZERO,
            "confirmed".to_string(),
            "cancelled".to_string()
        )
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_draft_wave_can_cancel_before_picking(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 11, 0, 0)
        .single()
        .expect("valid time");
    let order = create_read_order(
        &pool,
        &repo,
        &ctx,
        "WMS-WAVE-CANCEL-DRAFT-001",
        "ERP-CANCEL-DRAFT-001",
        now,
    )
    .await;
    seed_outbound_inventory(
        &pool,
        owner_id,
        "P-WMS-WAVE-CANCEL-DRAFT-001",
        "B-WMS-WAVE-CANCEL-DRAFT-001",
        6,
        now,
    )
    .await;
    let wave = repo
        .create_outbound_wave(
            &ctx,
            CreateOutboundWaveRequest {
                wave_no: "WAVE-CANCEL-DRAFT-001".to_string(),
                order_ids: vec![order.id],
            },
            now,
            "outbound-wave-cancel-draft-001",
            None,
        )
        .await
        .expect("wave should be created")
        .value;
    sqlx::query("UPDATE outbound_waves SET status = 'draft' WHERE owner_id = $1 AND id = $2")
        .bind(owner_id)
        .bind(wave.id)
        .execute(&pool)
        .await
        .expect("wave should be moved to draft for the regression case");

    let cancelled = repo
        .cancel_outbound_wave(&ctx, wave.id, now, "outbound-wave-cancel-draft-002", None)
        .await
        .expect("draft wave should be cancellable before picking")
        .value;
    assert_eq!(cancelled.status, "cancelled");
}
