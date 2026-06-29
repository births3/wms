//! Test-only HTTP entrypoint for real-data web-admin E2E.

use std::{env, error::Error, io, net::SocketAddr, sync::Arc};

use axum::{routing::get, Json, Router};
use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;
use uuid::Uuid;
use wms_api::{
    auth::{
        auth_runtime_layer, AuthRevocationStore, AuthRevocationStoreError, AuthRuntimePolicy,
        JWT_SECRET_ENV,
    },
    auth_handlers::{auth_router, AuthAppState},
    master_data_handlers::{master_data_router, MasterDataAppState},
    system_dictionary_handlers::{system_dictionary_router, SystemDictionaryAppState},
};
use wms_domain::HealthzResponse;

const BIND_ADDR_ENV: &str = "WMS_BIND_ADDR";
const DATABASE_URL_ENV: &str = "DATABASE_URL";
const WMS_DB_URL_ENV: &str = "WMS_DB_URL";
const E2E_SEED_ENV: &str = "WMS_E2E_SEED";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:19080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bind_addr = env::var(BIND_ADDR_ENV)
        .unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_string())
        .parse::<SocketAddr>()?;
    let jwt_secret = env::var(JWT_SECRET_ENV).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{JWT_SECRET_ENV} is required"),
        )
    })?;
    if jwt_secret.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{JWT_SECRET_ENV} must not be empty"),
        )
        .into());
    }

    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&database_url()?)
        .await?;
    if env::var(E2E_SEED_ENV).ok().as_deref() == Some("1") {
        sqlx::migrate!("../../migrations").run(&pool).await?;
        seed_e2e_data(&pool).await?;
    }

    let app = Router::new()
        .route("/api/v1/healthz", get(healthz))
        .merge(auth_router(AuthAppState::new(pool.clone())))
        .merge(master_data_router(MasterDataAppState::with_postgres(
            pool.clone(),
        )))
        .merge(system_dictionary_router(
            SystemDictionaryAppState::with_postgres(pool),
        ))
        .layer(auth_runtime_layer(AuthRuntimePolicy::new(Arc::new(
            AllowAllRevocationStore,
        ))));

    let listener = TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn database_url() -> Result<String, io::Error> {
    env::var(DATABASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var(WMS_DB_URL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{DATABASE_URL_ENV} or {WMS_DB_URL_ENV} is required"),
            )
        })
}

async fn healthz() -> Json<HealthzResponse> {
    Json(HealthzResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        generated_at: Utc::now(),
    })
}

struct AllowAllRevocationStore;

#[axum::async_trait]
impl AuthRevocationStore for AllowAllRevocationStore {
    async fn jti_is_blacklisted(&self, _jti: &str) -> Result<bool, AuthRevocationStoreError> {
        Ok(false)
    }

    async fn permissions_changed_at(
        &self,
        _user_id: Uuid,
    ) -> Result<Option<i64>, AuthRevocationStoreError> {
        Ok(None)
    }

    async fn blacklist_jti(
        &self,
        _jti: &str,
        _ttl_seconds: u64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }

    async fn set_permissions_changed_at(
        &self,
        _user_id: Uuid,
        _changed_at_unix: i64,
    ) -> Result<(), AuthRevocationStoreError> {
        Ok(())
    }
}

async fn seed_e2e_data(pool: &PgPool) -> Result<(), Box<dyn Error>> {
    let password_hash = bcrypt::hash("CorrectHorse1!", 4)?;
    sqlx::query(
        r#"
        INSERT INTO auth_owners (id, owner_code, owner_name)
        VALUES ('00000000-0000-0000-0000-000000000001', 'PY_OWNER', '鹏鹞药业')
        ON CONFLICT (id) DO UPDATE
        SET owner_code = EXCLUDED.owner_code,
            owner_name = EXCLUDED.owner_name
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_users (id, username, display_name, password_hash, status, failed_login_count, locked_until)
        VALUES ('00000000-0000-0000-0000-000000000101', 'admin', '系统管理员', $1, 'active', 0, NULL)
        ON CONFLICT (id) DO UPDATE
        SET username = EXCLUDED.username,
            display_name = EXCLUDED.display_name,
            password_hash = EXCLUDED.password_hash,
            status = 'active',
            failed_login_count = 0,
            locked_until = NULL,
            updated_at = now()
        "#,
    )
    .bind(password_hash)
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_user_owner_bindings (user_id, owner_id, is_active, is_primary)
        VALUES ('00000000-0000-0000-0000-000000000101', '00000000-0000-0000-0000-000000000001', TRUE, TRUE)
        ON CONFLICT (user_id, owner_id) DO UPDATE
        SET is_active = TRUE,
            is_primary = TRUE
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO auth_roles (id, owner_id, role_code, role_name)
        VALUES ('00000000-0000-0000-0000-000000000102', '00000000-0000-0000-0000-000000000001', 'system_admin', '系统管理员')
        ON CONFLICT (id) DO UPDATE
        SET role_code = EXCLUDED.role_code,
            role_name = EXCLUDED.role_name
        "#,
    )
    .execute(pool)
    .await?;
    for (id, code, name) in [
        (
            "00000000-0000-0000-0000-000000000111",
            "m1.master_data.read",
            "基础档案读取",
        ),
        (
            "00000000-0000-0000-0000-000000000112",
            "m1.system_dictionary.read",
            "系统字典读取",
        ),
        (
            "00000000-0000-0000-0000-000000000113",
            "m1.master_data.write",
            "基础档案写入",
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO auth_permissions (id, permission_code, permission_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE
            SET permission_code = EXCLUDED.permission_code,
                permission_name = EXCLUDED.permission_name
            "#,
        )
        .bind(Uuid::parse_str(id)?)
        .bind(code)
        .bind(name)
        .execute(pool)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO auth_role_permissions (role_id, permission_id)
            VALUES ('00000000-0000-0000-0000-000000000102', $1)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(Uuid::parse_str(id)?)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        r#"
        INSERT INTO auth_user_roles (user_id, owner_id, role_id)
        VALUES ('00000000-0000-0000-0000-000000000101', '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000102')
        ON CONFLICT DO NOTHING
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO products (
            id, owner_id, product_code, product_name, specification, dosage_form,
            storage_condition, special_drug_category, approval_no, manufacturer, status
        )
        VALUES (
            '00000000-0000-0000-0000-000000001001', '00000000-0000-0000-0000-000000000001',
            'P-M1-E2E-001', 'E2E 冷藏胰岛素', '10ml*1支', '注射剂',
            'cold', 'none', '国药准字E2E001', 'E2E 示例药业', 'active'
        )
        ON CONFLICT (owner_id, product_code) DO UPDATE
        SET product_name = EXCLUDED.product_name,
            specification = EXCLUDED.specification,
            storage_condition = EXCLUDED.storage_condition,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO suppliers (id, owner_id, supplier_code, supplier_name, uscc, contact_name, status)
        VALUES (
            '00000000-0000-0000-0000-000000001101', '00000000-0000-0000-0000-000000000001',
            'S-M1-E2E-001', 'E2E 供应商', '91310000E2E000001', '王供应', 'active'
        )
        ON CONFLICT (owner_id, supplier_code) DO UPDATE
        SET supplier_name = EXCLUDED.supplier_name,
            uscc = EXCLUDED.uscc,
            contact_name = EXCLUDED.contact_name,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO customers (id, owner_id, customer_code, customer_name, customer_type, contact_name, status)
        VALUES (
            '00000000-0000-0000-0000-000000001201', '00000000-0000-0000-0000-000000000001',
            'C-M1-E2E-001', 'E2E 客户门店', 'store', '李客户', 'active'
        )
        ON CONFLICT (owner_id, customer_code) DO UPDATE
        SET customer_name = EXCLUDED.customer_name,
            customer_type = EXCLUDED.customer_type,
            contact_name = EXCLUDED.contact_name,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO warehouses (id, owner_id, warehouse_code, warehouse_name, warehouse_type, address, status)
        VALUES (
            '00000000-0000-0000-0000-000000001301', '00000000-0000-0000-0000-000000000001',
            'WH-M1-E2E-001', 'E2E 冷链仓', 'physical', '上海 E2E 园区', 'active'
        )
        ON CONFLICT (owner_id, warehouse_code) DO UPDATE
        SET warehouse_name = EXCLUDED.warehouse_name,
            warehouse_type = EXCLUDED.warehouse_type,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO warehouse_zones (
            id, owner_id, warehouse_id, zone_code, zone_name, temperature_zone, quality_color, status
        )
        VALUES (
            '00000000-0000-0000-0000-000000001302', '00000000-0000-0000-0000-000000000001',
            '00000000-0000-0000-0000-000000001301', 'A01', 'E2E 冷藏区', 'cold',
            'qualified_green', 'active'
        )
        ON CONFLICT (owner_id, warehouse_id, zone_code) DO UPDATE
        SET zone_name = EXCLUDED.zone_name,
            temperature_zone = EXCLUDED.temperature_zone,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO warehouse_locations (
            id, owner_id, warehouse_id, zone_id, location_code, row_no, column_no, layer_no,
            max_volume_cm3, used_volume_cm3, max_sku_count, location_type, bound_owner_id, status
        )
        VALUES (
            '00000000-0000-0000-0000-000000001401', '00000000-0000-0000-0000-000000000001',
            '00000000-0000-0000-0000-000000001301', '00000000-0000-0000-0000-000000001302',
            'A01-01-02-03', 1, 2, 3, 1000000, 0, 1, 'storage', NULL, 'available'
        )
        ON CONFLICT (owner_id, location_code) DO UPDATE
        SET warehouse_id = EXCLUDED.warehouse_id,
            zone_id = EXCLUDED.zone_id,
            row_no = EXCLUDED.row_no,
            column_no = EXCLUDED.column_no,
            layer_no = EXCLUDED.layer_no,
            updated_at = now()
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
