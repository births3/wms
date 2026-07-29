use crate::stock_adjustment::StockAdjustmentError;

use super::{ReconciliationError, RunReconciliationRequest};

pub(super) fn normalize_request(
    mut request: RunReconciliationRequest,
) -> Result<RunReconciliationRequest, ReconciliationError> {
    request.window_key = request.window_key.trim().to_owned();
    for item in &mut request.items {
        item.product_code = item.product_code.trim().to_owned();
        item.batch_no = item.batch_no.trim().to_owned();
    }
    request.items.sort_by(|left, right| {
        (&left.product_code, &left.batch_no).cmp(&(&right.product_code, &right.batch_no))
    });
    if request.window_key.trim().is_empty()
        || request.items.iter().any(|item| {
            item.product_code.trim().is_empty()
                || item.batch_no.trim().is_empty()
                || item.qty_on_hand < 0
        })
    {
        return Err(ReconciliationError::InvalidRequest);
    }
    let mut keys = std::collections::HashSet::new();
    if request
        .items
        .iter()
        .any(|item| !keys.insert((item.product_code.trim(), item.batch_no.trim())))
    {
        return Err(ReconciliationError::InvalidRequest);
    }
    Ok(request)
}

pub(super) fn map_stock_adjustment(error: StockAdjustmentError) -> ReconciliationError {
    match error {
        StockAdjustmentError::NotFound
        | StockAdjustmentError::CrossOwner
        | StockAdjustmentError::InvalidRequest
        | StockAdjustmentError::QuantityExceeded
        | StockAdjustmentError::InvalidPutawayTarget => ReconciliationError::InvalidRequest,
        other => ReconciliationError::StockAdjustment(format!("{other:?}")),
    }
}
