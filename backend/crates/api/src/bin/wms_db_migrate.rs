use std::{env, error::Error, io};

use sqlx::postgres::PgPoolOptions;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

const DATABASE_URL_ENV: &str = "DATABASE_URL";
const WMS_DB_URL_ENV: &str = "WMS_DB_URL";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let database_url = database_url()?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("failed to connect PostgreSQL for migrations: {error:?}"),
            )
        })?;

    MIGRATOR.run(&pool).await.map_err(|error| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("failed to run PostgreSQL migrations: {error:?}"),
        )
    })?;

    Ok(())
}

fn database_url() -> Result<String, io::Error> {
    match env::var(DATABASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(value) => Ok(value),
        None => env::var(WMS_DB_URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{DATABASE_URL_ENV} or {WMS_DB_URL_ENV} is required"),
                )
            }),
    }
}
