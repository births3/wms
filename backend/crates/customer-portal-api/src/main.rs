use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr, path::PathBuf};
use wms_customer_portal_api::{export::spawn_export_worker, portal_router, PortalState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url =
        env::var("PORTAL_DATABASE_URL").map_err(|_| "PORTAL_DATABASE_URL is required")?;
    let jwt_secret = env::var("PORTAL_JWT_SECRET").map_err(|_| "PORTAL_JWT_SECRET is required")?;
    let projection_key =
        env::var("PORTAL_PROJECTION_KEY").map_err(|_| "PORTAL_PROJECTION_KEY is required")?;
    let storage_root = PathBuf::from(
        env::var("PORTAL_H_FILE_STORAGE_ROOT")
            .unwrap_or_else(|_| "var/customer-portal-files".to_string()),
    );
    tokio::fs::create_dir_all(&storage_root).await?;
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let state = PortalState::new(pool, jwt_secret, projection_key, storage_root);
    let _export_worker = spawn_export_worker(state.clone());
    let address = env::var("PORTAL_BIND")
        .unwrap_or_else(|_| "0.0.0.0:9010".to_string())
        .parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, portal_router(state)).await?;
    Ok(())
}
