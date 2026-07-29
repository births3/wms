//! M-RC service-only 调度认领、续租、失败与完成闭环。

use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    reconciliation::{
        db, lock_idempotency, replay_idempotency, request_hash, store_idempotency,
        IdempotentMutation, PgReconciliationRepository, ReconciliationError,
    },
    reconciliation_query::{
        ClaimReconciliationRequest, FailReconciliationClaimRequest, ReconciliationClaimFailureCode,
        ReconciliationClaimFailureStage, ReconciliationClaimMutation, ReconciliationClaimResponse,
        ReconciliationDueOwner, ReconciliationScheduleClaim, RenewReconciliationClaimRequest,
    },
};

type ReconciliationClaimRow = (Uuid, String, String, DateTime<Utc>, Option<Uuid>);
type FailedReconciliationClaimRow = (
    Uuid,
    String,
    String,
    DateTime<Utc>,
    Option<Uuid>,
    String,
    Option<String>,
    Option<String>,
);

impl PgReconciliationRepository {
    pub async fn claim_due_window(
        &self,
        ctx: &AuthContext,
        req: ClaimReconciliationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<ReconciliationClaimResponse>, ReconciliationError> {
        let worker_id = req.worker_id.trim().to_owned();
        if worker_id.is_empty() || worker_id.len() > 128 || !(30..=900).contains(&req.lease_seconds)
        {
            return Err(ReconciliationError::InvalidRequest);
        }
        let normalized = ClaimReconciliationRequest {
            worker_id,
            lease_seconds: req.lease_seconds,
        };
        let hash = request_hash(&normalized)?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        lock_idempotency(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        crate::reconciliation::lock_reconciliation_window(
            &mut tx,
            ctx.owner_id,
            "__schedule_claim__",
        )
        .await?;

        let due = select_due_owner_in_tx(&mut tx, ctx.owner_id, now).await?;
        let mut claim = None;
        if let Some(due) = due {
            let active: Option<(Uuid, DateTime<Utc>)> = sqlx::query_as(
                "SELECT id, lease_expires_at
                   FROM reconciliation_schedule_claims
                  WHERE owner_id=$1 AND window_key=$2 AND status='active'
                  FOR UPDATE",
            )
            .bind(ctx.owner_id)
            .bind(&due.window_key)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((active_id, lease_expires_at)) = active {
                if lease_expires_at > now {
                    let value = ReconciliationClaimResponse { claim: None };
                    store_idempotency(
                        &mut tx,
                        ctx.owner_id,
                        idempotency_key,
                        &hash,
                        "POST",
                        "/api/v1/reconciliation/claims",
                        "reconciliation_schedule_claim",
                        ctx.owner_id.to_string(),
                        &value,
                        now,
                    )
                    .await?;
                    tx.commit().await.map_err(db)?;
                    return Ok(IdempotentMutation {
                        value,
                        replayed: false,
                    });
                }
                sqlx::query(
                    "UPDATE reconciliation_schedule_claims
                        SET status='expired', failure_stage='lease',
                            failure_code='lease_expired', failed_at=$3, updated_at=$3
                      WHERE owner_id=$1 AND id=$2 AND status='active'",
                )
                .bind(ctx.owner_id)
                .bind(active_id)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                append_claim_audit(
                    &mut tx,
                    ctx,
                    "expire_reconciliation_claim",
                    active_id,
                    json!({"status": "active"}),
                    json!({"status": "expired", "failure_code": "lease_expired"}),
                    now,
                )
                .await?;
                insert_claim_notification(
                    &mut tx,
                    ctx.owner_id,
                    "rc.reconciliation.lease_expired",
                    active_id,
                    format!(
                        "库存对账调度租约已过期，窗口 {} 将由其他 Worker 接管",
                        due.window_key
                    ),
                    now,
                )
                .await?;
            }

            let attempt_no: i32 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(attempt_no), 0) + 1
                   FROM reconciliation_schedule_claims
                  WHERE owner_id=$1 AND window_key=$2",
            )
            .bind(ctx.owner_id)
            .bind(&due.window_key)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let id = Uuid::new_v4();
            let claim_token = Uuid::new_v4();
            let lease_expires_at = now + Duration::seconds(normalized.lease_seconds);
            sqlx::query(
                "INSERT INTO reconciliation_schedule_claims
                 (id, owner_id, window_key, claim_token, worker_id, attempt_no, status,
                  lease_expires_at, claimed_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,'active',$7,$8,$8)",
            )
            .bind(id)
            .bind(ctx.owner_id)
            .bind(&due.window_key)
            .bind(claim_token)
            .bind(&normalized.worker_id)
            .bind(attempt_no)
            .bind(lease_expires_at)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            append_claim_audit(
                &mut tx,
                ctx,
                "claim_reconciliation_window",
                id,
                json!({}),
                json!({
                    "window_key": due.window_key,
                    "worker_id": normalized.worker_id,
                    "attempt_no": attempt_no,
                    "lease_expires_at": lease_expires_at,
                }),
                now,
            )
            .await?;
            claim = Some(ReconciliationScheduleClaim {
                id,
                claim_token,
                owner_id: ctx.owner_id,
                window_key: due.window_key,
                worker_id: normalized.worker_id,
                attempt_no,
                lease_expires_at,
            });
        }
        let value = ReconciliationClaimResponse { claim };
        let resource_id = value
            .claim
            .as_ref()
            .map_or_else(|| ctx.owner_id.to_string(), |claim| claim.id.to_string());
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/reconciliation/claims",
            "reconciliation_schedule_claim",
            resource_id,
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn renew_claim(
        &self,
        ctx: &AuthContext,
        claim_id: Uuid,
        req: RenewReconciliationClaimRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<ReconciliationClaimMutation>, ReconciliationError> {
        let worker_id = req.worker_id.trim().to_owned();
        if worker_id.is_empty() || worker_id.len() > 128 || !(30..=900).contains(&req.lease_seconds)
        {
            return Err(ReconciliationError::InvalidRequest);
        }
        let normalized = RenewReconciliationClaimRequest {
            claim_token: req.claim_token,
            worker_id,
            lease_seconds: req.lease_seconds,
        };
        let hash = request_hash(&(claim_id, &normalized))?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        lock_idempotency(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let row: Option<ReconciliationClaimRow> = sqlx::query_as(
            "SELECT claim_token, worker_id, status, lease_expires_at, run_id
               FROM reconciliation_schedule_claims
              WHERE owner_id=$1 AND id=$2
              FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(claim_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((stored_token, stored_worker, status, previous_lease_expires_at, run_id)) = row
        else {
            return Err(ReconciliationError::ClaimInvalid);
        };
        if stored_token != normalized.claim_token || stored_worker != normalized.worker_id {
            return Err(ReconciliationError::ClaimInvalid);
        }
        if status != "active" {
            return Err(ReconciliationError::ClaimInvalid);
        }
        if previous_lease_expires_at <= now {
            return Err(ReconciliationError::ClaimExpired);
        }
        let lease_expires_at = now + Duration::seconds(normalized.lease_seconds);
        sqlx::query(
            "UPDATE reconciliation_schedule_claims
                SET lease_expires_at=$3, updated_at=$4
              WHERE owner_id=$1 AND id=$2 AND status='active'",
        )
        .bind(ctx.owner_id)
        .bind(claim_id)
        .bind(lease_expires_at)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        append_claim_audit(
            &mut tx,
            ctx,
            "renew_reconciliation_claim",
            claim_id,
            json!({"lease_expires_at": previous_lease_expires_at}),
            json!({"lease_expires_at": lease_expires_at}),
            now,
        )
        .await?;
        let value = ReconciliationClaimMutation {
            id: claim_id,
            status,
            lease_expires_at,
            run_id,
        };
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &format!("/api/v1/reconciliation/claims/{claim_id}/renew"),
            "reconciliation_schedule_claim",
            claim_id.to_string(),
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn fail_claim(
        &self,
        ctx: &AuthContext,
        claim_id: Uuid,
        req: FailReconciliationClaimRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<ReconciliationClaimMutation>, ReconciliationError> {
        if !valid_failure_pair(req.stage, req.error_code) {
            return Err(ReconciliationError::InvalidRequest);
        }
        let failure_code = claim_failure_code_text(req.error_code);
        let hash = request_hash(&(claim_id, &req))?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        lock_idempotency(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let row: Option<FailedReconciliationClaimRow> = sqlx::query_as(
            "SELECT claim_token, worker_id, status, lease_expires_at, run_id, window_key,
                    failure_stage, failure_code
               FROM reconciliation_schedule_claims
              WHERE owner_id=$1 AND id=$2
              FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(claim_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((
            stored_token,
            worker_id,
            status,
            lease_expires_at,
            run_id,
            window_key,
            stored_stage,
            stored_code,
        )) = row
        else {
            return Err(ReconciliationError::ClaimInvalid);
        };
        if stored_token != req.claim_token || status == "completed" {
            return Err(ReconciliationError::ClaimInvalid);
        }
        if status == "expired" {
            return Err(ReconciliationError::ClaimExpired);
        }
        if status != "active" && status != "failed" {
            return Err(ReconciliationError::ClaimInvalid);
        }
        let failure_stage = claim_failure_stage_text(req.stage);
        if status == "failed"
            && (stored_stage.as_deref() != Some(failure_stage)
                || stored_code.as_deref() != Some(failure_code))
        {
            return Err(ReconciliationError::IdempotencyConflict);
        }
        if status != "failed" {
            sqlx::query(
                "UPDATE reconciliation_schedule_claims
                    SET status='failed', failure_stage=$3, failure_code=$4,
                        failed_at=$5, updated_at=$5
                  WHERE owner_id=$1 AND id=$2 AND status='active'",
            )
            .bind(ctx.owner_id)
            .bind(claim_id)
            .bind(failure_stage)
            .bind(failure_code)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            append_claim_audit(
                &mut tx,
                ctx,
                "fail_reconciliation_claim",
                claim_id,
                json!({"status": status}),
                json!({
                    "status": "failed",
                    "window_key": window_key,
                    "worker_id": worker_id,
                    "failure_stage": failure_stage,
                    "failure_code": failure_code,
                }),
                now,
            )
            .await?;
            insert_claim_notification(
                &mut tx,
                ctx.owner_id,
                "rc.reconciliation.worker_failed",
                claim_id,
                format!(
                    "库存对账 Worker 执行失败：窗口 {}，阶段 {}，错误码 {}",
                    window_key, failure_stage, failure_code
                ),
                now,
            )
            .await?;
        }
        let value = ReconciliationClaimMutation {
            id: claim_id,
            status: "failed".into(),
            lease_expires_at,
            run_id,
        };
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            &format!("/api/v1/reconciliation/claims/{claim_id}/failed"),
            "reconciliation_schedule_claim",
            claim_id.to_string(),
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub(crate) async fn validate_claim_for_run(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        claim_id: Uuid,
        claim_token: Uuid,
        window_key: &str,
        now: DateTime<Utc>,
    ) -> Result<Uuid, ReconciliationError> {
        let row: Option<(Uuid, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT claim_token, status, lease_expires_at
               FROM reconciliation_schedule_claims
              WHERE owner_id=$1 AND id=$2 AND window_key=$3
              FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(claim_id)
        .bind(window_key)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        let Some((stored_token, status, lease_expires_at)) = row else {
            return Err(ReconciliationError::ClaimInvalid);
        };
        if stored_token != claim_token || status != "active" {
            return Err(ReconciliationError::ClaimInvalid);
        }
        if lease_expires_at <= now {
            return Err(ReconciliationError::ClaimExpired);
        }
        Ok(claim_id)
    }

    pub(crate) async fn complete_claim_for_run(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        claim_id: Uuid,
        run_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), ReconciliationError> {
        let updated = sqlx::query(
            "UPDATE reconciliation_schedule_claims
                SET status='completed', run_id=$3, completed_at=$4, updated_at=$4
              WHERE owner_id=$1 AND id=$2 AND status='active'",
        )
        .bind(ctx.owner_id)
        .bind(claim_id)
        .bind(run_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        if updated.rows_affected() != 1 {
            return Err(ReconciliationError::ClaimInvalid);
        }
        append_claim_audit(
            tx,
            ctx,
            "complete_reconciliation_claim",
            claim_id,
            json!({"status": "active"}),
            json!({"status": "completed", "run_id": run_id}),
            now,
        )
        .await
    }
}

async fn select_due_owner_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    now: DateTime<Utc>,
) -> Result<Option<ReconciliationDueOwner>, ReconciliationError> {
    let row: Option<(Uuid, i32, DateTime<Utc>)> = sqlx::query_as(
        "WITH latest_run AS (
             SELECT owner_id, MAX(created_at) AS last_run_at
               FROM reconciliation_runs
              WHERE status = 'completed'
              GROUP BY owner_id
         )
         SELECT owner.id,
                COALESCE(rule.interval_hours, 24) AS interval_hours,
                COALESCE(
                    latest.last_run_at
                        + make_interval(hours => COALESCE(rule.interval_hours, 24)),
                    date_trunc('hour', owner.created_at)
                ) AS next_due_at
           FROM auth_owners owner
           LEFT JOIN reconciliation_rules rule ON rule.owner_id = owner.id
           LEFT JOIN latest_run latest ON latest.owner_id = owner.id
          WHERE owner.id = $2
            AND COALESCE(rule.enabled, TRUE)
            AND (
                latest.last_run_at IS NULL
                OR latest.last_run_at
                    + make_interval(hours => COALESCE(rule.interval_hours, 24)) <= $1
            )
            AND EXISTS (
                SELECT 1
                  FROM h8_erp_connectors connector
                 WHERE connector.owner_id = owner.id
                   AND connector.status = 'active'
                   AND 'outbound' = ANY(connector.directions)
                   AND 'inventory_snapshot' = ANY(connector.message_types)
                   AND cardinality(connector.warehouse_ids) = 0
                   AND connector.api_base_url IS NOT NULL
            )",
    )
    .bind(now)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    Ok(row.map(
        |(owner_id, interval_hours, next_due_at)| ReconciliationDueOwner {
            owner_id,
            interval_hours,
            next_due_at,
            window_key: format!(
                "scheduled:{}:{}",
                owner_id,
                next_due_at.format("%Y%m%dT%H%M%SZ")
            ),
        },
    ))
}

async fn append_claim_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    claim_id: Uuid,
    before: serde_json::Value,
    after: serde_json::Value,
    now: DateTime<Utc>,
) -> Result<(), ReconciliationError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M-RC",
        "reconciliation_schedule_claim",
        claim_id.to_string(),
        Some(AuditDiff::compute(before, after)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map(|_| ())
        .map_err(|error| ReconciliationError::Audit(format!("{error:?}")))
}

async fn insert_claim_notification(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    event_type: &str,
    claim_id: Uuid,
    content: String,
    now: DateTime<Utc>,
) -> Result<(), ReconciliationError> {
    sqlx::query(
        "INSERT INTO h4_notification_records
         (id, owner_id, event_type, dedupe_key, recipient, channel, content,
          content_summary, status, failure_reason, created_at, updated_at)
         VALUES ($1,$2,$3,$4,'system_admin','wechat',$5,$5,'retrying',
                 'awaiting_wechat_delivery',$6,$6)
         ON CONFLICT (owner_id, event_type, recipient, dedupe_key) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(event_type)
    .bind(claim_id.to_string())
    .bind(content)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

fn claim_failure_stage_text(stage: ReconciliationClaimFailureStage) -> &'static str {
    match stage {
        ReconciliationClaimFailureStage::Pull => "pull",
        ReconciliationClaimFailureStage::Submit => "submit",
    }
}

fn claim_failure_code_text(code: ReconciliationClaimFailureCode) -> &'static str {
    match code {
        ReconciliationClaimFailureCode::ErpPullFailed => "erp_pull_failed",
        ReconciliationClaimFailureCode::SnapshotSubmitFailed => "snapshot_submit_failed",
    }
}

fn valid_failure_pair(
    stage: ReconciliationClaimFailureStage,
    code: ReconciliationClaimFailureCode,
) -> bool {
    matches!(
        (stage, code),
        (
            ReconciliationClaimFailureStage::Pull,
            ReconciliationClaimFailureCode::ErpPullFailed
        ) | (
            ReconciliationClaimFailureStage::Submit,
            ReconciliationClaimFailureCode::SnapshotSubmitFailed
        )
    )
}
