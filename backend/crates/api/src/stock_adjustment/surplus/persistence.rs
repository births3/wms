use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    DualPersonPolicy, StockAdjustmentSource, StockAdjustmentStatus, StockSurplusOrder,
    StockSurplusReason,
};

use super::*;

pub(super) async fn load_surplus_order_from_pool(
    pool: &PgPool,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<StockSurplusOrder, StockAdjustmentError> {
    let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
        "SELECT {} FROM stock_adjustment_orders WHERE owner_id = $1 AND id = $2 AND adjustment_type = 'surplus'",
        order_columns()
    ))
    .bind(owner_id)
    .bind(order_id)
    .fetch_optional(pool)
    .await
    .map_err(map_database_error)?;
    if let Some(row) = row {
        return row_to_surplus_domain(row);
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM stock_adjustment_orders WHERE id = $1 AND adjustment_type = 'surplus')",
    )
    .bind(order_id)
    .fetch_one(pool)
    .await
    .map_err(map_database_error)?;
    Err(if exists {
        StockAdjustmentError::CrossOwner
    } else {
        StockAdjustmentError::NotFound
    })
}

pub(super) async fn load_surplus_order_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_id: Uuid,
) -> Result<StockSurplusOrder, StockAdjustmentError> {
    let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
        "SELECT {} FROM stock_adjustment_orders WHERE owner_id = $1 AND id = $2 AND adjustment_type = 'surplus' FOR UPDATE",
        order_columns()
    ))
    .bind(owner_id)
    .bind(order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    if let Some(row) = row {
        return row_to_surplus_domain(row);
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM stock_adjustment_orders WHERE id = $1 AND adjustment_type = 'surplus')",
    )
    .bind(order_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    Err(if exists {
        StockAdjustmentError::CrossOwner
    } else {
        StockAdjustmentError::NotFound
    })
}

pub(super) fn row_to_surplus_domain(
    row: StockLossOrderRow,
) -> Result<StockSurplusOrder, StockAdjustmentError> {
    if row.adjustment_type != "surplus" {
        return Err(StockAdjustmentError::NotFound);
    }
    Ok(StockSurplusOrder {
        id: row.id,
        owner_id: row.owner_id,
        warehouse_id: row.warehouse_id,
        order_no: row.order_no,
        batch_id: row.batch_id,
        product_code: row.product_code,
        batch_no: row.batch_no,
        quantity: row.quantity,
        reason: StockSurplusReason::try_from(row.reason_code.as_str())
            .map_err(|_| StockAdjustmentError::Database("非法报溢原因".to_string()))?,
        source: StockAdjustmentSource::try_from(row.source.as_str())
            .map_err(|_| StockAdjustmentError::Database("非法报溢来源".to_string()))?,
        external_ref: row.external_ref,
        status: StockAdjustmentStatus::try_from(row.status.as_str())
            .map_err(|_| StockAdjustmentError::Database("非法报溢状态".to_string()))?,
        requires_quality_approval: row.requires_quality_approval,
        quality_liaison_id: row.quality_liaison_id,
        policy: row
            .policy
            .as_deref()
            .map(DualPersonPolicy::try_from)
            .transpose()
            .map_err(|_| StockAdjustmentError::Database("非法双人策略".to_string()))?,
        source_rule_id: row.source_rule_id,
        first_operator_id: row.first_operator_id,
        second_operator_id: row.second_operator_id,
        approval_record_id: row.approval_record_id,
        started_at: row.started_at,
        completed_at: row.completed_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}
