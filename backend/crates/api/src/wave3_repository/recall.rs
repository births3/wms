use chrono::{DateTime, Utc};
use sqlx::query_as;
use uuid::Uuid;
use wms_domain::{CancelInventoryRecallRequest, InventoryBatch, MarkInventoryRecallRequest};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    inventory::{STATUS_QUALIFIED, STATUS_QUARANTINED},
};

use super::{
    map_db_error, map_inventory_batch, replay_idempotency, request_hash, store_idempotency_success,
    IdempotentMutation, InventoryBatchRow, PgWave3Repository, Wave3RepositoryError,
};

const RECALL_SOURCE_M_QL: &str = "M-QL";
const RECALL_SOURCE_M_TC: &str = "M-TC";

impl PgWave3Repository {
    pub async fn mark_inventory_batch_recalled(
        &self,
        ctx: &AuthContext,
        req: MarkInventoryRecallRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InventoryBatch>, Wave3RepositoryError> {
        if req.reason.trim().is_empty()
            || req.approval_id.trim().is_empty()
            || !matches!(
                req.approval_source.as_str(),
                RECALL_SOURCE_M_QL | RECALL_SOURCE_M_TC
            )
        {
            return Err(Wave3RepositoryError::MissingApprovalSource);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
        let mut tx = self.begin().await?;
        super::lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<InventoryBatch>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let row = query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.batch_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let previous = map_inventory_batch(row);
        if previous.recall_flag {
            return Err(Wave3RepositoryError::RecallAlreadyActive);
        }
        let next_status = if previous.quality_status == STATUS_QUALIFIED {
            STATUS_QUARANTINED.to_string()
        } else {
            previous.quality_status.clone()
        };

        if next_status != previous.quality_status
            && !crate::inventory_status_config::is_transition_allowed_in_tx(
                &mut tx,
                ctx.owner_id,
                &previous.quality_status,
                &next_status,
                &req.approval_source,
            )
            .await
            .map_err(map_db_error)?
        {
            return Err(Wave3RepositoryError::InvalidStateTransition {
                from: previous.quality_status,
                to: next_status,
                approval_source: req.approval_source,
            });
        }

        if next_status != previous.quality_status {
            sqlx::query(
                r#"
                INSERT INTO inventory_status_changes (
                    id, owner_id, batch_id, from_status, to_status,
                    reason, approval_source, approval_id, occurred_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(previous.id)
            .bind(&previous.quality_status)
            .bind(&next_status)
            .bind(&req.reason)
            .bind(&req.approval_source)
            .bind(&req.approval_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        sqlx::query(
            r#"
            INSERT INTO inventory_recall_actions (
                id, owner_id, batch_id, recall_approval_source, recall_approval_id,
                previous_quality_status, marked_by, marked_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(previous.id)
        .bind(&req.approval_source)
        .bind(&req.approval_id)
        .bind(&previous.quality_status)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let updated = query_as::<_, InventoryBatchRow>(
            r#"
            UPDATE inventory_batches
               SET recall_flag = TRUE,
                   quality_status = $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2
            RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                      qty_on_hand, qty_locked, quality_status, location_id, location_code,
                      recall_flag, created_at, updated_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(previous.id)
        .bind(&next_status)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let batch = map_inventory_batch(updated);

        let mut audit_event = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "mark_inventory_recall",
                "M3",
                "inventory_batch",
                batch.id.to_string(),
                None,
            )
        });
        audit_event.occurred_at = now;
        audit_event.action = "mark_inventory_recall".to_string();
        audit_event.module = "M3".to_string();
        audit_event.resource_type = "inventory_batch".to_string();
        audit_event.resource_id = batch.id.to_string();
        audit_event.diff = Some(AuditDiff::compute(
            serde_json::json!({
                "recall_flag": previous.recall_flag,
                "quality_status": previous.quality_status,
            }),
            serde_json::json!({
                "recall_flag": true,
                "quality_status": batch.quality_status,
            }),
        ));
        append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/batches/recall",
            "inventory_batch",
            batch.id.to_string(),
            &batch,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: batch,
            replayed: false,
        })
    }

    pub async fn cancel_inventory_batch_recall(
        &self,
        ctx: &AuthContext,
        req: CancelInventoryRecallRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InventoryBatch>, Wave3RepositoryError> {
        if req.reason.trim().is_empty() || req.approval_id.trim().is_empty() {
            return Err(Wave3RepositoryError::MissingApprovalSource);
        }
        if req.second_approver_id == ctx.user_id {
            return Err(Wave3RepositoryError::SameApprover);
        }
        let request_hash = request_hash(&serde_json::json!({ "request": req }))?;
        let mut tx = self.begin().await?;
        super::lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<InventoryBatch>(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let second_approver_authorized: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM auth_user_owner_bindings binding
                  JOIN auth_users app_user
                    ON app_user.id = binding.user_id
                  JOIN auth_user_roles user_role
                    ON user_role.user_id = binding.user_id
                   AND user_role.owner_id = binding.owner_id
                  JOIN auth_role_permissions role_permission
                    ON role_permission.role_id = user_role.role_id
                  JOIN auth_permissions permission
                    ON permission.id = role_permission.permission_id
                 WHERE binding.owner_id = $1
                   AND binding.user_id = $2
                   AND binding.is_active
                   AND app_user.status = 'active'
                   AND permission.permission_code = 'm3.recall.approve'
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.second_approver_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if !second_approver_authorized {
            return Err(Wave3RepositoryError::SecondApproverNotAuthorized);
        }

        let active_recall: Option<(String, Uuid)> = sqlx::query_as(
            r#"
            SELECT previous_quality_status, marked_by
              FROM inventory_recall_actions
             WHERE owner_id = $1 AND batch_id = $2 AND canceled_at IS NULL
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.batch_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let (previous_quality_status, marked_by) =
            active_recall.ok_or(Wave3RepositoryError::RecallNotActive)?;

        let row = query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE owner_id = $1 AND id = $2
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.batch_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let previous = map_inventory_batch(row);
        if !previous.recall_flag {
            return Err(Wave3RepositoryError::RecallNotActive);
        }
        let expected_status = if previous_quality_status == STATUS_QUALIFIED {
            STATUS_QUARANTINED
        } else {
            previous_quality_status.as_str()
        };
        if previous.quality_status != expected_status {
            return Err(Wave3RepositoryError::RecallStateChanged);
        }

        let restored_status = previous_quality_status;
        let updated = query_as::<_, InventoryBatchRow>(
            r#"
            UPDATE inventory_batches
               SET recall_flag = FALSE,
                   quality_status = $3,
                   updated_at = $4,
                   version = version + 1
             WHERE owner_id = $1 AND id = $2 AND recall_flag = TRUE
            RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                      qty_on_hand, qty_locked, quality_status, location_id, location_code,
                      recall_flag, created_at, updated_at
            "#,
        )
        .bind(ctx.owner_id)
        .bind(previous.id)
        .bind(&restored_status)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let batch = map_inventory_batch(updated);

        if previous.quality_status != batch.quality_status {
            sqlx::query(
                r#"
                INSERT INTO inventory_status_changes (
                    id, owner_id, batch_id, from_status, to_status,
                    reason, approval_source, approval_id, occurred_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 'M3-002-RECALL-CANCEL', $7, $8)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(batch.id)
            .bind(&previous.quality_status)
            .bind(&batch.quality_status)
            .bind(&req.reason)
            .bind(&req.approval_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        sqlx::query(
            r#"
            UPDATE inventory_recall_actions
               SET canceled_by = $3,
                   canceled_at = $4,
                   cancel_approval_id = $5,
                   cancel_reason = $6
             WHERE owner_id = $1 AND batch_id = $2 AND canceled_at IS NULL
            "#,
        )
        .bind(ctx.owner_id)
        .bind(batch.id)
        .bind(ctx.user_id)
        .bind(now)
        .bind(&req.approval_id)
        .bind(&req.reason)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut audit_event = audit.unwrap_or_else(|| {
            AuditWriteRequest::from_auth_context(
                ctx,
                "cancel_inventory_recall",
                "M3",
                "inventory_batch",
                batch.id.to_string(),
                None,
            )
        });
        audit_event.occurred_at = now;
        audit_event.action = "cancel_inventory_recall".to_string();
        audit_event.module = "M3".to_string();
        audit_event.resource_type = "inventory_batch".to_string();
        audit_event.resource_id = batch.id.to_string();
        audit_event.diff = Some(AuditDiff::compute(
            serde_json::json!({
                "recall_flag": previous.recall_flag,
                "quality_status": previous.quality_status,
                "marked_by": marked_by,
            }),
            serde_json::json!({
                "recall_flag": batch.recall_flag,
                "quality_status": batch.quality_status,
                "cancelled_by": ctx.user_id,
                "second_approver_id": req.second_approver_id,
                "approval_id": req.approval_id,
            }),
        ));
        append_event_in_tx(&mut tx, &audit_event)
            .await
            .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/batches/recall/cancel",
            "inventory_batch",
            batch.id.to_string(),
            &batch,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: batch,
            replayed: false,
        })
    }
}
