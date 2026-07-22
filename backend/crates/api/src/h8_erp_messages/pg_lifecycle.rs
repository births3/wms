//! H8 Worker 生命周期状态的 PostgreSQL 原子更新。

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use wms_domain::{
    can_transition_message_status, sanitize_error_summary, H8ErpMessage, H8MessageError,
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
    let current: String =
        sqlx::query_scalar("SELECT sync_status FROM h8_erp_messages WHERE owner_id=$1 AND id=$2")
            .bind(owner_id)
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?
            .ok_or(H8ErpMessageRepoError::NotFound)?;
    can_transition_message_status(&current, target).map_err(H8ErpMessageRepoError::Domain)?;
    let result = sqlx::query(
        r#"UPDATE h8_erp_messages
           SET sync_status=$4, retry_count=retry_count+$5, last_error_summary=$6,
               claimed_by=$7, lease_expires_at=$8, completed_at=$9, updated_at=$10
           WHERE owner_id=$1 AND id=$2 AND sync_status=$3"#,
    )
    .bind(owner_id)
    .bind(id)
    .bind(current)
    .bind(target)
    .bind(i32::from(target == "failed"))
    .bind(error_summary.map(sanitize_error_summary))
    .bind((target == "processing").then(|| actor.to_string()))
    .bind((target == "processing").then(|| now + chrono::Duration::minutes(10)))
    .bind((target == "succeeded").then_some(now))
    .bind(now)
    .execute(pool)
    .await
    .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?;
    if result.rows_affected() != 1 {
        return Err(H8ErpMessageRepoError::Domain(
            H8MessageError::IllegalTransition,
        ));
    }
    let row = sqlx::query_as::<_, MessageRow>(
        r#"SELECT id, owner_id, warehouse_id, connector_id, connector_code, config_version,
                  direction, message_type, schema_version, channel, external_ref, wms_resource_id,
                  idempotency_key, correlation_id, sync_status, retry_count, next_retry_at,
                  last_error_summary, payload_digest, claimed_by, lease_expires_at,
                  created_at, updated_at, completed_at, acked_at
           FROM h8_erp_messages WHERE owner_id=$1 AND id=$2"#,
    )
    .bind(owner_id)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|error| H8ErpMessageRepoError::Db(error.to_string()))?;
    Ok(row.into())
}
