use std::{collections::HashMap, env, time::Duration};

use h8_erp_worker::{
    config::{parse_secret_map, BootstrapSettings},
    control_plane::ControlPlaneClient,
    error::WorkerError,
    mssql::MssqlRepository,
    outbound_runner::run_outbound_once,
    outbox_repository::PgOutboxRepository,
    receipts::process_receipts,
    runner::run_once,
};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("[h8-rust-worker] {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), WorkerError> {
    let bootstrap = BootstrapSettings::from_env()?;
    let control = ControlPlaneClient::new(&bootstrap)?;
    let secrets = secret_aliases()?;
    let runtime = control.load_runtime_settings(bootstrap, &secrets).await?;
    let mssql = MssqlRepository::connect(&runtime.mssql)?;
    mssql.healthcheck().await?;
    let outbox = match runtime.bootstrap.wms_db_url.as_deref() {
        Some(database_url) => {
            let repository = PgOutboxRepository::connect(database_url).await?;
            repository.healthcheck().await?;
            Some(repository)
        }
        None => None,
    };
    if env::args().any(|argument| argument == "--healthcheck") {
        return Ok(());
    }
    if env::args().any(|argument| argument == "--once") {
        let inbound = run_once(&runtime, &control, &mssql).await?;
        let outbound = match &outbox {
            Some(outbox) => {
                let published = run_outbound_once(&runtime, &control, &mssql, outbox).await?;
                let receipts = process_receipts(&runtime, &control, &mssql, outbox).await?;
                published.saturating_add(receipts)
            }
            None => 0,
        };
        println!("[h8-rust-worker] inbound={inbound} outbound={outbound}");
        return Ok(());
    }

    loop {
        let inbound = run_once(&runtime, &control, &mssql).await;
        let outbound = match &outbox {
            Some(outbox) => match run_outbound_once(&runtime, &control, &mssql, outbox).await {
                Ok(published) => process_receipts(&runtime, &control, &mssql, outbox)
                    .await
                    .map(|receipts| published.saturating_add(receipts)),
                Err(error) => Err(error),
            },
            None => Ok(0),
        };
        match (inbound, outbound) {
            (Ok(inbound), Ok(outbound)) if inbound > 0 || outbound > 0 => {
                println!("[h8-rust-worker] inbound={inbound} outbound={outbound}");
            }
            (Ok(_), Ok(_)) => {}
            (inbound, outbound) => {
                if let Err(error) = inbound {
                    eprintln!("[h8-rust-worker] inbound cycle failed: {error}");
                }
                if let Err(error) = outbound {
                    eprintln!("[h8-rust-worker] outbound cycle failed: {error}");
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(runtime.bootstrap.poll_interval_seconds)).await;
    }
}

fn secret_aliases() -> Result<HashMap<String, String>, WorkerError> {
    let raw = env::var("WMS_H8_SECRET_ALIASES")
        .ok()
        .or_else(|| env::var("WMS_SECRETS_MAP").ok());
    parse_secret_map(raw.as_deref())
}
