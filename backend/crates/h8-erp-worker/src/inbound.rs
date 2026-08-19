use rust_decimal::Decimal;
use serde_json::{json, Map, Value};

use crate::error::WorkerError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InboundContract {
    pub table: &'static str,
    pub message_type: &'static str,
}

const INBOUND: [InboundContract; 7] = [
    InboundContract {
        table: "x_wmsinter_GoodsInfo",
        message_type: "product_master",
    },
    InboundContract {
        table: "x_wmsinter_CustomerInfo",
        message_type: "customer_master",
    },
    InboundContract {
        table: "x_wmsinter_SupplierInfo",
        message_type: "supplier_master",
    },
    InboundContract {
        table: "x_wmsinter_InboundOrder",
        message_type: "asn",
    },
    InboundContract {
        table: "x_wmsinter_OutboundOrder",
        message_type: "outbound_order",
    },
    InboundContract {
        table: "x_wmsinter_OrderCommand",
        message_type: "order_cancel",
    },
    InboundContract {
        table: "x_wmsinter_InventoryPushHeader",
        message_type: "inventory_seed_snapshot",
    },
];

pub fn inbound_contract(table: &str) -> Option<&'static InboundContract> {
    INBOUND.iter().find(|contract| contract.table == table)
}

pub fn contracts() -> &'static [InboundContract] {
    &INBOUND
}

pub fn request_body(
    message_type: &str,
    row: &Value,
    children: &[Value],
) -> Result<Value, WorkerError> {
    let mut body = envelope(row)?;
    let fields = body
        .as_object_mut()
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "invalid envelope"))?;
    match message_type {
        "product_master" => product(fields, row)?,
        "customer_master" => customer(fields, row)?,
        "supplier_master" => supplier(fields, row)?,
        "asn" => asn(fields, row, children)?,
        "outbound_order" => outbound(fields, row, children)?,
        "order_cancel" => cancel(fields, row)?,
        "inventory_seed_snapshot" => snapshot(fields, row, children)?,
        _ => return Err(WorkerError::new("H8_WORKER_UNSUPPORTED_TYPE", message_type)),
    }
    Ok(body)
}

fn envelope(row: &Value) -> Result<Value, WorkerError> {
    Ok(json!({
        "schema_version": required(row, "SchemaVersion")?,
        "external_ref": external_ref(row)?,
        "correlation_id": required(row, "CorrelationID")?,
        "occurred_at": required(row, "inserttime")?,
        "payload_digest": required(row, "PayloadDigest")?,
        "source_version": row.get("SourceVersion").cloned().unwrap_or(Value::Null),
    }))
}

fn product(body: &mut Map<String, Value>, row: &Value) -> Result<(), WorkerError> {
    let packaging = match row.get("PackagingJson") {
        None | Some(Value::Null) => Value::Array(Vec::new()),
        Some(Value::String(raw)) => serde_json::from_str(raw)
            .map_err(|error| WorkerError::new("INVALID_DATA", error.to_string()))?,
        Some(value) => value.clone(),
    };
    let mut levels = packaging
        .as_array()
        .cloned()
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "PackagingJson must be array"))?;
    for (index, level) in levels.iter_mut().enumerate() {
        let object = level
            .as_object_mut()
            .ok_or_else(|| WorkerError::new("INVALID_DATA", "packaging level must be object"))?;
        object.entry("sort_order").or_insert(json!(index + 1));
    }
    extend(
        body,
        [
            ("entity_id", required(row, "GoodsID")?),
            ("op_type", required(row, "opType")?),
            ("product_code", value(row, "GoodsCode")),
            ("product_name", value(row, "GoodsName")),
            ("approval_no", value(row, "License")),
            ("spec", value(row, "Spec")),
            ("manufacturer", value(row, "ProduceCorp")),
            ("special_drug_category", value(row, "SpecialCategory")),
            ("storage_condition", value(row, "Deposite")),
            ("packaging_levels", Value::Array(levels)),
        ],
    );
    Ok(())
}

fn customer(body: &mut Map<String, Value>, row: &Value) -> Result<(), WorkerError> {
    extend(
        body,
        [
            ("entity_id", required(row, "ClientID")?),
            ("op_type", required(row, "opType")?),
            ("customer_code", value(row, "ClientCode")),
            ("customer_name", value(row, "ClientName")),
            ("customer_type", value(row, "CorpType")),
            ("address", value(row, "Address")),
            ("contact_name", value(row, "LinkMan")),
            ("contact_phone", value(row, "LinkPhone")),
            ("delivery_address", value(row, "DepotAddr")),
            ("delivery_contact", value(row, "DepotMan")),
            ("delivery_phone", value(row, "DepotCall")),
            ("delivery_mode", value(row, "SendWay")),
            (
                "stop_send",
                Value::Bool(integer(row, "StopSend")?.unwrap_or(0) != 0),
            ),
        ],
    );
    Ok(())
}

fn supplier(body: &mut Map<String, Value>, row: &Value) -> Result<(), WorkerError> {
    extend(
        body,
        [
            ("entity_id", required(row, "SupplierID")?),
            ("op_type", required(row, "opType")?),
            ("supplier_code", value(row, "SupplierCode")),
            ("supplier_name", value(row, "SupplierName")),
            ("address", value(row, "Address")),
            ("contact_name", value(row, "LinkMan")),
            ("contact_phone", value(row, "LinkPhone")),
        ],
    );
    Ok(())
}

fn asn(body: &mut Map<String, Value>, row: &Value, children: &[Value]) -> Result<(), WorkerError> {
    let lines = children
        .iter()
        .map(|item| {
            Ok(json!({
                "line_no": required(item, "LineNo")?,
                "product_code": required(item, "GoodsCode")?,
                "expected_qty": decimal4(item, "Amount")?,
                "batch_no": value(item, "BatchNo"),
                "production_date": value(item, "ProduceDate"),
                "expiry_date": value(item, "ValidDate"),
            }))
        })
        .collect::<Result<Vec<_>, WorkerError>>()?;
    extend(
        body,
        [
            ("erp_bill_id", required(row, "ERPBillID")?),
            ("erp_bill_code", required(row, "ERPBillCode")?),
            ("revision", required(row, "Revision")?),
            ("order_type", required(row, "OrderType")?),
            ("partner_type", value(row, "PartnerType")),
            ("partner_code", value(row, "PartnerCode")),
            ("depot_code", required(row, "DepotCode")?),
            ("business_date", required(row, "BusiDate")?),
            ("note_code", value(row, "NoteCode")),
            ("lines", Value::Array(lines)),
        ],
    );
    Ok(())
}

fn outbound(
    body: &mut Map<String, Value>,
    row: &Value,
    children: &[Value],
) -> Result<(), WorkerError> {
    let lines = children
        .iter()
        .map(|item| {
            Ok(json!({
                "line_no": required(item, "LineNo")?,
                "product_code": required(item, "GoodsCode")?,
                "batch_no": required(item, "BatchNo")?,
                "planned_qty": decimal4(item, "Amount")?,
            }))
        })
        .collect::<Result<Vec<_>, WorkerError>>()?;
    extend(
        body,
        [
            ("erp_bill_id", required(row, "ERPBillID")?),
            ("erp_bill_code", required(row, "ERPBillCode")?),
            ("revision", required(row, "Revision")?),
            ("order_type", required(row, "OrderType")?),
            ("customer_code", value(row, "ClientCode")),
            ("depot_code", required(row, "DepotCode")?),
            ("required_ship_at", required(row, "RequiredShipAt")?),
            ("send_mode", value(row, "SendMode")),
            ("erp_address_id", required(row, "ERPAddressID")?),
            ("address_code", required(row, "AddressCode")?),
            ("contact_name", value(row, "LinkMan")),
            ("contact_phone", value(row, "LinkCall")),
            ("address", required(row, "Address")?),
            ("lines", Value::Array(lines)),
        ],
    );
    Ok(())
}

fn cancel(body: &mut Map<String, Value>, row: &Value) -> Result<(), WorkerError> {
    extend(
        body,
        [
            ("command_id", required(row, "CommandID")?),
            ("command_type", required(row, "CommandType")?),
            ("erp_bill_code", required(row, "ERPBillCode")?),
            ("revision", required(row, "Revision")?),
            ("order_type", required(row, "OrderType")?),
            ("memo", value(row, "Memo")),
        ],
    );
    Ok(())
}

fn snapshot(
    body: &mut Map<String, Value>,
    row: &Value,
    children: &[Value],
) -> Result<(), WorkerError> {
    let items = children
        .iter()
        .map(|item| {
            Ok(json!({
                "row_no": required(item, "RowNo")?,
                "product_code": required(item, "GoodsCode")?,
                "batch_no": required(item, "BatchNo")?,
                "expiry_date": value(item, "ValidDate"),
                "location_code": value(item, "StallCode"),
                "goods_status": value(item, "GoodsStatus"),
                "quantity": decimal4(item, "RealAmount")?,
            }))
        })
        .collect::<Result<Vec<_>, WorkerError>>()?;
    extend(
        body,
        [
            ("snapshot_id", required(row, "SnapshotID")?),
            ("depot_code", required(row, "DepotCode")?),
            ("push_type", required(row, "PushType")?),
            ("push_time", required(row, "PushTime")?),
            ("items", Value::Array(items)),
        ],
    );
    Ok(())
}

fn external_ref(row: &Value) -> Result<Value, WorkerError> {
    for field in [
        "ERPBillCode",
        "CommandID",
        "SnapshotID",
        "GoodsCode",
        "ClientCode",
        "SupplierCode",
    ] {
        if row.get(field).is_some_and(|value| !value.is_null()) {
            return required(row, field);
        }
    }
    Err(WorkerError::new(
        "INVALID_DATA",
        "missing external reference",
    ))
}

fn required(row: &Value, field: &str) -> Result<Value, WorkerError> {
    row.get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .ok_or_else(|| WorkerError::new("INVALID_DATA", format!("missing {field}")))
}

fn value(row: &Value, field: &str) -> Value {
    row.get(field).cloned().unwrap_or(Value::Null)
}

fn integer(row: &Value, field: &str) -> Result<Option<i64>, WorkerError> {
    match row.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| WorkerError::new("INVALID_DATA", format!("invalid {field}"))),
        Some(Value::String(value)) => value
            .parse::<i64>()
            .map(Some)
            .map_err(|_| WorkerError::new("INVALID_DATA", format!("invalid {field}"))),
        _ => Err(WorkerError::new("INVALID_DATA", format!("invalid {field}"))),
    }
}

fn decimal4(row: &Value, field: &str) -> Result<Value, WorkerError> {
    let raw = match required(row, field)? {
        Value::String(value) => value,
        value => value.to_string(),
    };
    let value = raw
        .parse::<Decimal>()
        .map_err(|_| WorkerError::new("INVALID_DATA", format!("invalid {field}")))?;
    Ok(Value::String(format!("{value:.4}")))
}

fn extend<const N: usize>(body: &mut Map<String, Value>, values: [(&str, Value); N]) {
    body.extend(
        values
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value)),
    );
}
