//! 补货在途三命令：扩既有 inventory 上下文，不另起 Service。

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::Quantity;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryReplenishError {
    Insufficient,
    NotFound,
    InvalidQuantity,
    Database(String),
}

async fn set_lock_timeout(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), InventoryReplenishError> {
    sqlx::query("SET LOCAL lock_timeout = '3s'")
        .execute(&mut **tx)
        .await
        .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    Ok(())
}

struct SourceBatchRow {
    product_id: Option<Uuid>,
    product_code: Option<String>,
    batch_no: String,
    production_date: NaiveDate,
    expiry_date: NaiveDate,
}

#[allow(clippy::too_many_arguments)]
pub async fn reserve_replenish_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    source_batch_id: Uuid,
    target_location_id: Uuid,
    qty: Quantity,
    now: DateTime<Utc>,
) -> Result<Uuid, InventoryReplenishError> {
    if qty <= Quantity::ZERO {
        return Err(InventoryReplenishError::InvalidQuantity);
    }
    set_lock_timeout(tx).await?;

    let source = sqlx::query_as::<_, (Option<Uuid>, Option<String>, String, NaiveDate, NaiveDate)>(
        r#"
        SELECT product_id, product_code, batch_no, production_date, expiry_date
          FROM inventory_batches
         WHERE owner_id = $1
           AND id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(source_batch_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    let Some((product_id, product_code, batch_no, production_date, expiry_date)) = source else {
        return Err(InventoryReplenishError::NotFound);
    };
    let source_row = SourceBatchRow {
        product_id,
        product_code,
        batch_no,
        production_date,
        expiry_date,
    };

    let reserved = sqlx::query_scalar::<_, Quantity>(
        r#"
        UPDATE inventory_batches
           SET qty_replenish_out_transit = qty_replenish_out_transit + $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1
           AND id = $2
           AND qty_on_hand - qty_allocated - qty_frozen - qty_replenish_out_transit >= $3
        RETURNING qty_replenish_out_transit
        "#,
    )
    .bind(owner_id)
    .bind(source_batch_id)
    .bind(qty)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    if reserved.is_none() {
        return Err(InventoryReplenishError::Insufficient);
    }

    let existing_target = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
          FROM inventory_batches
         WHERE owner_id = $1
           AND location_id = $2
           AND batch_no = $3
           AND (
                ($4::uuid IS NOT NULL AND product_id = $4)
                OR ($4::uuid IS NULL AND product_code IS NOT DISTINCT FROM $5)
           )
           AND (container_lpn IS NULL OR container_lpn = '')
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(target_location_id)
    .bind(&source_row.batch_no)
    .bind(source_row.product_id)
    .bind(source_row.product_code.as_deref())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;

    let target_id = if let Some(id) = existing_target {
        id
    } else {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO inventory_batches (
                id, owner_id, product_id, product_code, batch_no,
                production_date, expiry_date, qty_on_hand, qty_frozen, qty_allocated,
                qty_replenish_in_transit, qty_replenish_out_transit,
                status, location_id, location_code, container_lpn,
                recall_flag, created_at, updated_at, version
            )
            VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, 0, 0, 0,
                0, 0,
                'qualified', $8, '', NULL,
                FALSE, $9, $9, 1
            )
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .bind(source_row.product_id)
        .bind(source_row.product_code.as_deref())
        .bind(&source_row.batch_no)
        .bind(source_row.production_date)
        .bind(source_row.expiry_date)
        .bind(target_location_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
        id
    };

    let incremented = sqlx::query_scalar::<_, Quantity>(
        r#"
        UPDATE inventory_batches
           SET qty_replenish_in_transit = qty_replenish_in_transit + $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1
           AND id = $2
        RETURNING qty_replenish_in_transit
        "#,
    )
    .bind(owner_id)
    .bind(target_id)
    .bind(qty)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    if incremented.is_none() {
        return Err(InventoryReplenishError::NotFound);
    }
    Ok(target_id)
}

#[allow(clippy::too_many_arguments)]
pub async fn confirm_replenish_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    source_batch_id: Uuid,
    target_batch_id: Uuid,
    qty: Quantity,
    source_document_id: Uuid,
    approval_source: &str,
    approval_id: &str,
    now: DateTime<Utc>,
) -> Result<(), InventoryReplenishError> {
    if qty <= Quantity::ZERO {
        return Err(InventoryReplenishError::InvalidQuantity);
    }

    let source_ok = sqlx::query_scalar::<_, Quantity>(
        r#"
        UPDATE inventory_batches
           SET qty_on_hand = qty_on_hand - $3,
               qty_replenish_out_transit = qty_replenish_out_transit - $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1
           AND id = $2
           AND qty_on_hand >= $3
           AND qty_replenish_out_transit >= $3
           AND NOT EXISTS (
                SELECT 1 FROM warehouse_locations wl
                 WHERE wl.id = inventory_batches.location_id
                   AND wl.owner_id = inventory_batches.owner_id
                   AND wl.agv_unreachable_at IS NOT NULL
           )
        RETURNING qty_on_hand
        "#,
    )
    .bind(owner_id)
    .bind(source_batch_id)
    .bind(qty)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    if source_ok.is_none() {
        return Err(InventoryReplenishError::Insufficient);
    }

    let target_ok = sqlx::query_scalar::<_, Quantity>(
        r#"
        UPDATE inventory_batches
           SET qty_on_hand = qty_on_hand + $3,
               qty_replenish_in_transit = qty_replenish_in_transit - $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1
           AND id = $2
           AND qty_replenish_in_transit >= $3
        RETURNING qty_on_hand
        "#,
    )
    .bind(owner_id)
    .bind(target_batch_id)
    .bind(qty)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    if target_ok.is_none() {
        return Err(InventoryReplenishError::Insufficient);
    }

    insert_replenish_movement(
        tx,
        owner_id,
        source_batch_id,
        -qty,
        source_document_id,
        approval_source,
        approval_id,
        now,
    )
    .await?;
    insert_replenish_movement(
        tx,
        owner_id,
        target_batch_id,
        qty,
        source_document_id,
        approval_source,
        approval_id,
        now,
    )
    .await?;
    Ok(())
}

pub async fn release_replenish_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    source_batch_id: Uuid,
    target_batch_id: Uuid,
    remaining: Quantity,
    now: DateTime<Utc>,
) -> Result<(), InventoryReplenishError> {
    if remaining <= Quantity::ZERO {
        return Err(InventoryReplenishError::InvalidQuantity);
    }

    let source_ok = sqlx::query_scalar::<_, Quantity>(
        r#"
        UPDATE inventory_batches
           SET qty_replenish_out_transit = qty_replenish_out_transit - $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1
           AND id = $2
           AND qty_replenish_out_transit >= $3
        RETURNING qty_replenish_out_transit
        "#,
    )
    .bind(owner_id)
    .bind(source_batch_id)
    .bind(remaining)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    if source_ok.is_none() {
        return Err(InventoryReplenishError::Insufficient);
    }

    let target_ok = sqlx::query_scalar::<_, Quantity>(
        r#"
        UPDATE inventory_batches
           SET qty_replenish_in_transit = qty_replenish_in_transit - $3,
               updated_at = $4,
               version = version + 1
         WHERE owner_id = $1
           AND id = $2
           AND qty_replenish_in_transit >= $3
        RETURNING qty_replenish_in_transit
        "#,
    )
    .bind(owner_id)
    .bind(target_batch_id)
    .bind(remaining)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    if target_ok.is_none() {
        return Err(InventoryReplenishError::Insufficient);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_replenish_movement(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    qty_delta: Quantity,
    source_document_id: Uuid,
    approval_source: &str,
    approval_id: &str,
    now: DateTime<Utc>,
) -> Result<(), InventoryReplenishError> {
    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, approval_source,
            approval_id, occurred_at
        ) VALUES ($1,$2,$3,'replenish',$4,'replenishment_task',$5,$6,$7,$8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(qty_delta)
    .bind(source_document_id)
    .bind(approval_source)
    .bind(approval_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| InventoryReplenishError::Database(error.to_string()))?;
    Ok(())
}
