use std::{net::IpAddr, time::Duration};

use chrono::{DateTime, NaiveDate, NaiveDateTime, SecondsFormat, Utc};
use deadpool_tiberius_rustls::{
    tiberius_rustls::{ColumnType, EncryptionLevel, Query, Row},
    Manager, Pool,
};
use rust_decimal::Decimal;
use serde_json::{Map, Number, Value};

use crate::{config::MssqlSettings, error::WorkerError};

pub use crate::mssql_publish::insert_statement;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChildContract {
    pub table: &'static str,
    pub foreign_key: &'static str,
    pub parent_key: &'static str,
    pub order_key: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TableContract {
    pub table: &'static str,
    pub primary_key: &'static str,
    pub primary_key_sql: &'static str,
    pub child: Option<ChildContract>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarkStatus {
    Accepted,
    Retry,
    Dead,
    Success,
}

#[derive(Clone, Copy, Debug)]
pub struct MarkOutcome<'a> {
    pub status: MarkStatus,
    pub message: Option<&'a str>,
    pub error_code: Option<&'a str>,
    pub retry_count: Option<u32>,
}

#[derive(Debug)]
pub struct ClaimedUnit {
    pub row: Value,
    pub children: Vec<Value>,
}

#[derive(Clone)]
pub struct MssqlRepository {
    pub(crate) pool: Pool,
}

const TABLES: [TableContract; 7] = [
    TableContract {
        table: "x_wmsinter_GoodsInfo",
        primary_key: "seqid",
        primary_key_sql: "int",
        child: None,
    },
    TableContract {
        table: "x_wmsinter_CustomerInfo",
        primary_key: "seqid",
        primary_key_sql: "int",
        child: None,
    },
    TableContract {
        table: "x_wmsinter_SupplierInfo",
        primary_key: "seqid",
        primary_key_sql: "int",
        child: None,
    },
    TableContract {
        table: "x_wmsinter_InboundOrder",
        primary_key: "OrderID",
        primary_key_sql: "int",
        child: Some(ChildContract {
            table: "x_wmsinter_InboundOrderItems",
            foreign_key: "OrderID",
            parent_key: "OrderID",
            order_key: "LineNo",
        }),
    },
    TableContract {
        table: "x_wmsinter_OutboundOrder",
        primary_key: "OrderID",
        primary_key_sql: "int",
        child: Some(ChildContract {
            table: "x_wmsinter_OutboundOrderItems",
            foreign_key: "OrderID",
            parent_key: "OrderID",
            order_key: "LineNo",
        }),
    },
    TableContract {
        table: "x_wmsinter_OrderCommand",
        primary_key: "CommandID",
        primary_key_sql: "varchar(32)",
        child: None,
    },
    TableContract {
        table: "x_wmsinter_InventoryPushHeader",
        primary_key: "PushID",
        primary_key_sql: "int",
        child: Some(ChildContract {
            table: "x_wmsinter_InventoryPushItems",
            foreign_key: "SnapshotID",
            parent_key: "SnapshotID",
            order_key: "RowNo",
        }),
    },
];

pub fn table_contract(table: &str) -> Option<&'static TableContract> {
    TABLES.iter().find(|contract| contract.table == table)
}

impl MssqlRepository {
    pub fn connect(settings: &MssqlSettings) -> Result<Self, WorkerError> {
        let mut manager = Manager::new()
            .host(&settings.host)
            .port(settings.port)
            .database(&settings.database)
            .basic_authentication(&settings.username, &settings.password);
        if is_loopback(&settings.host) {
            manager = manager.encryption(EncryptionLevel::NotSupported);
        } else {
            manager = manager.trust_cert();
        }
        let pool = manager
            .max_size(4)
            .wait_timeout(Duration::from_secs(5))
            .create_timeout(Duration::from_secs(10))
            .recycle_timeout(Duration::from_secs(5))
            .create_pool()
            .map_err(|error| mssql_error("create pool", error))?;
        Ok(Self { pool })
    }

    pub async fn healthcheck(&self) -> Result<(), WorkerError> {
        let mut connection = self
            .pool
            .get()
            .await
            .map_err(|error| mssql_error("connect", error))?;
        Query::new("SELECT 1 AS ok")
            .query(&mut *connection)
            .await
            .map_err(|error| mssql_error("healthcheck", error))?
            .into_first_result()
            .await
            .map_err(|error| mssql_error("healthcheck", error))?;
        Ok(())
    }

    pub async fn claim(
        &self,
        contract: &TableContract,
        batch_size: u32,
        worker_id: &str,
        lease_minutes: u32,
    ) -> Result<Vec<ClaimedUnit>, WorkerError> {
        let mut connection = self
            .pool
            .get()
            .await
            .map_err(|error| mssql_error("connect", error))?;
        simple_batch(&mut connection, "BEGIN TRANSACTION", "begin claim").await?;

        let result = async {
            let mut statement = Query::new(claim_statement(contract));
            statement.bind(i32::try_from(batch_size).unwrap_or(i32::MAX));
            statement.bind(worker_id.to_owned());
            statement.bind(i32::try_from(lease_minutes).unwrap_or(i32::MAX));
            let rows = statement
                .query(&mut *connection)
                .await
                .map_err(|error| mssql_error("claim", error))?
                .into_first_result()
                .await
                .map_err(|error| mssql_error("claim", error))?;
            let mut units = Vec::with_capacity(rows.len());
            for source in rows {
                let row = row_to_json(&source)?;
                let children = if let Some(child) = contract.child {
                    query_children(&mut connection, child, &row).await?
                } else {
                    Vec::new()
                };
                units.push(ClaimedUnit { row, children });
            }
            Ok::<_, WorkerError>(units)
        }
        .await;

        if result.is_ok() {
            simple_batch(&mut connection, "COMMIT TRANSACTION", "commit claim").await?;
        } else {
            let _ = simple_batch(
                &mut connection,
                "IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION",
                "rollback claim",
            )
            .await;
        }
        result
    }

    pub async fn mark(
        &self,
        contract: &TableContract,
        owner_code: &str,
        row_id: &Value,
        outcome: MarkOutcome<'_>,
    ) -> Result<(), WorkerError> {
        let mut connection = self
            .pool
            .get()
            .await
            .map_err(|error| mssql_error("connect", error))?;
        let mut statement = Query::new(mark_statement(contract, outcome.status));
        statement.bind(outcome.message.map(ToOwned::to_owned));
        statement.bind(outcome.error_code.map(ToOwned::to_owned));
        statement.bind(
            outcome
                .retry_count
                .and_then(|value| i32::try_from(value).ok()),
        );
        statement.bind(
            i32::try_from(retry_delay_seconds(outcome.retry_count.unwrap_or(1))).unwrap_or(60),
        );
        statement.bind(owner_code.to_owned());
        bind_row_key(&mut statement, contract, row_id)?;
        statement
            .execute(&mut *connection)
            .await
            .map_err(|error| mssql_error("mark", error))?;
        Ok(())
    }

    pub async fn requeue_manual_replay(
        &self,
        contract: &TableContract,
        owner_code: &str,
        idempotency_key: &str,
    ) -> Result<bool, WorkerError> {
        let mut connection = self
            .pool
            .get()
            .await
            .map_err(|error| mssql_error("connect", error))?;
        let mut query = Query::new(format!(
            "UPDATE dbo.{table} WITH (ROWLOCK) SET handelflag=0, handelmsg=NULL, error_code=NULL, next_retry_at=NULL, worker_id=NULL, lease_until=NULL, processtime=NULL OUTPUT INSERTED.IdempotencyKey WHERE OwnerCode=@P1 AND IdempotencyKey=@P2 AND handelflag IN (3,4)",
            table = contract.table
        ));
        query.bind(owner_code.to_owned());
        query.bind(idempotency_key.to_owned());
        let row = query
            .query(&mut *connection)
            .await
            .map_err(|error| mssql_error("requeue manual replay", error))?
            .into_row()
            .await
            .map_err(|error| mssql_error("requeue manual replay", error))?;
        Ok(row.is_some())
    }
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

pub fn claim_statement(contract: &TableContract) -> String {
    let table = contract.table;
    let pk = contract.primary_key;
    let pk_sql = contract.primary_key_sql;
    format!(
        r#"SET NOCOUNT ON;
DECLARE @claimed TABLE (id {pk_sql});
;WITH claimable AS (
    SELECT TOP (@P1) {pk}
      FROM dbo.{table} WITH (UPDLOCK, READPAST, ROWLOCK)
     WHERE handelflag = 0
        OR (handelflag = 3 AND next_retry_at <= SYSUTCDATETIME())
        OR (handelflag = 2 AND lease_until < SYSUTCDATETIME())
     ORDER BY inserttime, {pk}
)
UPDATE source
   SET handelflag = 2,
       worker_id = @P2,
       lease_until = DATEADD(MINUTE, @P3, SYSUTCDATETIME())
OUTPUT INSERTED.{pk} INTO @claimed
  FROM dbo.{table} source
  JOIN claimable ON claimable.{pk} = source.{pk};
SELECT source.*
  FROM dbo.{table} source
  JOIN @claimed claimed ON source.{pk} = claimed.id
 ORDER BY source.inserttime, source.{pk};"#
    )
}

pub fn mark_statement(contract: &TableContract, status: MarkStatus) -> String {
    let flag = match status {
        MarkStatus::Accepted => 1,
        MarkStatus::Retry => 3,
        MarkStatus::Dead => 4,
        MarkStatus::Success => 5,
    };
    format!(
        r#"UPDATE dbo.{table} WITH (ROWLOCK)
   SET handelflag = {flag},
       handelmsg = @P1,
       error_code = @P2,
       retry_count = COALESCE(@P3, retry_count),
       next_retry_at = CASE WHEN {flag} = 3
            THEN DATEADD(SECOND, @P4, SYSUTCDATETIME()) ELSE NULL END,
       lease_until = NULL,
       processtime = CASE WHEN {flag} IN (1, 4, 5)
            THEN SYSUTCDATETIME() ELSE NULL END
 WHERE OwnerCode = @P5 AND {pk} = @P6;"#,
        table = contract.table,
        pk = contract.primary_key,
    )
}

pub fn retry_delay_seconds(retry_count: u32) -> u32 {
    2_u32
        .saturating_pow(retry_count.saturating_sub(1).min(31))
        .min(60)
}

async fn query_children(
    connection: &mut deadpool_tiberius_rustls::deadpool::managed::Object<Manager>,
    child: ChildContract,
    parent: &Value,
) -> Result<Vec<Value>, WorkerError> {
    let owner = parent
        .get("OwnerCode")
        .and_then(Value::as_str)
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "missing OwnerCode"))?;
    let parent_key = parent
        .get(child.parent_key)
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "missing child parent key"))?;
    let mut statement = Query::new(format!(
        "SELECT * FROM dbo.{table} WHERE OwnerCode = @P1 AND {fk} = @P2 ORDER BY {order}",
        table = child.table,
        fk = child.foreign_key,
        order = child.order_key,
    ));
    statement.bind(owner.to_owned());
    bind_json_value(&mut statement, parent_key)?;
    let rows = statement
        .query(&mut **connection)
        .await
        .map_err(|error| mssql_error("read children", error))?
        .into_first_result()
        .await
        .map_err(|error| mssql_error("read children", error))?;
    rows.iter().map(row_to_json).collect()
}

fn bind_row_key(
    statement: &mut Query<'_>,
    contract: &TableContract,
    row_id: &Value,
) -> Result<(), WorkerError> {
    if contract.primary_key_sql == "int" {
        let value = row_id
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| WorkerError::new("INVALID_DATA", "invalid integer row id"))?;
        statement.bind(value);
    } else {
        let value = row_id
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| WorkerError::new("INVALID_DATA", "invalid text row id"))?;
        statement.bind(value.to_owned());
    }
    Ok(())
}

fn bind_json_value(statement: &mut Query<'_>, value: &Value) -> Result<(), WorkerError> {
    match value {
        Value::String(value) => statement.bind(value.clone()),
        Value::Number(value) => {
            let value = value
                .as_i64()
                .and_then(|value| i32::try_from(value).ok())
                .ok_or_else(|| WorkerError::new("INVALID_DATA", "invalid SQL key"))?;
            statement.bind(value);
        }
        _ => return Err(WorkerError::new("INVALID_DATA", "invalid SQL key")),
    }
    Ok(())
}

fn row_to_json(row: &Row) -> Result<Value, WorkerError> {
    let mut object = Map::with_capacity(row.len());
    for (index, column) in row.columns().iter().enumerate() {
        let value = match column.column_type() {
            ColumnType::Null => Value::Null,
            ColumnType::Bit | ColumnType::Bitn => {
                option_value(row.try_get::<bool, _>(index)?, Value::Bool)
            }
            ColumnType::Int1 => option_value(row.try_get::<u8, _>(index)?, |value| {
                Number::from(value).into()
            }),
            ColumnType::Int2 => option_value(row.try_get::<i16, _>(index)?, |value| {
                Number::from(value).into()
            }),
            ColumnType::Int4 | ColumnType::Intn => {
                option_value(row.try_get::<i32, _>(index)?, |value| {
                    Number::from(value).into()
                })
            }
            ColumnType::Int8 => option_value(row.try_get::<i64, _>(index)?, |value| {
                Number::from(value).into()
            }),
            ColumnType::Decimaln
            | ColumnType::Numericn
            | ColumnType::Money
            | ColumnType::Money4 => option_value(row.try_get::<Decimal, _>(index)?, |value| {
                Value::String(value.to_string())
            }),
            ColumnType::Float4 => option_value(row.try_get::<f32, _>(index)?, |value| {
                Number::from_f64(f64::from(value))
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }),
            ColumnType::Float8 | ColumnType::Floatn => {
                option_value(row.try_get::<f64, _>(index)?, |value| {
                    Number::from_f64(value)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                })
            }
            ColumnType::Daten => option_value(row.try_get::<NaiveDate, _>(index)?, |value| {
                Value::String(value.format("%Y-%m-%d").to_string())
            }),
            ColumnType::Datetime2 | ColumnType::DatetimeOffsetn => {
                option_value(row.try_get::<DateTime<Utc>, _>(index)?, |value| {
                    Value::String(value.to_rfc3339_opts(SecondsFormat::Millis, true))
                })
            }
            ColumnType::Datetime | ColumnType::Datetime4 | ColumnType::Datetimen => {
                option_value(row.try_get::<NaiveDateTime, _>(index)?, |value| {
                    Value::String(value.and_utc().to_rfc3339_opts(SecondsFormat::Millis, true))
                })
            }
            ColumnType::Guid => option_value(row.try_get::<uuid::Uuid, _>(index)?, |value| {
                Value::String(value.to_string())
            }),
            ColumnType::BigVarChar
            | ColumnType::BigChar
            | ColumnType::NVarchar
            | ColumnType::NChar
            | ColumnType::Text
            | ColumnType::NText
            | ColumnType::Xml => option_value(row.try_get::<&str, _>(index)?, |value| {
                Value::String(value.to_owned())
            }),
            other => {
                return Err(WorkerError::new(
                    "INVALID_DATA",
                    format!("unsupported MSSQL column type {other:?}"),
                ))
            }
        };
        object.insert(column.name().to_owned(), value);
    }
    Ok(Value::Object(object))
}

fn option_value<T>(value: Option<T>, map: impl FnOnce(T) -> Value) -> Value {
    value.map(map).unwrap_or(Value::Null)
}

pub(crate) fn mssql_error(context: &str, error: impl std::fmt::Display) -> WorkerError {
    WorkerError::new("H8_WORKER_MSSQL_FAILED", format!("{context}: {error}"))
}

pub(crate) async fn simple_batch(
    connection: &mut deadpool_tiberius_rustls::deadpool::managed::Object<Manager>,
    sql: &str,
    context: &str,
) -> Result<(), WorkerError> {
    connection
        .simple_query(sql)
        .await
        .map_err(|error| mssql_error(context, error))?
        .into_results()
        .await
        .map_err(|error| mssql_error(context, error))?;
    Ok(())
}
