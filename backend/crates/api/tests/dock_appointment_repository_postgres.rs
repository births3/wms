use chrono::{Duration, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    dock_appointment_repository::{DockAppointmentRepositoryError, PgDockAppointmentRepository},
};
use wms_domain::{
    ArriveDockAppointmentRequest, CancelDockAppointmentRequest, CreateDockAppointmentRequest,
    UpdateDockAppointmentRequest,
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "dock-appointment-test".to_string(),
        permissions: vec!["m1.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

fn at(hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 13, hour, 0, 0)
        .single()
        .expect("test timestamp should be valid")
}

async fn warehouse(pool: &PgPool, owner_id: Uuid, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1,$2,$3,$4,'pharmacy','active')",
    )
    .bind(id)
    .bind(owner_id)
    .bind(code)
    .bind(format!("{code} 仓"))
    .execute(pool)
    .await
    .expect("warehouse seed should persist");
    id
}

async fn dock(pool: &PgPool, warehouse_id: Uuid, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouse_docks (id, warehouse_id, dock_code, dock_type, temperature_zone) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(warehouse_id)
    .bind(code)
    .bind("both")
    .bind("normal")
    .execute(pool)
    .await
    .expect("dock seed should persist");
    id
}

fn request(
    dock_id: Uuid,
    warehouse_id: Uuid,
    no: &str,
    doc_no: &str,
) -> CreateDockAppointmentRequest {
    CreateDockAppointmentRequest {
        dock_id,
        warehouse_id,
        appointment_no: no.to_string(),
        document_type: "inbound".to_string(),
        document_no: doc_no.to_string(),
        window_start_at: at(9),
        window_end_at: at(10),
        vehicle_plate_no: Some("ABCD-001".to_string()),
        vehicle_type: "truck".to_string(),
        driver_name: "张三".to_string(),
        driver_phone: "13800000000".to_string(),
    }
}

fn audit(ctx: &AuthContext, id: &Uuid) -> AuditWriteRequest {
    audit_for(ctx, id, "create_dock_appointment")
}

fn audit_for(ctx: &AuthContext, id: &Uuid, action: &str) -> AuditWriteRequest {
    AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H2",
        "dock_appointment",
        id.to_string(),
        None,
    )
}

fn update_request(dock_id: Uuid, start_hour: u32) -> UpdateDockAppointmentRequest {
    UpdateDockAppointmentRequest {
        dock_id,
        window_start_at: at(start_hour),
        window_end_at: at(start_hour + 1),
        vehicle_plate_no: Some("ABCD-002".to_string()),
        vehicle_type: "truck".to_string(),
        driver_name: "李四".to_string(),
        driver_phone: "13900000000".to_string(),
        reason: Some("车辆调度变更".to_string()),
    }
}

fn arrival_request(appointment: &wms_domain::DockAppointment) -> ArriveDockAppointmentRequest {
    ArriveDockAppointmentRequest {
        appointment_no: appointment.appointment_no.clone(),
        vehicle_plate_no: appointment
            .vehicle_plate_no
            .clone()
            .expect("test appointment should have a plate"),
        driver_name: appointment.driver_name.clone(),
        vehicle_type: appointment.vehicle_type.clone(),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_with_audit_inserts_one_record(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-1").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-1").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);

    let created = repo
        .create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-1001", "DOC-1001"),
            at(8),
            "dock-app-create-1",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("create should succeed");

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND resource_type='dock_appointment' AND action='create_dock_appointment' AND resource_id=$2",
    )
    .bind(owner_id)
    .bind(created.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("audit count query should run");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn arrival_transitions_once_and_replays_without_duplicate_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-ARRIVE-1").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-ARRIVE-1").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let created = repo
        .create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-ARRIVE-1", "DOC-ARRIVE-1"),
            at(8),
            "dock-arrival-create-1",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("appointment should create");

    let arrived = repo
        .arrive_with_audit(
            &actor,
            created.id,
            arrival_request(&created),
            at(9),
            "dock-arrival-1",
            audit_for(&actor, &created.id, "arrive_dock_appointment"),
        )
        .await
        .expect("appointment should arrive");
    let replay = repo
        .arrive_with_audit(
            &actor,
            created.id,
            arrival_request(&created),
            at(9),
            "dock-arrival-2",
            audit_for(&actor, &created.id, "arrive_dock_appointment"),
        )
        .await
        .expect("arrived appointment should be idempotent");

    assert_eq!(arrived.status, "arrived");
    assert_eq!(arrived.arrived_at, Some(at(9)));
    assert_eq!(arrived.arrival_deviation_minutes, Some(0));
    assert_eq!(arrived.id, replay.id);
    assert_eq!(replay.arrival_deviation_minutes, Some(0));
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND action='arrive_dock_appointment' AND resource_id=$2",
    )
    .bind(owner_id)
    .bind(created.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("arrival audit count should query");
    assert_eq!(audit_count, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn arrival_persists_positive_and_negative_deviation(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-ARRIVE-DEVIATION").await;
    let late_dock_id = dock(&pool, warehouse_id, "DOCK-ARRIVE-LATE").await;
    let early_dock_id = dock(&pool, warehouse_id, "DOCK-ARRIVE-EARLY").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);

    let late = repo
        .create_with_audit(
            &actor,
            request(
                late_dock_id,
                warehouse_id,
                "APP-ARRIVE-LATE",
                "DOC-ARRIVE-LATE",
            ),
            at(8),
            "dock-arrival-deviation-create-late",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("late appointment should create");
    let late = repo
        .arrive_with_audit(
            &actor,
            late.id,
            arrival_request(&late),
            at(9) + Duration::minutes(30),
            "dock-arrival-deviation-late",
            audit_for(&actor, &late.id, "arrive_dock_appointment"),
        )
        .await
        .expect("late appointment should arrive");
    assert_eq!(late.arrival_deviation_minutes, Some(30));

    let early = repo
        .create_with_audit(
            &actor,
            request(
                early_dock_id,
                warehouse_id,
                "APP-ARRIVE-EARLY",
                "DOC-ARRIVE-EARLY",
            ),
            at(8),
            "dock-arrival-deviation-create-early",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("early appointment should create");
    let early = repo
        .arrive_with_audit(
            &actor,
            early.id,
            arrival_request(&early),
            at(8) + Duration::minutes(45),
            "dock-arrival-deviation-early",
            audit_for(&actor, &early.id, "arrive_dock_appointment"),
        )
        .await
        .expect("early appointment should arrive");
    assert_eq!(early.arrival_deviation_minutes, Some(-15));
}

#[sqlx::test(migrations = "../../migrations")]
async fn arrival_rejects_mismatched_vehicle_identity(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-ARRIVE-2").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-ARRIVE-2").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let created = repo
        .create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-ARRIVE-2", "DOC-ARRIVE-2"),
            at(8),
            "dock-arrival-create-2",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("appointment should create");
    let mut arrival = arrival_request(&created);
    arrival.vehicle_plate_no = "WRONG-PLATE".to_string();

    assert!(matches!(
        repo.arrive_with_audit(
            &actor,
            created.id,
            arrival,
            at(9),
            "dock-arrival-mismatch",
            audit_for(&actor, &created.id, "arrive_dock_appointment"),
        )
        .await,
        Err(DockAppointmentRepositoryError::ArrivalCheckMismatch)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn arrival_rejects_vehicle_for_cold_dock(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-ARRIVE-3").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-ARRIVE-3").await;
    sqlx::query("UPDATE warehouse_docks SET temperature_zone='cold' WHERE id=$1")
        .bind(dock_id)
        .execute(&pool)
        .await
        .expect("cold dock should persist");
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let created = repo
        .create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-ARRIVE-3", "DOC-ARRIVE-3"),
            at(8),
            "dock-arrival-create-3",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("appointment should create");

    assert!(matches!(
        repo.arrive_with_audit(
            &actor,
            created.id,
            arrival_request(&created),
            at(9),
            "dock-arrival-temperature-mismatch",
            audit_for(&actor, &created.id, "arrive_dock_appointment"),
        )
        .await,
        Err(DockAppointmentRepositoryError::TemperatureMismatch)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn same_owner_active_document_conflict(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-2").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-2").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);

    repo.create_with_audit(
        &actor,
        request(dock_id, warehouse_id, "APP-2001", "DOC-2001"),
        at(8),
        "dock-app-conflict-1",
        audit(&actor, &Uuid::new_v4()),
    )
    .await
    .expect("first appointment should create");

    assert!(matches!(
        repo.create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-2002", "DOC-2001"),
            at(8),
            "dock-app-conflict-2",
            audit(&actor, &Uuid::new_v4()),
        )
        .await,
        Err(DockAppointmentRepositoryError::ActiveAppointmentConflict)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn different_owner_same_document_no_allows(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let wh_a = warehouse(&pool, owner_a, "WH-DA-3A").await;
    let wh_b = warehouse(&pool, owner_b, "WH-DA-3B").await;
    let dock_a = dock(&pool, wh_a, "DOCK-3A").await;
    let dock_b = dock(&pool, wh_b, "DOCK-3B").await;
    let repo_a = PgDockAppointmentRepository::new(pool.clone());
    let repo_b = PgDockAppointmentRepository::new(pool.clone());

    let app_a = repo_a
        .create_with_audit(
            &ctx(owner_a),
            request(dock_a, wh_a, "APP-3001", "DOC-3001"),
            at(8),
            "dock-app-owner-a",
            audit(&ctx(owner_a), &Uuid::new_v4()),
        )
        .await
        .expect("owner A should create");
    let app_b = repo_b
        .create_with_audit(
            &ctx(owner_b),
            request(dock_b, wh_b, "APP-3002", "DOC-3001"),
            at(8),
            "dock-app-owner-b",
            audit(&ctx(owner_b), &Uuid::new_v4()),
        )
        .await
        .expect("owner B should create same document no");

    assert_ne!(app_a.id, app_b.id);
}

#[sqlx::test(migrations = "../../migrations")]
async fn cross_owner_dock_or_warehouse_reject(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let wh_a = warehouse(&pool, owner_a, "WH-DA-4A").await;
    let wh_b = warehouse(&pool, owner_b, "WH-DA-4B").await;
    let dock_b = dock(&pool, wh_b, "DOCK-4B").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());

    assert!(matches!(
        repo.create_with_audit(
            &ctx(owner_a),
            request(dock_b, wh_a, "APP-4001", "DOC-4001"),
            at(8),
            "dock-app-cross-1",
            audit(&ctx(owner_a), &Uuid::new_v4()),
        )
        .await,
        Err(DockAppointmentRepositoryError::OwnerWarehouseMismatch)
    ));
    assert_eq!(
        repo.list(&ctx(owner_a), wh_b, None, None, None, None)
            .await
            .err(),
        Some(DockAppointmentRepositoryError::OwnerWarehouseMismatch)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn idempotency_replay_and_conflict(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-5").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-5").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);

    let req = request(dock_id, warehouse_id, "APP-5001", "DOC-5001");
    let first = repo
        .create_with_audit(
            &actor,
            req.clone(),
            at(8),
            "dock-app-replay",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("first create should succeed");
    let second = repo
        .create_with_audit(
            &actor,
            req.clone(),
            at(8),
            "dock-app-replay",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("replay should return original");
    assert_eq!(first.id, second.id);

    let mut req2 = req;
    req2.appointment_no = "APP-5002".to_string();
    assert!(matches!(
        repo.create_with_audit(
            &actor,
            req2,
            at(8),
            "dock-app-replay",
            audit(&actor, &Uuid::new_v4()),
        )
        .await,
        Err(DockAppointmentRepositoryError::IdempotencyConflict)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn illegal_window_rejected(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-6").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-6").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);

    let mut req = request(dock_id, warehouse_id, "APP-6001", "DOC-6001");
    req.window_end_at = at(8);
    assert!(matches!(
        repo.create_with_audit(
            &actor,
            req.clone(),
            at(9),
            "dock-app-window-invalid",
            audit(&actor, &Uuid::new_v4()),
        )
        .await,
        Err(DockAppointmentRepositoryError::WindowInvalid)
    ));

    req.window_end_at = at(8) + Duration::hours(1);
    req.window_start_at = at(6);
    assert!(matches!(
        repo.create_with_audit(
            &actor,
            req.clone(),
            at(10),
            "dock-app-window-ended",
            audit(&actor, &Uuid::new_v4()),
        )
        .await,
        Err(DockAppointmentRepositoryError::WindowEnded)
    ));

    req.appointment_no.clear();
    assert!(matches!(
        repo.create_with_audit(
            &actor,
            req,
            at(8),
            "dock-app-required-field",
            audit(&actor, &Uuid::new_v4()),
        )
        .await,
        Err(DockAppointmentRepositoryError::Invalid(field)) if field == "appointment_no 不能为空"
    ));
    assert_eq!(
        repo.list(&actor, warehouse_id, None, Some(at(11)), Some(at(10)), None)
            .await
            .err(),
        Some(DockAppointmentRepositoryError::WindowInvalid)
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn change_preserves_previous_version_and_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-7").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-7").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let created = repo
        .create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-7001", "DOC-7001"),
            at(8),
            "dock-app-change-create",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("appointment should create");

    let changed = repo
        .change_with_audit(
            &actor,
            created.id,
            update_request(dock_id, 11),
            at(8),
            "dock-app-change-1",
            audit_for(&actor, &created.id, "change_dock_appointment"),
        )
        .await
        .expect("appointment should change");

    assert_eq!(changed.version, 2);
    assert_eq!(changed.supersedes_id, Some(created.id));
    assert_eq!(changed.document_no, created.document_no);
    assert_eq!(changed.appointment_no, "APP-7001-V2");
    let old_status: String =
        sqlx::query_scalar("SELECT status FROM dock_appointments WHERE id = $1")
            .bind(created.id)
            .fetch_one(&pool)
            .await
            .expect("old appointment should remain");
    assert_eq!(old_status, "cancelled");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND action='change_dock_appointment' AND resource_id=$2",
    )
    .bind(owner_id)
    .bind(changed.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("change audit should query");
    assert_eq!(audit_count, 1);
    let audit_diff: serde_json::Value = sqlx::query_scalar(
        "SELECT diff FROM audit_event WHERE owner_id=$1 AND action='change_dock_appointment' AND resource_id=$2",
    )
    .bind(owner_id)
    .bind(changed.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("change audit diff should query");
    assert_eq!(audit_diff["after"]["reason"], "车辆调度变更");
}

#[sqlx::test(migrations = "../../migrations")]
async fn change_conflict_rolls_back_previous_version(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-8").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-8").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let first = repo
        .create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-8001", "DOC-8001"),
            at(8),
            "dock-app-change-conflict-first",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("first appointment should create");
    let mut second_request = request(dock_id, warehouse_id, "APP-8002", "DOC-8002");
    second_request.window_start_at = at(11);
    second_request.window_end_at = at(12);
    repo.create_with_audit(
        &actor,
        second_request,
        at(8),
        "dock-app-change-conflict-second",
        audit(&actor, &Uuid::new_v4()),
    )
    .await
    .expect("second appointment should create");

    assert!(matches!(
        repo.change_with_audit(
            &actor,
            first.id,
            update_request(dock_id, 11),
            at(8),
            "dock-app-change-conflict-update",
            audit_for(&actor, &first.id, "change_dock_appointment"),
        )
        .await,
        Err(DockAppointmentRepositoryError::TimeConflict)
    ));
    let old_status: String =
        sqlx::query_scalar("SELECT status FROM dock_appointments WHERE id = $1")
            .bind(first.id)
            .fetch_one(&pool)
            .await
            .expect("old appointment should remain queryable");
    assert_eq!(old_status, "pending");
}

#[sqlx::test(migrations = "../../migrations")]
async fn cancel_is_idempotent_and_arrived_cannot_cancel(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-9").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-9").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let created = repo
        .create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-9001", "DOC-9001"),
            at(8),
            "dock-app-cancel-create",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("appointment should create");
    sqlx::query("UPDATE dock_appointments SET status='confirmed' WHERE id=$1")
        .bind(created.id)
        .execute(&pool)
        .await
        .expect("appointment should confirm for test");

    let cancel_request = CancelDockAppointmentRequest {
        reason: Some("车辆调度变化".to_string()),
    };
    let cancelled = repo
        .cancel_with_audit(
            &actor,
            created.id,
            cancel_request.clone(),
            at(8),
            "dock-app-cancel-1",
            audit_for(&actor, &created.id, "cancel_dock_appointment"),
        )
        .await
        .expect("confirmed appointment should cancel");
    let replay = repo
        .cancel_with_audit(
            &actor,
            created.id,
            cancel_request,
            at(8),
            "dock-app-cancel-1",
            audit_for(&actor, &created.id, "cancel_dock_appointment"),
        )
        .await
        .expect("cancel should replay");
    assert_eq!(cancelled.id, replay.id);
    assert_eq!(cancelled.status, "cancelled");

    let arrived = repo
        .create_with_audit(
            &actor,
            request(dock_id, warehouse_id, "APP-9002", "DOC-9002"),
            at(8),
            "dock-app-arrived-create",
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("arrived appointment should create");
    sqlx::query("UPDATE dock_appointments SET status='arrived' WHERE id=$1")
        .bind(arrived.id)
        .execute(&pool)
        .await
        .expect("appointment should arrive for test");
    assert!(matches!(
        repo.cancel_with_audit(
            &actor,
            arrived.id,
            CancelDockAppointmentRequest { reason: None },
            at(8),
            "dock-app-arrived-cancel",
            audit_for(&actor, &arrived.id, "cancel_dock_appointment"),
        )
        .await,
        Err(DockAppointmentRepositoryError::StatusNotCancellable)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn overlapping_window_is_rejected_but_different_dock_is_allowed(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-7").await;
    let dock_id = dock(&pool, warehouse_id, "DOCK-7A").await;
    let other_dock_id = dock(&pool, warehouse_id, "DOCK-7B").await;
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);

    repo.create_with_audit(
        &actor,
        request(dock_id, warehouse_id, "APP-7001", "DOC-7001"),
        at(8),
        "dock-app-overlap-1",
        audit(&actor, &Uuid::new_v4()),
    )
    .await
    .expect("first appointment should create");

    let mut overlapping = request(dock_id, warehouse_id, "APP-7002", "DOC-7002");
    overlapping.window_start_at = at(9) + Duration::minutes(30);
    overlapping.window_end_at = at(10) + Duration::minutes(30);
    assert!(matches!(
        repo.create_with_audit(
            &actor,
            overlapping,
            at(8),
            "dock-app-overlap-2",
            audit(&actor, &Uuid::new_v4()),
        )
        .await,
        Err(DockAppointmentRepositoryError::TimeConflict)
    ));

    let mut different_dock = request(other_dock_id, warehouse_id, "APP-7003", "DOC-7003");
    different_dock.window_start_at = at(9) + Duration::minutes(30);
    different_dock.window_end_at = at(10) + Duration::minutes(30);
    repo.create_with_audit(
        &actor,
        different_dock,
        at(8),
        "dock-app-overlap-3",
        audit(&actor, &Uuid::new_v4()),
    )
    .await
    .expect("different dock should allow the same window");
}

#[sqlx::test(migrations = "../../migrations")]
async fn list_filters_sorts_and_isolates_owner_appointments(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-DA-LIST").await;
    let dock_ids = [
        dock(&pool, warehouse_id, "DOCK-LIST-A").await,
        dock(&pool, warehouse_id, "DOCK-LIST-B").await,
    ];
    let repo = PgDockAppointmentRepository::new(pool.clone());
    let actor = ctx(owner_id);

    for (dock_id, suffix) in [(dock_ids[0], "1"), (dock_ids[1], "2")] {
        repo.create_with_audit(
            &actor,
            request(
                dock_id,
                warehouse_id,
                &format!("APP-LIST-{suffix}"),
                &format!("DOC-LIST-{suffix}"),
            ),
            at(8),
            &format!("dock-app-list-{suffix}"),
            audit(&actor, &Uuid::new_v4()),
        )
        .await
        .expect("list appointment should create");
    }
    let listed = repo
        .list(&actor, warehouse_id, None, Some(at(9)), Some(at(11)), None)
        .await
        .expect("owner list should succeed");
    assert!(listed[0].dock_id <= listed[1].dock_id);

    let filtered = repo
        .list(
            &actor,
            warehouse_id,
            Some(dock_ids[1]),
            Some(at(9)),
            Some(at(10)),
            Some("pending".to_string()),
        )
        .await
        .expect("dock/status list should succeed");
    assert_eq!(filtered.len(), 1);
}
