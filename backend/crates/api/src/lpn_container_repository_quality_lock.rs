use super::*;

pub(crate) const QUALITY_LOCK_PATH: &str = "/api/v1/master-data/lpn-containers/{id}/quality-lock";
pub(crate) const QUALITY_LOCK_RELEASE_PATH: &str =
    "/api/v1/master-data/lpn-containers/{id}/quality-lock/release";

/// 移库任务 `lock_move` 标记（落点：inventory_relocations.reason 前缀，
/// relocation_mode 值域 CHECK 不可扩展，标记与普通移库以 reason 前缀区分）。
pub const LOCK_MOVE_MARKER: &str = "lock_move:";
/// 解锁后移回合格区任务的标记前缀。
pub const LOCK_MOVE_BACK_MARKER: &str = "lock_move_back:";
/// 未上架容器的暂存占位库位编码（from_location_id 用 nil UUID，表中无外键约束）。
pub const LOCK_MOVE_STAGING_CODE: &str = "STAGING";
pub use wms_domain::{batch_status_for_lock_category, quality_color_for_lock_category};

/// 主档 FOR UPDATE 锁定读取（加锁 / 换原因 / 解锁共用）。
pub(crate) async fn lock_container_row_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<LpnContainerRow, LpnContainerRepositoryError> {
    sqlx::query_as::<_, LpnContainerRow>(
        r#"
        SELECT id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
               current_lock_category, current_lock_reason_item_code, created_at, updated_at
          FROM lpn_containers
         WHERE id = $1 AND owner_id = $2
         FOR UPDATE
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(LpnContainerRepositoryError::NotFound)
}

/// 锁类别对应质量区 ∩ 存储位（allows_container=true）的推荐空闲位。
/// 无可用目标位时返回 None（不生成任务，由运营配置质量区后人工移库）。
async fn recommend_qualified_move_back_target(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    quality_color: &str,
) -> Result<Option<(Uuid, String)>, LpnContainerRepositoryError> {
    sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT location.id, location.location_code
          FROM warehouse_locations location
          JOIN warehouse_zones zone
            ON zone.id = location.zone_id
           AND zone.owner_id = location.owner_id
         WHERE location.owner_id = $1
           AND zone.quality_color = $2
           AND zone.status = 'active'
           AND location.location_type = 'storage'
           AND location.allows_container = TRUE
           AND location.status IN ('available', 'occupied')
           AND location.lock_status IN ('normal', 'lock_out')
           AND (location.current_owner_id IS NULL OR location.current_owner_id = $1)
         ORDER BY location.used_volume_cm3 ASC, location.location_code
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(quality_color)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)
}

/// 库存流水（qty_delta=0，质量锁不改变数量）：movement_type 取 quality_lock / quality_lock_change / quality_lock_release。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn append_quality_lock_movement(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    container_id: Uuid,
    batch_id: Uuid,
    movement_type: &str,
    lpn_code: &str,
    operator_user_id: Uuid,
    operator_name: &str,
    approval_id: Option<Uuid>,
    occurred_at: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, approval_source,
            approval_id, lpn_code, operator_user_id, operator_name, occurred_at
        ) VALUES (
            $1, $2, $3, $4, 0,
            'container_quality_lock', $5, 'M-QL',
            $6, $7, $8, $9, $10
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(movement_type)
    .bind(container_id)
    .bind(approval_id)
    .bind(lpn_code)
    .bind(operator_user_id)
    .bind(operator_name)
    .bind(occurred_at)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// 批次状态联动的前后值记录（inventory_status_changes，纯 INSERT 审计）。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn append_quality_lock_status_change(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    from_status: &str,
    to_status: &str,
    reason: &str,
    approval_source: &str,
    approval_id: &str,
    occurred_at: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    sqlx::query(
        r#"
        INSERT INTO inventory_status_changes (
            id, owner_id, batch_id, from_status, to_status,
            reason, approval_source, approval_id, occurred_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(from_status)
    .bind(to_status)
    .bind(reason)
    .bind(approval_source)
    .bind(approval_id)
    .bind(occurred_at)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// 容器当前库位信息（location_id + code），未上架容器返回暂存占位。
async fn container_current_location(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    location_id: Option<Uuid>,
) -> Result<(Uuid, String), LpnContainerRepositoryError> {
    let Some(loc_id) = location_id else {
        return Ok((Uuid::nil(), LOCK_MOVE_STAGING_CODE.to_string()));
    };
    let code: Option<String> = sqlx::query_scalar(
        "SELECT location_code FROM warehouse_locations WHERE id = $1 AND owner_id = $2",
    )
    .bind(loc_id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(match code {
        Some(code) => (loc_id, code),
        None => (Uuid::nil(), LOCK_MOVE_STAGING_CODE.to_string()),
    })
}

/// 同事务生成隔离移库任务（lock_move）：目标 = 锁类别对应质量区推荐位。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_lock_move_task(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    lpn_code: &str,
    lock_category: &str,
    reason_dict_item_code: &str,
    first_batch_id: Option<Uuid>,
    current_location_id: Option<Uuid>,
    created_by: Uuid,
    now: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    let Some((to_location_id, to_location_code)) = recommend_qualified_move_back_target(
        tx,
        owner_id,
        quality_color_for_lock_category(lock_category),
    )
    .await?
    else {
        // 加锁已生效；无对应质量区空位时允许暂缓移库，不回滚整单。
        return Ok(());
    };
    let (from_location_id, from_location_code) =
        container_current_location(tx, owner_id, current_location_id).await?;
    let batch_status = batch_status_for_lock_category(lock_category);
    sqlx::query(
        r#"
        INSERT INTO inventory_relocations (
            id, owner_id, batch_id, product_code, batch_no, qty,
            from_location_id, from_location_code, to_location_id, to_location_code,
            relocation_mode, lpn_code, quality_status, status, reason,
            created_by, created_at, updated_at
        ) VALUES (
            gen_random_uuid(), $1, $2, 'CONTAINER_LOCK', 'LOCK_MOVE', 1,
            $3, $4, $5, $6,
            'lpn_full', $7, $8, 'pending_supervisor', $9,
            $10, $11, $11
        )
        "#,
    )
    .bind(owner_id)
    .bind(first_batch_id.unwrap_or_else(Uuid::nil))
    .bind(from_location_id)
    .bind(&from_location_code)
    .bind(to_location_id)
    .bind(&to_location_code)
    .bind(lpn_code)
    .bind(batch_status)
    .bind(format!(
        "{LOCK_MOVE_MARKER}{lock_category}:{reason_dict_item_code}"
    ))
    .bind(created_by)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// 解锁后生成移回合格区任务（lock_move_back）：目标 = 原库位或系统推荐合格位；
/// 容器无当前库位（未上架）或已在合格区时不生成。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_lock_move_back_task(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    lpn_code: &str,
    current_location_id: Option<Uuid>,
    first_batch_id: Option<Uuid>,
    created_by: Uuid,
    now: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    let Some(loc_id) = current_location_id else {
        return Ok(());
    };
    // 已在合格区则无需移回。
    let current_quality_color: Option<String> = sqlx::query_scalar(
        r#"
        SELECT zone.quality_color
          FROM warehouse_locations location
          JOIN warehouse_zones zone
            ON zone.id = location.zone_id
           AND zone.owner_id = location.owner_id
         WHERE location.id = $1 AND location.owner_id = $2
        "#,
    )
    .bind(loc_id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if current_quality_color.as_deref() == Some("qualified_green") {
        return Ok(());
    }
    let (from_location_id, from_location_code) =
        container_current_location(tx, owner_id, current_location_id).await?;

    // 优先移回原库位（lock_move 任务的 from，非暂存占位）。
    let original: Option<(Option<Uuid>, String)> = sqlx::query_as(
        r#"
        SELECT from_location_id, from_location_code
          FROM inventory_relocations
         WHERE owner_id = $1 AND lpn_code = $2
           AND reason LIKE $3
         ORDER BY created_at DESC
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(lpn_code)
    .bind(format!("{LOCK_MOVE_MARKER}%"))
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let target = match original {
        Some((Some(orig_id), orig_code)) if orig_code != LOCK_MOVE_STAGING_CODE => {
            Some((orig_id, orig_code))
        }
        _ => recommend_qualified_move_back_target(tx, owner_id, "qualified_green").await?,
    };
    let Some((to_location_id, to_location_code)) = target else {
        return Ok(());
    };
    sqlx::query(
        r#"
        INSERT INTO inventory_relocations (
            id, owner_id, batch_id, product_code, batch_no, qty,
            from_location_id, from_location_code, to_location_id, to_location_code,
            relocation_mode, lpn_code, quality_status, status, reason,
            created_by, created_at, updated_at
        ) VALUES (
            gen_random_uuid(), $1, $2, 'CONTAINER_LOCK', 'LOCK_MOVE', 1,
            $3, $4, $5, $6,
            'lpn_full', $7, 'qualified', 'pending_supervisor', $8,
            $9, $10, $10
        )
        "#,
    )
    .bind(owner_id)
    .bind(first_batch_id.unwrap_or_else(Uuid::nil))
    .bind(from_location_id)
    .bind(&from_location_code)
    .bind(to_location_id)
    .bind(&to_location_code)
    .bind(lpn_code)
    .bind(format!("{LOCK_MOVE_BACK_MARKER}qualified"))
    .bind(created_by)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

/// 批次下已分配量明细（outbox 订单行引用的来源）。
#[derive(FromRow)]
struct BatchAllocationRefRow {
    outbound_order_id: Uuid,
    line_no: i32,
    allocated_qty: i64,
}

/// 释放批次已分配量并发出波次重算 outbox 事件：
/// 事件 payload 携带 outbound_order_id + line_no + 释放数量，供波次侧精准回退重算；
/// 订单行"等待重新分配"由波次消费端据此重算（outbound_order_lines 无分配状态列，最小改动落点=事件携带订单行引用）。
pub(crate) async fn release_batch_allocations_with_outbox(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    container_id: Uuid,
    lpn_code: &str,
    lock_category: &str,
    reason_dict_item_code: &str,
    now: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    let refs = sqlx::query_as::<_, BatchAllocationRefRow>(
        r#"
        SELECT outbound_order_id, line_no, allocated_qty::BIGINT
          FROM inventory_allocations
         WHERE owner_id = $1 AND batch_id = $2 AND status = 'locked'
         ORDER BY outbound_order_id, line_no
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let released_total: i64 = refs.iter().map(|row| row.allocated_qty).sum();
    if !refs.is_empty() {
        sqlx::query(
            r#"
            DELETE FROM inventory_allocations
             WHERE owner_id = $1 AND batch_id = $2 AND status = 'locked'
            "#,
        )
        .bind(owner_id)
        .bind(batch_id)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }

    let base_payload = json!({
        "container_id": container_id,
        "lpn_code": lpn_code,
        "lock_category": lock_category,
        "reason_dict_item_code": reason_dict_item_code,
        "batch_id": batch_id,
        "released_allocated_qty": released_total,
    });
    // 每个 (订单, 行) 一条事件，携带订单行引用与释放数量。
    for (outbound_order_id, line_no, qty) in refs
        .iter()
        .map(|row| (row.outbound_order_id, row.line_no, row.allocated_qty))
    {
        let mut payload = base_payload.clone();
        payload["outbound_order_id"] = json!(outbound_order_id);
        payload["line_no"] = json!(line_no);
        payload["released_qty"] = json!(qty);
        sqlx::query(
            r#"
            INSERT INTO inventory_status_erp_feedback_outbox (
                id, owner_id, batch_id, status_change_id, event_type,
                payload, status, attempt_count, next_attempt_at, created_at, updated_at
            ) VALUES (
                gen_random_uuid(), $1, $2, gen_random_uuid(), 'container_quality_locked',
                $3, 'pending', 0, $4, $4, $4
            )
            "#,
        )
        .bind(owner_id)
        .bind(batch_id)
        .bind(payload)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    if refs.is_empty() {
        // 无分配明细：仍发批次级事件（供波次侧按批次感知锁状态）。
        sqlx::query(
            r#"
            INSERT INTO inventory_status_erp_feedback_outbox (
                id, owner_id, batch_id, status_change_id, event_type,
                payload, status, attempt_count, next_attempt_at, created_at, updated_at
            ) VALUES (
                gen_random_uuid(), $1, $2, gen_random_uuid(), 'container_quality_locked',
                $3, 'pending', 0, $4, $4, $4
            )
            "#,
        )
        .bind(owner_id)
        .bind(batch_id)
        .bind(base_payload)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

pub(crate) async fn create_liaison_for_lock(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    lpn_code: &str,
    now: DateTime<Utc>,
) -> Result<Uuid, LpnContainerRepositoryError> {
    sqlx::query(
        r#"
        INSERT INTO quality_liaison_types (
            id, owner_id, type_code, type_name, approval_template_id, approver_user_id,
            timeout_seconds, enabled, created_by, created_at, updated_at
        ) VALUES (
            $1, $2, 'container_quality_lock', '容器质量锁', 'container_quality_lock', $3,
            86400, true, $3, $4, $4
        )
        ON CONFLICT (owner_id, type_code) DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(ctx.owner_id)
    .bind(ctx.user_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let type_code = "container_quality_lock";
    let liaison_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO quality_liaison_orders (
            id, owner_id, liaison_no, type_code, related_document_type, related_document_no,
            problem_description, disposition_suggestion, trigger_source, business_payload,
            status, created_by, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, 'container_quality_lock', $5,
            $6, '按锁类别移入对应质量区并跟踪处置', 'container_quality_lock', $7::jsonb,
            'pending_approval', $8, $9, $9
        )
        "#,
    )
    .bind(liaison_id)
    .bind(ctx.owner_id)
    .bind(format!("MQL-LOCK-{}", &liaison_id.to_string()[..8]))
    .bind(type_code)
    .bind(lpn_code)
    .bind(format!("容器 {lpn_code} 质量锁"))
    .bind(serde_json::json!({ "lpn_code": lpn_code }))
    .bind(ctx.user_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(liaison_id)
}

pub(crate) async fn bind_liaison_to_container(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    liaison_id: Uuid,
    lpn_code: &str,
    now: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    sqlx::query(
        r#"
        UPDATE quality_liaison_orders
           SET related_document_type = 'container_quality_lock',
               related_document_no = $3,
               updated_at = $4
         WHERE id = $1 AND owner_id = $2
        "#,
    )
    .bind(liaison_id)
    .bind(owner_id)
    .bind(lpn_code)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

pub(crate) async fn classify_unlock_batches(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    lpn_code: &str,
    expected_status: &str,
    container_id: Uuid,
) -> Result<(Vec<(Uuid, String)>, Vec<UnlockSkippedBatch>), LpnContainerRepositoryError> {
    let batches = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT id, status
          FROM inventory_batches
         WHERE owner_id = $1 AND container_lpn = $2
           AND status NOT IN ('loss_deducted', 'pending_destruction')
        "#,
    )
    .bind(owner_id)
    .bind(lpn_code)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let mut rewrite = Vec::new();
    let mut skipped = Vec::new();
    for (batch_id, status) in batches {
        let last_is_this_lock: bool = sqlx::query_scalar(
            r#"
            SELECT COALESCE(
                (SELECT approval_source = 'M-QL' AND to_status = $3 AND approval_id = $4
                   FROM inventory_status_changes
                  WHERE owner_id = $1 AND batch_id = $2
                  ORDER BY occurred_at DESC
                  LIMIT 1),
                false
            )
            "#,
        )
        .bind(owner_id)
        .bind(batch_id)
        .bind(expected_status)
        .bind(container_id.to_string())
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
        if unlock_should_rewrite_batch(&status, expected_status, last_is_this_lock) {
            rewrite.push((batch_id, status));
        } else {
            skipped.push(UnlockSkippedBatch {
                batch_id,
                status,
                reason: if last_is_this_lock {
                    "status_changed_by_other_flow".to_string()
                } else {
                    "not_set_by_this_lock".to_string()
                },
            });
        }
    }
    Ok((rewrite, skipped))
}

impl PgLpnContainerRepository {
    pub async fn list_lock_move_owner_ids(&self) -> Result<Vec<Uuid>, LpnContainerRepositoryError> {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT owner_id
              FROM inventory_relocations
             WHERE reason LIKE $1
             ORDER BY owner_id
            "#,
        )
        .bind(format!("{LOCK_MOVE_MARKER}%"))
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)
    }

    pub async fn insert_overdue_lock_move_alert(
        &self,
        owner_id: Uuid,
        item: &OverdueLockMove,
        threshold_hours: i64,
        now: DateTime<Utc>,
    ) -> Result<bool, LpnContainerRepositoryError> {
        let content = format!(
            "容器 {} 加锁隔离移库任务超过 {} 小时未完成（任务生成于 {}），请尽快将实物移入对应质量区；暂缓期间容器禁止一切作业。",
            item.lpn_code,
            threshold_hours,
            item.task_created_at.format("%Y-%m-%d %H:%M"),
        );
        let dedupe_key = format!("container:{}", item.container_id);
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        let result = sqlx::query(
            r#"
            INSERT INTO h4_notification_records (
                id, owner_id, config_id, event_type, dedupe_key, recipient, channel,
                content, content_summary, status, retry_count, failure_reason, sent_at,
                created_at, updated_at
            ) VALUES (
                $1, $2, NULL, 'm1.quality_lock.overdue', $3, 'warehouse_manager', 'wechat',
                $4, $5, 'retrying', 0, 'awaiting_wechat_delivery', NULL, $6, $6
            )
            ON CONFLICT (owner_id, event_type, recipient, dedupe_key) DO UPDATE
               SET content = EXCLUDED.content,
                   content_summary = EXCLUDED.content_summary,
                   updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(&dedupe_key)
        .bind(&content)
        .bind(&content)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let inserted = result.rows_affected() == 1;
        if inserted {
            let audit = AuditWriteRequest {
                occurred_at: now,
                actor_id: Uuid::nil(),
                actor_name: "system-quality-lock-overdue".to_string(),
                owner_id,
                jti: format!("m1-quality-lock-overdue:{}", item.container_id),
                action: "alert_quality_lock_move_overdue".to_string(),
                module: "M1".to_string(),
                resource_type: "lpn_container".to_string(),
                resource_id: item.container_id.to_string(),
                diff: None,
                request_id: None,
                ip: None,
                user_agent: Some("wms-quality-lock-overdue-job".to_string()),
            };
            crate::audit::append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| LpnContainerRepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(inserted)
    }

    /// 扫描 lock_move 未完成超过阈值的容器。
    pub async fn scan_overdue_lock_moves(
        &self,
        owner_id: Uuid,
        threshold_hours: i64,
        now: DateTime<Utc>,
    ) -> Result<Vec<OverdueLockMove>, LpnContainerRepositoryError> {
        let threshold_time = now - chrono::Duration::hours(threshold_hours);
        let rows = sqlx::query_as::<_, (Uuid, String, DateTime<Utc>)>(
            r#"
            SELECT c.id, c.lpn_code, MAX(r.created_at)
              FROM inventory_relocations r
              JOIN lpn_containers c
                ON c.owner_id = r.owner_id
               AND c.lpn_code = r.lpn_code
             WHERE r.owner_id = $1
               AND r.reason LIKE $2
               AND r.status <> 'completed'
               AND r.created_at <= $3
             GROUP BY c.id, c.lpn_code
             ORDER BY MAX(r.created_at) ASC
            "#,
        )
        .bind(owner_id)
        .bind(format!("{LOCK_MOVE_MARKER}%"))
        .bind(threshold_time)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        let mut overdue = Vec::new();
        for (container_id, lpn_code, task_created_at) in rows {
            let lock_category: Option<String> = sqlx::query_scalar(
                "SELECT current_lock_category FROM lpn_containers WHERE id = $1 AND owner_id = $2",
            )
            .bind(container_id)
            .bind(owner_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .flatten();
            let Some(category) = lock_category else {
                continue;
            };
            if category != LPN_LOCK_CATEGORY_QUARANTINE && category != LPN_LOCK_CATEGORY_REJECTED {
                continue;
            }
            let expected_color = quality_color_for_lock_category(&category);
            let current_matches: bool = sqlx::query_scalar(
                r#"
                SELECT EXISTS (
                    SELECT 1
                      FROM lpn_containers container
                      JOIN warehouse_locations location
                        ON location.id = container.location_id
                       AND location.owner_id = container.owner_id
                      JOIN warehouse_zones zone
                        ON zone.id = location.zone_id
                       AND zone.owner_id = location.owner_id
                     WHERE container.id = $1
                       AND container.owner_id = $2
                       AND zone.quality_color = $3
                )
                "#,
            )
            .bind(container_id)
            .bind(owner_id)
            .bind(expected_color)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;
            if !current_matches {
                overdue.push(OverdueLockMove {
                    container_id,
                    lpn_code,
                    task_created_at,
                });
            }
        }
        Ok(overdue)
    }
}
