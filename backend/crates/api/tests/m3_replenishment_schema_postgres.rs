//! T02：补货任务表、策略水位 CHECK、权限与单据类型。

use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn replenishment_tasks_table_accepts_pending_row(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let batch_id = Uuid::new_v4();
    let product_id = Uuid::new_v4();

    let columns: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT column_name
          FROM information_schema.columns
         WHERE table_schema = 'public'
           AND table_name = 'replenishment_tasks'
         ORDER BY ordinal_position
        "#,
    )
    .fetch_all(&pool)
    .await
    .expect("query replenishment_tasks columns");

    for required in [
        "status",
        "claimed_at",
        "last_progress_at",
        "wave_id",
        "outbound_order_id",
        "outbound_line_no",
        "return_reason",
    ] {
        assert!(
            columns.iter().any(|name| name == required),
            "replenishment_tasks missing column {required}, have {columns:?}"
        );
    }

    sqlx::query(
        r#"
        INSERT INTO replenishment_tasks (
            id, owner_id, task_no, trigger_mode, priority,
            source_location_id, source_batch_id, target_location_id,
            product_id, batch_no, qty, status, created_by
        ) VALUES (
            $1, $2, 'RT-T02-001', 'manual', 'normal',
            $3, $4, $3, $5, 'B1', 1, 'pending', 'tester'
        )
        "#,
    )
    .bind(task_id)
    .bind(owner_id)
    .bind(location_id)
    .bind(batch_id)
    .bind(product_id)
    .execute(&pool)
    .await
    .expect("insert pending replenishment task");

    let status: String = sqlx::query_scalar(
        "SELECT status FROM replenishment_tasks WHERE id = $1 AND owner_id = $2",
    )
    .bind(task_id)
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("fetch inserted task");
    assert_eq!(status, "pending");

    sqlx::query("UPDATE replenishment_tasks SET status = 'suspended' WHERE id = $1")
        .bind(task_id)
        .execute(&pool)
        .await
        .expect("status check must allow suspended");
}

#[sqlx::test(migrations = "../../migrations")]
async fn replenishment_strategy_rejects_min_not_below_max(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let err = sqlx::query(
        r#"
        INSERT INTO replenishment_strategies (
            id, owner_id, strategy_code, strategy_name, scope_type, scope_ref,
            location_type, source_type, target_type,
            min_safety_threshold, max_replenish_target
        ) VALUES (
            $1, $2, 'BAD-MINMAX', '非法水位', 'product', $3,
            'piece_pick', 'storage', 'piece_pick', 10, 10
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect_err("min == max must violate CHECK");

    let message = err.to_string();
    assert!(
        message.contains("replenishment_strategies_minmax_check")
            || message.contains("check constraint"),
        "expected minmax check, got {message}"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn replenishment_permissions_and_document_type_exist(pool: PgPool) {
    let manage: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_permissions WHERE permission_code = 'm3.replenishment.manage'",
    )
    .fetch_one(&pool)
    .await
    .expect("query manage permission");
    let execute: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_permissions WHERE permission_code = 'm3.replenishment.execute'",
    )
    .fetch_one(&pool)
    .await
    .expect("query execute permission");
    assert_eq!(manage, 1, "m3.replenishment.manage must exist");
    assert_eq!(execute, 1, "m3.replenishment.execute must exist");

    let dict_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM system_dictionary_items
         WHERE dict_code = 'document_type'
           AND item_code = 'replenishment_task'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("query document_type");
    assert_eq!(dict_count, 1, "document_type replenishment_task must exist");

    let rule_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM document_number_rules
         WHERE document_type = 'replenishment_task'
           AND enabled
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("query numbering rule");
    assert!(
        rule_count >= 1,
        "M-CG must have an enabled replenishment_task rule"
    );
}
