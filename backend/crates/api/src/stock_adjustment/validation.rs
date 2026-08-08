use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CreateStockLossOrderRequest, DualPersonPolicy, StockAdjustmentSource, StockAdjustmentStatus,
    StockLossOrder, StockLossReason,
};

use super::StockAdjustmentError;

pub(super) fn validate_create_request(
    request: &CreateStockLossOrderRequest,
) -> Result<(), StockAdjustmentError> {
    if request.quantity <= wms_domain::Quantity::ZERO
        || request.source == StockAdjustmentSource::Erp
            && request
                .external_ref
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        || request.reason == StockLossReason::RecallDestruction
            && request
                .recall_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
    {
        return Err(StockAdjustmentError::InvalidRequest);
    }
    Ok(())
}

pub(super) fn same_create_request(
    order: &StockLossOrder,
    request: &CreateStockLossOrderRequest,
    recall_id: Option<&str>,
    external_ref: Option<&str>,
    requires_quality_approval: bool,
) -> bool {
    order.warehouse_id == request.warehouse_id
        && order.batch_id == request.batch_id
        && order.quantity == request.quantity
        && order.reason == request.reason
        && order.recall_id.as_deref() == recall_id
        && order.source == request.source
        && order.external_ref.as_deref() == external_ref
        && order.requires_quality_approval == requires_quality_approval
}

pub(super) async fn validate_second_operator(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    first_operator_id: Uuid,
    second_operator_id: Option<Uuid>,
    policy: DualPersonPolicy,
) -> Result<Option<Uuid>, StockAdjustmentError> {
    if policy == DualPersonPolicy::Single {
        return Ok(None);
    }
    let second_operator_id =
        second_operator_id.ok_or(StockAdjustmentError::MissingSecondOperator)?;
    if second_operator_id == first_operator_id {
        return Err(StockAdjustmentError::SameOperator);
    }
    ensure_custodian(tx, owner_id, second_operator_id).await?;
    Ok(Some(second_operator_id))
}

pub(super) async fn ensure_custodian(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    user_id: Uuid,
) -> Result<(), StockAdjustmentError> {
    let qualified = crate::dual_person_policy::is_active_operator_with_role_in_tx(
        tx,
        owner_id,
        user_id,
        "custodian",
    )
    .await
    .map_err(|error| StockAdjustmentError::Database(format!("操作人资质查询失败: {error:?}")))?;
    if qualified {
        Ok(())
    } else {
        Err(StockAdjustmentError::UnqualifiedOperator)
    }
}

pub(super) fn ensure_status(
    order: &StockLossOrder,
    expected: StockAdjustmentStatus,
) -> Result<(), StockAdjustmentError> {
    if order.status == expected {
        Ok(())
    } else {
        Err(StockAdjustmentError::InvalidStatus {
            expected: expected.as_str().to_string(),
            actual: order.status.as_str().to_string(),
        })
    }
}
