use std::{collections::HashSet, net::IpAddr, time::Duration};

use deadpool_tiberius_rustls::{
    tiberius_rustls::{EncryptionLevel, Query},
    Manager,
};
use wms_domain::{sanitize_error_summary, validate_direction_message_selection, H8ErpConnector};

pub(super) struct InterfaceTableContract {
    pub(super) table: &'static str,
    pub(super) columns: &'static [&'static str],
    pub(super) permissions: &'static [&'static str],
}

pub(super) const INBOUND_PERMISSIONS: &[&str] = &["SELECT"];
pub(super) const CHILD_READ_PERMISSIONS: &[&str] = &["SELECT"];
pub(super) const OUTBOUND_PERMISSIONS: &[&str] = &["INSERT", "SELECT"];
const CONTROL_UPDATE_COLUMNS: &[&str] = &[
    "handelflag",
    "handelmsg",
    "error_code",
    "retry_count",
    "next_retry_at",
    "worker_id",
    "lease_until",
    "processtime",
];
const CONTROL_COLUMNS: &[&str] = &[
    "OwnerCode",
    "SchemaVersion",
    "IdempotencyKey",
    "PayloadDigest",
    "CorrelationID",
    "SourceVersion",
    "handelflag",
    "handelmsg",
    "error_code",
    "retry_count",
    "next_retry_at",
    "worker_id",
    "lease_until",
    "inserttime",
    "processtime",
];
const GOODS_COLUMNS: &[&str] = &[
    "seqid",
    "GoodsID",
    "GoodsCode",
    "GoodsName",
    "SubName",
    "ClassCode",
    "BarCode",
    "Spec",
    "Unit",
    "Brand",
    "ProduceArea",
    "License",
    "IsDanger",
    "IsImport",
    "IsTCM",
    "ValidityType",
    "ValidityNum",
    "RetailPrice",
    "TaxRate",
    "ProduceCorp",
    "StoreMemo",
    "Deposite",
    "MedicalType",
    "SpecialCategory",
    "PackagingJson",
    "opType",
];
const CUSTOMER_COLUMNS: &[&str] = &[
    "seqid",
    "ClientID",
    "ClientCode",
    "ClientName",
    "SubCorpName",
    "CorpType",
    "Area",
    "Address",
    "Lawman",
    "PostCode",
    "LinkMan",
    "LinkPhone",
    "DepotAddr",
    "DepotMan",
    "DepotCall",
    "SendWay",
    "StopSend",
    "opType",
];
const SUPPLIER_COLUMNS: &[&str] = &[
    "seqid",
    "SupplierID",
    "SupplierCode",
    "SupplierName",
    "Lawman",
    "Address",
    "LinkMan",
    "LinkPhone",
    "opType",
];
const INBOUND_ORDER_COLUMNS: &[&str] = &[
    "OrderID",
    "ERPBillID",
    "ERPBillCode",
    "Revision",
    "OrderType",
    "PartnerType",
    "PartnerID",
    "PartnerCode",
    "PartnerName",
    "DepotID",
    "DepotCode",
    "DeptID",
    "BusiDate",
    "SumMoney",
    "NoteCode",
    "LineCount",
];
const INBOUND_ITEM_COLUMNS: &[&str] = &[
    "ItemID",
    "OrderID",
    "ERPBillID",
    "ERPBillCode",
    "Revision",
    "LineNo",
    "GoodsID",
    "GoodsCode",
    "GoodsName",
    "Amount",
    "Price",
    "Sums",
    "BatchNo",
    "ProduceDate",
    "ValidDate",
    "Unit",
    "OwnerCode",
    "CorrelationID",
    "IdempotencyKey",
    "inserttime",
];
const OUTBOUND_ORDER_COLUMNS: &[&str] = &[
    "OrderID",
    "ERPBillID",
    "ERPBillCode",
    "Revision",
    "OrderType",
    "ClientID",
    "ClientCode",
    "ClientName",
    "DepotID",
    "DepotCode",
    "DeptID",
    "BusiDate",
    "RequiredShipAt",
    "SumMoney",
    "SumTax",
    "SendMode",
    "ERPAddressID",
    "AddressCode",
    "LinkMan",
    "LinkCall",
    "Address",
    "PostCode",
    "IsTight",
    "SellType",
    "LineCount",
];
const OUTBOUND_ITEM_COLUMNS: &[&str] = &[
    "ItemID",
    "OrderID",
    "ERPBillID",
    "ERPBillCode",
    "Revision",
    "LineNo",
    "GoodsID",
    "GoodsCode",
    "GoodsName",
    "Amount",
    "Price",
    "Sums",
    "BatchNo",
    "Unit",
    "OwnerCode",
    "CorrelationID",
    "IdempotencyKey",
    "inserttime",
];
const ORDER_COMMAND_COLUMNS: &[&str] = &[
    "CommandID",
    "CommandType",
    "ERPBillCode",
    "Revision",
    "OrderType",
    "Memo",
];
const ORDER_FEEDBACK_COLUMNS: &[&str] = &[
    "FeedbackID",
    "IdempotencyKey",
    "ERPBillCode",
    "Revision",
    "OrderType",
    "FeedbackType",
    "CommandID",
    "ResultCount",
    "ResultCode",
    "ResultMessage",
    "WaybillNo",
    "ExpressCompany",
    "ShipTime",
    "FeedbackTime",
    "OperatorName",
];
const INBOUND_FEEDBACK_COLUMNS: &[&str] = &[
    "FeedbackID",
    "IdempotencyKey",
    "ERPBillCode",
    "Revision",
    "LineNo",
    "GoodsID",
    "GoodsCode",
    "ExpectedAmount",
    "ActualAmount",
    "RejectAmount",
    "ShortageAmount",
    "RejectReason",
    "ShortageReason",
    "BatchNo",
    "ProduceDate",
    "ValidDate",
    "StallCode",
    "OperatorName",
    "ScanTime",
];
const OUTBOUND_FEEDBACK_COLUMNS: &[&str] = &[
    "FeedbackID",
    "IdempotencyKey",
    "ERPBillCode",
    "Revision",
    "LineNo",
    "GoodsID",
    "GoodsCode",
    "BatchNo",
    "ExpectedAmount",
    "PickedAmount",
    "ShippedAmount",
    "OperatorName",
];
const EVENT_COLUMNS: &[&str] = &[
    "EventID",
    "IdempotencyKey",
    "EventType",
    "SchemaVersion",
    "PayloadJson",
    "EventTime",
];
const PUSH_HEADER_COLUMNS: &[&str] = &[
    "PushID",
    "SnapshotID",
    "DepotID",
    "DepotCode",
    "PushType",
    "PushTime",
    "TotalCount",
];
const PUSH_ITEM_COLUMNS: &[&str] = &[
    "ItemID",
    "SnapshotID",
    "RowNo",
    "GoodsID",
    "GoodsCode",
    "BatchID",
    "BatchNo",
    "ValidDate",
    "StallCode",
    "GoodsStatus",
    "RealAmount",
    "CanSell",
    "OwnerCode",
    "CorrelationID",
    "IdempotencyKey",
    "inserttime",
];
const RECEIVE_HEADER_COLUMNS: &[&str] = &["ReceiveID", "SnapshotID", "ReceiveTime", "TotalCount"];
const RECEIVE_ITEM_COLUMNS: &[&str] = &[
    "ItemID",
    "SnapshotID",
    "RowNo",
    "DepotCode",
    "GoodsCode",
    "BatchNo",
    "ValidDate",
    "GoodsStatus",
    "WMSAmount",
    "WMSPickable",
    "WMSAllocated",
    "WMSFrozen",
    "OwnerCode",
    "CorrelationID",
    "IdempotencyKey",
    "inserttime",
];

const INTERFACE_TABLE_CONTRACTS: &[InterfaceTableContract] = &[
    InterfaceTableContract {
        table: "x_wmsinter_GoodsInfo",
        columns: GOODS_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_CustomerInfo",
        columns: CUSTOMER_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_SupplierInfo",
        columns: SUPPLIER_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_InboundOrder",
        columns: INBOUND_ORDER_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_InboundOrderItems",
        columns: INBOUND_ITEM_COLUMNS,
        permissions: CHILD_READ_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_OutboundOrder",
        columns: OUTBOUND_ORDER_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_OutboundOrderItems",
        columns: OUTBOUND_ITEM_COLUMNS,
        permissions: CHILD_READ_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_OrderCommand",
        columns: ORDER_COMMAND_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_InventoryPushHeader",
        columns: PUSH_HEADER_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_InventoryPushItems",
        columns: PUSH_ITEM_COLUMNS,
        permissions: CHILD_READ_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_OrderFeedback",
        columns: ORDER_FEEDBACK_COLUMNS,
        permissions: OUTBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_InboundFeedback",
        columns: INBOUND_FEEDBACK_COLUMNS,
        permissions: OUTBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_OutboundFeedback",
        columns: OUTBOUND_FEEDBACK_COLUMNS,
        permissions: OUTBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_WmsEvent",
        columns: EVENT_COLUMNS,
        permissions: OUTBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_InventoryReceiveHeader",
        columns: RECEIVE_HEADER_COLUMNS,
        permissions: OUTBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "x_wmsinter_InventoryReceiveItems",
        columns: RECEIVE_ITEM_COLUMNS,
        permissions: OUTBOUND_PERMISSIONS,
    },
];

pub(super) async fn interface_table_probe(connector: &H8ErpConnector) -> Result<(), String> {
    let host = required_field(connector.interface_db_host.as_deref(), "interface_db_host")?;
    let port = connector
        .interface_db_port
        .filter(|value| (1..=65_535).contains(value))
        .ok_or_else(|| "interface_db_port missing".to_string())? as u16;
    let database = required_field(connector.interface_db_name.as_deref(), "interface_db_name")?;
    let username = required_field(
        connector.interface_db_username.as_deref(),
        "interface_db_username",
    )?;
    let password = crate::secrets::resolve_secret_alias_for_probe(
        connector.interface_db_password_alias.as_deref(),
    )
    .map_err(|message| format!("interface transport secret: {message}"))?;
    let contracts = required_interface_contracts(connector)?;

    let mut manager = Manager::new()
        .host(host)
        .port(port)
        .database(database)
        .basic_authentication(username, &password);
    if is_loopback_interface_host(host) {
        manager = manager.encryption(EncryptionLevel::NotSupported);
    }
    let pool = manager
        .max_size(1)
        .wait_timeout(Duration::from_secs(2))
        .create_timeout(Duration::from_secs(5))
        .recycle_timeout(Duration::from_secs(2))
        .create_pool()
        .map_err(|error| {
            sanitize_probe_error("interface db pool", &error.to_string(), &password)
        })?;
    let mut connection = tokio::time::timeout(Duration::from_secs(6), pool.get())
        .await
        .map_err(|_| "interface db login timeout".to_string())?
        .map_err(|error| {
            sanitize_probe_error("interface db login", &error.to_string(), &password)
        })?;

    tokio::time::timeout(Duration::from_secs(5), async {
        for contract in contracts {
            validate_columns(&mut connection, contract, &password).await?;
            validate_permissions(&mut connection, contract, &password).await?;
        }
        Ok(())
    })
    .await
    .map_err(|_| "interface schema probe timeout".to_string())?
}

pub(super) fn is_loopback_interface_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

async fn validate_columns(
    connection: &mut deadpool_tiberius_rustls::deadpool::managed::Object<Manager>,
    contract: &InterfaceTableContract,
    password: &str,
) -> Result<(), String> {
    let mut query = Query::new(
        "SELECT c.name AS column_name
           FROM sys.columns c
          WHERE c.object_id = OBJECT_ID(N'dbo.' + @P1, N'U')"
            .to_string(),
    );
    query.bind(contract.table.to_string());
    let rows = query
        .query(&mut **connection)
        .await
        .map_err(|error| {
            sanitize_probe_error("interface table columns", &error.to_string(), password)
        })?
        .into_first_result()
        .await
        .map_err(|error| {
            sanitize_probe_error("interface table columns", &error.to_string(), password)
        })?;
    let found = rows
        .iter()
        .filter_map(|row| row.get::<&str, _>("column_name"))
        .collect::<HashSet<_>>();
    if found.is_empty() {
        return Err(format!("interface table missing: dbo.{}", contract.table));
    }
    let required = contract.columns.iter().copied().chain(
        requires_control_columns(contract.table)
            .then_some(CONTROL_COLUMNS)
            .into_iter()
            .flatten()
            .copied(),
    );
    let missing = required
        .filter(|column| !found.contains(column))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "interface table {} missing columns: {}",
            contract.table,
            missing.join(",")
        ));
    }
    Ok(())
}

fn requires_control_columns(table: &str) -> bool {
    !matches!(
        table,
        "x_wmsinter_InboundOrderItems"
            | "x_wmsinter_OutboundOrderItems"
            | "x_wmsinter_InventoryPushItems"
            | "x_wmsinter_InventoryReceiveItems"
    )
}

pub(super) fn requires_control_column_updates(table: &str) -> bool {
    matches!(
        table,
        "x_wmsinter_GoodsInfo"
            | "x_wmsinter_CustomerInfo"
            | "x_wmsinter_SupplierInfo"
            | "x_wmsinter_InboundOrder"
            | "x_wmsinter_OutboundOrder"
            | "x_wmsinter_OrderCommand"
            | "x_wmsinter_InventoryPushHeader"
    )
}

async fn validate_permissions(
    connection: &mut deadpool_tiberius_rustls::deadpool::managed::Object<Manager>,
    contract: &InterfaceTableContract,
    password: &str,
) -> Result<(), String> {
    let mut query = Query::new(
        "SELECT
            CONVERT(INT, HAS_PERMS_BY_NAME(N'dbo.' + @P1, N'OBJECT', N'SELECT')) AS can_select,
            CONVERT(INT, HAS_PERMS_BY_NAME(N'dbo.' + @P1, N'OBJECT', N'INSERT')) AS can_insert,
            CONVERT(INT, HAS_PERMS_BY_NAME(N'dbo.' + @P1, N'OBJECT', N'UPDATE')) AS can_update,
            CONVERT(INT, HAS_PERMS_BY_NAME(N'dbo.' + @P1, N'OBJECT', N'DELETE')) AS can_delete"
            .to_string(),
    );
    query.bind(contract.table.to_string());
    let rows = query
        .query(&mut **connection)
        .await
        .map_err(|error| {
            sanitize_probe_error("interface table permissions", &error.to_string(), password)
        })?
        .into_first_result()
        .await
        .map_err(|error| {
            sanitize_probe_error("interface table permissions", &error.to_string(), password)
        })?;
    let row = rows
        .first()
        .ok_or_else(|| format!("interface table {} permissions missing", contract.table))?;
    let actual = [
        ("SELECT", "can_select"),
        ("INSERT", "can_insert"),
        ("UPDATE", "can_update"),
        ("DELETE", "can_delete"),
    ]
    .into_iter()
    .filter_map(|(permission, column)| {
        (row.get::<i32, _>(column).unwrap_or_default() == 1).then_some(permission)
    })
    .collect::<HashSet<_>>();
    let expected = contract.permissions.iter().copied().collect::<HashSet<_>>();
    let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
    let excessive = actual.difference(&expected).copied().collect::<Vec<_>>();
    if !missing.is_empty() || !excessive.is_empty() {
        return Err(format!(
            "interface table {} permissions invalid: missing [{}], excessive [{}]",
            contract.table,
            missing.join(","),
            excessive.join(",")
        ));
    }

    let mut query = Query::new(
        "SELECT c.name AS column_name,
                CONVERT(INT, HAS_PERMS_BY_NAME(
                    N'dbo.' + @P1, N'OBJECT', N'UPDATE', c.name, N'COLUMN'
                )) AS can_update
           FROM sys.columns c
          WHERE c.object_id = OBJECT_ID(N'dbo.' + @P1, N'U')"
            .to_string(),
    );
    query.bind(contract.table.to_string());
    let rows = query
        .query(&mut **connection)
        .await
        .map_err(|error| {
            sanitize_probe_error("interface column permissions", &error.to_string(), password)
        })?
        .into_first_result()
        .await
        .map_err(|error| {
            sanitize_probe_error("interface column permissions", &error.to_string(), password)
        })?;
    let control_updates = requires_control_column_updates(contract.table);
    let mut missing_columns = Vec::new();
    let mut excessive_columns = Vec::new();
    for row in rows {
        let Some(column) = row.get::<&str, _>("column_name") else {
            continue;
        };
        let actual = row.get::<i32, _>("can_update").unwrap_or_default() == 1;
        let expected = control_updates && CONTROL_UPDATE_COLUMNS.contains(&column);
        if expected && !actual {
            missing_columns.push(column.to_string());
        } else if actual && !expected {
            excessive_columns.push(column.to_string());
        }
    }
    if !missing_columns.is_empty() || !excessive_columns.is_empty() {
        return Err(format!(
            "interface table {} column UPDATE invalid: missing [{}], excessive [{}]",
            contract.table,
            missing_columns.join(","),
            excessive_columns.join(",")
        ));
    }
    Ok(())
}

fn required_field<'a>(value: Option<&'a str>, name: &str) -> Result<&'a str, String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} missing"))
}

pub(super) fn required_interface_contracts(
    connector: &H8ErpConnector,
) -> Result<Vec<&'static InterfaceTableContract>, String> {
    validate_connector_direction_message_selection(connector)?;
    let mut tables = Vec::new();
    if connector
        .directions
        .iter()
        .any(|direction| direction == "inbound")
    {
        for message_type in &connector.message_types {
            for table in interface_tables_for_message(message_type) {
                push_interface_contract(&mut tables, table);
            }
        }
        if !tables
            .iter()
            .any(|contract| requires_control_column_updates(contract.table))
        {
            return Err("interface inbound message types have no table mapping".into());
        }
    }
    if connector
        .directions
        .iter()
        .any(|direction| direction == "outbound")
    {
        for message_type in &connector.message_types {
            for table in interface_tables_for_message(message_type) {
                push_interface_contract(&mut tables, table);
            }
        }
    }
    if tables.is_empty() {
        return Err("interface directions have no table mapping".into());
    }
    Ok(tables)
}

fn interface_tables_for_message(message_type: &str) -> &'static [&'static str] {
    match message_type {
        "product_master" => &["x_wmsinter_GoodsInfo"],
        "customer_master" => &["x_wmsinter_CustomerInfo"],
        "supplier_master" => &["x_wmsinter_SupplierInfo"],
        "asn" => &["x_wmsinter_InboundOrder", "x_wmsinter_InboundOrderItems"],
        "outbound_order" => &["x_wmsinter_OutboundOrder", "x_wmsinter_OutboundOrderItems"],
        "order_cancel" => &["x_wmsinter_OrderCommand"],
        "inventory_seed_snapshot" => &[
            "x_wmsinter_InventoryPushHeader",
            "x_wmsinter_InventoryPushItems",
        ],
        "order_status" => &["x_wmsinter_OrderFeedback"],
        "putaway_complete" => &["x_wmsinter_InboundFeedback", "x_wmsinter_OrderFeedback"],
        "shipment_confirm" => &["x_wmsinter_OutboundFeedback", "x_wmsinter_OrderFeedback"],
        "inventory_status" | "stock_adjustment" | "archive_revision" | "reconciliation_diff" => {
            &["x_wmsinter_WmsEvent"]
        }
        "inventory_snapshot" => &[
            "x_wmsinter_InventoryReceiveHeader",
            "x_wmsinter_InventoryReceiveItems",
        ],
        _ => &[],
    }
}

fn validate_connector_direction_message_selection(
    connector: &H8ErpConnector,
) -> Result<(), String> {
    validate_direction_message_selection(&connector.directions, &connector.message_types)
        .map_err(|_| "interface direction/message selection invalid".to_string())
}

fn push_interface_contract(contracts: &mut Vec<&'static InterfaceTableContract>, table: &str) {
    if let Some(contract) = INTERFACE_TABLE_CONTRACTS
        .iter()
        .find(|contract| contract.table == table)
    {
        if !contracts
            .iter()
            .any(|existing| existing.table == contract.table)
        {
            contracts.push(contract);
        }
    }
}

fn sanitize_probe_error(context: &str, raw: &str, secret: &str) -> String {
    sanitize_error_summary(&format!("{context}: {}", raw.replace(secret, "***")))
}
