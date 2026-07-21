//! US-H8-004：通过 TCP 证明 MSSQL 探查账号仅能 SELECT。

use deadpool_tiberius_rustls::{tiberius_rustls::Query, Manager};
use std::{env, error::Error, io, time::Duration};

const UPDATE_PROBE: &str = "BEGIN TRY BEGIN TRANSACTION; UPDATE dbo.if_in_asn SET sync_status = sync_status WHERE external_doc_no = N'DEMO-ASN-001'; ROLLBACK TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION; THROW; END CATCH";
const DELETE_PROBE: &str = "BEGIN TRY BEGIN TRANSACTION; DELETE FROM dbo.if_in_asn WHERE external_doc_no = N'DEMO-ASN-001'; ROLLBACK TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION; THROW; END CATCH";
const INSERT_PROBE: &str = "BEGIN TRY BEGIN TRANSACTION; INSERT INTO dbo.if_in_asn (external_doc_no, owner_id, warehouse_id, supplier_id, product_code, expected_qty, expected_arrival_at, idempotency_key) VALUES (N'PROBE-DENY', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', N'PROBE', 1, SYSUTCDATETIME(), N'PROBE-DENY'); ROLLBACK TRANSACTION; END TRY BEGIN CATCH IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION; THROW; END CATCH";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let host = required("H8_MSSQL_HOST")?;
    let port = env::var("H8_MSSQL_PORT")
        .unwrap_or_else(|_| "1433".into())
        .parse::<u16>()?;
    let database = env::var("H8_MSSQL_DATABASE").unwrap_or_else(|_| "wms_erp_if".into());
    let username = env::var("H8_MSSQL_PROBE_USER").unwrap_or_else(|_| "wms_h8_probe".into());
    let password = required("H8_MSSQL_PROBE_PASSWORD")?;
    let pool = Manager::new()
        .host(host)
        .port(port)
        .database(database)
        .basic_authentication(username, password)
        .trust_cert()
        .max_size(1)
        .wait_timeout(Duration::from_secs(2))
        .create_timeout(Duration::from_secs(5))
        .recycle_timeout(Duration::from_secs(5))
        .create_pool()?;
    let mut connection = pool.get().await?;

    let seed_rows = Query::new(
        "SELECT external_doc_no, sync_status FROM dbo.if_in_asn WHERE external_doc_no = N'DEMO-ASN-001' AND sync_status = N'pending' UNION ALL SELECT external_doc_no, sync_status FROM dbo.if_in_product_master WHERE external_doc_no = N'DEMO-PM-001' AND sync_status = N'pending'",
    )
    .query(&mut *connection)
    .await?
    .into_first_result()
    .await?;
    if seed_rows.len() != 2 {
        return Err(io::Error::other("DEMO seed rows are not visible to probe account").into());
    }
    println!("SELECT allowed; DEMO-ASN-001 and DEMO-PM-001 pending rows visible");

    for (operation, statement) in [
        ("UPDATE", UPDATE_PROBE),
        ("DELETE", DELETE_PROBE),
        ("INSERT", INSERT_PROBE),
    ] {
        if Query::new(statement)
            .execute(&mut *connection)
            .await
            .is_ok()
        {
            return Err(io::Error::other(format!("{operation} unexpectedly allowed")).into());
        }
        println!("{operation} denied by MSSQL");
    }

    let residue = Query::new(
        "SELECT COUNT_BIG(1) AS residue_count FROM dbo.if_in_asn WHERE external_doc_no = N'PROBE-DENY'",
    )
    .query(&mut *connection)
    .await?
    .into_first_result()
    .await?;
    let residue_count = residue
        .first()
        .and_then(|row| row.get::<i64, _>("residue_count"))
        .unwrap_or(0);
    if residue_count != 0 {
        return Err(io::Error::other("DML probe left a residue row").into());
    }
    println!("SELECT-only evidence PASS; no DML residue");
    Ok(())
}

fn required(name: &str) -> Result<String, io::Error> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("{name} is required")))
}
