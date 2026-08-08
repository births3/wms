use wms_domain::{
    ReceivingOrderLine, RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND,
    RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
};

use super::ReceivingOrderError;

pub(super) fn validate_receiving_order_lines(
    document_type: &str,
    lines: &[ReceivingOrderLine],
) -> Result<(), ReceivingOrderError> {
    if lines.is_empty() {
        return Err(ReceivingOrderError::EmptyLines);
    }
    for line in lines {
        if line.line_no == 0 || line.expected_qty <= wms_domain::Quantity::ZERO {
            return Err(ReceivingOrderError::InvalidQuantity);
        }
        let has_batch = line
            .batch_no
            .as_deref()
            .is_some_and(|batch_no| !batch_no.trim().is_empty());
        match document_type {
            RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND
                if line.batch_no.is_some()
                    || line.production_date.is_some()
                    || line.expiry_date.is_some() =>
            {
                return Err(ReceivingOrderError::InvalidBatchPolicy);
            }
            RECEIVING_DOCUMENT_TYPE_SALES_RETURN if !has_batch => {
                return Err(ReceivingOrderError::InvalidBatchPolicy);
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(batch_no: Option<&str>) -> ReceivingOrderLine {
        ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-M2-001".to_string(),
            expected_qty: 1.into(),
            batch_no: batch_no.map(str::to_string),
            production_date: None,
            expiry_date: None,
        }
    }

    #[test]
    fn purchase_inbound_batch_is_only_created_at_inspection() {
        assert!(matches!(
            validate_receiving_order_lines(
                RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND,
                &[line(Some("B-001"))]
            ),
            Err(ReceivingOrderError::InvalidBatchPolicy)
        ));
    }

    #[test]
    fn purchase_inbound_empty_batch_is_not_a_batchless_line() {
        assert!(matches!(
            validate_receiving_order_lines(
                RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND,
                &[line(Some("  "))]
            ),
            Err(ReceivingOrderError::InvalidBatchPolicy)
        ));
    }

    #[test]
    fn sales_return_requires_original_batch() {
        assert!(matches!(
            validate_receiving_order_lines(RECEIVING_DOCUMENT_TYPE_SALES_RETURN, &[line(None)]),
            Err(ReceivingOrderError::InvalidBatchPolicy)
        ));
    }
}
