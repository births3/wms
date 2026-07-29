use std::{collections::HashSet, time::Duration};

use deadpool_tiberius_rustls::{tiberius_rustls::Query, Manager};
use wms_domain::{sanitize_error_summary, validate_direction_message_selection, H8ErpConnector};

pub(super) struct InterfaceTableContract {
    pub(super) table: &'static str,
    pub(super) columns: &'static [&'static str],
    pub(super) permissions: &'static [&'static str],
}

pub(super) const INBOUND_PERMISSIONS: &[&str] = &["SELECT", "UPDATE"];
pub(super) const OUTBOUND_PERMISSIONS: &[&str] = &["INSERT", "SELECT", "UPDATE"];
const IF_IN_ASN_COLUMNS: &[&str] = &[
    "id",
    "external_doc_no",
    "owner_id",
    "warehouse_id",
    "supplier_id",
    "product_code",
    "expected_qty",
    "expected_arrival_at",
    "document_type",
    "external_ref",
    "receipt_no",
    "schema_version",
    "sync_status",
    "retry_count",
    "last_error",
    "idempotency_key",
    "wms_resource_id",
    "created_at",
    "updated_at",
];
const IF_IN_OUTBOUND_ORDER_COLUMNS: &[&str] = &[
    "id",
    "external_doc_no",
    "owner_id",
    "warehouse_id",
    "customer_id",
    "document_type",
    "erp_order_no",
    "wms_order_no",
    "product_code",
    "batch_no",
    "planned_qty",
    "required_ship_at",
    "schema_version",
    "sync_status",
    "retry_count",
    "last_error",
    "idempotency_key",
    "wms_resource_id",
    "created_at",
    "updated_at",
];
const IF_IN_RETURN_ORDER_COLUMNS: &[&str] = &[
    "id",
    "external_doc_no",
    "owner_id",
    "warehouse_id",
    "customer_id",
    "supplier_id",
    "product_code",
    "expected_qty",
    "expected_arrival_at",
    "document_type",
    "external_ref",
    "receipt_no",
    "batch_no",
    "schema_version",
    "sync_status",
    "retry_count",
    "last_error",
    "idempotency_key",
    "wms_resource_id",
    "created_at",
    "updated_at",
];
const IF_IN_PRODUCT_MASTER_COLUMNS: &[&str] = &[
    "id",
    "external_doc_no",
    "owner_id",
    "product_code",
    "product_name",
    "approval_no",
    "spec",
    "dosage_form",
    "manufacturer",
    "special_drug_category",
    "storage_condition",
    "udi_code",
    "electronic_regulatory_code",
    "length_mm",
    "width_mm",
    "height_mm",
    "volume_cm3",
    "weight_g",
    "packaging_json",
    "schema_version",
    "sync_status",
    "retry_count",
    "last_error",
    "idempotency_key",
    "wms_resource_id",
    "created_at",
    "updated_at",
];
const IF_IN_PRODUCT_CHANGE_COLUMNS: &[&str] = &[
    "id",
    "external_doc_no",
    "owner_id",
    "product_code",
    "product_id",
    "field_name",
    "new_value",
    "liaison_id",
    "asn_id",
    "schema_version",
    "sync_status",
    "retry_count",
    "last_error",
    "idempotency_key",
    "wms_resource_id",
    "created_at",
    "updated_at",
];
const IF_OUT_MESSAGE_COLUMNS: &[&str] = &[
    "id",
    "event_type",
    "owner_id",
    "source_outbox_table",
    "source_outbox_id",
    "external_ref",
    "schema_version",
    "payload_json",
    "sync_status",
    "retry_count",
    "last_error",
    "idempotency_key",
    "erp_ack_ref",
    "created_at",
    "updated_at",
];

const INTERFACE_TABLE_CONTRACTS: &[InterfaceTableContract] = &[
    InterfaceTableContract {
        table: "if_in_asn",
        columns: IF_IN_ASN_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "if_in_outbound_order",
        columns: IF_IN_OUTBOUND_ORDER_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "if_in_return_order",
        columns: IF_IN_RETURN_ORDER_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "if_in_product_master",
        columns: IF_IN_PRODUCT_MASTER_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "if_in_product_change",
        columns: IF_IN_PRODUCT_CHANGE_COLUMNS,
        permissions: INBOUND_PERMISSIONS,
    },
    InterfaceTableContract {
        table: "if_out_message",
        columns: IF_OUT_MESSAGE_COLUMNS,
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
    if matches!(host, "127.0.0.1" | "localhost") {
        manager = manager.trust_cert();
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
    let missing = contract
        .columns
        .iter()
        .copied()
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

async fn validate_permissions(
    connection: &mut deadpool_tiberius_rustls::deadpool::managed::Object<Manager>,
    contract: &InterfaceTableContract,
    password: &str,
) -> Result<(), String> {
    let mut query = Query::new(
        "SELECT permission_name
           FROM fn_my_permissions(N'dbo.' + @P1, N'OBJECT')"
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
    let actual = rows
        .iter()
        .filter_map(|row| row.get::<&str, _>("permission_name"))
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
            let table = match message_type.as_str() {
                "asn" => Some("if_in_asn"),
                "outbound_order" => Some("if_in_outbound_order"),
                "return_order" => Some("if_in_return_order"),
                "product_master" => Some("if_in_product_master"),
                "product_change" => Some("if_in_product_change"),
                _ => None,
            };
            if let Some(table) = table {
                push_interface_contract(&mut tables, table);
            }
        }
        if !tables
            .iter()
            .any(|contract| contract.table.starts_with("if_in_"))
        {
            return Err("interface inbound message types have no table mapping".into());
        }
    }
    if connector
        .directions
        .iter()
        .any(|direction| direction == "outbound")
    {
        push_interface_contract(&mut tables, "if_out_message");
    }
    if tables.is_empty() {
        return Err("interface directions have no table mapping".into());
    }
    Ok(tables)
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
