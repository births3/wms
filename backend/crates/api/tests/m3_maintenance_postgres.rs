use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    audit::AuditWriteRequest,
    auth::AuthContext,
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{CreateMaintenanceRecordRequest, MaintenanceRecordQuery, MaintenanceTaskQuery};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m3-maintenance-test".to_string(),
        permissions: vec!["m3.read".to_string(), "m3.maintenance.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_batch(
    pool: &PgPool,
    owner_id: Uuid,
    batch_no: &str,
    expiry_date: NaiveDate,
    quality_status: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, 'P-M3-MAINT', $3, $4, $5, 10, 0, $6, $7, $8, FALSE, $9, $9)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(batch_no)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid production date"))
    .bind(expiry_date)
    .bind(quality_status)
    .bind(Uuid::new_v4())
    .bind(format!("M3-MAINT-{}", &id.to_string()[..8]))
    .bind(now)
    .execute(pool)
    .await
    .expect("seed maintenance batch");
    id
}

async fn seed_task(pool: &PgPool, owner_id: Uuid, batch_id: Uuid) -> Uuid {
    let task_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_maintenance_tasks (
            id, owner_id, inventory_batch_id, planned_at, status, created_at
        )
        VALUES ($1, $2, $3, $4, 'pending', $4)
        "#,
    )
    .bind(task_id)
    .bind(owner_id)
    .bind(batch_id)
    .bind(Utc::now())
    .execute(pool)
    .await
    .expect("seed maintenance task");
    task_id
}

fn request(task_id: Uuid) -> CreateMaintenanceRecordRequest {
    CreateMaintenanceRecordRequest {
        task_id,
        temperature_celsius: 22.5,
        humidity_percent: 45.0,
        appearance: "intact".to_string(),
        packaging: "intact".to_string(),
        pest: "none".to_string(),
        rodent: "none".to_string(),
        mildew: "none".to_string(),
        conclusion: "normal".to_string(),
        exception_type: None,
        notes: Some("定期养护正常".to_string()),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn maintenance_tasks_and_records_are_owner_scoped_and_written_atomically(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = seed_batch(
        &pool,
        owner_id,
        "B-M3-MAINT-001",
        NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"),
        "qualified",
    )
    .await;
    let task_id = seed_task(&pool, owner_id, batch_id).await;
    let context = ctx(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let audit = AuditWriteRequest::from_auth_context(
        &context,
        "create",
        "M3",
        "inventory_maintenance_record",
        task_id.to_string(),
        None,
    );

    let result = repository
        .create_maintenance_record_with_audit(
            &context,
            request(task_id),
            Utc::now(),
            "m3-maintenance-001",
            Some(audit),
        )
        .await
        .expect("maintenance result should be persisted");

    assert!(!result.replayed);
    assert_eq!(result.value.task_id, task_id);
    assert_eq!(result.value.batch_id, batch_id);
    assert_eq!(result.value.conclusion, "normal");

    let tasks = repository
        .list_maintenance_tasks(&context, MaintenanceTaskQuery::default())
        .await
        .expect("maintenance tasks should be queryable");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].status, "completed");

    let records = repository
        .list_maintenance_records(&context, MaintenanceRecordQuery::default())
        .await
        .expect("maintenance records should be queryable");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, result.value.id);

    let counts: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM inventory_maintenance_records WHERE owner_id = $1),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'create'
                AND resource_type = 'inventory_maintenance_record'),
            (SELECT COUNT(*) FROM inventory_maintenance_tasks
              WHERE owner_id = $1 AND status = 'completed')
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("maintenance write evidence should query");
    assert_eq!(counts, (1, 1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_maintenance_submission_replays_without_second_record_or_audit(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = seed_batch(
        &pool,
        owner_id,
        "B-M3-MAINT-002",
        NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"),
        "qualified",
    )
    .await;
    let task_id = seed_task(&pool, owner_id, batch_id).await;
    let context = ctx(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let audit = AuditWriteRequest::from_auth_context(
        &context,
        "create",
        "M3",
        "inventory_maintenance_record",
        task_id.to_string(),
        None,
    );
    let first = repository
        .create_maintenance_record_with_audit(
            &context,
            request(task_id),
            Utc::now(),
            "m3-maintenance-002",
            Some(audit.clone()),
        )
        .await
        .expect("first maintenance result should succeed");
    let replay = repository
        .create_maintenance_record_with_audit(
            &context,
            request(task_id),
            Utc::now(),
            "m3-maintenance-002",
            Some(audit),
        )
        .await
        .expect("duplicate maintenance result should replay");

    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.id, replay.value.id);

    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM inventory_maintenance_records WHERE task_id = $1), (SELECT COUNT(*) FROM audit_event WHERE owner_id = $2 AND resource_id = $3)",
    )
    .bind(task_id)
    .bind(owner_id)
    .bind(first.value.id.to_string())
    .fetch_one(&pool)
    .await
    .expect("duplicate maintenance evidence should query");
    assert_eq!(counts, (1, 1));

    let update_error = sqlx::query(
        "UPDATE inventory_maintenance_records SET notes = 'must remain append-only' WHERE id = $1",
    )
    .bind(first.value.id)
    .execute(&pool)
    .await
    .expect_err("maintenance records must be append-only");
    assert!(update_error.to_string().contains("append-only"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn maintenance_submission_rejects_cross_owner_task(pool: PgPool) {
    let task_owner = Uuid::new_v4();
    let caller_owner = Uuid::new_v4();
    let batch_id = seed_batch(
        &pool,
        task_owner,
        "B-M3-MAINT-003",
        NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"),
        "qualified",
    )
    .await;
    let task_id = seed_task(&pool, task_owner, batch_id).await;
    let repository = PgWave3Repository::new(pool.clone());

    let error = repository
        .create_maintenance_record_with_audit(
            &ctx(caller_owner),
            request(task_id),
            Utc::now(),
            "m3-maintenance-cross-owner",
            None,
        )
        .await
        .expect_err("cross-owner maintenance task must be hidden");
    assert_eq!(error, Wave3RepositoryError::NotFound);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM inventory_maintenance_records WHERE task_id = $1")
            .bind(task_id)
            .fetch_one(&pool)
            .await
            .expect("cross-owner maintenance evidence should query");
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn maintenance_submission_rejects_expired_and_unqualified_batches(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let expired_id = seed_batch(
        &pool,
        owner_id,
        "B-M3-MAINT-004-EXPIRED",
        NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid expiry date"),
        "qualified",
    )
    .await;
    let unqualified_id = seed_batch(
        &pool,
        owner_id,
        "B-M3-MAINT-004-UNQUALIFIED",
        NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"),
        "unqualified",
    )
    .await;
    let expired_task_id = seed_task(&pool, owner_id, expired_id).await;
    let unqualified_task_id = seed_task(&pool, owner_id, unqualified_id).await;
    let repository = PgWave3Repository::new(pool);
    let context = ctx(owner_id);

    let expired_error = repository
        .create_maintenance_record_with_audit(
            &context,
            request(expired_task_id),
            Utc::now(),
            "m3-maintenance-expired",
            None,
        )
        .await
        .expect_err("expired batch must not be maintained");
    assert_eq!(expired_error, Wave3RepositoryError::BatchExpired);

    let status_error = repository
        .create_maintenance_record_with_audit(
            &context,
            request(unqualified_task_id),
            Utc::now(),
            "m3-maintenance-unqualified",
            None,
        )
        .await
        .expect_err("unqualified batch must not be maintained");
    assert_eq!(status_error, Wave3RepositoryError::InvalidInventoryState);
}

#[sqlx::test(migrations = "../../migrations")]
async fn maintenance_abnormal_isolates_batch_and_writes_notification(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let batch_id = seed_batch(
        &pool,
        owner_id,
        "B-M3-MAINT-ABN",
        NaiveDate::from_ymd_opt(2028, 6, 1).expect("valid expiry date"),
        "qualified",
    )
    .await;
    let task_id = seed_task(&pool, owner_id, batch_id).await;
    let context = ctx(owner_id);
    let repository = PgWave3Repository::new(pool.clone());
    let mut req = request(task_id);
    req.conclusion = "abnormal".to_string();
    req.exception_type = Some("package_damage".to_string());
    req.notes = Some("发现外包装破损".to_string());

    let result = repository
        .create_maintenance_record_with_audit(
            &context,
            req,
            Utc::now(),
            "m3-maintenance-abnormal-001",
            None,
        )
        .await
        .expect("abnormal maintenance should persist and quarantine");

    assert_eq!(result.value.conclusion, "abnormal");
    let status: String = sqlx::query_scalar(
        "SELECT quality_status FROM inventory_batches WHERE id = $1 AND owner_id = $2",
    )
    .bind(batch_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("batch status");
    assert_eq!(status, "quarantined");

    let status_change: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM inventory_status_changes
         WHERE owner_id = $1 AND batch_id = $2
           AND approval_source = '养护异常' AND to_status = 'quarantined'
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("status change count");
    assert_eq!(status_change, 1);

    let notify: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM h4_notification_records
         WHERE owner_id = $1 AND event_type = 'm3.maintenance.abnormal'
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("notification count");
    assert_eq!(notify, 1);
}
