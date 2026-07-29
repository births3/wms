use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

use super::{db, ReconciliationError, ReconciliationItem, ReconciliationRun};

#[derive(FromRow)]
struct RunRow {
    id: Uuid,
    window_key: String,
    snapshot_at: DateTime<Utc>,
    matched_count: i32,
    wms_more_count: i32,
    erp_more_count: i32,
    created_at: DateTime<Utc>,
    request_hash: String,
}

#[derive(FromRow)]
struct ItemRow {
    id: Uuid,
    product_code: String,
    batch_no: String,
    wms_qty: i64,
    erp_qty: i64,
    difference_qty: i64,
    difference_type: String,
    resolution_status: String,
    stock_adjustment_order_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
}

pub(super) async fn load_existing_window(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    window_key: &str,
    request_hash: &str,
) -> Result<Option<ReconciliationRun>, ReconciliationError> {
    let row = sqlx::query_as::<_, RunRow>(
        "SELECT id, window_key, snapshot_at, matched_count, wms_more_count,
                erp_more_count, created_at, request_hash
           FROM reconciliation_runs
          WHERE owner_id = $1 AND window_key = $2
          FOR UPDATE",
    )
    .bind(owner_id)
    .bind(window_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.request_hash != request_hash {
        return Err(ReconciliationError::IdempotencyConflict);
    }
    let items = sqlx::query_as::<_, ItemRow>(
        "SELECT item.id, item.product_code, item.batch_no, item.wms_qty, item.erp_qty,
                item.difference_qty, item.difference_type, item.resolution_status,
                ARRAY(SELECT link.adjustment_order_id
                        FROM reconciliation_item_adjustments link
                       WHERE link.item_id = item.id
                       ORDER BY link.adjustment_order_id) AS stock_adjustment_order_ids,
                item.created_at
           FROM reconciliation_items item
          WHERE item.owner_id = $1 AND item.run_id = $2
          ORDER BY item.product_code, item.batch_no",
    )
    .bind(owner_id)
    .bind(row.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?
    .into_iter()
    .map(|item| ReconciliationItem {
        id: item.id,
        product_code: item.product_code,
        batch_no: item.batch_no,
        wms_qty: item.wms_qty,
        erp_qty: item.erp_qty,
        difference_qty: item.difference_qty,
        difference_type: item.difference_type,
        resolution_status: item.resolution_status,
        stock_adjustment_order_ids: item.stock_adjustment_order_ids,
        created_at: item.created_at,
    })
    .collect();
    Ok(Some(ReconciliationRun {
        id: row.id,
        owner_id,
        window_key: row.window_key,
        snapshot_at: row.snapshot_at,
        matched_count: row.matched_count,
        wms_more_count: row.wms_more_count,
        erp_more_count: row.erp_more_count,
        items,
        created_at: row.created_at,
    }))
}
