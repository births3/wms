use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
    inventory_status_config::{InventoryStatusConfigError, PgInventoryStatusConfigRepository},
    wave3_repository::{PgWave3Repository, Wave3RepositoryError},
};
use wms_domain::{ChangeInventoryStatusRequest, UpsertInventoryStatusTransitionRequest};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m3-status-transition-test".to_string(),
        permissions: vec!["m3.read".to_string(), "m3.write".to_string()],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

async fn seed_batch(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now();
    sqlx::query(
        r#"
        INSERT INTO inventory_batches (
            id, owner_id, product_code, batch_no, production_date, expiry_date,
            qty_on_hand, qty_locked, quality_status, location_id, location_code,
            recall_flag, created_at, updated_at
        )
        VALUES ($1, $2, 'P-M3-CONFIG', 'B-M3-CONFIG', $3, $4, 10, 0, $5, $6, 'M3-CONFIG-01', FALSE, $7, $7)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid production date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"))
    .bind(STATUS_QUALIFIED)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("seed inventory batch");
    id
}

#[sqlx::test(migrations = "../../migrations")]
async fn status_transition_config_is_owner_scoped_idempotent_and_audited(pool: PgPool) {
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    let repository = PgInventoryStatusConfigRepository::new(pool.clone());
    let now = Utc::now();
    let request = UpsertInventoryStatusTransitionRequest {
        owner_id: Some(owner_a),
        approval_sources: vec![" owner-source ".to_string(), "owner-source".to_string()],
        enabled: true,
    };

    let first = repository
        .upsert(
            &ctx(owner_a),
            STATUS_QUALIFIED,
            STATUS_QUARANTINED,
            request.clone(),
            now,
            "m3-status-transition-config-001",
        )
        .await
        .expect("owner transition should be persisted");
    let replay = repository
        .upsert(
            &ctx(owner_a),
            STATUS_QUALIFIED,
            STATUS_QUARANTINED,
            request,
            now,
            "m3-status-transition-config-001",
        )
        .await
        .expect("same request should replay");

    assert!(!first.replayed);
    assert!(replay.replayed);
    assert_eq!(first.value.id, replay.value.id);
    assert_eq!(first.value.approval_sources, vec!["owner-source"]);
    assert_eq!(first.value.owner_id, Some(owner_a));

    sqlx::query(
        "UPDATE idempotency_request SET method = 'POST', path = '/wrong-path' WHERE owner_id = $1 AND idempotency_key = $2",
    )
    .bind(owner_a)
    .bind("m3-status-transition-config-001")
    .execute(&pool)
    .await
    .expect("idempotency metadata should be mutable for the regression check");
    let metadata_conflict = repository
        .upsert(
            &ctx(owner_a),
            STATUS_QUALIFIED,
            STATUS_QUARANTINED,
            UpsertInventoryStatusTransitionRequest {
                owner_id: Some(owner_a),
                approval_sources: vec![" owner-source ".to_string(), "owner-source".to_string()],
                enabled: true,
            },
            now,
            "m3-status-transition-config-001",
        )
        .await
        .expect_err("method and path changes must invalidate a replay");
    assert_eq!(
        metadata_conflict,
        InventoryStatusConfigError::IdempotencyConflict
    );

    let owner_a_rules = repository
        .list_effective(&ctx(owner_a))
        .await
        .expect("owner A rules should load");
    let owner_a_rule = owner_a_rules
        .data
        .iter()
        .find(|rule| rule.from_status == STATUS_QUALIFIED && rule.to_status == STATUS_QUARANTINED)
        .expect("owner A override should be effective");
    assert_eq!(owner_a_rule.approval_sources, vec!["owner-source"]);
    assert_eq!(owner_a_rule.owner_id, Some(owner_a));

    let owner_b_rules = repository
        .list_effective(&ctx(owner_b))
        .await
        .expect("owner B rules should load");
    let owner_b_rule = owner_b_rules
        .data
        .iter()
        .find(|rule| rule.from_status == STATUS_QUALIFIED && rule.to_status == STATUS_QUARANTINED)
        .expect("owner B should use global default");
    assert_eq!(owner_b_rule.owner_id, None);
    assert!(owner_b_rule
        .approval_sources
        .iter()
        .any(|source| source == "M-QL"));

    let error = repository
        .upsert(
            &ctx(owner_a),
            STATUS_QUALIFIED,
            STATUS_QUARANTINED,
            UpsertInventoryStatusTransitionRequest {
                owner_id: Some(owner_b),
                approval_sources: vec!["cross-owner".to_string()],
                enabled: true,
            },
            now,
            "m3-status-transition-config-cross-owner",
        )
        .await
        .expect_err("cross-owner rule must be rejected");
    assert_eq!(error, InventoryStatusConfigError::CrossOwnerAccess);

    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'upsert_inventory_status_transition'), (SELECT COUNT(*) FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2)",
    )
    .bind(owner_a)
    .bind("m3-status-transition-config-001")
    .fetch_one(&pool)
    .await
    .expect("config audit and idempotency evidence should query");
    assert_eq!(counts, (1, 1));
}

#[sqlx::test(migrations = "../../migrations")]
async fn inventory_status_change_uses_owner_transition_override(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let context = ctx(owner_id);
    let config = PgInventoryStatusConfigRepository::new(pool.clone());
    config
        .upsert(
            &context,
            STATUS_QUALIFIED,
            STATUS_QUARANTINED,
            UpsertInventoryStatusTransitionRequest {
                owner_id: Some(owner_id),
                approval_sources: vec!["OWNER-STATUS-REVIEW".to_string()],
                enabled: true,
            },
            Utc::now(),
            "m3-status-transition-config-002",
        )
        .await
        .expect("owner transition override should persist");

    let batch_id = seed_batch(&pool, owner_id).await;
    let repository = PgWave3Repository::new(pool.clone());
    let denied = repository
        .change_inventory_status_with_audit(
            &context,
            ChangeInventoryStatusRequest {
                batch_id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "owner rule should reject global source".to_string(),
                approval_source: "M-QL".to_string(),
                approval_id: "M3-CONFIG-002-DENIED".to_string(),
            },
            Utc::now(),
            "m3-status-transition-change-denied",
            None,
        )
        .await
        .expect_err("owner override must replace the global source list");
    assert!(matches!(
        denied,
        Wave3RepositoryError::InvalidStateTransition { .. }
    ));

    let accepted = repository
        .change_inventory_status_with_audit(
            &context,
            ChangeInventoryStatusRequest {
                batch_id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "owner rule accepted".to_string(),
                approval_source: "OWNER-STATUS-REVIEW".to_string(),
                approval_id: "M3-CONFIG-002-ACCEPTED".to_string(),
            },
            Utc::now(),
            "m3-status-transition-change-accepted",
            None,
        )
        .await
        .expect("owner source should be accepted");
    assert_eq!(accepted.value.quality_status, STATUS_QUARANTINED);
}

#[sqlx::test(migrations = "../../migrations")]
async fn status_transition_config_rejects_unknown_status_and_empty_sources(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let repository = PgInventoryStatusConfigRepository::new(pool);
    let invalid_status = repository
        .upsert(
            &ctx(owner_id),
            "unknown",
            STATUS_QUARANTINED,
            UpsertInventoryStatusTransitionRequest {
                owner_id: Some(owner_id),
                approval_sources: vec!["source".to_string()],
                enabled: true,
            },
            Utc::now(),
            "m3-status-transition-config-invalid-status",
        )
        .await
        .expect_err("unknown status must be rejected");
    assert_eq!(invalid_status, InventoryStatusConfigError::InvalidStatus);

    let empty_sources = repository
        .upsert(
            &ctx(owner_id),
            STATUS_QUALIFIED,
            STATUS_QUARANTINED,
            UpsertInventoryStatusTransitionRequest {
                owner_id: Some(owner_id),
                approval_sources: vec![" ".to_string()],
                enabled: true,
            },
            Utc::now(),
            "m3-status-transition-config-empty-source",
        )
        .await
        .expect_err("empty approval sources must be rejected");
    assert_eq!(
        empty_sources,
        InventoryStatusConfigError::InvalidApprovalSources
    );
}
