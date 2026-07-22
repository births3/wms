//! H8 Worker 生命周期状态的 PostgreSQL 原子更新。

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    can_transition_message_status, sanitize_error_summary, standard_retry_delay_millis,
    H8ErpMessage, H8MessageError,
};

use super::{error::H8ErpMessageRepoError, pg_rows::MessageRow};

pub(super) async fn transition_lifecycle_status(
    pool: &PgPool,
    owner_id: Uuid,
    id: Uuid,
    target: &str,
    error_summary: Option<&str>,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<H8ErpMessage, H8ErpMessageRepoError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?;
    let current = sqlx::query_as::<_, MessageRow>(
        r#"SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                  direction, message_type, schema_version, channel, external_ref, wms_resource_id,
                  idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                  last_error_summary, payload_digest, claimed_by, lease_expires_at,
                  created_at, updated_at, completed_at, acked_at
           FROM h8_erp_messages WHERE owner_id=$1 AND id=$2 FOR UPDATE"#,
    )
    .bind(owner_id)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?
    .ok_or(H8ErpMessageRepoError::NotFound)?;
    can_transition_message_status(&current.sync_status, target)
        .map_err(H8ErpMessageRepoError::Domain)?;
    let summary = error_summary.map(sanitize_error_summary);
    let next_retry_at = (target == "failed").then(|| {
        now + chrono::Duration::milliseconds(standard_retry_delay_millis(
            current.retry_count + 1,
            &current.idempotency_key,
        ))
    });
    let next = sqlx::query_as::<_, MessageRow>(
        r#"UPDATE h8_erp_messages
           SET sync_status=$4, retry_count=retry_count+$5, next_retry_at=$6,
               last_error_summary=$7, claimed_by=$8, lease_expires_at=$9,
               completed_at=$10, updated_at=$11
           WHERE owner_id=$1 AND id=$2 AND sync_status=$3
           RETURNING id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                     direction, message_type, schema_version, channel, external_ref, wms_resource_id,
                     idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                     last_error_summary, payload_digest, claimed_by, lease_expires_at,
                     created_at, updated_at, completed_at, acked_at"#,
    )
    .bind(owner_id)
    .bind(id)
    .bind(&current.sync_status)
    .bind(target)
    .bind(i32::from(target == "failed"))
    .bind(next_retry_at)
    .bind(&summary)
    .bind((target == "processing").then(|| actor.to_string()))
    .bind((target == "processing").then(|| now + chrono::Duration::minutes(10)))
    .bind((target == "succeeded").then_some(now))
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?
    .ok_or(H8ErpMessageRepoError::Domain(
        H8MessageError::IllegalTransition,
    ))?;
    if matches!(target, "failed" | "succeeded") {
        let attempt_no: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(attempt_no), 0) + 1 FROM h8_erp_message_attempts WHERE message_id=$1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?;
        sqlx::query(
            r#"INSERT INTO h8_erp_message_attempts
               (id, message_id, owner_id, attempt_no, channel, started_at, finished_at,
                result, error_summary, actor)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)"#,
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(owner_id)
        .bind(attempt_no)
        .bind(&current.channel)
        .bind(current.updated_at)
        .bind(now)
        .bind(target)
        .bind(summary)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?;
    }
    tx.commit()
        .await
        .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?;
    Ok(next.into())
}
