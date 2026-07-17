use super::*;

pub(super) fn map_receiving_order(
    row: ReceivingOrderRow,
    lines: Vec<ReceivingOrderLine>,
) -> ReceivingOrder {
    ReceivingOrder {
        id: row.id,
        owner_id: row.owner_id,
        receipt_no: row.receipt_no,
        document_type: row.document_type,
        supplier_id: row.supplier_id,
        warehouse_id: row.warehouse_id,
        external_ref: row.external_ref,
        status: row.status,
        expected_arrival_at: row.expected_arrival_at,
        lines,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub(super) fn map_receiving_order_line(row: ReceivingOrderLineRow) -> ReceivingOrderLine {
    ReceivingOrderLine {
        line_no: row.line_no as u32,
        product_id: row.product_id,
        product_code: row.product_code,
        expected_qty: row.expected_qty,
        batch_no: row.batch_no,
        production_date: row.production_date.map(|date| date.to_string()),
        expiry_date: row.expiry_date.map(|date| date.to_string()),
    }
}

pub(super) fn map_receiving_order_receipt(row: ReceivingOrderReceiptRow) -> ReceivingOrderReceipt {
    ReceivingOrderReceipt {
        id: row.id,
        receiving_order_id: row.receiving_order_id,
        owner_id: row.owner_id,
        actual_qty: row.actual_qty,
        shortage_qty: row.shortage_qty,
        rejected_qty: row.rejected_qty,
        arrival_temperature_celsius: row.arrival_temperature_celsius,
        exception_note: row.exception_note,
        details: row.receiving_details.map(|value| value.0),
        occurred_at: row.occurred_at,
    }
}

pub(super) fn map_receiving_inspection(row: ReceivingInspectionRow) -> ReceivingInspectionRecord {
    ReceivingInspectionRecord {
        id: row.id,
        receiving_order_id: row.receiving_order_id,
        owner_id: row.owner_id,
        batch_no: row.batch_no,
        accepted_qty: row.accepted_qty,
        rejected_qty: row.rejected_qty,
        quality_status: row.quality_status,
        occurred_at: row.occurred_at,
    }
}

pub(super) fn map_inspection_signature(row: InspectionSignatureRow) -> InspectionSignatureRecord {
    InspectionSignatureRecord {
        id: row.id,
        receiving_order_id: row.receiving_order_id,
        owner_id: row.owner_id,
        first_signer_id: row.first_signer_id,
        second_signer_id: row.second_signer_id,
        strategy_rule_id: row.strategy_rule_id,
        approval_record_id: row.approval_record_id,
        signed_at: row.signed_at,
    }
}

pub(super) fn map_inventory_batch(row: InventoryBatchRow) -> InventoryBatch {
    InventoryBatch {
        id: row.id,
        owner_id: row.owner_id,
        product_code: row.product_code,
        product_name: row.product_name,
        specification: row.specification,
        manufacturer: row.manufacturer,
        batch_no: row.batch_no,
        production_date: row.production_date.to_string(),
        expiry_date: row.expiry_date.to_string(),
        qty_on_hand: row.qty_on_hand,
        qty_locked: row.qty_locked,
        quality_status: row.quality_status,
        location_id: row.location_id,
        location_code: row.location_code,
        row_no: row.row_no,
        column_no: row.column_no,
        layer_no: row.layer_no,
        zone_code: row.zone_code,
        temperature_zone: row.temperature_zone,
        quality_color: row.quality_color,
        max_volume_cm3: row.max_volume_cm3,
        used_volume_cm3: row.used_volume_cm3,
        remaining_volume_cm3: row.remaining_volume_cm3,
        max_sku_count: row.max_sku_count,
        current_sku_count: row.current_sku_count,
        container_lpn: row.container_lpn,
        recall_flag: row.recall_flag,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub(super) fn map_temperature_reading(row: TemperatureReadingRow) -> TemperatureReading {
    TemperatureReading {
        id: row.id,
        owner_id: row.owner_id,
        device_code: row.device_code,
        temperature_celsius: row.temperature_celsius,
        humidity_percent: row.humidity_percent,
        captured_at: row.captured_at,
        external_report_url: row.external_report_url,
        out_of_range: row.out_of_range,
    }
}

pub(super) fn map_cold_chain_device(row: ColdChainDeviceRow) -> wms_domain::ColdChainDevice {
    wms_domain::ColdChainDevice {
        id: row.id,
        owner_id: row.owner_id,
        device_code: row.device_code,
        device_type: row.device_type,
        installed_at_location_code: row.installed_at_location_code,
        calibration_due_at: row.calibration_due_at,
        status: row.status,
        created_at: row.created_at,
    }
}

pub(super) fn map_temperature_excursion(
    row: TemperatureExcursionEventRow,
) -> TemperatureExcursionEvent {
    TemperatureExcursionEvent {
        id: row.id,
        owner_id: row.owner_id,
        external_event_id: row.external_event_id,
        device_code: row.device_code,
        location_code: row.location_code,
        started_at: row.started_at,
        ended_at: row.ended_at,
        min_temperature_celsius: row.min_temperature_celsius,
        max_temperature_celsius: row.max_temperature_celsius,
        affected_batch_ids: row.affected_batch_ids,
        status: row.status,
        created_at: row.created_at,
    }
}
