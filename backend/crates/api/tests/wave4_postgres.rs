use chrono::{NaiveDate, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::{seal_audit_chain, AuditWriteRequest},
    auth::AuthContext,
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
    traceability_code::TraceabilityPlatformResponse,
    wave4_repository::{
        PgWave4Repository, Wave4RepositoryError, APPROVAL_SOURCE_TEMPERATURE_EXCURSION,
    },
};
use wms_domain::{
    CompletePickTaskRequest, CreateOutboundOrderLineRequest, CreateOutboundOrderRequest,
    CreateOutboundWaveRequest, ReviewOutboundOrderRequest, ShipOutboundOrderRequest,
    TraceabilityOutboundReportRequest, TraceabilityStatusChangeEvent,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "wave4-postgres-test".to_string(),
        permissions: vec![
            "m3.write".to_string(),
            "m4.write".to_string(),
            "m5.write".to_string(),
            "m-tc.write".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_inventory_batch(pool: &PgPool, owner_id: Uuid, now: chrono::DateTime<Utc>) -> Uuid {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, 'P-COLD-001', 'B-TEMP-001', $3, $4, 10, 0, $5, $6, 'COLD-A-01', FALSE, $7, $7)
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
    .bind(STATUS_QUALIFIED)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("seed inventory batch");
    batch_id
}

async fn seed_outbound_inventory(
    pool: &PgPool,
    owner_id: Uuid,
    product_code: &str,
    batch_no: &str,
    qty: i64,
    now: chrono::DateTime<Utc>,
) -> Uuid {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $9, 'OUT-A-01', FALSE, $10, $10)
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(product_code)
    .bind(batch_no)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid date"))
    .bind(qty)
    .bind(STATUS_QUALIFIED)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("seed outbound inventory");
    batch_id
}

async fn seed_temperature_excursion(
    pool: &PgPool,
    owner_id: Uuid,
    external_event_id: &str,
    affected_batch_ids: Vec<Uuid>,
    now: chrono::DateTime<Utc>,
) -> Uuid {
    let event_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO temperature_excursion_events (
            id, owner_id, external_event_id, device_code, location_code,
            started_at, ended_at, min_temperature_celsius, max_temperature_celsius,
            affected_batch_ids, status, created_at
        )
        VALUES ($1, $2, $3, 'TEMP-W4-001', 'COLD-A', $4, $5, 1.0, 12.0, $6, 'pending_disposition', $5)
        "#,
    )
    .bind(event_id)
    .bind(owner_id)
    .bind(external_event_id)
    .bind(now - chrono::Duration::minutes(20))
    .bind(now)
    .bind(&affected_batch_ids)
    .execute(pool)
    .await
    .expect("seed temperature excursion");
    event_id
}

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_short_pick_must_be_replenished_before_ship_and_deducts_inventory(pool: PgPool) {
    let owner_id = Uuid::new_v4();
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

    let short = repo
        .complete_pick_task(
            &ctx,
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

    let reviewed_short = repo
        .review_outbound_order(
            &ctx,
            order.id,
            ReviewOutboundOrderRequest {
                reviewer_id: Uuid::new_v4(),
                review_mode: "pda_loose".to_string(),
                second_reviewer_id: None,
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

    repo.complete_pick_task(
        &ctx,
        order.id,
        CompletePickTaskRequest {
            line_no: 1,
            picked_qty: 10,
            exception_code: None,
            exception_note: Some("补拣补齐".to_string()),
        },
        now,
        "outbound-pick-replenished-1",
        None,
    )
    .await
    .expect("replenishment pick should clear short pick");
    repo.review_outbound_order(
        &ctx,
        order.id,
        ReviewOutboundOrderRequest {
            reviewer_id: Uuid::new_v4(),
            review_mode: "pda_loose".to_string(),
            second_reviewer_id: None,
        },
        now,
        "outbound-review-replenished-1",
        None,
    )
    .await
    .expect("replenished order should be reviewed again");

    let shipped = repo
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
            "outbound-ship-1",
            None,
        )
        .await
        .expect("replenished order can ship")
        .value;
    assert_eq!(shipped.status, "shipped");
    assert_eq!(shipped.lines[0].shipped_qty, 10);

    let counts: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT qty_on_hand FROM inventory_batches
              WHERE owner_id = $1 AND product_code = 'P-OUT-001' AND batch_no = 'B-OUT-001'),
            (SELECT COALESCE(SUM(qty_delta), 0)::BIGINT FROM inventory_movements
              WHERE owner_id = $1 AND source_document_type = 'outbound_order'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'ship_outbound_order')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (0, -10, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn traceability_outbound_report_persists_pending_replay_queue_and_audits(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 10, 0, 0)
        .single()
        .expect("valid time");
    let event_id = Uuid::new_v4();
    let report = repo
        .create_traceability_outbound_report(
            &ctx,
            TraceabilityOutboundReportRequest {
                events: vec![TraceabilityStatusChangeEvent {
                    event_id,
                    trace_code: "TC-W4-PG-001".to_string(),
                    status_change_type: "已入库→已出库".to_string(),
                    occurred_at: now,
                }],
            },
            now,
            "traceability-report-1",
            None,
        )
        .await
        .expect("traceability report should persist")
        .value;

    assert_eq!(report.platform, "码上放心");
    assert_eq!(report.status, "queued");
    assert_eq!(report.queued_count, 1);
    assert_eq!(report.events[0].event_id, event_id);

    let counts: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM traceability_outbound_reports
              WHERE owner_id = $1 AND id = $2 AND status = 'queued'),
            (SELECT COUNT(*) FROM traceability_outbound_report_events
              WHERE owner_id = $1 AND report_id = $2 AND report_status = 'queued'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1
                AND module = 'M-TC'
                AND action = 'create_traceability_outbound_report')
        "#,
    )
    .bind(owner_id)
    .bind(report.report_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, 1, 1));

    let replay = repo
        .create_traceability_outbound_report(
            &ctx,
            TraceabilityOutboundReportRequest {
                events: vec![TraceabilityStatusChangeEvent {
                    event_id,
                    trace_code: "TC-W4-PG-001".to_string(),
                    status_change_type: "已入库→已出库".to_string(),
                    occurred_at: now,
                }],
            },
            now,
            "traceability-report-1",
            None,
        )
        .await
        .expect("same idempotency key should replay");

    assert!(replay.replayed);
    assert_eq!(replay.value.report_id, report.report_id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn traceability_platform_response_updates_replay_queue_and_audits(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 10, 30, 0)
        .single()
        .expect("valid time");
    let event_id = Uuid::new_v4();
    let report = repo
        .create_traceability_outbound_report(
            &ctx,
            TraceabilityOutboundReportRequest {
                events: vec![TraceabilityStatusChangeEvent {
                    event_id,
                    trace_code: "TC-W4-PG-RESP-001".to_string(),
                    status_change_type: "已入库→已出库".to_string(),
                    occurred_at: now,
                }],
            },
            now,
            "traceability-report-response-1",
            None,
        )
        .await
        .expect("traceability report should persist")
        .value;

    let retry = repo
        .apply_traceability_platform_response(
            &ctx,
            event_id,
            TraceabilityPlatformResponse {
                success: false,
                platform_receipt_id: None,
                error_code: Some("RATE_LIMITED".to_string()),
                retryable: true,
                trace_id: "trace-w4-retry-001".to_string(),
            },
            now,
            None,
        )
        .await
        .expect("retryable failure should update replay queue");
    assert_eq!(retry.status, "pending_replay");
    assert!(retry.should_retry);

    let retry_counts: (String, i32, Option<String>, String, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT report_status FROM traceability_outbound_report_events
              WHERE owner_id = $1 AND event_id = $2),
            (SELECT retry_count FROM traceability_outbound_report_events
              WHERE owner_id = $1 AND event_id = $2),
            (SELECT last_error_code FROM traceability_outbound_report_events
              WHERE owner_id = $1 AND event_id = $2),
            (SELECT status FROM traceability_outbound_reports
              WHERE owner_id = $1 AND id = $3),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'traceability.report.retry_scheduled')
        "#,
    )
    .bind(owner_id)
    .bind(event_id)
    .bind(report.report_id)
    .fetch_one(&pool)
    .await
    .expect("retry counts");
    assert_eq!(
        retry_counts,
        (
            "pending_replay".to_string(),
            1,
            Some("RATE_LIMITED".to_string()),
            "pending_replay".to_string(),
            1,
        )
    );

    let success = repo
        .apply_traceability_platform_response(
            &ctx,
            event_id,
            TraceabilityPlatformResponse {
                success: true,
                platform_receipt_id: Some("MASXF-RCPT-001".to_string()),
                error_code: None,
                retryable: false,
                trace_id: "trace-w4-success-001".to_string(),
            },
            now,
            None,
        )
        .await
        .expect("success response should mark event reported");
    assert_eq!(success.status, "reported");

    let success_counts: (String, Option<String>, String, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT report_status FROM traceability_outbound_report_events
              WHERE owner_id = $1 AND event_id = $2),
            (SELECT platform_receipt_id FROM traceability_outbound_report_events
              WHERE owner_id = $1 AND event_id = $2),
            (SELECT status FROM traceability_outbound_reports
              WHERE owner_id = $1 AND id = $3),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'traceability.report.success')
        "#,
    )
    .bind(owner_id)
    .bind(event_id)
    .bind(report.report_id)
    .fetch_one(&pool)
    .await
    .expect("success counts");
    assert_eq!(
        success_counts,
        (
            "reported".to_string(),
            Some("MASXF-RCPT-001".to_string()),
            "reported".to_string(),
            1,
        )
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn temperature_excursion_disposition_quarantines_selected_batches_and_audits(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 9, 0, 0)
        .single()
        .expect("valid time");
    let batch_id = seed_inventory_batch(&pool, owner_id, now).await;
    let event_id =
        seed_temperature_excursion(&pool, owner_id, "TEMP-EXT-W4-001", vec![batch_id], now).await;
    let mut audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "dispose_temperature_excursion",
        "M5",
        "temperature_excursion",
        event_id.to_string(),
        None,
    );
    audit.occurred_at = now;

    let disposition = repo
        .dispose_temperature_excursion_and_quarantine_batches(
            &ctx,
            "TEMP-EXT-W4-001",
            vec![batch_id],
            now,
            Some(audit),
        )
        .await
        .expect("temperature excursion disposition should commit");

    assert_eq!(disposition.event.status, "disposed");
    assert_eq!(disposition.quarantined_batches.len(), 1);
    assert_eq!(
        disposition.quarantined_batches[0].quality_status,
        STATUS_QUARANTINED
    );

    let counts: (i64, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM inventory_status_changes
              WHERE owner_id = $1
                AND batch_id = $2
                AND approval_source = $3
                AND approval_id = 'TEMP-EXT-W4-001'),
            (SELECT quality_status FROM inventory_batches WHERE owner_id = $1 AND id = $2),
            (SELECT COUNT(*) FROM temperature_excursion_events
              WHERE owner_id = $1 AND external_event_id = 'TEMP-EXT-W4-001' AND status = 'disposed'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'dispose_temperature_excursion')
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .bind(APPROVAL_SOURCE_TEMPERATURE_EXCURSION)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (1, STATUS_QUARANTINED.to_string(), 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn temperature_excursion_disposition_rejects_unaffected_batch_without_side_effects(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 9, 30, 0)
        .single()
        .expect("valid time");
    let affected_batch_id = seed_inventory_batch(&pool, owner_id, now).await;
    let unrelated_batch_id = seed_inventory_batch(&pool, owner_id, now).await;
    seed_temperature_excursion(
        &pool,
        owner_id,
        "TEMP-EXT-W4-002",
        vec![affected_batch_id],
        now,
    )
    .await;

    let result = repo
        .dispose_temperature_excursion_and_quarantine_batches(
            &ctx,
            "TEMP-EXT-W4-002",
            vec![unrelated_batch_id],
            now,
            None,
        )
        .await
        .expect_err("unaffected batch should be rejected");

    assert!(matches!(
        result,
        Wave4RepositoryError::BatchNotAffected(id) if id == unrelated_batch_id
    ));
    let counts: (i64, String) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM inventory_status_changes WHERE owner_id = $1),
            (SELECT status FROM temperature_excursion_events
              WHERE owner_id = $1 AND external_event_id = 'TEMP-EXT-W4-002')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("counts");
    assert_eq!(counts, (0, "pending_disposition".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn temperature_excursion_disposition_audit_event_is_append_only_and_hash_chain_seals(
    pool: PgPool,
) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 11, 0, 0)
        .single()
        .expect("valid time");
    let batch_id = seed_inventory_batch(&pool, owner_id, now).await;
    let event_id =
        seed_temperature_excursion(&pool, owner_id, "TEMP-EXT-W4-003", vec![batch_id], now).await;
    let mut audit = AuditWriteRequest::from_auth_context(
        &ctx,
        "dispose_temperature_excursion",
        "M5",
        "temperature_excursion",
        event_id.to_string(),
        None,
    );
    audit.occurred_at = now;

    repo.dispose_temperature_excursion_and_quarantine_batches(
        &ctx,
        "TEMP-EXT-W4-003",
        vec![batch_id],
        now,
        Some(audit),
    )
    .await
    .expect("temperature excursion disposition should commit");

    let update_result =
        sqlx::query("UPDATE audit_event SET action = 'tampered' WHERE owner_id = $1")
            .bind(owner_id)
            .execute(&pool)
            .await;
    assert!(
        update_result.is_err(),
        "audit_event append_only invariant must reject UPDATE"
    );

    let delete_result = sqlx::query("DELETE FROM audit_event WHERE owner_id = $1")
        .bind(owner_id)
        .execute(&pool)
        .await;
    assert!(
        delete_result.is_err(),
        "audit_event append_only invariant must reject DELETE"
    );

    let seal = seal_audit_chain(&pool, now.date_naive(), now)
        .await
        .expect("Wave 4 audit hash chain should seal");
    assert_eq!(seal.seal_date, now.date_naive());
}
