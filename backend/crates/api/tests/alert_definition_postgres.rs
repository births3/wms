use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn owner(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $2)")
        .bind(id)
        .bind(format!("AL-{id}"))
        .execute(pool)
        .await
        .expect("alert owner should seed");
    id
}

fn occurred_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 13, 10, 0, 0)
        .single()
        .expect("test timestamp should be valid")
}

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|database| database.code().map(|code| code.to_string()))
}

async fn insert_definition(pool: &PgPool, owner_id: Uuid, code: &str, forced: bool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO alert_definitions (
               id, owner_id, alert_code, name, event_type, condition_expression,
               default_severity, recipient_roles, escalation_ref, silence_period_seconds,
               is_disable_allowed, message_template, is_gsp_forced, created_at, updated_at
           ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$14)"#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(code)
    .bind("测试告警")
    .bind("inventory.changed")
    .bind("quantity < 0")
    .bind("warning")
    .bind(vec!["warehouse_manager".to_string()])
    .bind(Some("AL-ESC-001"))
    .bind(300_i64)
    .bind(!forced)
    .bind("库存异常：{{product_code}}")
    .bind(forced)
    .bind(occurred_at())
    .execute(pool)
    .await
    .expect("alert definition should seed");
    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn alert_code_is_unique_per_owner_but_reusable_across_owners(pool: PgPool) {
    let first_owner = owner(&pool).await;
    let second_owner = owner(&pool).await;
    insert_definition(&pool, first_owner, "inventory.low", false).await;

    let duplicate = sqlx::query(
        "INSERT INTO alert_definitions (id, owner_id, alert_code, name, event_type, condition_expression, default_severity, message_template) VALUES ($1,$2,$3,'重复','event','true','warning','message')",
    )
    .bind(Uuid::new_v4())
    .bind(first_owner)
    .bind("inventory.low")
    .execute(&pool)
    .await
    .expect_err("same owner and alert code should be rejected");
    assert_eq!(sqlstate(&duplicate), Some("23505".to_string()));

    insert_definition(&pool, second_owner, "inventory.low", false).await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM alert_definitions WHERE alert_code = 'inventory.low'",
    )
    .fetch_one(&pool)
    .await
    .expect("alert count should query");
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn gsp_forced_alert_cannot_be_created_or_updated_as_disableable(pool: PgPool) {
    let owner_id = owner(&pool).await;
    let invalid_insert = sqlx::query(
        "INSERT INTO alert_definitions (id, owner_id, alert_code, name, event_type, condition_expression, default_severity, is_disable_allowed, message_template, is_gsp_forced) VALUES ($1,$2,'gsp.invalid','GSP','event','true','critical',true,'message',true)",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect_err("GSP forced alert should not allow disable");
    assert_eq!(sqlstate(&invalid_insert), Some("23514".to_string()));

    let id = insert_definition(&pool, owner_id, "gsp.valid", true).await;
    let invalid_update =
        sqlx::query("UPDATE alert_definitions SET is_disable_allowed = true WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .expect_err("GSP forced alert should remain non-disableable");
    assert_eq!(sqlstate(&invalid_update), Some("23514".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn alert_definition_delete_is_rejected_when_trigger_history_exists(pool: PgPool) {
    let owner_id = owner(&pool).await;
    let definition_id = insert_definition(&pool, owner_id, "inventory.triggered", false).await;
    sqlx::query(
        "INSERT INTO alert_definition_triggers (id, alert_definition_id, event_type, occurred_at, payload) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(definition_id)
    .bind("inventory.changed")
    .bind(occurred_at())
    .bind(json!({"quantity": -1}))
    .execute(&pool)
    .await
    .expect("trigger history should seed");

    let delete = sqlx::query("DELETE FROM alert_definitions WHERE id = $1")
        .bind(definition_id)
        .execute(&pool)
        .await
        .expect_err("alert with trigger history should not be deleted");
    assert_eq!(sqlstate(&delete), Some("23503".to_string()));
}

#[sqlx::test(migrations = "../../migrations")]
async fn gsp_seed_covers_existing_and_new_owners_idempotently(pool: PgPool) {
    let existing_owner = owner(&pool).await;
    sqlx::query("DELETE FROM alert_definitions WHERE owner_id = $1")
        .bind(existing_owner)
        .execute(&pool)
        .await
        .expect("existing owner seed should be reset");
    let new_owner = owner(&pool).await;
    let expected: Vec<String> = [
        "qualification_expiry_30d",
        "near_expiry_6m",
        "maintenance_overdue_3d",
        "quarantine_overdue_24h",
        "cold_chain_break_received",
        "destruction_approval_overdue_48h",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut expected = expected;
    expected.sort();
    sqlx::query("SELECT seed_h1_gsp_alert_definitions($1)")
        .bind(existing_owner)
        .execute(&pool)
        .await
        .expect("existing owner seed should run");
    for owner_id in [existing_owner, new_owner] {
        let codes: Vec<String> = sqlx::query_scalar(
            "SELECT ARRAY_AGG(alert_code ORDER BY alert_code) FROM alert_definitions WHERE owner_id = $1 AND is_gsp_forced AND NOT is_disable_allowed",
        )
        .bind(owner_id)
        .fetch_one(&pool)
        .await
        .expect("GSP definitions should be queried");
        assert_eq!(codes, expected);
    }
    sqlx::query("SELECT seed_h1_gsp_alert_definitions($1)")
        .bind(existing_owner)
        .execute(&pool)
        .await
        .expect("re-running GSP seed should be idempotent");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM alert_definitions WHERE owner_id = $1")
            .bind(existing_owner)
            .fetch_one(&pool)
            .await
            .expect("seed count should be queried");
    assert_eq!(count, 6);
}
