use super::*;

pub(super) fn validate_receiving_gsp_fields(
    document_type: &str,
    lines: &[ReceivingOrderLine],
    req: &ReceiveReceivingOrderRequest,
    actor_id: Uuid,
) -> Result<(), Wave3RepositoryError> {
    let details = req
        .details
        .as_ref()
        .ok_or_else(|| Wave3RepositoryError::MissingRequiredField("details".to_string()))?;
    let required = [
        ("vehicle_no", details.vehicle_no.as_deref()),
        ("origin", details.origin.as_deref()),
        ("transport_mode", details.transport_mode.as_deref()),
        ("carrier", details.carrier.as_deref()),
        ("contact_name", details.contact_name.as_deref()),
        ("contact_phone", details.contact_phone.as_deref()),
        ("contact_id_no", details.contact_id_no.as_deref()),
        ("seal_checked", details.seal_checked.as_deref()),
        ("filing_checked", details.filing_checked.as_deref()),
    ];
    for (field, value) in required {
        if value
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .is_none()
        {
            return Err(Wave3RepositoryError::MissingRequiredField(
                field.to_string(),
            ));
        }
    }
    for (field, present) in [
        ("departure_at", details.departure_at.is_some()),
        ("arrival_at", details.arrival_at.is_some()),
        ("storage_at", details.storage_at.is_some()),
    ] {
        if !present {
            return Err(Wave3RepositoryError::MissingRequiredField(
                field.to_string(),
            ));
        }
    }
    if details.delivery_qty <= wms_domain::Quantity::ZERO {
        return Err(Wave3RepositoryError::MissingRequiredField(
            "delivery_qty".to_string(),
        ));
    }
    if details.second_receiver_id == Some(actor_id) {
        return Err(Wave3RepositoryError::SameSigner);
    }
    if req.rejected_qty > wms_domain::Quantity::ZERO
        && req
            .exception_note
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(Wave3RepositoryError::MissingRequiredField(
            "exception_note".to_string(),
        ));
    }
    validate_sales_return_batches(document_type, lines, req)
}

fn validate_sales_return_batches(
    document_type: &str,
    lines: &[ReceivingOrderLine],
    req: &ReceiveReceivingOrderRequest,
) -> Result<(), Wave3RepositoryError> {
    let details = req
        .details
        .as_ref()
        .ok_or_else(|| Wave3RepositoryError::MissingRequiredField("details".to_string()))?;
    if document_type != RECEIVING_DOCUMENT_TYPE_SALES_RETURN {
        return if details.sales_return_batches.is_empty() {
            Ok(())
        } else {
            Err(Wave3RepositoryError::InvalidBatchPolicy)
        };
    }
    if details.sales_return_batches.is_empty() {
        return Err(Wave3RepositoryError::InvalidBatchPolicy);
    }
    let original_batches = lines.iter().fold(
        std::collections::HashMap::<&str, wms_domain::Quantity>::new(),
        |mut quantities, line| {
            if let Some(batch_no) = line.batch_no.as_deref() {
                *quantities.entry(batch_no.trim()).or_default() += line.expected_qty;
            }
            quantities
        },
    );
    let mut batch_numbers = std::collections::HashSet::new();
    for batch in &details.sales_return_batches {
        let batch_no = batch.batch_no.trim();
        if batch_no.is_empty()
            || !batch_numbers.insert(batch_no.to_owned())
            || original_batches
                .get(batch_no)
                .is_none_or(|expected_qty| batch.quantity > *expected_qty)
            || batch.quantity <= wms_domain::Quantity::ZERO
            || batch.rejected_qty < wms_domain::Quantity::ZERO
            || batch.rejected_qty > batch.quantity
            || (batch.rejected_qty > wms_domain::Quantity::ZERO
                && batch
                    .reject_reason
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none())
        {
            return Err(Wave3RepositoryError::InvalidBatchPolicy);
        }
    }
    let batch_qty = details
        .sales_return_batches
        .iter()
        .map(|batch| batch.quantity)
        .sum::<wms_domain::Quantity>();
    let batch_rejected_qty = details
        .sales_return_batches
        .iter()
        .map(|batch| batch.rejected_qty)
        .sum::<wms_domain::Quantity>();
    if batch_qty != details.delivery_qty || batch_rejected_qty != req.rejected_qty {
        return Err(Wave3RepositoryError::InvalidBatchPolicy);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sales_return_lines() -> Vec<ReceivingOrderLine> {
        [("B1", 7_i64), ("B2", 3_i64)]
            .into_iter()
            .enumerate()
            .map(|(index, (batch_no, quantity))| ReceivingOrderLine {
                line_no: (index + 1) as u32,
                product_id: None,
                product_code: "P-M2-001".to_string(),
                expected_qty: quantity.into(),
                batch_no: Some(batch_no.to_string()),
                production_date: None,
                expiry_date: None,
            })
            .collect()
    }

    fn sales_return_request() -> ReceiveReceivingOrderRequest {
        ReceiveReceivingOrderRequest {
            actual_qty: 7.into(),
            shortage_qty: wms_domain::Quantity::ZERO,
            rejected_qty: 3.into(),
            arrival_temperature_celsius: None,
            exception_note: Some("B2 外包装破损".to_string()),
            details: Some(ReceivingReceiptDetails {
                delivery_qty: 10.into(),
                temperature_control_method: Some("常温".to_string()),
                vehicle_no: Some("苏A12345".to_string()),
                origin: Some("南京配送中心".to_string()),
                departure_at: Some(Utc::now()),
                arrival_at: Some(Utc::now()),
                storage_at: Some(Utc::now()),
                transport_mode: Some("公路".to_string()),
                carrier: Some("华东医药物流".to_string()),
                contact_name: Some("张三".to_string()),
                contact_phone: Some("13800000000".to_string()),
                contact_id_no: Some("320101199001011234".to_string()),
                seal_checked: Some("已核对".to_string()),
                filing_checked: Some("已核对".to_string()),
                second_receiver_id: None,
                sales_return_batches: vec![
                    wms_domain::SalesReturnReceivingBatch {
                        batch_no: "B1".to_string(),
                        quantity: 7.into(),
                        rejected_qty: wms_domain::Quantity::ZERO,
                        reject_reason: None,
                    },
                    wms_domain::SalesReturnReceivingBatch {
                        batch_no: "B2".to_string(),
                        quantity: 3.into(),
                        rejected_qty: 3.into(),
                        reject_reason: Some("外包装破损".to_string()),
                    },
                ],
            }),
        }
    }

    #[test]
    fn sales_return_batch_rejections_must_close_to_receipt_rejected_quantity() {
        let request = sales_return_request();
        assert!(validate_receiving_gsp_fields(
            RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
            &sales_return_lines(),
            &request,
            Uuid::new_v4(),
        )
        .is_ok());

        let mut mismatch = request;
        mismatch
            .details
            .as_mut()
            .expect("details")
            .sales_return_batches[1]
            .rejected_qty = 2.into();
        assert!(matches!(
            validate_receiving_gsp_fields(
                RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
                &sales_return_lines(),
                &mismatch,
                Uuid::new_v4(),
            ),
            Err(Wave3RepositoryError::InvalidBatchPolicy)
        ));
    }

    #[test]
    fn second_receiver_must_be_different_from_current_receiver() {
        let actor_id = Uuid::new_v4();
        let mut request = sales_return_request();
        request
            .details
            .as_mut()
            .expect("details")
            .second_receiver_id = Some(actor_id);
        assert!(matches!(
            validate_receiving_gsp_fields(
                RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
                &sales_return_lines(),
                &request,
                actor_id,
            ),
            Err(Wave3RepositoryError::SameSigner)
        ));
    }

    #[test]
    fn sales_return_receipt_rejects_batch_outside_original_order() {
        let mut request = sales_return_request();
        request
            .details
            .as_mut()
            .expect("details")
            .sales_return_batches[0]
            .batch_no = "OTHER-BATCH".to_string();
        assert!(matches!(
            validate_receiving_gsp_fields(
                RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
                &sales_return_lines(),
                &request,
                Uuid::new_v4(),
            ),
            Err(Wave3RepositoryError::InvalidBatchPolicy)
        ));
    }
}
