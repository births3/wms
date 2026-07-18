fn require_any_permission(ctx: &AuthContext, permissions: &[&str]) -> Result<(), AuthError> {
    if permissions
        .iter()
        .any(|permission| ctx.has_permission(permission))
    {
        Ok(())
    } else {
        Err(AuthError::PermissionDenied(permissions.join("|")))
    }
}

fn idempotency_key_from_headers(headers: &HeaderMap) -> Result<String, Wave3HandlerError> {
    headers
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(Wave3HandlerError::MissingIdempotencyKey)
}

fn cold_chain_external_context(
    state: &Wave3AppState,
    headers: &HeaderMap,
) -> Result<(AuthContext, String), Wave3HandlerError> {
    let idempotency_key = idempotency_key_from_headers(headers)?;
    let config = state
        .cold_chain_api_key
        .as_ref()
        .ok_or(Wave3HandlerError::ExternalAuthConfigMissing)?;
    let configured_hash = config.key_sha256.trim();
    if configured_hash.len() != 64
        || !configured_hash
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        return Err(Wave3HandlerError::ExternalAuthConfigInvalid);
    }

    let api_key = headers
        .get(EXTERNAL_API_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(Wave3HandlerError::ExternalAuthMissing)?;
    let provided_hash = sha256_hex(api_key.as_bytes());
    if !constant_time_eq(
        provided_hash.as_bytes(),
        configured_hash.to_ascii_lowercase().as_bytes(),
    ) {
        return Err(Wave3HandlerError::ExternalAuthInvalid);
    }

    Ok((
        AuthContext {
            user_id: Uuid::nil(),
            owner_id: config.owner_id,
            actor_name: config.actor_name.clone(),
            permissions: vec!["m5.write".to_string()],
            jti: format!("m5-cold-chain:{idempotency_key}"),
            warehouse_scope: None,
        },
        idempotency_key,
    ))
}

fn sha256_hex(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hex::encode(hasher.finalize())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut diff = left.len() ^ right.len();
    let max_len = left.len().max(right.len());
    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        diff |= (left_byte ^ right_byte) as usize;
    }
    diff == 0
}

async fn append_audit(
    state: &Wave3AppState,
    ctx: &AuthContext,
    action: &'static str,
    module: &'static str,
    resource_type: &'static str,
    resource_id: String,
) {
    append_audit_with_diff(state, ctx, action, module, resource_type, resource_id, None).await;
}

async fn append_audit_with_diff(
    state: &Wave3AppState,
    ctx: &AuthContext,
    action: &'static str,
    module: &'static str,
    resource_type: &'static str,
    resource_id: String,
    diff: Option<AuditDiff>,
) {
    let mut audit_log = state.audit_log.lock().await;
    audit_log.append_event(AuditWriteRequest::from_auth_context(
        ctx,
        action,
        module,
        resource_type,
        resource_id,
        diff,
    ));
}
