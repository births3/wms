use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
    wave3_repository::PgWave3Repository,
};
use wms_domain::{CancelInventoryRecallRequest, MarkInventoryRecallRequest};

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "m3-recall-test".to_string(),
        permissions: vec!["m3.write".to_string()],
        jti: Uuid::new_v4().to_string(),
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
        VALUES ($1, $2, 'P-RECALL-001', 'B-RECALL-001', $3, $4, 10, 0, $5, $6, 'R-01', FALSE, $7, $7)
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .bind(NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid production date"))
    .bind(NaiveDate::from_ymd_opt(2028, 1, 1).expect("valid expiry date"))
    .bind(STATUS_QUALIFIED)
    .bind(Uuid::new_v4())
    .bind(now)
    .execute(pool)
    .await
    .expect("seed inventory batch");
    id
}

async fn seed_quality_approver(pool: &PgPool, owner_id: Uuid, user_id: Uuid) {
    let role_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_owners(id, owner_code, owner_name) VALUES ($1, 'M3-TEST-OWNER', 'M3 test owner') ON CONFLICT (id) DO NOTHING",
    )
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed auth owner");
    sqlx::query(
        "INSERT INTO auth_users(id, username, display_name, password_hash, status) VALUES ($1, $2, 'M3 quality approver', 'test-hash', 'active')",
    )
    .bind(user_id)
    .bind(format!("m3-quality-{}", user_id.simple()))
    .execute(pool)
    .await
    .expect("seed quality approver");
    sqlx::query(
        "INSERT INTO auth_user_owner_bindings(user_id, owner_id, is_active) VALUES ($1, $2, TRUE)",
    )
    .bind(user_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("bind quality approver");
    sqlx::query(
        "INSERT INTO auth_roles(id, owner_id, role_code, role_name) VALUES ($1, $2, 'm3_quality_approver', 'M3 质量审批人')",
    )
    .bind(role_id)
    .bind(owner_id)
    .execute(pool)
    .await
    .expect("seed quality role");
    sqlx::query("INSERT INTO auth_user_roles(user_id, owner_id, role_id) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(owner_id)
        .bind(role_id)
        .execute(pool)
        .await
        .expect("bind quality role");
    sqlx::query(
        "INSERT INTO auth_role_permissions(role_id, permission_id) SELECT $1, id FROM auth_permissions WHERE permission_code = 'm3.recall.approve'",
    )
    .bind(role_id)
    .execute(pool)
    .await
    .expect("grant recall approval permission");
}

#[sqlx::test(migrations = "../../migrations")]
async fn quality_recall_marks_batch_isolated_and_replays_idempotently(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let repo = PgWave3Repository::new(pool.clone());
    let batch_id = seed_batch(&pool, owner_id).await;

    let first = repo
        .mark_inventory_batch_recalled(
            &ctx,
            MarkInventoryRecallRequest {
                batch_id,
                approval_source: "M-QL".to_string(),
                approval_id: "QL-RECALL-001".to_string(),
                reason: "质量联系单确认召回".to_string(),
            },
            Utc::now(),
            "m3-recall-001",
            None,
        )
        .await
        .expect("recall mark should persist");
    assert!(!first.replayed);
    assert!(first.value.recall_flag);
    assert_eq!(first.value.quality_status, STATUS_QUARANTINED);

    let evidence: (bool, String, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            recall_flag,
            quality_status,
            (SELECT COUNT(*) FROM inventory_status_changes
              WHERE owner_id = $1 AND batch_id = $2
                AND approval_source = 'M-QL' AND approval_id = 'QL-RECALL-001'),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'mark_inventory_recall'
                AND resource_id = $2::TEXT)
          FROM inventory_batches
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("recall evidence");
    assert_eq!(evidence, (true, STATUS_QUARANTINED.to_string(), 1, 1));

    let replay = repo
        .mark_inventory_batch_recalled(
            &ctx,
            MarkInventoryRecallRequest {
                batch_id,
                approval_source: "M-QL".to_string(),
                approval_id: "QL-RECALL-001".to_string(),
                reason: "质量联系单确认召回".to_string(),
            },
            Utc::now(),
            "m3-recall-001",
            None,
        )
        .await
        .expect("same recall request should replay");
    assert!(replay.replayed);
    assert_eq!(replay.value.id, first.value.id);

    let status_changes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM inventory_status_changes WHERE owner_id = $1 AND batch_id = $2 AND approval_id = 'QL-RECALL-001'",
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("status change count");
    assert_eq!(status_changes, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn quality_recall_cancel_requires_second_approver_and_restores_previous_status(pool: PgPool) {
    let owner_id = Uuid::new_v4();
    let ctx = ctx(owner_id);
    let second_approver_id = Uuid::new_v4();
    seed_quality_approver(&pool, owner_id, second_approver_id).await;
    let repo = PgWave3Repository::new(pool.clone());
    let batch_id = seed_batch(&pool, owner_id).await;

    repo.mark_inventory_batch_recalled(
        &ctx,
        MarkInventoryRecallRequest {
            batch_id,
            approval_source: "M-QL".to_string(),
            approval_id: "QL-CANCEL-001".to_string(),
            reason: "质量联系单确认召回".to_string(),
        },
        Utc::now(),
        "m3-recall-cancel-mark-001",
        None,
    )
    .await
    .expect("recall mark should persist");

    let cancel_request = CancelInventoryRecallRequest {
        batch_id,
        approval_id: "QL-CANCEL-APPROVAL-001".to_string(),
        second_approver_id,
        reason: "质量负责人复核后取消召回".to_string(),
    };
    let canceled = repo
        .cancel_inventory_batch_recall(
            &ctx,
            cancel_request.clone(),
            Utc::now(),
            "m3-recall-cancel-001",
            None,
        )
        .await
        .expect("recall cancel should persist");
    assert!(!canceled.replayed);
    assert!(!canceled.value.recall_flag);
    assert_eq!(canceled.value.quality_status, STATUS_QUALIFIED);

    let evidence: (bool, String, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            recall_flag,
            quality_status,
            (SELECT COUNT(*) FROM inventory_status_changes
              WHERE owner_id = $1 AND batch_id = $2),
            (SELECT COUNT(*) FROM audit_event
              WHERE owner_id = $1 AND action = 'cancel_inventory_recall'
                AND resource_id = $2::TEXT),
            (SELECT COUNT(*) FROM inventory_recall_actions
              WHERE owner_id = $1 AND batch_id = $2 AND canceled_at IS NOT NULL)
          FROM inventory_batches
         WHERE owner_id = $1 AND id = $2
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("recall cancel evidence");
    assert_eq!(evidence, (false, STATUS_QUALIFIED.to_string(), 2, 1, 1));

    let replay = repo
        .cancel_inventory_batch_recall(
            &ctx,
            cancel_request,
            Utc::now(),
            "m3-recall-cancel-001",
            None,
        )
        .await
        .expect("same cancel request should replay");
    assert!(replay.replayed);
    assert!(!replay.value.recall_flag);

    let cancel_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_event WHERE owner_id = $1 AND action = 'cancel_inventory_recall' AND resource_id = $2::TEXT",
    )
    .bind(owner_id)
    .bind(batch_id)
    .fetch_one(&pool)
    .await
    .expect("cancel audit count");
    assert_eq!(cancel_audit_count, 1);
}
