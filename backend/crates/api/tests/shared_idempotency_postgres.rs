use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    task_type::{PgTaskTypeRepository, TaskTypeError},
};
use wms_domain::{TaskReleaseStrategy, UpsertTaskTypeRequest};

fn context(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "shared-idempotency-test".to_string(),
        permissions: vec!["mte.task_type.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn request() -> UpsertTaskTypeRequest {
    UpsertTaskTypeRequest {
        task_type_name: "共享幂等测试".to_string(),
        default_priority: 100,
        estimated_minutes: 10,
        mergeable: true,
        insertable: true,
        release_strategy: TaskReleaseStrategy::Immediate,
        release_interval_minutes: None,
        release_batch_size: None,
        enabled: true,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn shared_postgres_idempotency_replays_conflicts_and_expires(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '共享幂等测试货主')",
    )
    .bind(owner_id)
    .bind(format!("IDEM-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("owner should insert");

    let repository = PgTaskTypeRepository::new(pool.clone());
    let ctx = context(owner_id);
    let first = repository
        .upsert(
            &ctx,
            "shared_idempotency",
            request(),
            Utc::now(),
            "shared-key",
        )
        .await
        .expect("first mutation should persist");
    let replay = repository
        .upsert(
            &ctx,
            "SHARED_IDEMPOTENCY",
            request(),
            Utc::now(),
            "shared-key",
        )
        .await
        .expect("same identity should replay");
    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.id, replay.value.id);

    let conflict = repository
        .set_enabled(
            &ctx,
            "shared_idempotency",
            wms_domain::SetTaskTypeEnabledRequest { enabled: false },
            Utc::now(),
            "shared-key",
        )
        .await
        .expect_err("same key with another method/path must conflict");
    assert_eq!(conflict, TaskTypeError::IdempotencyConflict);

    let identity: (Uuid, String, String, chrono::DateTime<Utc>, chrono::DateTime<Utc>) =
        sqlx::query_as(
            "SELECT owner_id, method, path, expires_at, created_at FROM idempotency_request WHERE owner_id=$1 AND idempotency_key='shared-key'",
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("shared idempotency row should exist");
    assert_eq!(identity.0, owner_id);
    assert_eq!(identity.1, "PUT");
    assert!(identity.2.ends_with("/shared_idempotency"));
    assert!(identity.3 > identity.4 + chrono::Duration::hours(23));

    sqlx::query("UPDATE idempotency_request SET expires_at = now() - interval '1 second' WHERE owner_id=$1 AND idempotency_key='shared-key'")
        .bind(owner_id)
        .execute(&pool)
        .await
        .expect("expiry should be writable by test fixture");
    let rerun = repository
        .upsert(
            &ctx,
            "shared_idempotency",
            request(),
            Utc::now(),
            "shared-key",
        )
        .await
        .expect("expired identity should run again");
    assert!(!rerun.replayed);
}

#[sqlx::test(migrations = "../../migrations")]
async fn shared_postgres_idempotency_serializes_same_key_concurrently(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, '共享幂等并发测试货主')",
    )
    .bind(owner_id)
    .bind(format!("IDEM-CONCURRENT-{}", &owner_id.to_string()[..8]))
    .execute(&pool)
    .await
    .expect("owner should insert");

    let repository = PgTaskTypeRepository::new(pool.clone());
    let left_repository = repository.clone();
    let right_repository = repository.clone();
    let left_context = context(owner_id);
    let right_context = left_context.clone();
    let (left, right) = tokio::join!(
        left_repository.upsert(
            &left_context,
            "shared_idempotency_concurrent",
            request(),
            Utc::now(),
            "concurrent-key",
        ),
        right_repository.upsert(
            &right_context,
            "shared_idempotency_concurrent",
            request(),
            Utc::now(),
            "concurrent-key",
        ),
    );
    let left = left.expect("first concurrent mutation should persist");
    let right = right.expect("second concurrent mutation should replay");

    assert_ne!(left.replayed, right.replayed);
    assert_eq!(left.value.id, right.value.id);
    let task_type_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM task_types WHERE owner_id = $1 AND task_type_code = 'shared_idempotency_concurrent'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("concurrent mutation should create one task type");
    assert_eq!(task_type_count, 1);
    let idempotency_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'concurrent-key'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("concurrent mutation should persist one idempotency result");
    assert_eq!(idempotency_count, 1);
}
