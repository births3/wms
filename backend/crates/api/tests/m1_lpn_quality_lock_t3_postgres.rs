use sqlx::PgPool;
use uuid::Uuid;
use wms_api::{
    auth::AuthContext,
    lpn_container_repository::PgLpnContainerRepository,
};
use wms_domain::{
    ApplyContainerQualityLockRequest, ChangeContainerQualityLockReasonRequest,
    ReleaseContainerQualityLockRequest, CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING,
    CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY, LPN_LOCK_CATEGORY_QUALIFIED,
    LPN_LOCK_CATEGORY_QUARANTINE,
};

#[path = "support/lpn_container.rs"]
mod lpn_support;
mod postgres_test_support;

fn ctx(owner_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4(),
        owner_id,
        actor_name: "quality-lock-t3".to_string(),
        permissions: vec![
            "m1.master_data.write".to_string(),
            "m1.quality-lock.manage".to_string(),
        ],
        jti: Uuid::new_v4().to_string(),
        warehouse_scope: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn quality_lock_apply_change_and_release_write_audit(pool: PgPool) {
    // POST /api/v1/master-data/lpn-containers/{id}/quality-lock
    // PATCH /api/v1/master-data/lpn-containers/{id}/quality-lock
    // POST /api/v1/master-data/lpn-containers/{id}/quality-lock/release
    let owner_id = Uuid::new_v4();
    let actor = ctx(owner_id);
    let witness_id = Uuid::new_v4();
    postgres_test_support::ensure_audit_partition(&pool, lpn_support::at(0)).await;
    lpn_support::seed_lpn_numbering(&pool, lpn_support::at(0), owner_id).await;

    let repo = PgLpnContainerRepository::new(pool.clone());
    let container = lpn_support::setup_container_in_use(&repo, &actor, "t3-quality-lock").await;

    let locked = repo
        .quality_lock()
        .apply_quality_lock(
            &actor,
            container.id,
            ApplyContainerQualityLockRequest {
                lock_category: LPN_LOCK_CATEGORY_QUARANTINE.to_string(),
                reason_dict_item_code: CONTAINER_QUARANTINE_REASON_TEMP_ANOMALY.to_string(),
                reason_desc: Some("T3 审计证据".to_string()),
                evidence_urls: vec![],
                quality_liaison_id: None,
                witness_id,
                note: None,
                create_liaison: false,
            },
            lpn_support::at(3),
            "t3-quality-lock-apply",
        )
        .await
        .expect("apply quality lock");
    assert_eq!(
        locked.current_lock_category.as_deref(),
        Some(LPN_LOCK_CATEGORY_QUARANTINE)
    );

    let changed = repo
        .quality_lock()
        .change_quality_lock_reason(
            &actor,
            container.id,
            ChangeContainerQualityLockReasonRequest {
                lock_category: None,
                reason_dict_item_code: CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING.to_string(),
                reason_desc: Some("T3 原因变更证据".to_string()),
                evidence_urls: vec![],
                quality_liaison_id: None,
                witness_id,
                note: None,
            },
            lpn_support::at(4),
            "t3-quality-lock-change",
        )
        .await
        .expect("change quality lock reason");
    assert_eq!(
        changed.current_lock_reason_item_code.as_deref(),
        Some(CONTAINER_QUARANTINE_REASON_SALES_RETURN_PENDING)
    );

    let released = repo
        .quality_lock()
        .release_quality_lock(
            &actor,
            container.id,
            ReleaseContainerQualityLockRequest {
                witness_id,
                reason_desc: Some("T3 解锁证据".to_string()),
                quality_liaison_id: None,
                note: None,
            },
            lpn_support::at(5),
            "t3-quality-lock-release",
        )
        .await
        .expect("release quality lock");
    assert_eq!(
        released.container.current_lock_category.as_deref(),
        Some(LPN_LOCK_CATEGORY_QUALIFIED)
    );

    postgres_test_support::audit_event(&pool, owner_id, 5).await;
}
