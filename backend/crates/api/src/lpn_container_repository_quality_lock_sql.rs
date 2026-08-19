pub(crate) async fn quality_liaison_exists(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    liaison_id: Uuid,
) -> Result<bool, LpnContainerRepositoryError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM quality_liaison_orders WHERE id = $1 AND owner_id = $2)",
    )
    .bind(liaison_id)
    .bind(owner_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub(crate) async fn update_container_lock_fields(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
    lock_category: &str,
    reason_dict_item_code: &str,
    now: DateTime<Utc>,
) -> Result<LpnContainer, LpnContainerRepositoryError> {
    let row = sqlx::query_as::<_, LpnContainerRow>(
        r#"
        UPDATE lpn_containers
           SET current_lock_category = $3,
               current_lock_reason_item_code = $4,
               updated_at = $5
         WHERE id = $1 AND owner_id = $2
        RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                  current_lock_category, current_lock_reason_item_code, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(lock_category)
    .bind(reason_dict_item_code)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_write_error)?;
    Ok(row.into())
}

pub(crate) async fn clear_container_lock_fields(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
    now: DateTime<Utc>,
) -> Result<LpnContainer, LpnContainerRepositoryError> {
    let row = sqlx::query_as::<_, LpnContainerRow>(
        r#"
        UPDATE lpn_containers
           SET current_lock_category = $4,
               current_lock_reason_item_code = NULL,
               updated_at = $3
         WHERE id = $1 AND owner_id = $2
        RETURNING id, owner_id, lpn_code, container_type, capacity_cm3, status, location_id,
                  current_lock_category, current_lock_reason_item_code, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(now)
    .bind(LPN_LOCK_CATEGORY_QUALIFIED)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_write_error)?;
    Ok(row.into())
}

pub(crate) async fn list_container_batches_for_lock(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    lpn_code: &str,
) -> Result<Vec<(Uuid, i64, String)>, LpnContainerRepositoryError> {
    sqlx::query_as::<_, (Uuid, i64, String)>(
        r#"
        SELECT id, qty_allocated::BIGINT, status
          FROM inventory_batches
         WHERE owner_id = $1 AND container_lpn = $2
           AND status NOT IN ('loss_deducted', 'pending_destruction')
        "#,
    )
    .bind(owner_id)
    .bind(lpn_code)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

pub(crate) async fn list_container_batch_statuses(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    lpn_code: &str,
) -> Result<Vec<(Uuid, String)>, LpnContainerRepositoryError> {
    sqlx::query_as::<_, (Uuid, String)>(
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
    .map_err(map_db_error)
}

pub(crate) async fn update_batch_lock_status(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    status: &str,
    now: DateTime<Utc>,
    clear_allocated: bool,
) -> Result<(), LpnContainerRepositoryError> {
    if clear_allocated {
        sqlx::query(
            r#"
            UPDATE inventory_batches
               SET status = $3,
                   qty_allocated = 0,
                   updated_at = $4
             WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(batch_id)
        .bind(owner_id)
        .bind(status)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
        return Ok(());
    }
    sqlx::query(
        r#"
        UPDATE inventory_batches
           SET status = $3,
               updated_at = $4
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .bind(status)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

pub(crate) async fn rewrite_batch_qualified(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    expected_status: &str,
    now: DateTime<Utc>,
) -> Result<(), LpnContainerRepositoryError> {
    sqlx::query(
        r#"
        UPDATE inventory_batches
           SET status = $4,
               updated_at = $5
         WHERE owner_id = $1 AND id = $2 AND status = $3
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .bind(expected_status)
    .bind(STATUS_QUALIFIED)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

pub(crate) async fn latest_container_liaison_id(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    container_id: Uuid,
) -> Result<Option<Uuid>, LpnContainerRepositoryError> {
    Ok(sqlx::query_scalar::<_, Option<Uuid>>(
        r#"
        SELECT quality_liaison_id
          FROM container_quality_lock_events
         WHERE owner_id = $1 AND container_id = $2 AND quality_liaison_id IS NOT NULL
         ORDER BY occurred_at DESC
         LIMIT 1
        "#,
    )
    .bind(owner_id)
    .bind(container_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .flatten())
}

pub(crate) async fn quality_liaison_status(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    liaison_id: Uuid,
) -> Result<Option<String>, LpnContainerRepositoryError> {
    sqlx::query_scalar("SELECT status FROM quality_liaison_orders WHERE id = $1 AND owner_id = $2")
        .bind(liaison_id)
        .bind(owner_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_db_error)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn insert_quality_lock_event(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    container_id: Uuid,
    lpn_code: &str,
    event_type: &str,
    lock_category: Option<&str>,
    reason_dict_item_code: Option<&str>,
    reason_desc: Option<&str>,
    evidence_urls: &serde_json::Value,
    quality_liaison_id: Option<Uuid>,
    operated_by: Uuid,
    witness_id: Uuid,
    now: DateTime<Utc>,
    note: Option<&str>,
) -> Result<(), LpnContainerRepositoryError> {
    sqlx::query(
        r#"
        INSERT INTO container_quality_lock_events (
            id, owner_id, container_id, lpn_code, event_type, lock_category,
            reason_dict_item_code, reason_desc, evidence_urls, quality_liaison_id,
            operated_by, witness_id, occurred_at, note
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, $4, $5,
            $6, $7, $8, $9,
            $10, $11, $12, $13
        )
        "#,
    )
    .bind(owner_id)
    .bind(container_id)
    .bind(lpn_code)
    .bind(event_type)
    .bind(lock_category)
    .bind(reason_dict_item_code)
    .bind(reason_desc)
    .bind(evidence_urls)
    .bind(quality_liaison_id)
    .bind(operated_by)
    .bind(witness_id)
    .bind(now)
    .bind(note)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}
