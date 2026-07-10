use axum::http::Uri;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CreateH4ApprovalRequest, SendH4NotificationRequest, UpsertH4NotificationConfigRequest,
    UpsertH4WechatSettingsRequest,
};

use crate::wechat_notify_service::{models::ConfigRow, WechatNotifyError};

use super::map_db_error;

pub(crate) async fn load_enabled_config(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    event_type: &str,
) -> Result<ConfigRow, WechatNotifyError> {
    sqlx::query_as::<_, ConfigRow>(
        r#"
        SELECT id, owner_id, event_type, enabled, template, recipient_rule,
               channels, created_at, updated_at, version
          FROM h4_notification_configs
         WHERE owner_id = $1 AND event_type = $2 AND enabled = TRUE
        "#,
    )
    .bind(owner_id)
    .bind(event_type)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(WechatNotifyError::EventNotFound)
}

pub(crate) fn validate_config_request(
    req: &UpsertH4NotificationConfigRequest,
) -> Result<(), WechatNotifyError> {
    if req.event_type.trim().is_empty() || req.template.trim().is_empty() {
        return Err(WechatNotifyError::InvalidRequest);
    }
    let has_recipient = req
        .recipient_rule
        .as_object()
        .is_some_and(|rule| rule.values().any(has_non_empty_string));
    if !has_recipient
        || !normalize_channels(req.channels.clone())
            .iter()
            .any(|v| v == "wechat")
    {
        return Err(WechatNotifyError::NoRecipients);
    }
    Ok(())
}

fn has_non_empty_string(value: &Value) -> bool {
    value.as_array().is_some_and(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .any(|item| !item.trim().is_empty())
    })
}

pub(crate) fn validate_wechat_settings_request(
    req: &UpsertH4WechatSettingsRequest,
) -> Result<(), WechatNotifyError> {
    if [
        req.corp_id.as_str(),
        req.agent_id.as_str(),
        req.secret_alias.as_str(),
        req.callback_token_alias.as_str(),
        req.aes_key_alias.as_str(),
        req.callback_url.as_str(),
        req.approval_callback_path.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(WechatNotifyError::InvalidRequest);
    }
    let callback_url = req.callback_url.trim().parse::<Uri>().ok();
    let valid_callback_url = callback_url.as_ref().is_some_and(|uri| {
        matches!(uri.scheme_str(), Some("http" | "https")) && uri.authority().is_some()
    });
    if !valid_callback_url || !is_valid_callback_path(&req.approval_callback_path) {
        return Err(WechatNotifyError::InvalidRequest);
    }
    if req.retry_max_attempts < 0
        || req.retry_max_attempts > 10
        || req.retry_interval_seconds < 1
        || req.retry_interval_seconds > 3600
    {
        return Err(WechatNotifyError::InvalidRequest);
    }
    Ok(())
}

pub(crate) fn validate_send_request(
    req: &SendH4NotificationRequest,
) -> Result<(), WechatNotifyError> {
    if req.event_type.trim().is_empty() || req.dedupe_key.trim().is_empty() {
        return Err(WechatNotifyError::InvalidRequest);
    }
    if req.recipients.iter().all(|value| value.trim().is_empty()) {
        return Err(WechatNotifyError::NoRecipients);
    }
    Ok(())
}

pub(crate) fn validate_approval_request(
    req: &CreateH4ApprovalRequest,
) -> Result<(), WechatNotifyError> {
    if [
        req.scenario.as_str(),
        req.business_ref.as_str(),
        req.dedupe_key.as_str(),
        req.approver_user.as_str(),
        req.process_id.as_str(),
        req.callback_path.as_str(),
        req.summary.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(WechatNotifyError::InvalidRequest);
    }
    if !is_valid_callback_path(&req.callback_path) {
        return Err(WechatNotifyError::InvalidRequest);
    }
    Uuid::parse_str(req.approver_user.trim()).map_err(|_| WechatNotifyError::InvalidRequest)?;
    Ok(())
}

fn is_valid_callback_path(value: &str) -> bool {
    let path = value.trim();
    path.starts_with('/') && !path.starts_with("//") && !path.chars().any(char::is_whitespace)
}

pub(crate) fn render_template(
    template: &str,
    payload: &Value,
) -> Result<String, WechatNotifyError> {
    let mut content = template.to_string();
    let object = payload
        .as_object()
        .ok_or(WechatNotifyError::TemplateInvalid)?;
    for (key, value) in object {
        let rendered = value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string());
        content = content.replace(&format!("{{{{{key}}}}}"), &rendered);
    }
    if content.contains("{{") || content.contains("}}") {
        return Err(WechatNotifyError::TemplateInvalid);
    }
    Ok(content)
}

pub(crate) fn summarize(content: &str) -> String {
    content.chars().take(80).collect()
}

pub(crate) fn normalize_channels(channels: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = channels
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();
    if !normalized.iter().any(|value| value == "wechat") {
        normalized.push("wechat".to_string());
    }
    normalized.sort();
    normalized.dedup();
    normalized
}
