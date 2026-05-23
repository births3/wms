//! SPIKE-004 SQLx offline 编译模式
//!
//! 验证：
//! - H1: cargo sqlx prepare --workspace 生成 .sqlx/ 缓存
//! - H2: 无 DATABASE_URL + 无运行 PG，cargo build --offline 通过
//! - H3: schema 改但忘记重生缓存 → 编译报错（错误明确）
//! - H4: sqlx-cli 内置 migrate run + migrations/<ts>_<name>.sql 满足
//! - H5: #[sqlx::test] 自动建临时 schema + 跑 migrations + 测试隔离
//! - H6: CI 友好（先 prepare 后 build）

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

// ============================================================
// Domain types
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Item {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub expiry: NaiveDate,
    pub stock: i32,
    pub owner_id: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("数据库错误：{0}")]
    Db(#[from] sqlx::Error),
}

pub type RepoResult<T> = Result<T, RepoError>;

// ============================================================
// Repository — 用 query! / query_as! 触发编译期校验
// ============================================================

pub async fn create_pool(database_url: &str) -> RepoResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// 写：插入 item（编译期会校验 SQL 语法 + 列名 + 类型）
pub async fn insert_item(pool: &PgPool, item: &Item) -> RepoResult<()> {
    sqlx::query!(
        r#"
        INSERT INTO items (id, code, name, expiry, stock, owner_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        item.id,
        item.code,
        item.name,
        item.expiry,
        item.stock,
        item.owner_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// 读：按 owner_id 列出（多租户隔离演示）
pub async fn list_items_by_owner(pool: &PgPool, owner_id: Uuid) -> RepoResult<Vec<Item>> {
    let items = sqlx::query_as!(
        Item,
        r#"
        SELECT id, code, name, expiry, stock, owner_id
        FROM items
        WHERE owner_id = $1
        ORDER BY code ASC
        "#,
        owner_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(items)
}

/// 读：按 code 单条查询
pub async fn get_item_by_code(pool: &PgPool, code: &str) -> RepoResult<Option<Item>> {
    let item = sqlx::query_as!(
        Item,
        r#"
        SELECT id, code, name, expiry, stock, owner_id
        FROM items
        WHERE code = $1
        "#,
        code,
    )
    .fetch_optional(pool)
    .await?;
    Ok(item)
}

/// 写：更新库存（演示 UPDATE）
pub async fn update_stock(pool: &PgPool, code: &str, delta: i32) -> RepoResult<i32> {
    let row = sqlx::query!(
        r#"
        UPDATE items
        SET stock = stock + $2
        WHERE code = $1
        RETURNING stock
        "#,
        code,
        delta,
    )
    .fetch_one(pool)
    .await?;
    Ok(row.stock)
}

/// 演示返回标量
pub async fn count_items_by_owner(pool: &PgPool, owner_id: Uuid) -> RepoResult<i64> {
    let row = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM items WHERE owner_id = $1"#,
        owner_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}
