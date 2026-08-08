use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};
use rust_decimal::Decimal;
use serde_json::{Number, Value};
use sha2::{Digest, Sha256};

use crate::error::WorkerError;

#[derive(Clone, Copy)]
enum Kind {
    Text,
    Integer,
    Decimal(u32),
    DateTime,
    Json,
}

type Field = (&'static str, Kind);

const GOODS: &[Field] = &[
    ("GoodsID", Kind::Integer),
    ("GoodsCode", Kind::Text),
    ("GoodsName", Kind::Text),
    ("SubName", Kind::Text),
    ("ClassCode", Kind::Text),
    ("BarCode", Kind::Text),
    ("Spec", Kind::Text),
    ("Unit", Kind::Text),
    ("Brand", Kind::Text),
    ("ProduceArea", Kind::Text),
    ("License", Kind::Text),
    ("IsDanger", Kind::Integer),
    ("ValidityType", Kind::Text),
    ("ValidityNum", Kind::Integer),
    ("RetailPrice", Kind::Decimal(6)),
    ("TaxRate", Kind::Integer),
    ("ProduceCorp", Kind::Text),
    ("StoreMemo", Kind::Text),
    ("Deposite", Kind::Text),
    ("MedicalType", Kind::Text),
    ("PackagingJson", Kind::Json),
    ("opType", Kind::Text),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("IdempotencyKey", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const CUSTOMER: &[Field] = &[
    ("ClientID", Kind::Integer),
    ("ClientCode", Kind::Text),
    ("ClientName", Kind::Text),
    ("SubCorpName", Kind::Text),
    ("CorpType", Kind::Text),
    ("Area", Kind::Text),
    ("Address", Kind::Text),
    ("Lawman", Kind::Text),
    ("PostCode", Kind::Text),
    ("LinkMan", Kind::Text),
    ("LinkPhone", Kind::Text),
    ("DepotAddr", Kind::Text),
    ("DepotMan", Kind::Text),
    ("DepotCall", Kind::Text),
    ("SendWay", Kind::Integer),
    ("StopSend", Kind::Integer),
    ("opType", Kind::Text),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("IdempotencyKey", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const SUPPLIER: &[Field] = &[
    ("SupplierID", Kind::Integer),
    ("SupplierCode", Kind::Text),
    ("SupplierName", Kind::Text),
    ("Lawman", Kind::Text),
    ("Address", Kind::Text),
    ("LinkMan", Kind::Text),
    ("LinkPhone", Kind::Text),
    ("opType", Kind::Text),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("IdempotencyKey", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const INBOUND_ORDER: &[Field] = &[
    ("ERPBillID", Kind::Integer),
    ("ERPBillCode", Kind::Text),
    ("Revision", Kind::Integer),
    ("OrderType", Kind::Integer),
    ("PartnerType", Kind::Text),
    ("PartnerID", Kind::Integer),
    ("PartnerCode", Kind::Text),
    ("PartnerName", Kind::Text),
    ("DepotID", Kind::Integer),
    ("DepotCode", Kind::Text),
    ("DeptID", Kind::Integer),
    ("BusiDate", Kind::Text),
    ("SumMoney", Kind::Decimal(4)),
    ("NoteCode", Kind::Text),
    ("LineCount", Kind::Integer),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("IdempotencyKey", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const INBOUND_ITEM: &[Field] = &[
    ("OrderID", Kind::Integer),
    ("ERPBillID", Kind::Integer),
    ("ERPBillCode", Kind::Text),
    ("Revision", Kind::Integer),
    ("LineNo", Kind::Integer),
    ("GoodsID", Kind::Integer),
    ("GoodsCode", Kind::Text),
    ("GoodsName", Kind::Text),
    ("Amount", Kind::Decimal(4)),
    ("Price", Kind::Decimal(8)),
    ("Sums", Kind::Decimal(4)),
    ("BatchNo", Kind::Text),
    ("ProduceDate", Kind::Text),
    ("ValidDate", Kind::Text),
    ("Unit", Kind::Text),
    ("OwnerCode", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("IdempotencyKey", Kind::Text),
];
const OUTBOUND_ORDER: &[Field] = &[
    ("ERPBillID", Kind::Integer),
    ("ERPBillCode", Kind::Text),
    ("Revision", Kind::Integer),
    ("OrderType", Kind::Integer),
    ("ClientID", Kind::Integer),
    ("ClientCode", Kind::Text),
    ("ClientName", Kind::Text),
    ("DepotID", Kind::Integer),
    ("DepotCode", Kind::Text),
    ("DeptID", Kind::Integer),
    ("BusiDate", Kind::Text),
    ("RequiredShipAt", Kind::DateTime),
    ("SumMoney", Kind::Decimal(4)),
    ("SumTax", Kind::Decimal(4)),
    ("SendMode", Kind::Integer),
    ("ERPAddressID", Kind::Integer),
    ("AddressCode", Kind::Text),
    ("LinkMan", Kind::Text),
    ("LinkCall", Kind::Text),
    ("Address", Kind::Text),
    ("PostCode", Kind::Text),
    ("IsTight", Kind::Integer),
    ("SellType", Kind::Integer),
    ("LineCount", Kind::Integer),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("IdempotencyKey", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const OUTBOUND_ITEM: &[Field] = &[
    ("OrderID", Kind::Integer),
    ("ERPBillID", Kind::Integer),
    ("ERPBillCode", Kind::Text),
    ("Revision", Kind::Integer),
    ("LineNo", Kind::Integer),
    ("GoodsID", Kind::Integer),
    ("GoodsCode", Kind::Text),
    ("GoodsName", Kind::Text),
    ("Amount", Kind::Decimal(4)),
    ("Price", Kind::Decimal(8)),
    ("Sums", Kind::Decimal(4)),
    ("BatchNo", Kind::Text),
    ("Unit", Kind::Text),
    ("OwnerCode", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("IdempotencyKey", Kind::Text),
];
const ORDER_COMMAND: &[Field] = &[
    ("CommandID", Kind::Text),
    ("CommandType", Kind::Integer),
    ("ERPBillCode", Kind::Text),
    ("Revision", Kind::Integer),
    ("OrderType", Kind::Integer),
    ("Memo", Kind::Text),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("IdempotencyKey", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const ORDER_FEEDBACK: &[Field] = &[
    ("IdempotencyKey", Kind::Text),
    ("ERPBillCode", Kind::Text),
    ("Revision", Kind::Integer),
    ("OrderType", Kind::Integer),
    ("FeedbackType", Kind::Integer),
    ("CommandID", Kind::Text),
    ("ResultCount", Kind::Integer),
    ("ResultCode", Kind::Text),
    ("ResultMessage", Kind::Text),
    ("WaybillNo", Kind::Text),
    ("ExpressCompany", Kind::Text),
    ("ShipTime", Kind::DateTime),
    ("FeedbackTime", Kind::DateTime),
    ("OperatorName", Kind::Text),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const INBOUND_FEEDBACK: &[Field] = &[
    ("IdempotencyKey", Kind::Text),
    ("ERPBillCode", Kind::Text),
    ("Revision", Kind::Integer),
    ("LineNo", Kind::Integer),
    ("GoodsID", Kind::Integer),
    ("GoodsCode", Kind::Text),
    ("ExpectedAmount", Kind::Decimal(4)),
    ("ActualAmount", Kind::Decimal(4)),
    ("RejectAmount", Kind::Decimal(4)),
    ("ShortageAmount", Kind::Decimal(4)),
    ("RejectReason", Kind::Text),
    ("ShortageReason", Kind::Text),
    ("BatchNo", Kind::Text),
    ("ProduceDate", Kind::Text),
    ("ValidDate", Kind::Text),
    ("StallCode", Kind::Text),
    ("OperatorName", Kind::Text),
    ("ScanTime", Kind::DateTime),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const OUTBOUND_FEEDBACK: &[Field] = &[
    ("IdempotencyKey", Kind::Text),
    ("ERPBillCode", Kind::Text),
    ("Revision", Kind::Integer),
    ("LineNo", Kind::Integer),
    ("GoodsID", Kind::Integer),
    ("GoodsCode", Kind::Text),
    ("BatchNo", Kind::Text),
    ("ExpectedAmount", Kind::Decimal(4)),
    ("PickedAmount", Kind::Decimal(4)),
    ("ShippedAmount", Kind::Decimal(4)),
    ("OperatorName", Kind::Text),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const WMS_EVENT: &[Field] = &[
    ("IdempotencyKey", Kind::Text),
    ("EventType", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("PayloadJson", Kind::Json),
    ("EventTime", Kind::DateTime),
    ("OwnerCode", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const PUSH_HEADER: &[Field] = &[
    ("SnapshotID", Kind::Text),
    ("DepotID", Kind::Integer),
    ("DepotCode", Kind::Text),
    ("PushType", Kind::Integer),
    ("PushTime", Kind::DateTime),
    ("TotalCount", Kind::Integer),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("IdempotencyKey", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const PUSH_ITEM: &[Field] = &[
    ("SnapshotID", Kind::Text),
    ("RowNo", Kind::Integer),
    ("GoodsID", Kind::Integer),
    ("GoodsCode", Kind::Text),
    ("BatchID", Kind::Integer),
    ("BatchNo", Kind::Text),
    ("ValidDate", Kind::Text),
    ("StallCode", Kind::Text),
    ("GoodsStatus", Kind::Text),
    ("RealAmount", Kind::Decimal(4)),
    ("CanSell", Kind::Decimal(4)),
    ("OwnerCode", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("IdempotencyKey", Kind::Text),
];
const RECEIVE_HEADER: &[Field] = &[
    ("SnapshotID", Kind::Text),
    ("ReceiveTime", Kind::DateTime),
    ("TotalCount", Kind::Integer),
    ("OwnerCode", Kind::Text),
    ("SchemaVersion", Kind::Text),
    ("IdempotencyKey", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("SourceVersion", Kind::Integer),
];
const RECEIVE_ITEM: &[Field] = &[
    ("SnapshotID", Kind::Text),
    ("RowNo", Kind::Integer),
    ("DepotCode", Kind::Text),
    ("GoodsCode", Kind::Text),
    ("BatchNo", Kind::Text),
    ("ValidDate", Kind::Text),
    ("GoodsStatus", Kind::Text),
    ("WMSAmount", Kind::Decimal(4)),
    ("WMSPickable", Kind::Decimal(4)),
    ("WMSAllocated", Kind::Decimal(4)),
    ("WMSFrozen", Kind::Decimal(4)),
    ("OwnerCode", Kind::Text),
    ("CorrelationID", Kind::Text),
    ("IdempotencyKey", Kind::Text),
];

pub fn canonical_payload_json(
    table: &str,
    row: &Value,
    children: &[Value],
) -> Result<String, WorkerError> {
    let head = canonical_record_json(table, row)?;
    let Some((child_table, order_field)) = child_contract(table) else {
        return Ok(head);
    };
    let mut ordered = children.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|child| integer_value(child.get(order_field)).unwrap_or(i64::MAX));
    let mut records = Vec::with_capacity(ordered.len() + 1);
    records.push(head);
    for child in ordered {
        records.push(canonical_record_json(child_table, child)?);
    }
    Ok(format!("[{}]", records.join(",")))
}

pub fn payload_digest(table: &str, row: &Value, children: &[Value]) -> Result<String, WorkerError> {
    Ok(sha256_hex(
        canonical_payload_json(table, row, children)?.as_bytes(),
    ))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn validate_published_unit(
    table: &str,
    row: &Value,
    children: &[Value],
) -> Result<(), WorkerError> {
    if let Some((_child_table, _)) = child_contract(table) {
        let count_field = if row.get("TotalCount").is_some() {
            "TotalCount"
        } else {
            "LineCount"
        };
        let expected = integer_value(row.get(count_field))
            .ok_or_else(|| WorkerError::new("INVALID_DATA", format!("missing {count_field}")))?;
        if usize::try_from(expected).ok() != Some(children.len()) {
            return Err(WorkerError::new(
                "LINE_COUNT_MISMATCH",
                format!("{count_field}={expected}, actual={}", children.len()),
            ));
        }
        for child in children {
            if child.get("OwnerCode") != row.get("OwnerCode")
                || child.get("CorrelationID") != row.get("CorrelationID")
            {
                return Err(WorkerError::new("INVALID_DATA", "child envelope mismatch"));
            }
            if row.get("Revision").is_some() && child.get("Revision") != row.get("Revision") {
                return Err(WorkerError::new("INVALID_DATA", "child revision mismatch"));
            }
            if row.get("SnapshotID").is_some() && child.get("SnapshotID") != row.get("SnapshotID") {
                return Err(WorkerError::new("INVALID_DATA", "child snapshot mismatch"));
            }
        }
    }
    let expected = row
        .get("PayloadDigest")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if payload_digest(table, row, children)? != expected {
        return Err(WorkerError::new("INVALID_DATA", "PayloadDigest mismatch"));
    }
    Ok(())
}

fn canonical_record_json(table: &str, row: &Value) -> Result<String, WorkerError> {
    let fields = field_specs(table).ok_or_else(|| {
        WorkerError::new(
            "H8_WORKER_UNSUPPORTED_TABLE",
            format!("unsupported v1.9 table: {table}"),
        )
    })?;
    let mut pairs = Vec::with_capacity(fields.len());
    for (name, kind) in fields {
        let key = serde_json::to_string(name).map_err(json_error)?;
        let value = canonical_value(row.get(*name).unwrap_or(&Value::Null), *kind)?;
        let value = serde_json::to_string(&value).map_err(json_error)?;
        pairs.push(format!("{key}:{value}"));
    }
    Ok(format!("{{{}}}", pairs.join(",")))
}

fn canonical_value(value: &Value, kind: Kind) -> Result<Value, WorkerError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    match kind {
        Kind::Text => Ok(Value::String(text_value(value))),
        Kind::Integer => integer_value(Some(value))
            .map(Number::from)
            .map(Value::Number)
            .ok_or_else(|| WorkerError::new("INVALID_DATA", "invalid integer")),
        Kind::Decimal(scale) => {
            let number = text_value(value).parse::<Decimal>().map_err(|_| {
                WorkerError::new("INVALID_DATA", format!("invalid decimal: {value}"))
            })?;
            Ok(Value::String(format!(
                "{number:.precision$}",
                precision = scale as usize
            )))
        }
        Kind::DateTime => Ok(Value::String(canonical_datetime(&text_value(value))?)),
        Kind::Json => {
            let compact = match value {
                Value::String(raw) => compact_json(raw)?,
                other => serde_json::to_string(other).map_err(json_error)?,
            };
            Ok(Value::String(compact))
        }
    }
}

fn canonical_datetime(raw: &str) -> Result<String, WorkerError> {
    if let Ok(value) = DateTime::parse_from_rfc3339(raw) {
        return Ok(value
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Millis, true));
    }
    for pattern in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(value) = NaiveDateTime::parse_from_str(raw, pattern) {
            return Ok(value.and_utc().to_rfc3339_opts(SecondsFormat::Millis, true));
        }
    }
    Err(WorkerError::new(
        "INVALID_DATA",
        format!("invalid datetime: {raw}"),
    ))
}

fn compact_json(raw: &str) -> Result<String, WorkerError> {
    serde_json::from_str::<Value>(raw).map_err(json_error)?;
    let mut compact = String::with_capacity(raw.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in raw.chars() {
        if in_string {
            compact.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
        } else if character == '"' {
            in_string = true;
            compact.push(character);
        } else if !character.is_whitespace() {
            compact.push(character);
        }
    }
    Ok(compact)
}

fn text_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn integer_value(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(value) => value.as_i64(),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn json_error(error: serde_json::Error) -> WorkerError {
    WorkerError::new("INVALID_DATA", error.to_string())
}

fn child_contract(table: &str) -> Option<(&'static str, &'static str)> {
    match table {
        "x_wmsinter_InboundOrder" => Some(("x_wmsinter_InboundOrderItems", "LineNo")),
        "x_wmsinter_OutboundOrder" => Some(("x_wmsinter_OutboundOrderItems", "LineNo")),
        "x_wmsinter_InventoryPushHeader" => Some(("x_wmsinter_InventoryPushItems", "RowNo")),
        "x_wmsinter_InventoryReceiveHeader" => Some(("x_wmsinter_InventoryReceiveItems", "RowNo")),
        _ => None,
    }
}

fn field_specs(table: &str) -> Option<&'static [Field]> {
    match table {
        "x_wmsinter_GoodsInfo" => Some(GOODS),
        "x_wmsinter_CustomerInfo" => Some(CUSTOMER),
        "x_wmsinter_SupplierInfo" => Some(SUPPLIER),
        "x_wmsinter_InboundOrder" => Some(INBOUND_ORDER),
        "x_wmsinter_InboundOrderItems" => Some(INBOUND_ITEM),
        "x_wmsinter_OutboundOrder" => Some(OUTBOUND_ORDER),
        "x_wmsinter_OutboundOrderItems" => Some(OUTBOUND_ITEM),
        "x_wmsinter_OrderCommand" => Some(ORDER_COMMAND),
        "x_wmsinter_OrderFeedback" => Some(ORDER_FEEDBACK),
        "x_wmsinter_InboundFeedback" => Some(INBOUND_FEEDBACK),
        "x_wmsinter_OutboundFeedback" => Some(OUTBOUND_FEEDBACK),
        "x_wmsinter_WmsEvent" => Some(WMS_EVENT),
        "x_wmsinter_InventoryPushHeader" => Some(PUSH_HEADER),
        "x_wmsinter_InventoryPushItems" => Some(PUSH_ITEM),
        "x_wmsinter_InventoryReceiveHeader" => Some(RECEIVE_HEADER),
        "x_wmsinter_InventoryReceiveItems" => Some(RECEIVE_ITEM),
        _ => None,
    }
}
