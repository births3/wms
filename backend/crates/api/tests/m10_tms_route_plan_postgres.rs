use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    tms_plus::{ReceiveTmsRoutePlanRequest, TmsRouteStopRequest},
    wave5_repository::{PgWave5Repository, Wave5RepositoryError},
};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m10-route-plan-postgres-test".to_string(),
        permissions: vec!["m10.write".to_string()],
        jti: Uuid::new_v4().to_string(),
    }
}

async fn seed_owner_driver(pool: &PgPool, owner_id: Uuid, driver_id: Uuid) {
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $3)")
        .bind(owner_id)
        .bind(format!("M10-OWNER-{}", owner_id.simple()))
        .bind("M10 测试货主")
        .execute(pool)
        .await
        .expect("seed TMS owner");
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash)
        VALUES ($1, $2, 'M10 测试司机', 'test-only-hash')
        "#,
    )
    .bind(driver_id)
    .bind(format!("m10-driver-{}", driver_id.simple()))
    .execute(pool)
    .await
    .expect("seed TMS driver");
    sqlx::query("INSERT INTO auth_user_owner_bindings (user_id, owner_id) VALUES ($1, $2)")
        .bind(driver_id)
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("bind TMS driver to owner");
}

async fn seed_outbound_order(pool: &PgPool, owner_id: Uuid, now: chrono::DateTime<Utc>) -> Uuid {
    let order_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO outbound_orders (
            id, owner_id, wms_order_no, customer_id, warehouse_id,
            status, short_pick, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, 'reviewed', FALSE, $6, $6)
        "#,
    )
    .bind(order_id)
    .bind(owner_id)
    .bind(format!("M10-ORDER-{}", order_id.simple()))
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("seed TMS outbound order");
    order_id
}

fn request(
    now: chrono::DateTime<Utc>,
    driver_id: Uuid,
    order_ids: Vec<Uuid>,
) -> ReceiveTmsRoutePlanRequest {
    ReceiveTmsRoutePlanRequest {
        dispatch_result_id: "TMS-ROUTE-RESULT-001".to_string(),
        delivery_date: now.date_naive(),
        vehicle_no: "VH-001".to_string(),
        plate_no: "沪A12345".to_string(),
        driver_user_id: driver_id,
        version: 3,
        outbound_order_ids: order_ids.clone(),
        stops: vec![
            TmsRouteStopRequest {
                store_id: Uuid::new_v4(),
                sequence: 1,
                estimated_arrival_at: now + Duration::minutes(30),
                outbound_order_ids: vec![order_ids[0]],
            },
            TmsRouteStopRequest {
                store_id: Uuid::new_v4(),
                sequence: 2,
                estimated_arrival_at: now + Duration::minutes(60),
                outbound_order_ids: vec![order_ids[1]],
            },
        ],
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn receives_route_plan_with_owner_scope_idempotency_and_append_only_audit(pool: PgPool) {
    let now = Utc::now();
    let owner_id = Uuid::new_v4();
    let driver_id = Uuid::new_v4();
    seed_owner_driver(&pool, owner_id, driver_id).await;
    let order_ids = vec![
        seed_outbound_order(&pool, owner_id, now).await,
        seed_outbound_order(&pool, owner_id, now).await,
    ];
    let repo = PgWave5Repository::new(pool.clone());
    let context = ctx(owner_id);
    let route_request = request(now, driver_id, order_ids.clone());

    let first = repo
        .receive_tms_route_plan(
            &context,
            route_request.clone(),
            now,
            "m10-route-key-1",
            None,
        )
        .await
        .expect("receive route plan")
        .value;
    assert_eq!(first.status, "received");
    assert_eq!(first.version, 3);
    assert_eq!(first.outbound_order_ids, order_ids);
    assert_eq!(first.stops.len(), 2);

    let same_key = repo
        .receive_tms_route_plan(
            &context,
            route_request.clone(),
            now,
            "m10-route-key-1",
            None,
        )
        .await
        .expect("same idempotency key should replay");
    assert!(same_key.replayed);
    assert_eq!(same_key.value.id, first.id);

    let same_result_id = repo
        .receive_tms_route_plan(&context, route_request, now, "m10-route-key-2", None)
        .await
        .expect("same TMS result should be deduplicated");
    assert!(same_result_id.replayed);
    assert_eq!(same_result_id.value.id, first.id);

    let mut changed_result = request(now, driver_id, first.outbound_order_ids.clone());
    changed_result.version = 4;
    assert_eq!(
        repo.receive_tms_route_plan(&context, changed_result, now, "m10-route-key-3", None,)
            .await
            .expect_err("same TMS result with changed payload must conflict"),
        Wave5RepositoryError::IdempotencyConflict
    );

    let route_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM tms_route_plans WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("count route plans");
    let stop_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM tms_route_stops WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("count route stops");
    let order_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM tms_route_orders WHERE owner_id = $1")
            .bind(owner_id)
            .fetch_one(&pool)
            .await
            .expect("count route orders");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_event WHERE owner_id = $1 AND action = 'receive_tms_route_plan'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("count route audit events");
    assert_eq!(route_count, 1);
    assert_eq!(stop_count, 2);
    assert_eq!(order_count, 2);
    assert_eq!(audit_count, 1, "replays must not duplicate audit events");
}

#[sqlx::test(migrations = "../../migrations")]
async fn rejects_missing_owner_order_and_idempotency_payload_conflict(pool: PgPool) {
    let now = Utc::now();
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let driver_a = Uuid::new_v4();
    seed_owner_driver(&pool, owner_a, driver_a).await;
    seed_owner_driver(&pool, owner_b, Uuid::new_v4()).await;
    let order_a = seed_outbound_order(&pool, owner_a, now).await;
    let repo = PgWave5Repository::new(pool.clone());
    let context_a = ctx(owner_a);
    let context_b = ctx(owner_b);
    let route_request = request(now, driver_a, vec![order_a, Uuid::new_v4()]);

    assert_eq!(
        repo.receive_tms_route_plan(
            &context_b,
            route_request.clone(),
            now,
            "m10-cross-owner-key",
            None,
        )
        .await
        .expect_err("cross-owner order must be rejected"),
        Wave5RepositoryError::NotFound
    );

    let owner_order_ids = vec![order_a, seed_outbound_order(&pool, owner_a, now).await];
    let valid_request = request(now, driver_a, owner_order_ids);
    repo.receive_tms_route_plan(
        &context_a,
        valid_request.clone(),
        now,
        "m10-route-conflict-key",
        None,
    )
    .await
    .expect("first route result");
    let mut changed_request = valid_request;
    changed_request.version = 4;
    assert_eq!(
        repo.receive_tms_route_plan(
            &context_a,
            changed_request,
            now,
            "m10-route-conflict-key",
            None,
        )
        .await
        .expect_err("same key with changed payload must conflict"),
        Wave5RepositoryError::IdempotencyConflict
    );
}
