use sqlx::postgres::PgPoolOptions;
use std::{env, net::SocketAddr, path::PathBuf};
use uuid::Uuid;
use wms_customer_portal_api::{export::spawn_export_worker, portal_router, PortalState};

const CUSTOMER_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000007001);
const ADDRESS_A_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000007101);
const ADDRESS_B_ID: Uuid = Uuid::from_u128(0x00000000000000000000000000007102);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url =
        env::var("PORTAL_DATABASE_URL").map_err(|_| "PORTAL_DATABASE_URL is required")?;
    let jwt_secret = env::var("PORTAL_JWT_SECRET").map_err(|_| "PORTAL_JWT_SECRET is required")?;
    let projection_key =
        env::var("PORTAL_PROJECTION_KEY").map_err(|_| "PORTAL_PROJECTION_KEY is required")?;
    let storage_root = PathBuf::from(
        env::var("PORTAL_H_FILE_STORAGE_ROOT")
            .unwrap_or_else(|_| "var/customer-portal-e2e-files".to_string()),
    );
    tokio::fs::create_dir_all(&storage_root).await?;
    let pool = PgPoolOptions::new()
        .max_connections(12)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    seed_users(&pool).await?;

    let state = PortalState::new(pool, jwt_secret, projection_key, storage_root);
    let _export_worker = spawn_export_worker(state.clone());
    let address = env::var("PORTAL_BIND")
        .unwrap_or_else(|_| "127.0.0.1:19190".to_string())
        .parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, portal_router(state)).await?;
    Ok(())
}

async fn seed_users(pool: &sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let password_hash = bcrypt::hash("CorrectHorse1!", 4)?;
    sqlx::query(
        "INSERT INTO portal_customers (id, customer_code, customer_name, updated_at)
         VALUES ($1, 'PORTAL-E2E', 'E2E 连锁客户', now())
         ON CONFLICT (id) DO UPDATE
         SET customer_name = EXCLUDED.customer_name, updated_at = now()",
    )
    .bind(CUSTOMER_ID)
    .execute(pool)
    .await?;
    for (id, code, name) in [
        (ADDRESS_A_ID, "A", "上海浦东一店"),
        (ADDRESS_B_ID, "B", "上海闵行二店"),
    ] {
        sqlx::query(
            "INSERT INTO portal_customer_addresses (
                id, customer_id, address_code, address_name, address_snapshot, updated_at
             )
             VALUES ($1, $2, $3, $4, $5, now())
             ON CONFLICT (id) DO UPDATE
             SET address_name = EXCLUDED.address_name,
                 address_snapshot = EXCLUDED.address_snapshot,
                 updated_at = now()",
        )
        .bind(id)
        .bind(CUSTOMER_ID)
        .bind(code)
        .bind(name)
        .bind(serde_json::json!({ "address_name": name }))
        .execute(pool)
        .await?;
    }
    for (id, username, display_name, role, can_history) in [
        (
            Uuid::from_u128(0x00000000000000000000000000007201),
            "portal-admin",
            "客户管理员",
            "customer_admin",
            true,
        ),
        (
            Uuid::from_u128(0x00000000000000000000000000007202),
            "portal-multi",
            "多地址账号",
            "customer_user",
            false,
        ),
        (
            Uuid::from_u128(0x00000000000000000000000000007203),
            "portal-none",
            "无地址账号",
            "customer_user",
            false,
        ),
        (
            Uuid::from_u128(0x00000000000000000000000000007204),
            "portal-history",
            "历史权限账号",
            "customer_user",
            true,
        ),
    ] {
        sqlx::query(
            "INSERT INTO portal_users (
                id, customer_id, username, display_name, password_hash,
                role, can_view_report_history, status, failed_login_count,
                locked_until, created_at, updated_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', 0, NULL, now(), now())
             ON CONFLICT (username) DO UPDATE
             SET display_name = EXCLUDED.display_name,
                 password_hash = EXCLUDED.password_hash,
                 role = EXCLUDED.role,
                 can_view_report_history = EXCLUDED.can_view_report_history,
                 status = 'active',
                 failed_login_count = 0,
                 locked_until = NULL,
                 updated_at = now()",
        )
        .bind(id)
        .bind(CUSTOMER_ID)
        .bind(username)
        .bind(display_name)
        .bind(&password_hash)
        .bind(role)
        .bind(can_history)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        "DELETE FROM portal_user_addresses
         WHERE user_id IN ($1, $2, $3)",
    )
    .bind(Uuid::from_u128(0x00000000000000000000000000007202))
    .bind(Uuid::from_u128(0x00000000000000000000000000007203))
    .bind(Uuid::from_u128(0x00000000000000000000000000007204))
    .execute(pool)
    .await?;
    for (user_id, address_id) in [
        (
            Uuid::from_u128(0x00000000000000000000000000007202),
            ADDRESS_A_ID,
        ),
        (
            Uuid::from_u128(0x00000000000000000000000000007202),
            ADDRESS_B_ID,
        ),
        (
            Uuid::from_u128(0x00000000000000000000000000007204),
            ADDRESS_A_ID,
        ),
    ] {
        sqlx::query(
            "INSERT INTO portal_user_addresses (user_id, address_id)
             VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(address_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}
