use chrono::{DateTime, TimeZone, Utc};
use sqlx::Error;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    dock_repository::{DockRepositoryError, PgDockRepository},
};
use wms_domain::{CreateDockImportRequest, CreateDockRequest, UpdateDockRequest};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "dock-test".to_string(),
        permissions: Vec::new(),
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 13, hour, 0, 0)
        .single()
        .expect("test timestamp should be valid")
}

fn is_db_code(err: &Error, code: &str) -> bool {
    match err.as_database_error() {
        Some(database_error) => match database_error.code() {
            Some(err_code) => err_code == code,
            None => false,
        },
        None => false,
    }
}

async fn warehouse(pool: &PgPool, owner_id: Uuid, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1,$2,$3,$4,'pharmacy','active')",
    )
    .bind(id)
    .bind(owner_id)
    .bind(code)
    .bind(format!("{code} 测试仓"))
    .execute(pool)
    .await
    .expect("warehouse seed should persist");
    id
}

fn request(warehouse_id: Uuid, code: &str) -> CreateDockRequest {
    CreateDockRequest {
        warehouse_id,
        dock_code: code.to_string(),
        dock_type: "both".to_string(),
        temperature_zone: "normal".to_string(),
        location_description: Some("东门".to_string()),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn dock_create_and_list_respect_warehouse_owner_boundary(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let second_owner = Uuid::new_v4();
    let first_warehouse = warehouse(&pool, owner_id, "WH-1").await;
    let second_warehouse = warehouse(&pool, second_owner, "WH-2").await;
    let repo = PgDockRepository::new(pool.clone());
    let actor = ctx(owner_id);

    let create_request = request(first_warehouse, "D-01");
    let first = repo
        .create_dock(&actor, create_request.clone(), at(9), "dock-create-1")
        .await
        .expect("dock should be created");
    assert_eq!(first.warehouse_id, first_warehouse);
    let replay = repo
        .create_dock(&actor, create_request.clone(), at(9), "dock-create-1")
        .await
        .expect("same create should replay");
    assert_eq!(replay.id, first.id);
    sqlx::query(
        "UPDATE idempotency_request SET method = 'PATCH', path = '/wrong-path' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_id)
    .bind("dock-create-1")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be mutable for the regression check");
    let metadata_conflict = repo
        .create_dock(&actor, create_request, at(9), "dock-create-1")
        .await
        .expect_err("method and path changes must invalidate a replay");
    assert_eq!(metadata_conflict, DockRepositoryError::IdempotencyConflict);
    assert!(matches!(
        repo.create_dock(
            &actor,
            request(first_warehouse, "D-01"),
            at(9),
            "dock-create-duplicate",
        )
        .await,
        Err(DockRepositoryError::DuplicateCode)
    ));
    repo.create_dock(
        &ctx(second_owner),
        request(second_warehouse, "D-01"),
        at(9),
        "dock-create-other-owner",
    )
    .await
    .expect("same code in another warehouse should be allowed");
    assert!(repo
        .list_docks(&actor, second_warehouse)
        .await
        .expect("cross-owner warehouse query should be safe")
        .is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn dock_update_changes_status_and_maintenance_recovery_date(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-1").await;
    let repo = PgDockRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let dock = repo
        .create_dock(
            &actor,
            request(warehouse_id, "D-01"),
            at(9),
            "dock-create-update",
        )
        .await
        .expect("dock should be created");
    let recovery_at = at(18);
    let updated = repo
        .update_dock(
            &actor,
            dock.id,
            UpdateDockRequest {
                status: "maintenance".to_string(),
                maintenance_recovery_at: Some(recovery_at),
            },
            at(10),
            "dock-update-1",
        )
        .await
        .expect("dock status should update");
    assert_eq!(updated.status, "maintenance");
    assert_eq!(updated.maintenance_recovery_at, Some(recovery_at));
    let audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id=$1 AND resource_id=$2",
    )
    .bind(owner_id)
    .bind(dock.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("dock audit count should query");
    assert_eq!(audits, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn dock_import_is_transactional_and_delete_respects_active_appointments(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-IMPORT").await;
    let repo = PgDockRepository::new(pool.clone());
    let actor = ctx(owner_id);
    let import_request = CreateDockImportRequest {
        warehouse_id,
        docks: vec![request(warehouse_id, "D-01"), request(warehouse_id, "D-02")],
    };
    let imported = repo
        .import_docks(&actor, import_request.clone(), at(9), "dock-import-1")
        .await
        .expect("bulk import should create all rows");
    assert_eq!(imported.len(), 2);
    let replay = repo
        .import_docks(&actor, import_request, at(9), "dock-import-1")
        .await
        .expect("same bulk import should replay");
    assert_eq!(
        replay.iter().map(|dock| dock.id).collect::<Vec<_>>(),
        imported.iter().map(|dock| dock.id).collect::<Vec<_>>()
    );
    let import_evidence: (i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT COUNT(*) FROM audit_event
             WHERE owner_id = $1 AND action = 'import_dock'),
           (SELECT COUNT(*) FROM idempotency_request
             WHERE owner_id = $1 AND idempotency_key = 'dock-import-1')",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("dock import audit and idempotency evidence should query");
    assert_eq!(import_evidence, (2, 1));

    sqlx::query(
        "INSERT INTO dock_appointments
         (id, dock_id, owner_id, warehouse_id, appointment_no, document_type, document_no,
          window_start_at, window_end_at, vehicle_type, driver_name, status)
         VALUES ($1,$2,$3,$4,$5,'inbound',$6,$7,$8,'truck','测试司机','confirmed')",
    )
    .bind(Uuid::new_v4())
    .bind(imported[0].id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind("APP-DELETE-GUARD")
    .bind("DOC-DELETE-GUARD")
    .bind(at(9))
    .bind(at(10))
    .execute(&pool)
    .await
    .expect("appointment seed should persist");
    assert!(matches!(
        repo.delete_dock(&actor, imported[0].id, at(10), "dock-delete-in-use")
            .await,
        Err(DockRepositoryError::InUse(1))
    ));

    repo.delete_dock(&actor, imported[1].id, at(11), "dock-delete-1")
        .await
        .expect("dock without appointments should be deletable");
    let remaining = repo
        .list_docks(&actor, warehouse_id)
        .await
        .expect("dock list should remain readable");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].dock_code, "D-01");
}

#[sqlx::test(migrations = "../../migrations")]
async fn dock_manage_is_granted_only_to_warehouse_manager_and_system_admin(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $2)")
        .bind(owner_id)
        .bind(format!("DOCK-PERM-{owner_id}"))
        .execute(&pool)
        .await
        .expect("owner should seed default roles");

    let grants: Vec<(String, i64)> = sqlx::query_as(
        "SELECT role.role_code, COUNT(permission.permission_code)::BIGINT
           FROM auth_roles role
           LEFT JOIN auth_role_permissions role_permission ON role_permission.role_id = role.id
           LEFT JOIN auth_permissions permission
             ON permission.id = role_permission.permission_id
            AND permission.permission_code = 'dock.manage'
          WHERE role.owner_id = $1
          GROUP BY role.role_code
          ORDER BY role.role_code",
    )
    .bind(owner_id)
    .fetch_all(&pool)
    .await
    .expect("role permission grants should be queryable");

    let grant_for = |role_code: &str| {
        grants
            .iter()
            .find(|(code, _)| code == role_code)
            .map(|(_, count)| *count)
            .expect("default role should be seeded")
    };

    assert_eq!(grant_for("system_admin"), 1);
    assert_eq!(grant_for("warehouse_manager"), 1);
    assert_eq!(grant_for("receiving_clerk"), 0);
    assert_eq!(grant_for("owner_user"), 0);
}

async fn dock(pool: &PgPool, warehouse_id: Uuid, dock_code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouse_docks (id, warehouse_id, dock_code, dock_type, temperature_zone) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(warehouse_id)
    .bind(dock_code)
    .bind("both")
    .bind("normal")
    .execute(pool)
    .await
    .expect("dock seed should persist");
    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn dock_appointments_requires_active_status_unique_per_owner_document(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-APP-2").await;
    let dock_id = dock(&pool, warehouse_id, "D-APP-2").await;
    let start = at(10);
    let end = at(11);

    let first = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO dock_appointments
         (id, dock_id, warehouse_id, owner_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_type, driver_name, updated_at, version)
         VALUES ($1,$2,$3,$4,'pending',$5,$6,$7,$8,$9,$10,$11,$12,1)",
    )
    .bind(first)
    .bind(dock_id)
    .bind(warehouse_id)
    .bind(owner_id)
    .bind("APP-1001")
    .bind("inbound")
    .bind("DOC-1001")
    .bind(start)
    .bind(end)
    .bind("truck")
    .bind("Alice")
    .bind(at(12))
    .execute(&pool)
    .await
    .expect("first active appointment should insert");

    let conflict = sqlx::query(
        "INSERT INTO dock_appointments
         (id, dock_id, warehouse_id, owner_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_type, driver_name, updated_at, version)
         VALUES ($1,$2,$3,$4,'confirmed',$5,$6,$7,$8,$9,$10,$11,$12,1)",
    )
    .bind(Uuid::new_v4())
    .bind(dock_id)
    .bind(warehouse_id)
    .bind(owner_id)
    .bind("APP-1002")
    .bind("inbound")
    .bind("DOC-1001")
    .bind(start + chrono::Duration::minutes(30))
    .bind(end + chrono::Duration::minutes(30))
    .bind("truck")
    .bind("Alice")
    .bind(at(12))
    .execute(&pool)
    .await;
    let is_unique_conflict = conflict
        .err()
        .as_ref()
        .map(|error| is_db_code(error, "23505"))
        .unwrap_or(false);
    assert!(is_unique_conflict, "active uniqueness should conflict");
}

#[sqlx::test(migrations = "../../migrations")]
async fn dock_appointments_allows_new_active_after_completed(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-APP-3").await;
    let dock_id = dock(&pool, warehouse_id, "D-APP-3").await;

    for (status, suffix) in [("completed", "C"), ("pending", "P")] {
        sqlx::query(
            "INSERT INTO dock_appointments
             (id, dock_id, warehouse_id, owner_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_type, driver_name, updated_at, version)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,1)",
        )
        .bind(Uuid::new_v4())
        .bind(dock_id)
        .bind(warehouse_id)
        .bind(owner_id)
        .bind(status)
        .bind(format!("APP-{suffix}"))
        .bind("inbound")
        .bind("DOC-1002")
        .bind(at(14))
        .bind(at(15))
        .bind("truck")
        .bind("Bob")
        .bind(at(14))
        .execute(&pool)
        .await
        .unwrap_or_else(|_| panic!("appointment with status {status} should insert"));
    }

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM dock_appointments WHERE owner_id = $1 AND document_type = $2 AND document_no = $3",
    )
    .bind(owner_id)
    .bind("inbound")
    .bind("DOC-1002")
    .fetch_one(&pool)
    .await
    .expect("appointment count should query");
    assert_eq!(total, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn dock_appointments_rejects_invalid_window(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let warehouse_id = warehouse(&pool, owner_id, "WH-APP-4").await;
    let dock_id = dock(&pool, warehouse_id, "D-APP-4").await;
    let start = at(16);
    let invalid = sqlx::query(
        "INSERT INTO dock_appointments
         (id, dock_id, warehouse_id, owner_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_type, driver_name, updated_at, version)
         VALUES ($1,$2,$3,$4,'pending',$5,$6,$7,$8,$9,$10,$11,$12,1)",
    )
    .bind(Uuid::new_v4())
    .bind(dock_id)
    .bind(warehouse_id)
    .bind(owner_id)
    .bind("APP-INV-1")
    .bind("inbound")
    .bind("DOC-INV-1")
    .bind(start)
    .bind(start)
    .bind("truck")
    .bind("Alice")
    .bind(at(16))
    .execute(&pool)
    .await;
    let is_check_violation = invalid
        .err()
        .as_ref()
        .map(|error| is_db_code(error, "23514"))
        .unwrap_or(false);
    assert!(
        is_check_violation,
        "window_end_at <= window_start_at should be rejected"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn dock_appointments_rejects_cross_owner_warehouse_fk(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let other_owner_id = Uuid::new_v4();
    let owner_warehouse = warehouse(&pool, owner_id, "WH-OWNER").await;
    let _other_warehouse = warehouse(&pool, other_owner_id, "WH-OTHER").await;
    let dock_id = dock(&pool, owner_warehouse, "D-APP-5").await;
    let invalid = sqlx::query(
        "INSERT INTO dock_appointments
         (id, dock_id, warehouse_id, owner_id, status, appointment_no, document_type, document_no, window_start_at, window_end_at, vehicle_type, driver_name, updated_at, version)
         VALUES ($1,$2,$3,$4,'confirmed',$5,$6,$7,$8,$9,$10,$11,$12,1)",
    )
    .bind(Uuid::new_v4())
    .bind(dock_id)
    .bind(owner_warehouse)
    .bind(other_owner_id)
    .bind("APP-OWN-1")
    .bind("inbound")
    .bind("DOC-OWN-1")
    .bind(at(17))
    .bind(at(18))
    .bind("truck")
    .bind("Alice")
    .bind(at(17))
    .execute(&pool)
    .await;
    let is_fk_violation = invalid
        .err()
        .as_ref()
        .map(|error| is_db_code(error, "23503"))
        .unwrap_or(false);
    assert!(
        is_fk_violation,
        "cross-owner warehouse fk should be rejected"
    );
}
