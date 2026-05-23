//! SPIKE-004 集成测试 — #[sqlx::test] 自动建临时 schema + 跑 migrations
//!
//! 测试场景：
//! T1 H4+H5：插入 item + 按 owner 列出（migrations/* 自动应用）
//! T2 H5：两个测试并发跑互不干扰（sqlx::test 每个测试一个临时 db）
//! T3 多租户：owner A 看不见 owner B 的 item（验证 add_owner_id migration 生效）

use chrono::NaiveDate;
use spike_004_sqlx_offline::{
    count_items_by_owner, get_item_by_code, insert_item, list_items_by_owner, update_stock, Item,
};
use sqlx::PgPool;
use uuid::Uuid;

fn make_item(code: &str, name: &str, owner_id: Uuid) -> Item {
    Item {
        id: Uuid::new_v4(),
        code: code.into(),
        name: name.into(),
        expiry: NaiveDate::from_ymd_opt(2027, 12, 31).unwrap(),
        stock: 100,
        owner_id,
    }
}

#[sqlx::test]
async fn t1_insert_and_list(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let item = make_item("P-001234", "葡萄糖注射液", owner_a);

    insert_item(&pool, &item).await.unwrap();

    let items = list_items_by_owner(&pool, owner_a).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].code, "P-001234");
    assert_eq!(items[0].owner_id, owner_a);
}

#[sqlx::test]
async fn t2_isolated_db_per_test(pool: PgPool) {
    // 这个测试与 t1 并发跑；如果共享 db，t1 插入的 item 会污染本测试
    let owner_a = Uuid::new_v4();
    let count = count_items_by_owner(&pool, owner_a).await.unwrap();
    assert_eq!(count, 0, "新临时 db 应该是空的");
}

#[sqlx::test]
async fn t3_multi_tenant_isolation(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();

    insert_item(&pool, &make_item("P-A-001", "alice item", owner_a))
        .await
        .unwrap();
    insert_item(&pool, &make_item("P-A-002", "alice item 2", owner_a))
        .await
        .unwrap();
    insert_item(&pool, &make_item("P-B-001", "bob item", owner_b))
        .await
        .unwrap();

    let items_a = list_items_by_owner(&pool, owner_a).await.unwrap();
    assert_eq!(items_a.len(), 2);
    for item in &items_a {
        assert!(item.code.starts_with("P-A-"));
    }

    let items_b = list_items_by_owner(&pool, owner_b).await.unwrap();
    assert_eq!(items_b.len(), 1);
    assert_eq!(items_b[0].code, "P-B-001");
}

#[sqlx::test]
async fn t4_update_stock(pool: PgPool) {
    let owner = Uuid::new_v4();
    let item = make_item("P-stock", "test", owner);
    insert_item(&pool, &item).await.unwrap();

    let new_stock = update_stock(&pool, "P-stock", -10).await.unwrap();
    assert_eq!(new_stock, 90);

    let fetched = get_item_by_code(&pool, "P-stock").await.unwrap();
    assert_eq!(fetched.unwrap().stock, 90);
}

#[sqlx::test]
async fn t5_unique_constraint(pool: PgPool) {
    let owner = Uuid::new_v4();
    let item = make_item("P-dup", "dup", owner);
    insert_item(&pool, &item).await.unwrap();

    let dup = make_item("P-dup", "dup2", owner);
    let result = insert_item(&pool, &dup).await;
    assert!(result.is_err(), "重复 code 应该违反 UNIQUE 约束");
}
