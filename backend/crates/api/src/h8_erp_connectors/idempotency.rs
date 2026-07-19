//! AC15 幂等与 AC9 scope 校验。

use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use wms_domain::{
    api_key_scopes_cover_messages, required_inbound_scopes, H8ErpConnector, H8ErpConnectorError,
};

use super::error::{H8ErpConnectorHandlerError, H8ErpConnectorRepoError};
use super::state::H8ErpConnectorAppState;

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
        let row: Option<(String, i16, Value)> = sqlx::query_as(
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
    let guard = state.idempotency.lock().expect("idempotency lock");
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
    let response_body = serde_json::to_value(body).map_err(|e| {
        H8ErpConnectorHandlerError::Repo(H8ErpConnectorRepoError::Db(e.to_string()))
    })?;
    if let Some(pool) = &state.audit_pool {
        let now = Utc::now();
        let _ = sqlx::query(
            r#"
            INSERT INTO idempotency_request (
                id, owner_id, idempotency_key, request_hash, method, path,
                status_code, response_body, resource_type, resource_id, expires_at, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'h8_erp_connector',$9,$10,$11)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(key)
        .bind(hash)
        .bind(method)
        .bind(path)
        .bind(status.as_u16() as i16)
        .bind(&response_body)
        .bind(Uuid::nil().to_string())
        .bind(now + chrono::Duration::hours(24))
        .bind(now)
        .execute(pool)
        .await;
        return Ok(());
    }
    let mut guard = state.idempotency.lock().expect("idempotency lock");
    guard.insert(
        cache_key(owner_id, key),
        (hash.to_string(), status.as_u16(), response_body),
    );
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
        // 开发库可能尚未有 Key 记录；仅在能读到 scopes 时强制
        return Ok(());
    };
    api_key_scopes_cover_messages(&connector.message_types, &scopes)
        .map_err(H8ErpConnectorRepoError::Domain)?;
    let _ = required_inbound_scopes(&connector.message_types);
    Ok(())
}
