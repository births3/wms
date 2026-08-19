use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    config::RuntimeSettings, control_plane::ControlPlaneClient, error::WorkerError,
    mssql::MssqlRepository, outbox_repository::PgOutboxRepository,
};

pub fn interface_receipt_table(message_type: &str) -> Option<&'static str> {
    match message_type {
        "order_status" | "shipment_confirm" => Some("x_wmsinter_OrderFeedback"),
        "putaway_complete" => Some("x_wmsinter_InboundFeedback"),
        "inventory_status" | "stock_adjustment" | "archive_revision" | "reconciliation_diff" => {
            Some("x_wmsinter_WmsEvent")
        }
        "inventory_snapshot" => Some("x_wmsinter_InventoryReceiveHeader"),
        _ => None,
    }
}

pub fn parse_outbox_identity(value: &str) -> Option<(&'static str, Uuid)> {
    let mut parts = value.split(':');
    if parts.next()? != "out" {
        return None;
    }
    let table = parts.next()?;
    let source = crate::outbox_repository::outbox_sources()
        .iter()
        .find(|source| source.table == table)?;
    let id = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((source.table, id))
}

pub async fn process_receipts(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    mssql: &MssqlRepository,
    outbox: &PgOutboxRepository,
) -> Result<u32, WorkerError> {
    let connector = settings.bootstrap.connector_id.to_string();
    let mut cursor: Option<String> = None;
    let mut processed = 0_u32;
    loop {
        let mut query = vec![
            ("direction", "outbound"),
            ("status", "awaiting_receipt"),
            ("connector_id", connector.as_str()),
            ("created_from", "1970-01-01T00:00:00Z"),
            ("limit", "200"),
        ];
        if let Some(cursor) = cursor.as_deref() {
            query.push(("cursor", cursor));
        }
        let response = control
            .get_query("/api/v1/integration/erp-messages", &query)
            .await?;
        let messages = response
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                WorkerError::new("H8_WORKER_HTTP_INVALID_RESPONSE", "receipt list missing")
            })?;
        for message in messages {
            if process_message(settings, control, mssql, outbox, message)
                .await
                .unwrap_or(false)
            {
                processed = processed.saturating_add(1);
            }
        }
        cursor = response
            .pointer("/page/next_cursor")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    Ok(processed)
}

async fn process_message(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    mssql: &MssqlRepository,
    outbox: &PgOutboxRepository,
    message: &Value,
) -> Result<bool, WorkerError> {
    let idempotency = text(message, "idempotency_key")?;
    let (table, outbox_id) = parse_outbox_identity(idempotency)
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "invalid outbound idempotency key"))?;
    let message_type = text(message, "message_type")?;
    if mssql
        .has_business_receipt(
            message_type,
            &settings.bootstrap.owner_code,
            outbox_id,
            text(message, "external_ref")?,
        )
        .await?
    {
        emit_message_stage(control, message, "receipt", "ok").await?;
        return Ok(true);
    }
    let due = message
        .get("next_retry_at")
        .and_then(Value::as_str)
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|value| value.with_timezone(&Utc) <= Utc::now());
    if !due {
        return Ok(false);
    }
    if message
        .get("retry_count")
        .and_then(Value::as_i64)
        .unwrap_or_default()
        < 4
    {
        outbox.requeue(table, outbox_id, settings.owner_id).await?;
    }
    emit_message_stage(
        control,
        message,
        "final_failure",
        "business receipt timeout",
    )
    .await?;
    Ok(true)
}

async fn emit_message_stage(
    control: &ControlPlaneClient,
    message: &Value,
    stage: &str,
    result: &str,
) -> Result<(), WorkerError> {
    let body = json!({
        "stage": stage, "result": result, "direction": "outbound",
        "message_type": text(message, "message_type")?,
        "schema_version": text(message, "schema_version")?,
        "external_ref": text(message, "external_ref")?,
        "idempotency_key": text(message, "idempotency_key")?,
        "correlation_id": text(message, "correlation_id")?,
        "channel": text(message, "channel")?,
        "connector_id": message.get("connector_id").cloned().unwrap_or(Value::Null),
        "connector_code": message.get("connector_code").cloned().unwrap_or(Value::Null),
        "config_version": message.get("config_version").cloned().unwrap_or(Value::Null),
        "message_id": text(message, "id")?
    });
    control
        .post_idempotent(
            "/api/v1/integration/erp-messages/lifecycle",
            &body,
            &format!("h8-{stage}-{}", text(message, "id")?),
        )
        .await?;
    Ok(())
}

fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, WorkerError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            WorkerError::new(
                "H8_WORKER_HTTP_INVALID_RESPONSE",
                format!("missing {field}"),
            )
        })
}
