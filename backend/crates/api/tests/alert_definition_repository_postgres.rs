use chrono::{TimeZone, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::alert_definition_repository::{
    AlertDefinitionRepositoryError, PgAlertDefinitionRepository,
};
use wms_domain::CreateAlertDefinitionRequest;

async fn owner(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth_owners (id, owner_code, owner_name) VALUES ($1, $2, $2)")
        .bind(id)
        .bind(format!("AL-REPO-{id}"))
        .execute(pool)
        .await
        .expect("owner should seed");
    id
}

fn at(hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 13, hour, 0, 0)
        .single()
        .expect("test timestamp should be valid")
}

fn request(code: &str, forced: bool, disable_allowed: bool) -> CreateAlertDefinitionRequest {
    CreateAlertDefinitionRequest {
        alert_code: code.to_string(),
        name: format!("{code} 告警"),
        event_type: "inventory.changed".to_string(),
        condition_expression: "quantity < safety_stock".to_string(),
        default_severity: "warning".to_string(),
        recipient_roles: vec!["warehouse_manager".to_string()],
        escalation_ref: None,
        silence_period_seconds: 300,
        is_disable_allowed: disable_allowed,
        message_template: "库存低于安全阈值".to_string(),
        is_gsp_forced: forced,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn create_is_owner_scoped_and_same_code_is_reusable_across_owners(pool: PgPool) {
    let owner_a = owner(&pool).await;
    let owner_b = owner(&pool).await;
    let repo = PgAlertDefinitionRepository::new(pool.clone());

    let first = repo
        .create(owner_a, &request("inventory.low", false, true), at(9))
        .await
        .expect("owner A definition should be created");
    let second = repo
        .create(owner_b, &request("inventory.low", false, true), at(10))
        .await
        .expect("owner B may reuse the code");

    assert_eq!(first.owner_id, owner_a);
    assert_eq!(second.owner_id, owner_b);
    let owner_counts: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT owner_id, COUNT(*)::BIGINT FROM alert_definitions WHERE owner_id IN ($1, $2) AND alert_code = 'inventory.low' GROUP BY owner_id ORDER BY owner_id",
    )
    .bind(owner_a)
    .bind(owner_b)
    .fetch_all(&pool)
    .await
    .expect("owner-scoped rows should query");
    assert_eq!(owner_counts.len(), 2);
    assert!(owner_counts.iter().all(|(_, count)| *count == 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_code_fails_for_the_same_owner(pool: PgPool) {
    let owner_id = owner(&pool).await;
    let repo = PgAlertDefinitionRepository::new(pool);
    repo.create(owner_id, &request("inventory.low", false, true), at(9))
        .await
        .expect("first definition should be created");

    assert!(matches!(
        repo.create(owner_id, &request("inventory.low", false, true), at(10))
            .await,
        Err(AlertDefinitionRepositoryError::DuplicateCode)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn duplicate_name_fails_for_the_same_owner(pool: PgPool) {
    let owner_id = owner(&pool).await;
    let repo = PgAlertDefinitionRepository::new(pool);
    let first = request("inventory.low", false, true);
    let mut second = request("inventory.lower", false, true);
    second.name = first.name.clone();
    repo.create(owner_id, &first, at(9))
        .await
        .expect("first definition should be created");

    assert!(matches!(
        repo.create(owner_id, &second, at(10)).await,
        Err(AlertDefinitionRepositoryError::DuplicateCode)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn gsp_forced_alert_cannot_be_created_or_changed_to_disableable(pool: PgPool) {
    let owner_id = owner(&pool).await;
    let repo = PgAlertDefinitionRepository::new(pool);

    assert!(matches!(
        repo.create(owner_id, &request("gsp.invalid", true, true), at(9))
            .await,
        Err(AlertDefinitionRepositoryError::GspForcedCannotDisable)
    ));
    let forced = repo
        .create(owner_id, &request("gsp.valid", true, false), at(10))
        .await
        .expect("non-disableable GSP definition should be created");
    assert!(matches!(
        repo.set_disable_allowed(owner_id, forced.id, true, at(11))
            .await,
        Err(AlertDefinitionRepositoryError::GspForcedCannotDisable)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_fails_when_trigger_history_exists(pool: PgPool) {
    let owner_id = owner(&pool).await;
    let repo = PgAlertDefinitionRepository::new(pool.clone());
    let definition = repo
        .create(
            owner_id,
            &request("inventory.triggered", false, true),
            at(9),
        )
        .await
        .expect("definition should be created");
    sqlx::query(
        "INSERT INTO alert_definition_triggers (id, alert_definition_id, event_type, occurred_at, payload) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(definition.id)
    .bind("inventory.changed")
    .bind(at(10))
    .bind(json!({"quantity": 0}))
    .execute(&pool)
    .await
    .expect("trigger history should seed");

    assert!(matches!(
        repo.delete(owner_id, definition.id).await,
        Err(AlertDefinitionRepositoryError::InUse)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_is_owner_scoped_and_missing_rows_are_not_found(pool: PgPool) {
    let owner_a = owner(&pool).await;
    let owner_b = owner(&pool).await;
    let repo = PgAlertDefinitionRepository::new(pool);
    let definition = repo
        .create(owner_a, &request("inventory.delete", false, true), at(9))
        .await
        .expect("definition should be created");

    assert!(matches!(
        repo.delete(owner_b, definition.id).await,
        Err(AlertDefinitionRepositoryError::NotFound)
    ));
    assert!(matches!(
        repo.delete(owner_a, Uuid::new_v4()).await,
        Err(AlertDefinitionRepositoryError::NotFound)
    ));
    repo.delete(owner_a, definition.id)
        .await
        .expect("owner should delete its own definition");
}

#[sqlx::test(migrations = "../../migrations")]
async fn gsp_forced_alert_cannot_be_deleted(pool: PgPool) {
    let owner_id = owner(&pool).await;
    let repo = PgAlertDefinitionRepository::new(pool.clone());
    let forced_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM alert_definitions WHERE owner_id = $1 AND is_gsp_forced ORDER BY alert_code LIMIT 1",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("GSP forced alert should seed");

    assert_eq!(
        repo.delete(owner_id, forced_id)
            .await
            .expect_err("GSP forced alert must not be deleted"),
        AlertDefinitionRepositoryError::GspForcedCannotDelete
    );
}
