use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CreateStockLossOrderRequest, StockAdjustmentSource, StockAdjustmentStatus, StockLossOrder,
    StockLossReason,
};

use crate::{
    auth::AuthContext,
    document_numbering::{GenerateDocumentNumberRequest, PgDocumentNumberingService},
};

use super::*;

pub(crate) struct ApprovedStockLossRequest {
    pub warehouse_id: Uuid,
    pub batch_id: Uuid,
    pub quantity: i64,
    pub reason: StockLossReason,
    pub recall_id: Option<String>,
    pub quality_liaison_id: Uuid,
}

pub(crate) async fn create_approved_stock_loss_order_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    request: ApprovedStockLossRequest,
    now: DateTime<Utc>,
) -> Result<StockLossOrder, StockAdjustmentError> {
    let create_request = CreateStockLossOrderRequest {
        warehouse_id: request.warehouse_id,
        batch_id: request.batch_id,
        quantity: request.quantity,
        reason: request.reason,
        recall_id: request.recall_id,
        source: StockAdjustmentSource::Manual,
        external_ref: None,
        requires_quality_approval: true,
    };
    validate_create_request(&create_request)?;
    if !create_request.reason.is_destruction() {
        return Err(StockAdjustmentError::InvalidRequest);
    }
    let warehouse_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM warehouses WHERE owner_id = $1 AND id = $2 AND status = 'active')",
    )
    .bind(ctx.owner_id)
    .bind(create_request.warehouse_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    if !warehouse_exists {
        return Err(StockAdjustmentError::NotFound);
    }
    let batch: Option<(String, String, i64)> = sqlx::query_as(
        "SELECT product_code, batch_no, qty_on_hand - qty_locked FROM inventory_batches WHERE owner_id = $1 AND id = $2",
    )
    .bind(ctx.owner_id)
    .bind(create_request.batch_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let (product_code, batch_no, available_quantity) =
        batch.ok_or(StockAdjustmentError::NotFound)?;
    if create_request.quantity > available_quantity {
        return Err(StockAdjustmentError::QuantityExceeded);
    }
    let order_id = Uuid::new_v4();
    let order_no = PgDocumentNumberingService::new()
        .generate_in_tx(
            tx,
            ctx,
            GenerateDocumentNumberRequest {
                document_type: STOCK_LOSS_DOCUMENT_TYPE.to_string(),
                idempotency_key: format!("mql-destruction:{}", request.quality_liaison_id),
                source_module: "M-QL".to_string(),
                source_document_id: Some(order_id),
            },
            now,
        )
        .await
        .map_err(|error| StockAdjustmentError::DocumentNumbering(format!("{error:?}")))?
        .value
        .generated_no;
    let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
        r#"
        INSERT INTO stock_adjustment_orders (
            id, owner_id, warehouse_id, order_no, adjustment_type, batch_id,
            product_code, batch_no, quantity, reason_code, recall_id, source,
            status, requires_quality_approval, quality_liaison_id, created_by,
            created_at, updated_at
        )
        VALUES ($1,$2,$3,$4,'loss',$5,$6,$7,$8,$9,$10,'manual',$11,TRUE,$12,$13,$14,$14)
        RETURNING {}
        "#,
        order_columns()
    ))
    .bind(order_id)
    .bind(ctx.owner_id)
    .bind(create_request.warehouse_id)
    .bind(order_no)
    .bind(create_request.batch_id)
    .bind(product_code)
    .bind(batch_no)
    .bind(create_request.quantity)
    .bind(create_request.reason.as_str())
    .bind(normalize_optional(create_request.recall_id))
    .bind(StockAdjustmentStatus::PendingExecution.as_str())
    .bind(request.quality_liaison_id.to_string())
    .bind(ctx.user_id)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_error)?;
    let order = row_to_domain(row)?;
    append_order_audit(
        tx,
        ctx,
        "create_stock_loss_order_from_quality_liaison",
        order.id,
        None,
        &order,
        now,
    )
    .await?;
    Ok(order)
}
