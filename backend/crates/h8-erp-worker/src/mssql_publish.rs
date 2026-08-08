use deadpool_tiberius_rustls::{
    deadpool::managed::Object,
    tiberius_rustls::{error::Error as TiberiusError, Query},
    Manager,
};
use serde_json::Value;

use crate::{
    error::WorkerError,
    mssql::{mssql_error, simple_batch, MssqlRepository},
    outbound::{PublishedRecord, PublishedUnit},
};

#[derive(Clone, Copy)]
enum SqlType {
    Text,
    Integer,
    BigInt,
    Decimal4,
    Date,
    DateTime,
}

type SqlField = (&'static str, SqlType);

const ORDER_FEEDBACK_FIELDS: &[SqlField] = &[
    ("IdempotencyKey", SqlType::Text),
    ("ERPBillCode", SqlType::Text),
    ("Revision", SqlType::Integer),
    ("OrderType", SqlType::Integer),
    ("FeedbackType", SqlType::Integer),
    ("CommandID", SqlType::Text),
    ("ResultCount", SqlType::Integer),
    ("ResultCode", SqlType::Text),
    ("ResultMessage", SqlType::Text),
    ("WaybillNo", SqlType::Text),
    ("ExpressCompany", SqlType::Text),
    ("ShipTime", SqlType::DateTime),
    ("FeedbackTime", SqlType::DateTime),
    ("OperatorName", SqlType::Text),
    ("OwnerCode", SqlType::Text),
    ("SchemaVersion", SqlType::Text),
    ("PayloadDigest", SqlType::Text),
    ("CorrelationID", SqlType::Text),
    ("SourceVersion", SqlType::BigInt),
];

const INBOUND_FEEDBACK_FIELDS: &[SqlField] = &[
    ("IdempotencyKey", SqlType::Text),
    ("ERPBillCode", SqlType::Text),
    ("Revision", SqlType::Integer),
    ("LineNo", SqlType::Integer),
    ("GoodsID", SqlType::Integer),
    ("GoodsCode", SqlType::Text),
    ("ExpectedAmount", SqlType::Decimal4),
    ("ActualAmount", SqlType::Decimal4),
    ("RejectAmount", SqlType::Decimal4),
    ("ShortageAmount", SqlType::Decimal4),
    ("RejectReason", SqlType::Text),
    ("ShortageReason", SqlType::Text),
    ("BatchNo", SqlType::Text),
    ("ProduceDate", SqlType::Date),
    ("ValidDate", SqlType::Date),
    ("StallCode", SqlType::Text),
    ("OperatorName", SqlType::Text),
    ("ScanTime", SqlType::DateTime),
    ("OwnerCode", SqlType::Text),
    ("SchemaVersion", SqlType::Text),
    ("PayloadDigest", SqlType::Text),
    ("CorrelationID", SqlType::Text),
    ("SourceVersion", SqlType::BigInt),
];

const OUTBOUND_FEEDBACK_FIELDS: &[SqlField] = &[
    ("IdempotencyKey", SqlType::Text),
    ("ERPBillCode", SqlType::Text),
    ("Revision", SqlType::Integer),
    ("LineNo", SqlType::Integer),
    ("GoodsID", SqlType::Integer),
    ("GoodsCode", SqlType::Text),
    ("BatchNo", SqlType::Text),
    ("ExpectedAmount", SqlType::Decimal4),
    ("PickedAmount", SqlType::Decimal4),
    ("ShippedAmount", SqlType::Decimal4),
    ("OperatorName", SqlType::Text),
    ("OwnerCode", SqlType::Text),
    ("SchemaVersion", SqlType::Text),
    ("PayloadDigest", SqlType::Text),
    ("CorrelationID", SqlType::Text),
    ("SourceVersion", SqlType::BigInt),
];

const WMS_EVENT_FIELDS: &[SqlField] = &[
    ("IdempotencyKey", SqlType::Text),
    ("EventType", SqlType::Text),
    ("SchemaVersion", SqlType::Text),
    ("PayloadJson", SqlType::Text),
    ("EventTime", SqlType::DateTime),
    ("OwnerCode", SqlType::Text),
    ("PayloadDigest", SqlType::Text),
    ("CorrelationID", SqlType::Text),
    ("SourceVersion", SqlType::BigInt),
];

const RECEIVE_HEADER_FIELDS: &[SqlField] = &[
    ("SnapshotID", SqlType::Text),
    ("ReceiveTime", SqlType::DateTime),
    ("TotalCount", SqlType::Integer),
    ("OwnerCode", SqlType::Text),
    ("SchemaVersion", SqlType::Text),
    ("IdempotencyKey", SqlType::Text),
    ("PayloadDigest", SqlType::Text),
    ("CorrelationID", SqlType::Text),
    ("SourceVersion", SqlType::BigInt),
];

const RECEIVE_ITEM_FIELDS: &[SqlField] = &[
    ("SnapshotID", SqlType::Text),
    ("RowNo", SqlType::Integer),
    ("DepotCode", SqlType::Text),
    ("GoodsCode", SqlType::Text),
    ("BatchNo", SqlType::Text),
    ("ValidDate", SqlType::Date),
    ("GoodsStatus", SqlType::Text),
    ("WMSAmount", SqlType::Decimal4),
    ("WMSPickable", SqlType::Decimal4),
    ("WMSAllocated", SqlType::Decimal4),
    ("WMSFrozen", SqlType::Decimal4),
    ("OwnerCode", SqlType::Text),
    ("CorrelationID", SqlType::Text),
    ("IdempotencyKey", SqlType::Text),
];

impl MssqlRepository {
    pub async fn publish(&self, unit: &PublishedUnit) -> Result<(), WorkerError> {
        let mut connection = self
            .pool
            .get()
            .await
            .map_err(|error| mssql_error("connect", error))?;
        simple_batch(&mut connection, "BEGIN TRANSACTION", "begin publish").await?;
        let result = async {
            match unit {
                PublishedUnit::Transaction(records) => {
                    for record in records {
                        insert_main(&mut connection, record).await?;
                    }
                }
                PublishedUnit::HeaderChildren { header, children } => {
                    if insert_main(&mut connection, header).await? {
                        for child in children {
                            insert_child(&mut connection, child).await?;
                        }
                    }
                }
            }
            Ok::<_, WorkerError>(())
        }
        .await;
        if result.is_ok() {
            simple_batch(&mut connection, "COMMIT TRANSACTION", "commit publish").await?;
        } else {
            let _ = simple_batch(
                &mut connection,
                "IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION",
                "rollback publish",
            )
            .await;
        }
        result
    }

    pub async fn has_business_receipt(
        &self,
        message_type: &str,
        owner_code: &str,
        outbox_id: uuid::Uuid,
        external_ref: &str,
    ) -> Result<bool, WorkerError> {
        let table = crate::receipts::interface_receipt_table(message_type)
            .ok_or_else(|| WorkerError::new("H8_WORKER_UNSUPPORTED_MESSAGE", message_type))?;
        let primary_key = outbox_id.to_string();
        let secondary_key = if message_type == "inventory_snapshot" {
            external_ref.to_owned()
        } else {
            primary_key.clone()
        };
        let mut connection = self
            .pool
            .get()
            .await
            .map_err(|error| mssql_error("connect", error))?;
        let mut query = Query::new(format!(
            "SELECT TOP (1) 1 FROM dbo.{table} WHERE OwnerCode=@P1 AND handelflag=5 AND IdempotencyKey IN (@P2,@P3)"
        ));
        query.bind(owner_code.to_owned());
        query.bind(primary_key);
        query.bind(secondary_key);
        let row = query
            .query(&mut *connection)
            .await
            .map_err(|error| mssql_error("read business receipt", error))?
            .into_row()
            .await
            .map_err(|error| mssql_error("read business receipt", error))?;
        Ok(row.is_some())
    }
}

pub fn insert_statement(table: &str) -> Option<String> {
    let fields = insert_fields(table)?;
    let main = table != "x_wmsinter_InventoryReceiveItems";
    let columns = fields
        .iter()
        .map(|(name, _)| format!("[{name}]"))
        .collect::<Vec<_>>();
    let values = fields
        .iter()
        .enumerate()
        .map(|(index, (_, kind))| sql_parameter(index + 1, *kind))
        .collect::<Vec<_>>();
    let controls = if main {
        ", [handelflag], [retry_count], [inserttime]"
    } else {
        ", [inserttime]"
    };
    let control_values = if main {
        ", 0, 0, SYSUTCDATETIME()"
    } else {
        ", SYSUTCDATETIME()"
    };
    Some(format!(
        "INSERT INTO dbo.{table} ({columns}{controls}) VALUES ({values}{control_values});",
        columns = columns.join(", "),
        values = values.join(", "),
    ))
}

fn insert_fields(table: &str) -> Option<&'static [SqlField]> {
    match table {
        "x_wmsinter_OrderFeedback" => Some(ORDER_FEEDBACK_FIELDS),
        "x_wmsinter_InboundFeedback" => Some(INBOUND_FEEDBACK_FIELDS),
        "x_wmsinter_OutboundFeedback" => Some(OUTBOUND_FEEDBACK_FIELDS),
        "x_wmsinter_WmsEvent" => Some(WMS_EVENT_FIELDS),
        "x_wmsinter_InventoryReceiveHeader" => Some(RECEIVE_HEADER_FIELDS),
        "x_wmsinter_InventoryReceiveItems" => Some(RECEIVE_ITEM_FIELDS),
        _ => None,
    }
}

fn sql_parameter(index: usize, kind: SqlType) -> String {
    match kind {
        SqlType::Text => format!("@P{index}"),
        SqlType::Integer => format!("TRY_CONVERT(int, @P{index})"),
        SqlType::BigInt => format!("TRY_CONVERT(bigint, @P{index})"),
        SqlType::Decimal4 => format!("TRY_CONVERT(numeric(19,4), @P{index})"),
        SqlType::Date => format!("TRY_CONVERT(date, @P{index}, 23)"),
        SqlType::DateTime => format!("TRY_CONVERT(datetime2(3), @P{index}, 127)"),
    }
}

async fn insert_main(
    connection: &mut Object<Manager>,
    record: &PublishedRecord,
) -> Result<bool, WorkerError> {
    let owner = record
        .row
        .get("OwnerCode")
        .and_then(Value::as_str)
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "published OwnerCode missing"))?;
    let idempotency = record
        .row
        .get("IdempotencyKey")
        .and_then(Value::as_str)
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "published IdempotencyKey missing"))?;
    let digest = record
        .row
        .get("PayloadDigest")
        .and_then(Value::as_str)
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "published PayloadDigest missing"))?;
    if let Some(existing) = existing_digest(connection, record.table, owner, idempotency).await? {
        if existing == digest {
            return Ok(false);
        }
        return Err(WorkerError::new(
            "IDEMPOTENCY_CONFLICT",
            "published digest conflicts with existing row",
        ));
    }
    match execute_insert(connection, record).await {
        Ok(()) => Ok(true),
        Err(error) if matches!(error.code(), Some(2601 | 2627)) => {
            match existing_digest(connection, record.table, owner, idempotency).await? {
                Some(existing) if existing == digest => Ok(false),
                Some(_) => Err(WorkerError::new(
                    "IDEMPOTENCY_CONFLICT",
                    "published digest conflicts with existing row",
                )),
                None => Err(WorkerError::new(
                    "BUSINESS_KEY_CONFLICT",
                    "published business key already exists",
                )),
            }
        }
        Err(error) => Err(mssql_error("publish", error)),
    }
}

async fn insert_child(
    connection: &mut Object<Manager>,
    record: &PublishedRecord,
) -> Result<(), WorkerError> {
    execute_insert(connection, record)
        .await
        .map_err(|error| mssql_error("publish child", error))
}

async fn execute_insert(
    connection: &mut Object<Manager>,
    record: &PublishedRecord,
) -> Result<(), TiberiusError> {
    let fields = insert_fields(record.table).expect("published table registry is exhaustive");
    let mut query = Query::new(
        insert_statement(record.table).expect("published insert registry is exhaustive"),
    );
    for (field, _) in fields {
        query.bind(sql_value(record.row.get(*field)));
    }
    query.execute(&mut **connection).await?;
    Ok(())
}

async fn existing_digest(
    connection: &mut Object<Manager>,
    table: &str,
    owner: &str,
    idempotency: &str,
) -> Result<Option<String>, WorkerError> {
    if insert_fields(table).is_none() || table == "x_wmsinter_InventoryReceiveItems" {
        return Err(WorkerError::new("H8_WORKER_UNSUPPORTED_TABLE", table));
    }
    let mut query = Query::new(format!(
        "SELECT PayloadDigest FROM dbo.{table} WHERE OwnerCode=@P1 AND IdempotencyKey=@P2"
    ));
    query.bind(owner.to_owned());
    query.bind(idempotency.to_owned());
    let row = query
        .query(&mut **connection)
        .await
        .map_err(|error| mssql_error("read published digest", error))?
        .into_row()
        .await
        .map_err(|error| mssql_error("read published digest", error))?;
    Ok(row.and_then(|row| row.get::<&str, _>(0).map(ToOwned::to_owned)))
}

fn sql_value(value: Option<&Value>) -> Option<String> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.clone()),
        Some(value @ (Value::Object(_) | Value::Array(_))) => Some(value.to_string()),
        Some(value) => Some(value.to_string()),
    }
}
