use chrono::{NaiveDate, Utc};
use sqlx::postgres::PgPoolOptions;
use std::{env, error::Error, io};

use wms_api::audit::seal_audit_chain;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let seal_date = parse_seal_date(env::args().skip(1))?;
    let database_url = env::var("DATABASE_URL").map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "DATABASE_URL is required for audit maintenance",
        )
    })?;

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    sqlx::query("SELECT create_current_partition(CURRENT_DATE)")
        .execute(&pool)
        .await?;
    sqlx::query("SELECT create_next_partition(CURRENT_DATE)")
        .execute(&pool)
        .await?;

    let seal = seal_audit_chain(&pool, seal_date, Utc::now())
        .await
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to seal audit chain: {error:?}"),
            )
        })?;

    println!(
        "audit maintenance ok seal_date={} last_id={} last_self_hash={}",
        seal.seal_date, seal.last_id, seal.last_self_hash
    );
    Ok(())
}

fn parse_seal_date<I>(mut args: I) -> Result<NaiveDate, Box<dyn Error>>
where
    I: Iterator<Item = String>,
{
    let mut seal_date = Utc::now().date_naive().pred_opt().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "cannot compute yesterday seal date",
        )
    })?;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seal-date" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--seal-date requires a value")
                })?;
                seal_date = NaiveDate::parse_from_str(&value, "%Y-%m-%d")?;
            }
            "-h" | "--help" => {
                println!("Usage: audit-maintenance [--seal-date YYYY-MM-DD]");
                std::process::exit(0);
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {arg}"),
                )
                .into());
            }
        }
    }

    Ok(seal_date)
}
