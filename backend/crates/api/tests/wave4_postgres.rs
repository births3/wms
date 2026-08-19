use std::sync::Arc;

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
    wave4_service::Wave4ShippingService,
};
use wms_domain::{
    CompletePickTaskRequest, CreateOutboundOrderLineRequest, CreateOutboundOrderRequest,
    CreateOutboundWaveRequest, OutboundColdChainPackage, OutboundOrder,
    ReviewOutboundOrderLineRequest, ReviewOutboundOrderRequest, ShipOutboundOrderRequest,
    TraceabilityOutboundReportRequest, TraceabilityStatusChangeEvent,
};

#[path = "support/wave4.rs"]
mod support;
use support::seed_inventory_batch;
#[path = "support/h9.rs"]
mod h9_support;
use h9_support::seed_outbound_route_binding;

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
        warehouse_scope: None,
    }
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
    let warehouse_id = Uuid::new_v4();
    let zone_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, 'M4 出库测试货主') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .bind(format!("M4-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed outbound owner");
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, 'M4 出库测试仓', 'normal', 'active')",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("M4-WH-{}", &warehouse_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed outbound warehouse");
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, $4, 'M4 出库测试区', 'normal_10_30', 'qualified_green', 'active')",
    )
    .bind(zone_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(format!("M4-ZONE-{}", &zone_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("seed outbound zone");
    sqlx::query(
        "INSERT INTO warehouse_locations (id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no, max_volume_cm3, used_volume_cm3, max_sku_count, location_type, status) VALUES ($1, $2, $3, $4, 'OUT-A-01', 1, 1, 1, 100000, 0, 100, 'storage', 'available')",
    )
    .bind(location_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(zone_id)
    .execute(pool)
    .await
    .expect("seed outbound location");
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_frozen, status, location_id, location_code,
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
    .bind(location_id)
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

async fn create_read_order(
    pool: &PgPool,
    repo: &PgWave4Repository,
    ctx: &AuthContext,
    wms_order_no: &str,
    erp_order_no: &str,
    now: chrono::DateTime<Utc>,
) -> OutboundOrder {
    let customer_id = Uuid::new_v4();
    let warehouse_id = Uuid::new_v4();
    let delivery_address_id =
        seed_outbound_route_binding(pool, ctx.owner_id, warehouse_id, customer_id, now).await;
    repo.create_outbound_order(
        ctx,
        CreateOutboundOrderRequest {
            document_type: "sales_outbound".to_string(),
            wms_order_no: wms_order_no.to_string(),
            erp_order_no: Some(erp_order_no.to_string()),
            invoice_no: None,
            transport_mode_code: None,
            department_code: None,
            sales_group_code: None,
            order_group_no: None,
            business_type_code: None,
            customer_id,
            warehouse_id,
            delivery_address_id,
            required_ship_at: Some(now),
            lines: vec![CreateOutboundOrderLineRequest {
                line_no: 1,
                product_code: format!("P-{wms_order_no}"),
                batch_no: format!("B-{wms_order_no}"),
                planned_qty: 6.into(),
            }],
        },
        now,
        &format!("idem-{wms_order_no}"),
        None,
    )
    .await
    .expect("read fixture outbound order should be created")
    .value
}

include!("wave4_postgres/document_numbering.rs");
include!("wave4_postgres/temperature.rs");
include!("wave4_postgres/wave_reads.rs");
include!("wave4_postgres/review.rs");
include!("wave4_postgres/rollback.rs");

#[sqlx::test(migrations = "../../migrations")]
async fn outbound_order_reads_are_owner_scoped_filterable_and_include_lines(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let owner_ctx = ctx(owner_id);
    let other_ctx = ctx(other_owner_id);
    let repo = PgWave4Repository::new(pool.clone());
    let now = Utc
        .with_ymd_and_hms(2026, 6, 5, 8, 30, 0)
        .single()
        .expect("valid time");

    let first = create_read_order(
        &pool,
        &repo,
        &owner_ctx,
        "WMS-R-READ-001",
        "ERP-READ-001",
        now,
    )
    .await;
    let second = create_read_order(
        &pool,
        &repo,
        &owner_ctx,
        "WMS-R-READ-002",
        "ERP-READ-002",
        now,
    )
    .await;
    seed_outbound_inventory(
        &pool,
        owner_id,
        "P-WMS-R-READ-002",
        "B-WMS-R-READ-002",
        6,
        now,
    )
    .await;
    repo.create_outbound_wave(
        &owner_ctx,
        CreateOutboundWaveRequest {
            wave_no: "WAVE-READ-001".to_string(),
            order_ids: vec![second.id],
        },
        now,
        "outbound-read-wave-1",
        None,
    )
    .await
    .expect("second order should enter wave");

    create_read_order(
        &pool,
        &repo,
        &other_ctx,
        "WMS-R-READ-OTHER",
        "ERP-READ-001",
        now,
    )
    .await;

    let confirmed = repo
        .list_outbound_orders(&owner_ctx, Some("confirmed"), None, Some(10))
        .await
        .expect("confirmed orders should list");
    assert_eq!(confirmed.len(), 1);
    assert_eq!(confirmed[0].id, first.id);
    assert_eq!(confirmed[0].lines[0].product_code, "P-WMS-R-READ-001");

    let searched = repo
        .list_outbound_orders(&owner_ctx, None, Some("ERP-READ-001"), Some(10))
        .await
        .expect("query should match wms or erp order number");
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].owner_id, owner_id);
    assert_eq!(searched[0].wms_order_no, "WMS-R-READ-001");

    let limited = repo
        .list_outbound_orders(&owner_ctx, None, None, Some(1))
        .await
        .expect("limit should apply");
    assert_eq!(limited.len(), 1);

    for index in 0..49 {
        create_read_order(
            &pool,
            &repo,
            &owner_ctx,
            &format!("WMS-R-READ-FILLER-{index:03}"),
            &format!("ERP-R-READ-FILLER-{index:03}"),
            now,
        )
        .await;
    }
    let target = create_read_order(
        &pool,
        &repo,
        &owner_ctx,
        "WMS-R-READ-TARGET",
        "ERP-R-READ-TARGET",
        now,
    )
    .await;
    let default_window = repo
        .list_outbound_orders(&owner_ctx, None, None, None)
        .await
        .expect("default list should be bounded");
    assert_eq!(default_window.len(), 50);
    assert!(default_window.iter().all(|order| order.id != target.id));

    let targeted = repo
        .list_outbound_orders(&owner_ctx, Some("confirmed"), Some("TARGET"), Some(50))
        .await
        .expect("q, status, and limit should find a record outside the default window");
    assert_eq!(targeted.len(), 1);
    assert_eq!(targeted[0].id, target.id);

    let detail = repo
        .get_outbound_order(&owner_ctx, first.id)
        .await
        .expect("detail should load for same owner");
    assert_eq!(detail.id, first.id);
    assert_eq!(detail.lines[0].line_no, 1);
    assert_eq!(detail.lines[0].batch_no, "B-WMS-R-READ-001");

    let other_owner_detail = repo
        .get_outbound_order(&other_ctx, first.id)
        .await
        .expect_err("other owner must not read order detail");
    assert!(matches!(other_owner_detail, Wave4RepositoryError::NotFound));

    let other_owner_review = repo
        .review_outbound_order(
            &other_ctx,
            first.id,
            ReviewOutboundOrderRequest {
                reviewer_id: other_ctx.user_id,
                review_mode: "pda_loose".to_string(),
                second_reviewer_id: None,
                lines: vec![ReviewOutboundOrderLineRequest {
                    line_no: 1,
                    product_code: "P-WMS-R-READ-001".to_string(),
                    reviewed_qty: 6.into(),
                }],
            },
            now,
            "outbound-cross-owner-review-1",
            None,
        )
        .await
        .expect_err("other owner must not submit review");
    assert!(matches!(other_owner_review, Wave4RepositoryError::NotFound));
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
        disposition.quarantined_batches[0].status,
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
            (SELECT status FROM inventory_batches WHERE owner_id = $1 AND id = $2),
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
