use rust_decimal::Decimal;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::{contract::payload_digest, error::WorkerError};

#[derive(Clone, Debug)]
pub struct OutboxRow {
    pub table: &'static str,
    pub id: Uuid,
    pub owner_id: Uuid,
    pub event_type: String,
    pub payload: Value,
    pub external_ref: String,
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct PublishedRecord {
    pub table: &'static str,
    pub row: Value,
}

#[derive(Clone, Debug)]
pub enum PublishedUnit {
    Transaction(Vec<PublishedRecord>),
    HeaderChildren {
        header: PublishedRecord,
        children: Vec<PublishedRecord>,
    },
}

pub fn build_published_unit(
    source: &OutboxRow,
    owner_code: &str,
) -> Result<PublishedUnit, WorkerError> {
    match source.event_type.as_str() {
        "inbound_putaway_completed" => single(inbound_feedback(source, owner_code)?),
        "shipment_confirm" => shipment(source, owner_code),
        "inventory_snapshot" => inventory_snapshot(source, owner_code),
        "order_status" => single(order_feedback(source, owner_code, source.id.to_string())?),
        "inventory_status"
        | "inventory_status_changed"
        | "stock_adjustment"
        | "stock_loss_completed"
        | "stock_surplus_completed"
        | "archive_revision"
        | "reconciliation_diff" => single(wms_event(source, owner_code)?),
        _ => Err(invalid(format!(
            "unsupported v1.9 outbound event {}",
            source.event_type
        ))),
    }
}

fn single(record: PublishedRecord) -> Result<PublishedUnit, WorkerError> {
    Ok(PublishedUnit::Transaction(vec![record]))
}

fn inbound_feedback(source: &OutboxRow, owner_code: &str) -> Result<PublishedRecord, WorkerError> {
    let payload = object(&source.payload)?;
    let actual = decimal(payload, "actual_amount")?;
    let rejected = optional_decimal(payload, "reject_amount")?.unwrap_or_default();
    let shortage = optional_decimal(payload, "shortage_amount")?.unwrap_or_default();
    if actual.is_sign_negative() || rejected.is_sign_negative() || shortage.is_sign_negative() {
        return Err(invalid("inbound feedback quantity must be non-negative"));
    }
    if actual > Decimal::ZERO {
        required(payload, "batch_no")?;
        required(payload, "location_code")?;
    }
    if rejected > Decimal::ZERO {
        required(payload, "reject_reason")?;
    }
    if shortage > Decimal::ZERO {
        required(payload, "shortage_reason")?;
    }
    let mut row = json!({
        "IdempotencyKey": source.id.to_string(),
        "ERPBillCode": required(payload, "erp_bill_code")?,
        "Revision": integer(payload, "revision")?,
        "LineNo": integer(payload, "line_no")?,
        "GoodsID": integer(payload, "goods_id")?,
        "GoodsCode": required(payload, "product_code")?,
        "ExpectedAmount": decimal4(decimal(payload, "expected_amount")?),
        "ActualAmount": decimal4(actual),
        "RejectAmount": decimal4(rejected),
        "ShortageAmount": decimal4(shortage),
        "RejectReason": optional(payload, "reject_reason"),
        "ShortageReason": optional(payload, "shortage_reason"),
        "BatchNo": optional(payload, "batch_no"),
        "ProduceDate": optional(payload, "production_date"),
        "ValidDate": optional(payload, "expiry_date"),
        "StallCode": optional(payload, "location_code"),
        "OperatorName": optional(payload, "operator_name"),
        "ScanTime": optional(payload, "scan_time"),
        "OwnerCode": owner_code,
        "SchemaVersion": "1",
        "CorrelationID": required(payload, "correlation_id")?,
        "SourceVersion": null
    });
    add_digest("x_wmsinter_InboundFeedback", &mut row, &[])?;
    Ok(PublishedRecord {
        table: "x_wmsinter_InboundFeedback",
        row,
    })
}

fn shipment(source: &OutboxRow, owner_code: &str) -> Result<PublishedUnit, WorkerError> {
    let payload = object(&source.payload)?;
    let lines = payload
        .get("lines")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("shipment lines required"))?;
    if lines.is_empty()
        || lines.len()
            != usize::try_from(integer(payload, "line_count")?)
                .ok()
                .unwrap_or(0)
    {
        return Err(invalid("outbound feedback line_count mismatch"));
    }
    let correlation_id = required(payload, "correlation_id")?;
    let mut sorted = lines.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|line| {
        line.get("line_no")
            .and_then(value_integer)
            .unwrap_or(i64::MAX)
    });
    let mut records = Vec::with_capacity(lines.len() + 1);
    for line in sorted {
        let line = object(line)?;
        let line_no = integer(line, "line_no")?;
        let expected = decimal(line, "expected_amount")?;
        let picked = decimal(line, "picked_amount")?;
        let shipped = decimal(line, "shipped_amount")?;
        if expected.is_sign_negative() || picked != expected || shipped != expected {
            return Err(invalid(
                "outbound feedback requires picked=shipped=expected",
            ));
        }
        let mut row = json!({
            "IdempotencyKey": line.get("idempotency_key").and_then(Value::as_str)
                .map(ToOwned::to_owned).unwrap_or_else(|| format!("{}:{line_no}", source.id)),
            "ERPBillCode": required(payload, "erp_bill_code")?,
            "Revision": integer(payload, "revision")?,
            "LineNo": line_no,
            "GoodsID": integer(line, "goods_id")?,
            "GoodsCode": required(line, "product_code")?,
            "BatchNo": required(line, "batch_no")?,
            "ExpectedAmount": decimal4(expected),
            "PickedAmount": decimal4(picked),
            "ShippedAmount": decimal4(shipped),
            "OperatorName": optional(payload, "operator_name"),
            "OwnerCode": owner_code,
            "SchemaVersion": "1",
            "CorrelationID": correlation_id,
            "SourceVersion": null
        });
        add_digest("x_wmsinter_OutboundFeedback", &mut row, &[])?;
        records.push(PublishedRecord {
            table: "x_wmsinter_OutboundFeedback",
            row,
        });
    }
    let ship_time = required(payload, "ship_time")?;
    let barrier = Map::from_iter([
        (
            "erp_bill_code".to_owned(),
            Value::String(required(payload, "erp_bill_code")?.to_owned()),
        ),
        (
            "revision".to_owned(),
            Value::from(integer(payload, "revision")?),
        ),
        ("order_type".to_owned(), Value::from(2)),
        ("feedback_type".to_owned(), Value::from(6)),
        ("result_count".to_owned(), Value::from(lines.len())),
        ("waybill_no".to_owned(), optional(payload, "waybill_no")),
        (
            "express_company".to_owned(),
            optional(payload, "express_company"),
        ),
        ("ship_time".to_owned(), Value::String(ship_time.to_owned())),
        (
            "feedback_time".to_owned(),
            Value::String(ship_time.to_owned()),
        ),
        (
            "operator_name".to_owned(),
            optional(payload, "operator_name"),
        ),
        (
            "correlation_id".to_owned(),
            Value::String(correlation_id.to_owned()),
        ),
    ]);
    records.push(order_feedback_from_payload(
        &barrier,
        owner_code,
        source.id.to_string(),
    )?);
    Ok(PublishedUnit::Transaction(records))
}

fn order_feedback(
    source: &OutboxRow,
    owner_code: &str,
    idempotency_key: String,
) -> Result<PublishedRecord, WorkerError> {
    order_feedback_from_payload(object(&source.payload)?, owner_code, idempotency_key)
}

fn order_feedback_from_payload(
    payload: &Map<String, Value>,
    owner_code: &str,
    idempotency_key: String,
) -> Result<PublishedRecord, WorkerError> {
    let feedback_type = integer(payload, "feedback_type")?;
    if matches!(feedback_type, 2 | 6) && optional(payload, "result_count").is_null() {
        return Err(invalid("completion barrier result_count required"));
    }
    if feedback_type == 6 && optional(payload, "ship_time").is_null() {
        return Err(invalid("shipment barrier ship_time required"));
    }
    if feedback_type == 9 && optional(payload, "result_code").is_null() {
        return Err(invalid("rejection result_code required"));
    }
    if feedback_type == 100 && optional(payload, "command_id").is_null() {
        return Err(invalid("cancellation command_id required"));
    }
    let mut row = json!({
        "IdempotencyKey": idempotency_key,
        "ERPBillCode": required(payload, "erp_bill_code")?,
        "Revision": integer(payload, "revision")?,
        "OrderType": integer(payload, "order_type")?,
        "FeedbackType": feedback_type,
        "CommandID": optional(payload, "command_id"),
        "ResultCount": optional(payload, "result_count"),
        "ResultCode": optional(payload, "result_code"),
        "ResultMessage": optional(payload, "result_message"),
        "WaybillNo": optional(payload, "waybill_no"),
        "ExpressCompany": optional(payload, "express_company"),
        "ShipTime": optional(payload, "ship_time"),
        "FeedbackTime": required(payload, "feedback_time")?,
        "OperatorName": optional(payload, "operator_name"),
        "OwnerCode": owner_code,
        "SchemaVersion": "1",
        "CorrelationID": required(payload, "correlation_id")?,
        "SourceVersion": null
    });
    add_digest("x_wmsinter_OrderFeedback", &mut row, &[])?;
    Ok(PublishedRecord {
        table: "x_wmsinter_OrderFeedback",
        row,
    })
}

fn inventory_snapshot(source: &OutboxRow, owner_code: &str) -> Result<PublishedUnit, WorkerError> {
    let payload = object(&source.payload)?;
    let snapshot_id = required(payload, "snapshot_id")?;
    let correlation_id = payload
        .get("correlation_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| source.id.to_string());
    let lines = payload
        .get("lines")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut sorted = lines.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|line| {
        line.get("row_no")
            .and_then(value_integer)
            .unwrap_or(i64::MAX)
    });
    let mut children = Vec::with_capacity(lines.len());
    for (index, line) in sorted.into_iter().enumerate() {
        let line = object(line)?;
        let row_no = integer(line, "row_no")?;
        if row_no != i64::try_from(index + 1).unwrap_or(i64::MAX) {
            return Err(invalid(
                "inventory snapshot RowNo must be contiguous from 1",
            ));
        }
        let amount = decimal(line, "wms_amount")?;
        let pickable = decimal(line, "wms_pickable")?;
        let allocated = optional_decimal(line, "wms_allocated")?.unwrap_or_default();
        let frozen = optional_decimal(line, "wms_frozen")?.unwrap_or_default();
        if amount.is_sign_negative()
            || pickable.is_sign_negative()
            || allocated.is_sign_negative()
            || frozen.is_sign_negative()
            || pickable > amount
        {
            return Err(invalid("inventory snapshot quantity constraint failed"));
        }
        children.push(PublishedRecord {
            table: "x_wmsinter_InventoryReceiveItems",
            row: json!({
                "SnapshotID": snapshot_id,
                "RowNo": row_no,
                "DepotCode": line.get("depot_code").and_then(Value::as_str)
                    .unwrap_or(required(payload, "depot_code")?),
                "GoodsCode": required(line, "product_code")?,
                "BatchNo": required(line, "batch_no")?,
                "ValidDate": optional(line, "valid_date"),
                "GoodsStatus": required(line, "goods_status")?,
                "WMSAmount": decimal4(amount),
                "WMSPickable": decimal4(pickable),
                "WMSAllocated": decimal4(allocated),
                "WMSFrozen": decimal4(frozen),
                "OwnerCode": owner_code,
                "CorrelationID": correlation_id,
                "IdempotencyKey": format!("{snapshot_id}:{row_no}")
            }),
        });
    }
    let child_rows = children
        .iter()
        .map(|record| record.row.clone())
        .collect::<Vec<_>>();
    let mut row = json!({
        "SnapshotID": snapshot_id,
        "ReceiveTime": payload.get("receive_time").and_then(Value::as_str).unwrap_or(&source.created_at),
        "TotalCount": children.len(),
        "OwnerCode": owner_code,
        "SchemaVersion": "1",
        "IdempotencyKey": snapshot_id,
        "CorrelationID": correlation_id,
        "SourceVersion": null
    });
    add_digest("x_wmsinter_InventoryReceiveHeader", &mut row, &child_rows)?;
    Ok(PublishedUnit::HeaderChildren {
        header: PublishedRecord {
            table: "x_wmsinter_InventoryReceiveHeader",
            row,
        },
        children,
    })
}

fn wms_event(source: &OutboxRow, owner_code: &str) -> Result<PublishedRecord, WorkerError> {
    let payload = object(&source.payload)?;
    let (event_type, event_time, event_payload) = match source.event_type.as_str() {
        "inventory_status" | "inventory_status_changed" => {
            let time = event_time(source, payload, "occur_time")?;
            (
                "inventory_status",
                time.clone(),
                json!({
                    "depot_code": required(payload, "depot_code")?,
                    "product_code": required(payload, "product_code")?,
                    "batch_no": required(payload, "batch_no")?,
                    "goods_status": required(payload, "to_status")?,
                    "amount": decimal4(decimal(payload, "qty")?), "occur_time": time
                }),
            )
        }
        "stock_adjustment" | "stock_loss_completed" | "stock_surplus_completed" => {
            let time = event_time(source, payload, "completed_at")?;
            let adjust_type = if source.event_type.contains("loss") {
                "损"
            } else {
                "溢"
            };
            (
                "stock_adjustment",
                time.clone(),
                json!({
                    "depot_code": required(payload, "depot_code")?,
                    "product_code": required(payload, "product_code")?,
                    "batch_no": required(payload, "batch_no")?, "adjust_type": adjust_type,
                    "amount": decimal4(decimal(payload, "quantity")?),
                    "reason": required(payload, "reason")?, "adjust_time": time
                }),
            )
        }
        "archive_revision" => {
            let time = event_time(source, payload, "submitted_at")?;
            (
                "archive_revision",
                time.clone(),
                json!({
                    "liaison_id": required(payload, "liaison_id")?, "asn_id": required(payload, "asn_id")?,
                    "receipt_record_id": required(payload, "receipt_record_id")?,
                    "product_code": required(payload, "product_code")?, "field_name": required(payload, "field_name")?,
                    "current_value": optional(payload, "current_value"), "new_value": optional(payload, "new_value"),
                    "photo_urls": payload.get("photo_urls").cloned().ok_or_else(|| invalid("photo_urls required"))?,
                    "operator_id": required(payload, "operator_id")?, "submitted_at": time
                }),
            )
        }
        "reconciliation_diff" => {
            let time = event_time(source, payload, "diff_at")?;
            (
                "reconciliation_diff",
                time.clone(),
                json!({
                    "depot_code": required(payload, "depot_code")?, "product_code": required(payload, "product_code")?,
                    "batch_no": required(payload, "batch_no")?, "erp_amount": decimal4(decimal(payload, "erp_qty")?),
                    "wms_amount": decimal4(decimal(payload, "wms_qty")?),
                    "diff_amount": decimal4(decimal(payload, "difference_qty")?), "diff_at": time
                }),
            )
        }
        _ => return Err(invalid("unsupported WMS event")),
    };
    let correlation_id = payload
        .get("correlation_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| source.id.to_string());
    let mut row = json!({
        "IdempotencyKey": source.id.to_string(), "EventType": event_type, "SchemaVersion": "1",
        "PayloadJson": event_payload, "EventTime": event_time, "OwnerCode": owner_code,
        "CorrelationID": correlation_id, "SourceVersion": null
    });
    add_digest("x_wmsinter_WmsEvent", &mut row, &[])?;
    Ok(PublishedRecord {
        table: "x_wmsinter_WmsEvent",
        row,
    })
}

fn add_digest(table: &str, row: &mut Value, children: &[Value]) -> Result<(), WorkerError> {
    let digest = payload_digest(table, row, children)?;
    row.as_object_mut()
        .ok_or_else(|| invalid("published row must be object"))?
        .insert("PayloadDigest".to_owned(), Value::String(digest));
    Ok(())
}

fn object(value: &Value) -> Result<&Map<String, Value>, WorkerError> {
    value
        .as_object()
        .ok_or_else(|| invalid("payload object required"))
}

fn required<'a>(payload: &'a Map<String, Value>, field: &str) -> Result<&'a str, WorkerError> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(format!("missing {field}")))
}

fn integer(payload: &Map<String, Value>, field: &str) -> Result<i64, WorkerError> {
    payload
        .get(field)
        .and_then(value_integer)
        .ok_or_else(|| invalid(format!("invalid {field}")))
}

fn value_integer(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str()?.parse().ok())
}

fn decimal(payload: &Map<String, Value>, field: &str) -> Result<Decimal, WorkerError> {
    optional_decimal(payload, field)?.ok_or_else(|| invalid(format!("missing {field}")))
}

fn optional_decimal(
    payload: &Map<String, Value>,
    field: &str,
) -> Result<Option<Decimal>, WorkerError> {
    let Some(value) = payload.get(field).filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let text = value
        .as_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| value.to_string());
    text.parse::<Decimal>()
        .map(Some)
        .map_err(|_| invalid(format!("invalid {field}")))
}

fn optional(payload: &Map<String, Value>, field: &str) -> Value {
    payload.get(field).cloned().unwrap_or(Value::Null)
}

fn event_time(
    source: &OutboxRow,
    payload: &Map<String, Value>,
    field: &str,
) -> Result<String, WorkerError> {
    Ok(payload
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(&source.created_at)
        .to_owned())
}

fn decimal4(value: Decimal) -> String {
    format!("{value:.4}")
}

fn invalid(message: impl Into<String>) -> WorkerError {
    WorkerError::new("INVALID_DATA", message)
}
