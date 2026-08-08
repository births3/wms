use super::*;
use sha2::{Digest, Sha256};

pub(super) fn idempotency_key(headers: &HeaderMap) -> Result<&str, H8InboundError> {
    headers
        .get(IDEMPOTENCY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(H8InboundError::BadRequest(
            "Idempotency-Key header is required",
        ))
}

pub(super) fn payload_digest<T: Serialize>(body: &T) -> Result<String, H8InboundError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(body)
                .map_err(|error| H8InboundError::Internal(error.to_string()))?
        )
    ))
}

pub(super) fn validate_payload_digest(value: &str) -> Result<String, H8InboundError> {
    let value = value.trim();
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(value.to_string())
    } else {
        Err(H8InboundError::Unprocessable(
            "payload_digest must be 64 lowercase hex characters".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::validate_payload_digest;

    #[test]
    fn payload_digest_uses_publisher_canonical_digest() {
        let digest = "a".repeat(64);
        assert_eq!(validate_payload_digest(&digest).unwrap(), digest);
        assert!(validate_payload_digest(&"A".repeat(64)).is_err());
        assert!(validate_payload_digest("abc").is_err());
    }
}

pub(super) struct InboundMetadata {
    pub message_type: &'static str,
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub warehouse_id: Option<Uuid>,
    pub payload_digest: String,
}

pub(super) struct PreparedMessage {
    pub message: H8ErpMessage,
    pub connector_id: Uuid,
    pub config_version: i64,
    pub connector_code: String,
    pub replayed: bool,
}

pub(super) async fn prepare_message(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    idempotency_key: &str,
    metadata: InboundMetadata,
) -> Result<PreparedMessage, H8InboundError> {
    if let Some(existing) = find_message(state, ctx, idempotency_key, &metadata).await? {
        return prepare_existing(state, ctx, existing, &metadata).await;
    }
    let active = state
        .connectors
        .repository
        .list_active(ctx.owner_id)
        .await
        .map_err(|error| H8InboundError::Internal(format!("{error:?}")))?;
    let connector = resolve_active_connector(
        &active,
        metadata.warehouse_id,
        "inbound",
        metadata.message_type,
    )
    .map_err(|error| H8InboundError::Unprocessable(format!("{error:?}")))?
    .clone();
    if connector.api_key_id != Some(ctx.user_id) {
        return Err(AuthError::PermissionDenied("connector API Key binding".to_string()).into());
    }
    let now = Utc::now();
    let message = H8ErpMessage {
        id: Uuid::new_v4(),
        owner_id: ctx.owner_id,
        warehouse_id: metadata.warehouse_id,
        connector_id: Some(connector.id),
        connector_code: Some(connector.connector_code.clone()),
        config_version: Some(connector.config_version),
        direction: "inbound".to_string(),
        message_type: metadata.message_type.to_string(),
        schema_version: metadata.schema_version.clone(),
        channel: "rest".to_string(),
        external_ref: metadata.external_ref.clone(),
        wms_resource_id: None,
        idempotency_key: idempotency_key.to_string(),
        correlation_id: metadata.correlation_id.clone(),
        sync_status: "pending".to_string(),
        retry_count: 0,
        next_retry_at: None,
        last_error_summary: None,
        payload_digest: metadata.payload_digest.clone(),
        claimed_by: None,
        lease_expires_at: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
        acked_at: None,
    };
    if let Err(insert_error) = state.messages.repository.upsert_for_test(&message).await {
        let existing = find_message(state, ctx, idempotency_key, &metadata)
            .await?
            .ok_or_else(|| H8InboundError::Internal(format!("{insert_error:?}")))?;
        return prepare_existing(state, ctx, existing, &metadata).await;
    }
    let message = apply_stage(state, ctx, message, "receive", "ok", None).await?;
    Ok(PreparedMessage {
        message,
        connector_id: connector.id,
        config_version: connector.config_version,
        connector_code: connector.connector_code,
        replayed: false,
    })
}

async fn find_message(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    idempotency_key: &str,
    metadata: &InboundMetadata,
) -> Result<Option<H8ErpMessage>, H8InboundError> {
    state
        .messages
        .repository
        .find_by_idempotency(
            ctx.owner_id,
            metadata.message_type,
            &metadata.external_ref,
            idempotency_key,
        )
        .await
        .map_err(|error| H8InboundError::Internal(format!("{error:?}")))
}

async fn prepare_existing(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    existing: H8ErpMessage,
    metadata: &InboundMetadata,
) -> Result<PreparedMessage, H8InboundError> {
    let connector_id = existing
        .connector_id
        .ok_or_else(|| H8InboundError::Internal("missing connector binding".to_string()))?;
    let config_version = existing
        .config_version
        .ok_or_else(|| H8InboundError::Internal("missing config binding".to_string()))?;
    let binding = state
        .connectors
        .repository
        .get_version(ctx.owner_id, connector_id, config_version)
        .await
        .map_err(|error| H8InboundError::Internal(format!("{error:?}")))?;
    if binding.api_key_id != Some(ctx.user_id) {
        return Err(AuthError::PermissionDenied("connector API Key binding".to_string()).into());
    }
    if existing.payload_digest != metadata.payload_digest {
        return Err(H8InboundError::Conflict(
            "Idempotency-Key was used for a different payload",
        ));
    }
    if !matches!(
        existing.sync_status.as_str(),
        "pending" | "processing" | "failed" | "succeeded"
    ) {
        return Err(H8InboundError::Conflict("message is already in progress"));
    }
    let message = if existing.sync_status == "succeeded" {
        existing
    } else {
        apply_stage(state, ctx, existing, "receive", "ok", None).await?
    };
    Ok(PreparedMessage {
        message,
        connector_id,
        config_version,
        connector_code: binding.connector_code,
        replayed: true,
    })
}

async fn apply_stage(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    message: H8ErpMessage,
    stage: &str,
    result: &str,
    resource_id: Option<&str>,
) -> Result<H8ErpMessage, H8InboundError> {
    crate::h8_erp_messages::apply_lifecycle_status(
        &state.messages,
        ctx,
        message,
        stage,
        result,
        resource_id,
        Utc::now(),
    )
    .await
    .map_err(|error| H8InboundError::Internal(format!("{error:?}")))
}

pub(super) async fn record_convert_message(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    message: H8ErpMessage,
) -> Result<H8ErpMessage, H8InboundError> {
    apply_stage(state, ctx, message, "convert", "ok", None).await
}

pub(super) async fn succeed_message(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    message_id: Uuid,
    resource_id: &str,
) -> Result<H8ErpMessage, H8InboundError> {
    let current = state
        .messages
        .repository
        .get(ctx.owner_id, message_id)
        .await
        .map_err(|error| H8InboundError::Internal(format!("{error:?}")))?;
    let current = match apply_stage(state, ctx, current, "business_api", "ok", None).await {
        Ok(message) => message,
        Err(error) => {
            let current = state
                .messages
                .repository
                .get(ctx.owner_id, message_id)
                .await
                .map_err(|get_error| H8InboundError::Internal(format!("{get_error:?}")))?;
            if current.sync_status == "succeeded"
                && current.wms_resource_id.as_deref() == Some(resource_id)
            {
                return Ok(current);
            } else {
                return Err(error);
            }
        }
    };
    apply_stage(state, ctx, current, "receipt", "ok", Some(resource_id)).await
}
