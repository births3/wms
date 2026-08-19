//! H4 企业微信通知幂等与审计收尾工具。

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    idempotency,
    operation_context::OperationContext as AuthContext,
    wechat_notify_service::WechatNotifyError,
};

#[allow(clippy::too_many_arguments)]
pub(crate) async fn finish_mutation<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    action: &str,
    now: DateTime<Utc>,
) -> Result<(), WechatNotifyError> {
    store_idempotency_success(
        tx,
        ctx.owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        resource_id.clone(),
        response,
        now,
    )
    .await?;
    append_mutation_audit(tx, ctx, action, resource_type, resource_id, response).await
}

pub(crate) async fn append_mutation_audit<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
) -> Result<(), WechatNotifyError> {
    append_event_in_tx(
        tx,
        &AuditWriteRequest::from_auth_context(
            ctx,
            action,
            "H4",
            resource_type,
            resource_id,
            Some(AuditDiff::compute(
                serde_json::json!({}),
                serde_json::to_value(response)
                    .map_err(|error| WechatNotifyError::Serialize(error.to_string()))?,
            )),
        ),
    )
    .await
    .map(|_| ())
    .map_err(|error| WechatNotifyError::Audit(format!("{error:?}")))
}

pub(crate) async fn update_idempotency_response<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    response: &T,
) -> Result<(), WechatNotifyError> {
    idempotency::update_response(tx, owner_id, idempotency_key, request_hash, response)
        .await
        .map_err(map_idempotency_error)
}

pub(crate) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), WechatNotifyError> {
    idempotency::lock_key(tx, "wechat-notify", owner_id, idempotency_key)
        .await
        .map_err(map_idempotency_error)
}

pub(crate) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, WechatNotifyError> {
    idempotency::replay_hash_only(tx, owner_id, idempotency_key, request_hash, now)
        .await
        .map_err(map_idempotency_error)
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), WechatNotifyError> {
    idempotency::store_success(
        tx,
        owner_id,
        idempotency_key,
        request_hash,
        method,
        path,
        resource_type,
        &resource_id,
        response,
        now,
    )
    .await
    .map_err(map_idempotency_error)
}

pub(crate) fn json_request_hash<T: Serialize>(value: &T) -> Result<String, WechatNotifyError> {
    idempotency::request_hash(value).map_err(map_idempotency_error)
}

fn map_idempotency_error(error: idempotency::IdempotencyError) -> WechatNotifyError {
    match error {
        idempotency::IdempotencyError::Conflict => WechatNotifyError::IdempotencyConflict,
        idempotency::IdempotencyError::Database(error) => {
            WechatNotifyError::Database(error.to_string())
        }
        idempotency::IdempotencyError::Serialize(error) => WechatNotifyError::Serialize(error),
    }
}
