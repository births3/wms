//! Phase 1 容器质量锁补充测试：lock_move 移库任务生成、L11 幂等重放、L3 并发行锁串行化。
//! 与 phase1_container_quality_lock_postgres.rs 分文件以控制单文件行数（check_page_size 门禁）。

use chrono::{DateTime, TimeZone, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{auth::AuthContext, lpn_container_repository::PgLpnContainerRepository};
use wms_domain::{
    ApplyContainerQualityLockRequest, ReleaseContainerQualityLockRequest,
    CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY, LPN_LOCK_CATEGORY_QUARANTINE,
};

#[path = "support/lpn_container.rs"]
mod lpn_support;
mod postgres_test_support;
use lpn_support::{seed_lpn_numbering, setup_container_in_use};
use postgres_test_support::ensure_audit_partition;

fn test_ctx(user_id: Uuid, owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id,
        owner_id,
        actor_name: "test-operator".to_string(),
        permissions: vec![
            "m1.master_data.read".to_string(),
            "m1.master_data.write".to_string(),
            "m1.quality-lock.manage".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 18, hour, 0, 0)
        .single()
        .expect("valid timestamp")
}

/// 质量锁移库测试种子：仓库 + 合格区存储位（容器所在位）+ 隔离区存储位（lock_move 目标位）。
/// 返回 (合格区库位 id, 合格区库位编码, 隔离区库位 id, 隔离区库位编码)。
#[allow(clippy::type_complexity)]
async fn seed_lock_move_locations(pool: &PgPool, owner_id: Uuid) -> (Uuid, String, Uuid, String) {
    let warehouse_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, status) VALUES ($1, $2, $3, '质量锁移库测试仓', 'normal', 'active') ON CONFLICT DO NOTHING",
    )
    .bind(warehouse_id)
    .bind(owner_id)
    .bind(format!("WH-QLM-{}", &owner_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("warehouse");

    let qualified_zone = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, 'ZONE-QLM-GREEN', '合格区', 'normal_10_30', 'qualified_green', 'active') ON CONFLICT DO NOTHING",
    )
    .bind(qualified_zone)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("qualified zone");

    let quarantine_zone = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO warehouse_zones (id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status) VALUES ($1, $2, $3, 'ZONE-QLM-YELLOW', '隔离区', 'normal_10_30', 'quarantine_yellow', 'active') ON CONFLICT DO NOTHING",
    )
    .bind(quarantine_zone)
    .bind(owner_id)
    .bind(warehouse_id)
    .execute(pool)
    .await
    .expect("quarantine zone");

    let qualified_loc = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3, max_sku_count
        ) VALUES ($1, $2, $3, $4, 'LOC-QLM-GREEN-01', 1, 1, 1, 'storage', true, 'occupied', 10000000, 0, 3)
        "#,
    )
    .bind(qualified_loc)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(qualified_zone)
    .execute(pool)
    .await
    .expect("qualified location");

    let quarantine_loc = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            location_type, allows_container, status, max_volume_cm3, used_volume_cm3, max_sku_count
        ) VALUES ($1, $2, $3, $4, 'LOC-QLM-YELLOW-01', 1, 1, 1, 'storage', true, 'available', 10000000, 0, 3)
        "#,
    )
    .bind(quarantine_loc)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(quarantine_zone)
    .execute(pool)
    .await
    .expect("quarantine location");

    (
        qualified_loc,
        "LOC-QLM-GREEN-01".to_string(),
        quarantine_loc,
        "LOC-QLM-YELLOW-01".to_string(),
    )
}

/// 容器下库存批次种子（qty_allocated 可配，用于释放/幂等联动断言），返回批次 id。
async fn seed_batch_on_container(
    pool: &PgPool,
    owner_id: Uuid,
    container_lpn: &str,
    location_id: Uuid,
    qty_allocated: i64,
) -> Uuid {
    let batch_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            location_id, location_code, container_lpn, qty_on_hand, qty_allocated, qty_frozen, status
        ) VALUES ($1, $2, 'PROD-QLM-01', 'BATCH-QLM-01', '2026-01-01', '2027-12-31',
                  $3, 'LOC-QLM-GREEN-01', $4, 100, $5, 0, 'qualified')
        "#,
    )
    .bind(batch_id)
    .bind(owner_id)
    .bind(location_id)
    .bind(container_lpn)
    .bind(qty_allocated)
    .execute(pool)
    .await
    .expect("batch");
    batch_id
}

/// 容器所在库位绑定（模拟已上架 / PDA 移库后的容器位置）。
async fn bind_container_location(pool: &PgPool, container_id: Uuid, location_id: Uuid) {
    sqlx::query("UPDATE lpn_containers SET location_id = $1 WHERE id = $2")
        .bind(location_id)
        .bind(container_id)
        .execute(pool)
        .await
        .expect("bind container location");
}

async fn count_quality_lock_events(pool: &PgPool, owner_id: Uuid, container_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM container_quality_lock_events WHERE owner_id = $1 AND container_id = $2",
    )
    .bind(owner_id)
    .bind(container_id)
    .fetch_one(pool)
    .await
    .expect("count lock events")
}

async fn count_lock_move_tasks(pool: &PgPool, owner_id: Uuid, reason_prefix: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_relocations WHERE owner_id = $1 AND reason LIKE $2",
    )
    .bind(owner_id)
    .bind(format!("{reason_prefix}%"))
    .fetch_one(pool)
    .await
    .expect("count lock move tasks")
}

/// 加锁同事务生成 lock_move 隔离移库任务（目标 = 锁类别对应质量区推荐位，from = 当前库位），
/// 解锁后生成 lock_move_back 移回合格区任务（目标 = 原库位）。
#[sqlx::test(migrations = "../../migrations")]
async fn test_lock_move_task_generated_on_lock_and_move_back_on_release(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let (qualified_loc, qualified_code, quarantine_loc, quarantine_code) =
        seed_lock_move_locations(&pool, owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "lock-move-test").await;
    // 模拟容器已上架在合格区库位
    bind_container_location(&pool, container.id, qualified_loc).await;
    let batch_id =
        seed_batch_on_container(&pool, owner_id, &container.lpn_code, qualified_loc, 0).await;

    let lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: Some("温控异常加锁".to_string()),
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
        create_liaison: false,
    };
    repo.quality_lock()
        .apply_quality_lock(&actor, container.id, lock_req, at(3), "lm-lock-1")
        .await
        .expect("apply quarantine lock");

    // lock_move 任务：from=当前库位、to=隔离区推荐位、单条、容器级占位约定
    let (
        from_code,
        to_code,
        reason,
        status,
        mode,
        lpn,
        product_code,
        batch_no,
        qty,
        task_batch_id,
    ): (String, String, String, String, String, String, String, String, i64, Uuid) =
        sqlx::query_as(
            r#"
            SELECT from_location_code, to_location_code, reason, status, relocation_mode,
                   lpn_code, product_code, batch_no, qty::BIGINT, batch_id
              FROM inventory_relocations
             WHERE owner_id = $1 AND reason LIKE 'lock_move:%'
            "#,
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("lock_move task");
    assert_eq!(from_code, qualified_code, "from 应为容器当前库位");
    assert_eq!(to_code, quarantine_code, "to 应为隔离区推荐存储位");
    assert_eq!(reason, "lock_move:quarantine:temp_anomaly");
    assert_eq!(status, "pending_supervisor");
    assert_eq!(mode, "lpn_full");
    assert_eq!(lpn, container.lpn_code);
    assert_eq!(product_code, "CONTAINER_LOCK");
    assert_eq!(batch_no, "LOCK_MOVE");
    assert_eq!(qty, 1);
    assert_eq!(task_batch_id, batch_id);
    assert_eq!(
        count_lock_move_tasks(&pool, owner_id, "lock_move:").await,
        1,
        "加锁只生成一条 lock_move 任务"
    );

    // 模拟 PDA 扫描移入隔离区
    bind_container_location(&pool, container.id, quarantine_loc).await;

    // 解锁 → lock_move_back 移回合格区任务（目标 = lock_move 的原库位）
    let rel_req = ReleaseContainerQualityLockRequest {
        witness_id,
        reason_desc: Some("隔离核验合格".to_string()),
        quality_liaison_id: None,
        note: None,
    };
    repo.quality_lock()
        .release_quality_lock(&actor, container.id, rel_req, at(5), "lm-rel-1")
        .await
        .expect("release lock");

    let (from_code, to_code, reason, status): (String, String, String, String) = sqlx::query_as(
        r#"
        SELECT from_location_code, to_location_code, reason, status
          FROM inventory_relocations
         WHERE owner_id = $1 AND reason LIKE 'lock_move_back:%'
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("lock_move_back task");
    assert_eq!(
        from_code, quarantine_code,
        "移回任务 from 应为当前隔离区库位"
    );
    assert_eq!(to_code, qualified_code, "移回任务 to 应为原合格区库位");
    assert_eq!(reason, "lock_move_back:qualified");
    assert_eq!(status, "pending_supervisor");
}

/// 未上架容器（location_id 为空）加锁同样生成 lock_move 任务，from 使用 STAGING 暂存占位。
#[sqlx::test(migrations = "../../migrations")]
async fn test_lock_move_task_for_not_stored_container_uses_staging_placeholder(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    // 仅隔离区目标位即可（未上架容器无当前库位）
    seed_lock_move_locations(&pool, owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "staging-lock-test").await;
    assert_eq!(container.location_id, None, "夹具容器应未上架");

    let lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: None,
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
        create_liaison: false,
    };
    repo.quality_lock()
        .apply_quality_lock(&actor, container.id, lock_req, at(3), "staging-lock-1")
        .await
        .expect("lock not-stored container");

    let (from_id, from_code, to_code): (Uuid, String, String) = sqlx::query_as(
        r#"
        SELECT from_location_id, from_location_code, to_location_code
          FROM inventory_relocations
         WHERE owner_id = $1 AND reason LIKE 'lock_move:%'
        "#,
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("lock_move task for not-stored container");
    assert_eq!(
        from_id,
        Uuid::nil(),
        "未上架容器 from_location_id 应为 nil UUID 占位"
    );
    assert_eq!(from_code, "STAGING", "未上架容器 from 应为 STAGING 占位");
    assert_eq!(to_code, "LOC-QLM-YELLOW-01");
    assert_eq!(
        count_lock_move_tasks(&pool, owner_id, "lock_move:").await,
        1
    );
}

/// L11：加锁/解锁接口 Idempotency-Key 重放不重复生成事件、不重复联动批次与移库任务；
/// 同键不同请求体返回 IdempotencyConflict。
#[sqlx::test(migrations = "../../migrations")]
async fn test_idempotency_replay_no_duplicate_events_or_batch_linkage(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let (qualified_loc, _qualified_code, _quarantine_loc, _quarantine_code) =
        seed_lock_move_locations(&pool, owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "idem-replay-test").await;
    bind_container_location(&pool, container.id, qualified_loc).await;
    let batch_id =
        seed_batch_on_container(&pool, owner_id, &container.lpn_code, qualified_loc, 25).await;

    let lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: Some("首次加锁".to_string()),
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
        create_liaison: false,
    };
    let first = repo
        .quality_lock()
        .apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            at(3),
            "replay-lock-1",
        )
        .await
        .expect("first lock");
    assert_eq!(
        count_quality_lock_events(&pool, owner_id, container.id).await,
        1
    );
    assert_eq!(
        count_lock_move_tasks(&pool, owner_id, "lock_move:").await,
        1
    );
    let outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_status_erp_feedback_outbox WHERE owner_id = $1 AND event_type = 'container_quality_locked'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("outbox count");
    assert_eq!(outbox_count, 1, "加锁应产生一条批次联动 outbox 事件");

    // 同一 Idempotency-Key 重放：返回已存结果，不重复产生任何副作用
    let replay = repo
        .quality_lock()
        .apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            at(4),
            "replay-lock-1",
        )
        .await
        .expect("replay lock");
    assert_eq!(replay.id, first.id, "重放应返回与首次相同的容器");
    assert_eq!(
        count_quality_lock_events(&pool, owner_id, container.id).await,
        1,
        "重放不得重复生成审计事件"
    );
    assert_eq!(
        count_lock_move_tasks(&pool, owner_id, "lock_move:").await,
        1,
        "重放不得重复生成移库任务"
    );
    let outbox_after_replay: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_status_erp_feedback_outbox WHERE owner_id = $1 AND event_type = 'container_quality_locked'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("outbox count after replay");
    assert_eq!(outbox_after_replay, 1, "重放不得重复联动批次/outbox");
    let (status, alloc): (String, i64) =
        sqlx::query_as("SELECT status, qty_allocated::BIGINT FROM inventory_batches WHERE id = $1")
            .bind(batch_id)
            .fetch_one(&pool)
            .await
            .expect("batch after replay");
    assert_eq!(status, "quarantined");
    assert_eq!(alloc, 0);

    // 同键不同请求体 → 幂等冲突
    let diff_req = ApplyContainerQualityLockRequest {
        reason_desc: Some("重放但换原因描述".to_string()),
        ..lock_req.clone()
    };
    let conflict = repo
        .quality_lock()
        .apply_quality_lock(&actor, container.id, diff_req, at(4), "replay-lock-1")
        .await
        .expect_err("same key with different request must conflict");
    assert_eq!(
        conflict,
        wms_api::lpn_container_repository::LpnContainerRepositoryError::IdempotencyConflict
    );

    // 解锁 + 重放：事件总数为 lock + release 两条，不重复
    let rel_req = ReleaseContainerQualityLockRequest {
        witness_id,
        reason_desc: Some("重放解锁".to_string()),
        quality_liaison_id: None,
        note: None,
    };
    repo.quality_lock()
        .release_quality_lock(&actor, container.id, rel_req.clone(), at(5), "replay-rel-1")
        .await
        .expect("release");
    repo.quality_lock()
        .release_quality_lock(&actor, container.id, rel_req, at(6), "replay-rel-1")
        .await
        .expect("release replay");
    let event_types: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM container_quality_lock_events WHERE owner_id = $1 AND container_id = $2 ORDER BY occurred_at",
    )
    .bind(owner_id)
    .bind(container.id)
    .fetch_all(&pool)
    .await
    .expect("event types after release replay");
    assert_eq!(
        event_types,
        vec!["lock".to_string(), "release".to_string()],
        "解锁重放不得重复生成 release 事件"
    );
}

/// L3：多线程同时请求同一容器加锁（同一 Idempotency-Key，客户端重试场景），
/// 幂等键/行锁串行化后仅一次真实加锁：单条事件、单条 lock_move 任务、单次批次联动。
#[sqlx::test(migrations = "../../migrations")]
async fn test_concurrent_same_key_lock_serialized_single_event_and_task(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let (qualified_loc, _qualified_code, _quarantine_loc, _quarantine_code) =
        seed_lock_move_locations(&pool, owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "concurrent-same-key").await;
    bind_container_location(&pool, container.id, qualified_loc).await;
    seed_batch_on_container(&pool, owner_id, &container.lpn_code, qualified_loc, 25).await;

    let lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: Some("并发加锁".to_string()),
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
        create_liaison: false,
    };
    let now = at(3);
    let ql = repo.quality_lock();
    let (r1, r2, r3, r4, r5, r6) = tokio::join!(
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "concurrent-key"
        ),
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "concurrent-key"
        ),
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "concurrent-key"
        ),
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "concurrent-key"
        ),
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "concurrent-key"
        ),
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "concurrent-key"
        ),
    );
    for (i, result) in [r1, r2, r3, r4, r5, r6].into_iter().enumerate() {
        assert!(result.is_ok(), "并发请求 {i} 应全部成功（串行化 + 重放）");
    }

    assert_eq!(
        count_quality_lock_events(&pool, owner_id, container.id).await,
        1,
        "并发重放只允许生成一条 lock 事件"
    );
    assert_eq!(
        count_lock_move_tasks(&pool, owner_id, "lock_move:").await,
        1,
        "并发重放只允许生成一条 lock_move 任务"
    );
    let outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_status_erp_feedback_outbox WHERE owner_id = $1 AND event_type = 'container_quality_locked'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("outbox count");
    assert_eq!(outbox_count, 1, "并发重放只允许联动批次一次");
    let idem_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = 'concurrent-key'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("idempotency count");
    assert_eq!(idem_count, 1);
}

/// L3：多线程不同 Idempotency-Key 同时请求同一容器加锁，
/// FOR UPDATE 行锁串行化后全部成功且互不覆盖（每次请求一条独立事件，无死锁/冲突）。
#[sqlx::test(migrations = "../../migrations")]
async fn test_concurrent_distinct_keys_serialized_by_for_update_row_lock(pool: PgPool) {
    ensure_audit_partition(&pool, at(0)).await;
    let owner_id = Uuid::new_v4();
    let operator_id = Uuid::new_v4();
    let witness_id = Uuid::new_v4();
    seed_lpn_numbering(&pool, at(0), owner_id).await;
    let (qualified_loc, _qualified_code, _quarantine_loc, _quarantine_code) =
        seed_lock_move_locations(&pool, owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let actor = test_ctx(operator_id, owner_id);
    let container = setup_container_in_use(&repo, &actor, "concurrent-distinct").await;
    bind_container_location(&pool, container.id, qualified_loc).await;
    seed_batch_on_container(&pool, owner_id, &container.lpn_code, qualified_loc, 25).await;

    let lock_req = ApplyContainerQualityLockRequest {
        lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
        reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
        reason_desc: None,
        evidence_urls: vec![],
        quality_liaison_id: None,
        witness_id,
        note: None,
        create_liaison: false,
    };
    let now = at(3);
    let ql = repo.quality_lock();
    let (r1, r2, r3, r4) = tokio::join!(
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "distinct-key-1"
        ),
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "distinct-key-2"
        ),
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "distinct-key-3"
        ),
        ql.apply_quality_lock(
            &actor,
            container.id,
            lock_req.clone(),
            now,
            "distinct-key-4"
        ),
    );
    for (i, result) in [r1, r2, r3, r4].into_iter().enumerate() {
        assert!(result.is_ok(), "行锁串行化后并发请求 {i} 不应失败");
    }
    // 每次请求都是一次独立事务：各自产生一条 lock 事件与一条 lock_move 任务，最终状态一致
    assert_eq!(
        count_quality_lock_events(&pool, owner_id, container.id).await,
        4
    );
    assert_eq!(
        count_lock_move_tasks(&pool, owner_id, "lock_move:").await,
        4
    );
    let category: Option<String> =
        sqlx::query_scalar("SELECT current_lock_category FROM lpn_containers WHERE id = $1")
            .bind(container.id)
            .fetch_one(&pool)
            .await
            .expect("container lock category");
    assert_eq!(category.as_deref(), Some(LPN_LOCK_CATEGORY_QUARANTINE));
}
