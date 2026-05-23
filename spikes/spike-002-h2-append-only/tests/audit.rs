//! SPIKE-002 集成测试 — 5 假设验证
//!
//! T1 (H1) trigger 阻止 UPDATE/DELETE：直接 SQL UPDATE → 失败
//! T2 (H1) 角色权限：wms_app 无 UPDATE 权限（spike 简化：仅靠 trigger 验证；
//!     真用户切换需要 PG 配置 SET ROLE，spike 不做）
//! T3 (H3) JSONB diff + jsonb_path_ops：插入带 diff 的事件，按 changed_keys 过滤
//! T4 (H2) partition pruning：插入跨月数据，EXPLAIN 仅扫单个分区
//! T5 (H5) hash chain 完整性 + 篡改检测
//! T6 (H4) 简单吞吐：100 条插入 timing 报告

use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use spike_002_h2_append_only::{
    append_event, verify_hash_chain, AuditDiff, AuditWriteRequest,
};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

fn make_req(action: &str, module: &str, occurred_at: DateTime<Utc>) -> AuditWriteRequest {
    AuditWriteRequest {
        occurred_at,
        actor_id: Uuid::from_u128(0xa1),
        actor_name: "alice".into(),
        owner_id: Uuid::from_u128(0xff),
        action: action.into(),
        module: module.into(),
        resource_type: Some("PO".into()),
        resource_id: Some("PO-2026-0001".into()),
        diff: None,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn t1_trigger_blocks_update(pool: PgPool) {
    let req = make_req("create", "M2", Utc.with_ymd_and_hms(2026, 6, 15, 10, 0, 0).unwrap());
    let id = append_event(&pool, &req).await.unwrap();

    // 用原生 SQL 直接 UPDATE，应被 trigger 拦下
    let result = sqlx::query("UPDATE audit_event SET action = 'hacked' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;
    assert!(result.is_err(), "UPDATE 应被 trigger 拒绝");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("audit_event is append-only"),
        "trigger 错误信息应明示 append-only: {err}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn t1b_trigger_blocks_delete(pool: PgPool) {
    let req = make_req("create", "M2", Utc.with_ymd_and_hms(2026, 6, 15, 10, 0, 0).unwrap());
    append_event(&pool, &req).await.unwrap();

    let result = sqlx::query("DELETE FROM audit_event").execute(&pool).await;
    assert!(result.is_err(), "DELETE 应被 trigger 拒绝");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("audit_event is append-only"), "trigger 错误信息: {err}");
}

#[sqlx::test(migrations = "./migrations")]
async fn t1c_trigger_blocks_truncate(pool: PgPool) {
    let result = sqlx::query("TRUNCATE audit_event").execute(&pool).await;
    assert!(result.is_err(), "TRUNCATE 应被 trigger 拒绝");
}

#[sqlx::test(migrations = "./migrations")]
async fn t3_jsonb_diff_and_index(pool: PgPool) {
    // 插入两条带 diff 的事件
    let mut req = make_req("update", "M1", Utc.with_ymd_and_hms(2026, 6, 15, 10, 0, 0).unwrap());
    req.diff = Some(AuditDiff::compute(
        &json!({"name": "old", "batch_no": "20260101", "stock": 100}),
        &json!({"name": "new", "batch_no": "20260101", "stock": 50}),
    ));
    append_event(&pool, &req).await.unwrap();

    let mut req2 = make_req("update", "M1", Utc.with_ymd_and_hms(2026, 6, 15, 10, 1, 0).unwrap());
    req2.diff = Some(AuditDiff::compute(
        &json!({"batch_no": "old"}),
        &json!({"batch_no": "new"}),
    ));
    append_event(&pool, &req2).await.unwrap();

    // 用 jsonb_path_ops 索引查询：找出 changed_keys 含 "stock" 的事件
    let stock_events: Vec<i64> = sqlx::query_scalar(
        r#"SELECT id FROM audit_event
           WHERE diff @> '{"changed_keys": ["stock"]}'::jsonb"#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(stock_events.len(), 1, "应仅匹配 1 条改了 stock 的事件");

    // 验证 diff 字段结构
    let row = sqlx::query("SELECT diff FROM audit_event WHERE id = $1")
        .bind(stock_events[0])
        .fetch_one(&pool)
        .await
        .unwrap();
    use sqlx::Row;
    let diff_value: serde_json::Value = row.get("diff");
    let changed: Vec<String> =
        serde_json::from_value(diff_value["changed_keys"].clone()).unwrap();
    assert!(changed.contains(&"name".to_string()));
    assert!(changed.contains(&"stock".to_string()));
    assert!(!changed.contains(&"batch_no".to_string()), "未变字段不该出现");
}

#[sqlx::test(migrations = "./migrations")]
async fn t4_partition_pruning(pool: PgPool) {
    // 插入跨 3 个月的数据（春 / 夏 / 秋）
    for (month, day) in &[(3u32, 15u32), (6, 15), (9, 15)] {
        let req = make_req(
            "create",
            "M2",
            Utc.with_ymd_and_hms(2026, *month, *day, 10, 0, 0).unwrap(),
        );
        append_event(&pool, &req).await.unwrap();
    }

    // EXPLAIN：查 6 月的事件，应仅扫 audit_event_2026_06 一个分区
    let explain: Vec<(String,)> = sqlx::query_as(
        r#"
        EXPLAIN (FORMAT TEXT)
        SELECT * FROM audit_event
        WHERE occurred_at >= '2026-06-01' AND occurred_at < '2026-07-01'
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let plan = explain.iter().map(|(s,)| s.as_str()).collect::<Vec<_>>().join("\n");
    println!("== EXPLAIN plan ==\n{plan}");

    // 验证：plan 中包含 "audit_event_2026_06"，且不包含其他月份分区
    assert!(plan.contains("audit_event_2026_06"), "应扫 6 月分区: {plan}");
    assert!(
        !plan.contains("audit_event_2026_03"),
        "不应扫 3 月分区（partition pruning 失败）: {plan}"
    );
    assert!(
        !plan.contains("audit_event_2026_09"),
        "不应扫 9 月分区: {plan}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn t5_hash_chain_integrity(pool: PgPool) {
    // 插入 5 条
    for i in 0..5 {
        let req = make_req(
            "create",
            "M2",
            Utc.with_ymd_and_hms(2026, 6, 15, 10, i as u32, 0).unwrap(),
        );
        append_event(&pool, &req).await.unwrap();
    }

    // 完整性校验应通过
    verify_hash_chain(&pool).await.expect("hash chain 应完整");

    // 模拟篡改：直接 SQL UPDATE 改不动（trigger 会拦下），所以用 raw INSERT
    // 模拟 DBA 越过 trigger 修改某条 self_hash
    sqlx::query("ALTER TABLE audit_event_2026_06 DISABLE TRIGGER trg_no_update")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE audit_event_2026_06 SET self_hash = 'tampered_hash' WHERE id = 3")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE audit_event_2026_06 ENABLE TRIGGER trg_no_update")
        .execute(&pool)
        .await
        .unwrap();

    // 现在 hash chain 应该被检测到断裂
    let result = verify_hash_chain(&pool).await;
    assert!(result.is_err(), "篡改后 hash chain 应被检测到");
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("hash chain 不完整"), "错误信息: {err_str}");
}

#[sqlx::test(migrations = "./migrations")]
async fn t6_throughput_baseline(pool: PgPool) {
    // 100 条 INSERT 单线程 timing；不严格的 P99，给基线参考
    let count = 100;
    let start = Instant::now();
    for i in 0..count {
        let req = make_req(
            "create",
            "M2",
            Utc.with_ymd_and_hms(2026, 6, 15, 10, i as u32 % 60, i as u32 % 60).unwrap(),
        );
        append_event(&pool, &req).await.unwrap();
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as f64 / count as f64;
    println!(
        "== 吞吐基线 ==\n{} 条插入耗时 {:.2}s, 平均 {:.2}ms/条",
        count,
        elapsed.as_secs_f64(),
        avg_ms
    );
    // 单线程含 SELECT prev_hash + INSERT；典型 < 5ms/条（局域网 PG）
    // 不做硬断言（CI 抖动），仅打印作 H4 数据点
}
