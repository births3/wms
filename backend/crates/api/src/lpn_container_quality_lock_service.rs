use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    batch_status_for_lock_category, validate_apply_lock, validate_change_reason,
    validate_release_lock, ApplyContainerQualityLockRequest,
    ChangeContainerQualityLockReasonRequest, LpnContainer, ReleaseContainerQualityLockRequest,
    ReleaseContainerQualityLockResponse, LPN_CONTAINER_STATUS_DISABLED,
    LPN_LOCK_CATEGORY_QUALIFIED,
};

use crate::{
    inventory::STATUS_QUALIFIED,
    lpn_container_repository::{
        append_lpn_audit, append_quality_lock_movement, append_quality_lock_status_change,
        bind_liaison_to_container, classify_unlock_batches, clear_container_lock_fields,
        create_liaison_for_lock, insert_lock_move_back_task, insert_lock_move_task,
        insert_quality_lock_event, latest_container_liaison_id, list_container_batch_statuses,
        list_container_batches_for_lock, lock_container_row_for_update, lock_idempotency_key,
        map_db_error, quality_liaison_exists, quality_liaison_status,
        release_batch_allocations_with_outbox, replay_idempotency, request_hash,
        rewrite_batch_qualified, store_idempotency_success, update_batch_lock_status,
        update_container_lock_fields, LpnContainerRepositoryError, QUALITY_LOCK_PATH,
        QUALITY_LOCK_RELEASE_PATH,
    },
    operation_context::OperationContext as AuthContext,
    system_dictionary::validate_container_quality_lock_reason_in_tx,
};

#[derive(Clone)]
pub struct LpnContainerQualityLockService {
    pool: PgPool,
}

impl LpnContainerQualityLockService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn from_repository(
        repository: &crate::lpn_container_repository::PgLpnContainerRepository,
    ) -> Self {
        Self::new(repository.pool())
    }
}

include!("lpn_container_repository_quality_lock_ops.rs");
