//! AC15 幂等与 AC9 scope 校验。

use axum::http::{HeaderMap, StatusCode};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use wms_domain::{
    api_key_scopes_cover_messages, required_inbound_scopes, H8ErpConnector, H8ErpConnectorError,
};

use super::error::{H8ErpConnectorHandlerError, H8ErpConnectorRepoError};
use super::state::H8ErpConnectorAppState;
use crate::sync::lock_recover;

#[derive(Clone, Debug)]
pub struct H8IdempotencyWrite {
    pub(crate) owner_id: Uuid,
    pub(crate) key: String,
    pub(crate) request_hash: String,
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) status_code: i32,
    pub(crate) response_body: Value,
    pub(crate) resource_id: String,
}

impl H8IdempotencyWrite {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        owner_id: Uuid,
        key: &str,
        request_hash: &str,
        method: &str,
        path: &str,
        status: StatusCode,
        resource_id: Uuid,
        body: &impl Serialize,
    ) -> Result<Self, H8ErpConnectorHandlerError> {
        Ok(Self {
            owner_id,
            key: key.to_string(),
            request_hash: request_hash.to_string(),
            method: method.to_string(),
            path: path.to_string(),
            status_code: i32::from(status.as_u16()),
            response_body: serde_json::to_value(body).map_err(|error| {
                H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(error.to_string()))
            })?,
            resource_id: resource_id.to_string(),
        })
    }
}

pub(crate) fn idempotency_key(headers: &HeaderMap) -> Result<String, H8ErpConnectorHandlerError> {
    headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or(H8ErpConnectorHandlerError::MissingIdempotencyKey)
}

pub(crate) fn request_hash(payload: &impl Serialize) -> Result<String, H8ErpConnectorHandlerError> {
    let bytes = serde_json::to_vec(payload).map_err(|e| {
        H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(e.to_string()))
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(crate) fn cache_key(owner_id: Uuid, key: &str) -> String {
    format!("{owner_id}:{key}")
}

pub(crate) async fn load_idempotent_response(
    state: &H8ErpConnectorAppState,
    owner_id: Uuid,
    key: &str,
    hash: &str,
) -> Result<Option<(StatusCode, Value)>, H8ErpConnectorHandlerError> {
    if let Some(pool) = &state.audit_pool {
        let row: Option<(String, i32, Value)> = sqlx::query_as(
            r#"
            SELECT request_hash, status_code, response_body
              FROM idempotency_request
             WHERE owner_id = $1 AND idempotency_key = $2
               AND expires_at > now()
            "#,
        )
        .bind(owner_id)
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(e.to_string()))
        })?;
        if let Some((stored_hash, status_code, body)) = row {
            if stored_hash != hash {
                return Err(H8ErpConnectorHandlerError::Repo(
                    H8ErpConnectorRepoError::Domain(H8ErpConnectorError::IdempotencyConflict),
                ));
            }
            let status = StatusCode::from_u16(status_code as u16).unwrap_or(StatusCode::OK);
            return Ok(Some((status, body)));
        }
        return Ok(None);
    }
    let guard = lock_recover(&state.idempotency);
    if let Some((stored_hash, status, body)) = guard.get(&cache_key(owner_id, key)) {
        if stored_hash != hash {
            return Err(H8ErpConnectorHandlerError::Repo(
                H8ErpConnectorRepoError::Domain(H8ErpConnectorError::IdempotencyConflict),
            ));
        }
        return Ok(Some((
            StatusCode::from_u16(*status).unwrap_or(StatusCode::OK),
            body.clone(),
        )));
    }
    Ok(None)
}

pub(crate) fn cache_idempotent_response(
    state: &H8ErpConnectorAppState,
    record: &H8IdempotencyWrite,
) {
    if state.audit_pool.is_some() {
        return;
    }
    let mut guard = lock_recover(&state.idempotency);
    guard.insert(
        cache_key(record.owner_id, &record.key),
        (
            record.request_hash.clone(),
            record.status_code as u16,
            record.response_body.clone(),
        ),
    );
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn store_idempotent_response(
    state: &H8ErpConnectorAppState,
    owner_id: Uuid,
    key: &str,
    hash: &str,
    method: &str,
    path: &str,
    status: StatusCode,
    body: &impl Serialize,
) -> Result<(), H8ErpConnectorHandlerError> {
    let record =
        H8IdempotencyWrite::new(owner_id, key, hash, method, path, status, Uuid::nil(), body)?;
    cache_idempotent_response(state, &record);
    Ok(())
}

pub(crate) async fn ensure_inbound_api_key_scopes(
    state: &H8ErpConnectorAppState,
    owner_id: Uuid,
    connector: &H8ErpConnector,
) -> Result<(), H8ErpConnectorHandlerError> {
    if !connector.directions.iter().any(|d| d == "inbound") {
        return Ok(());
    }
    let Some(api_key_id) = connector.api_key_id else {
        return Ok(());
    };
    let Some(scopes) = state
        .repository
        .load_api_key_scopes(owner_id, api_key_id)
        .await?
    else {
        return Err(
            H8ErpConnectorRepoError::Domain(H8ErpConnectorError::InsufficientApiKeyScope).into(),
        );
    };
    api_key_scopes_cover_messages(&connector.message_types, &scopes)
        .map_err(H8ErpConnectorRepoError::Domain)?;
    let _ = required_inbound_scopes(&connector.message_types);
    Ok(())
}
