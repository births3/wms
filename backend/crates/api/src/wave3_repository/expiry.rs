use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::query_as;
use uuid::Uuid;
use wms_domain::InventoryBatch;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    inventory::{APPROVAL_SOURCE_EXPIRY, STATUS_QUALIFIED, STATUS_UNQUALIFIED},
};

use super::{
    map_db_error, map_inventory_batch, replay_idempotency, request_hash, store_idempotency_success,
    IdempotentMutation, InventoryBatchRow, PgWave3Repository, Wave3RepositoryError,
};

impl PgWave3Repository {
    pub async fn list_near_expiry_batches(
        &self,
        ctx: &AuthContext,
        as_of: NaiveDate,
        requested_warning_days: Option<i64>,
    ) -> Result<Vec<InventoryBatch>, Wave3RepositoryError> {
        let warning_days = match requested_warning_days {
            Some(days) => validate_warning_days(days)?,
            None => self.resolve_expiry_warning_days(ctx, as_of).await?,
        };
        let end_date = as_of
            .checked_add_signed(Duration::days(warning_days))
            .ok_or_else(|| Wave3RepositoryError::InvalidDate(as_of.to_string()))?;
        let start_date = as_of.to_string();
        let end_date = end_date.to_string();
        let mut data = self.list_inventory_batches(ctx).await?;
        data.retain(|batch| {
            let expiry_date = batch.expiry_date.as_str();
            expiry_date >= start_date.as_str() && expiry_date <= end_date.as_str()
        });
        data.sort_by(|left, right| {
            left.expiry_date
                .cmp(&right.expiry_date)
                .then_with(|| left.product_code.cmp(&right.product_code))
                .then_with(|| left.batch_no.cmp(&right.batch_no))
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(data)
    }

    /// 读取货主生效的近效期预警天数（系统字典 inventory_policy.expiry_warning_days）。
    pub(crate) async fn resolve_expiry_warning_days(
        &self,
        ctx: &AuthContext,
        as_of: NaiveDate,
    ) -> Result<i64, Wave3RepositoryError> {
        let effective_at = DateTime::<Utc>::from_naive_utc_and_offset(
            as_of
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| Wave3RepositoryError::InvalidDate(as_of.to_string()))?,
            Utc,
        );
        let items = crate::system_dictionary::PgSystemDictionaryRepository::new(self.pool.clone())
            .list_effective_items(ctx, "inventory_policy", effective_at)
            .await
            .map_err(map_system_dictionary_error)?;
        let warning_days = items
            .into_iter()
            .find(|item| item.item_code == "expiry_warning_days")
            .and_then(|item| {
                item.params
                    .get("warning_days")
                    .and_then(serde_json::Value::as_i64)
            })
            .ok_or(Wave3RepositoryError::InvalidQuantity)?;
        validate_warning_days(warning_days)
    }

    pub async fn isolate_expired_inventory_batches(
        &self,
        ctx: &AuthContext,
        as_of: NaiveDate,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<Vec<InventoryBatch>>, Wave3RepositoryError> {
        let request_hash = request_hash(&serde_json::json!({ "as_of": as_of }))?;
        let mut tx = self.begin().await?;
        super::lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) = replay_idempotency::<Vec<InventoryBatch>>(
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

        let expired_rows = query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE owner_id = $1
               AND expiry_date <= $2
               AND quality_status = $3
             ORDER BY expiry_date ASC, id ASC
             FOR UPDATE
            "#,
        )
        .bind(ctx.owner_id)
        .bind(as_of)
        .bind(STATUS_QUALIFIED)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let mut isolated = Vec::with_capacity(expired_rows.len());
        for row in expired_rows {
            let previous = map_inventory_batch(row);
            if !crate::inventory_status_config::is_transition_allowed_in_tx(
                &mut tx,
                ctx.owner_id,
                &previous.quality_status,
                STATUS_UNQUALIFIED,
                APPROVAL_SOURCE_EXPIRY,
            )
            .await
            .map_err(map_db_error)?
            {
                return Err(Wave3RepositoryError::InvalidStateTransition {
                    from: previous.quality_status,
                    to: STATUS_UNQUALIFIED.to_string(),
                    approval_source: APPROVAL_SOURCE_EXPIRY.to_string(),
                });
            }

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
            .bind(STATUS_UNQUALIFIED)
            .bind("有效期到期自动隔离")
            .bind(APPROVAL_SOURCE_EXPIRY)
            .bind(previous.id.to_string())
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

            let updated = query_as::<_, InventoryBatchRow>(
                r#"
                UPDATE inventory_batches
                   SET quality_status = $3, updated_at = $4, version = version + 1
                 WHERE id = $1 AND owner_id = $2 AND quality_status = $5
                RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                          qty_on_hand, qty_locked, quality_status, location_id, location_code,
                          recall_flag, created_at, updated_at
                "#,
            )
            .bind(previous.id)
            .bind(ctx.owner_id)
            .bind(STATUS_UNQUALIFIED)
            .bind(now)
            .bind(STATUS_QUALIFIED)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            let batch = map_inventory_batch(updated);

            let mut audit_event = audit.clone().unwrap_or_else(|| {
                AuditWriteRequest::from_auth_context(
                    ctx,
                    "isolate_expired_inventory_batch",
                    "M3",
                    "inventory_batch",
                    batch.id.to_string(),
                    None,
                )
            });
            audit_event.occurred_at = now;
            audit_event.action = "isolate_expired_inventory_batch".to_string();
            audit_event.module = "M3".to_string();
            audit_event.resource_type = "inventory_batch".to_string();
            audit_event.resource_id = batch.id.to_string();
            audit_event.diff = Some(AuditDiff::compute(
                serde_json::json!({ "quality_status": previous.quality_status }),
                serde_json::json!({ "quality_status": STATUS_UNQUALIFIED }),
            ));
            append_event_in_tx(&mut tx, &audit_event)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
            isolated.push(batch);
        }

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/batches/expire",
            "inventory_expiry_job",
            format!("{}:{}", ctx.owner_id, as_of),
            &isolated,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: isolated,
            replayed: false,
        })
    }
}

fn validate_warning_days(days: i64) -> Result<i64, Wave3RepositoryError> {
    if (1..=3650).contains(&days) {
        Ok(days)
    } else {
        Err(Wave3RepositoryError::InvalidQuantity)
    }
}

fn map_system_dictionary_error(
    error: crate::system_dictionary::SystemDictionaryError,
) -> Wave3RepositoryError {
    match error {
        crate::system_dictionary::SystemDictionaryError::Database(message) => {
            Wave3RepositoryError::Database(message)
        }
        crate::system_dictionary::SystemDictionaryError::Serialize(message) => {
            Wave3RepositoryError::Serialize(message)
        }
        crate::system_dictionary::SystemDictionaryError::Audit(message) => {
            Wave3RepositoryError::Audit(message)
        }
        _ => Wave3RepositoryError::InvalidQuantity,
    }
}
