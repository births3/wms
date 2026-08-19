use serde_json::{json, Value};

use crate::{
    config::RuntimeSettings,
    control_plane::ControlPlaneClient,
    error::WorkerError,
    mssql::MssqlRepository,
    outbound::{build_published_unit, OutboxRow},
    outbox_repository::{outbox_sources, OutboxSource, PgOutboxRepository},
};

pub fn effective_message_type<'a>(catalog_type: &'a str, row: &OutboxRow) -> &'a str {
    if row.event_type == "order_status" {
        "order_status"
    } else {
        catalog_type
    }
}

pub fn lifecycle_body(
    row: &OutboxRow,
    message_type: &str,
    stage: &str,
    result: &str,
    message_id: Option<&str>,
    binding: &RouteBinding,
) -> Value {
    let idempotency_key = format!("out:{}:{}", row.table, row.id);
    let correlation_id = row
        .payload
        .get("correlation_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("h8-{message_type}-{idempotency_key}"));
    let mut body = json!({
        "stage": stage, "result": result, "direction": "outbound",
        "message_type": message_type, "schema_version": "1",
        "external_ref": if row.external_ref.is_empty() { row.id.to_string() } else { row.external_ref.clone() },
        "idempotency_key": idempotency_key, "correlation_id": correlation_id,
        "channel": "interface_table", "connector_id": binding.connector_id,
        "connector_code": binding.connector_code, "config_version": binding.config_version
    });
    if let Some(message_id) = message_id {
        body["message_id"] = Value::String(message_id.to_owned());
    }
    if stage == "receive" {
        body["payload"] = row.payload.clone();
    }
    body
}

#[derive(Clone)]
pub struct RouteBinding {
    pub connector_id: String,
    pub connector_code: String,
    pub config_version: i64,
}

pub async fn run_outbound_once(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    mssql: &MssqlRepository,
    outbox: &PgOutboxRepository,
) -> Result<u32, WorkerError> {
    let _ = control
        .post_heartbeat(settings, &["inbound", "outbound"], 0)
        .await;
    if !claim_allowed(settings, control).await? {
        return Ok(0);
    }
    let mut processed = 0_u32;
    for source in outbox_sources() {
        let rows = outbox
            .claim(source, settings.bootstrap.batch_size, settings.owner_id)
            .await?;
        let _ = control
            .post_heartbeat(settings, &["inbound", "outbound"], rows.len() as u32)
            .await;
        for row in rows {
            processed = processed.saturating_add(1);
            process_row(settings, control, mssql, outbox, source, &row).await?;
        }
    }
    Ok(processed)
}

async fn process_row(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    mssql: &MssqlRepository,
    outbox: &PgOutboxRepository,
    source: &OutboxSource,
    row: &OutboxRow,
) -> Result<(), WorkerError> {
    let message_type = effective_message_type(source.message_type, row);
    let result = publish_row(settings, control, mssql, row, message_type).await;
    outbox.mark(source, row, result.as_ref().err()).await
}

async fn publish_row(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    mssql: &MssqlRepository,
    row: &OutboxRow,
    message_type: &str,
) -> Result<(), WorkerError> {
    let binding = resolve_binding(settings, control, row, message_type).await?;
    let mut message_id =
        emit_stage(control, row, message_type, "receive", "ok", None, &binding).await?;
    let result = async {
        emit_stage(
            control,
            row,
            message_type,
            "convert",
            "ok",
            message_id.as_deref(),
            &binding,
        )
        .await?;
        let unit = build_published_unit(row, &settings.bootstrap.owner_code)?;
        emit_stage(
            control,
            row,
            message_type,
            "send",
            "started",
            message_id.as_deref(),
            &binding,
        )
        .await?;
        mssql.publish(&unit).await?;
        message_id = emit_stage(
            control,
            row,
            message_type,
            "send",
            "ok",
            message_id.as_deref(),
            &binding,
        )
        .await?;
        Ok::<_, WorkerError>(())
    }
    .await;
    if let Err(error) = &result {
        let _ = emit_stage(
            control,
            row,
            message_type,
            "final_failure",
            error.code(),
            message_id.as_deref(),
            &binding,
        )
        .await;
    }
    result
}

async fn resolve_binding(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    row: &OutboxRow,
    message_type: &str,
) -> Result<RouteBinding, WorkerError> {
    let idempotency = format!("out:{}:{}", row.table, row.id);
    if row.attempt_count > 1 {
        let existing = control
            .get_query(
                "/api/v1/integration/erp-messages",
                &[
                    ("direction", "outbound"),
                    ("message_type", message_type),
                    ("idempotency_key", &idempotency),
                    ("created_from", "1970-01-01T00:00:00Z"),
                    ("limit", "2"),
                ],
            )
            .await?;
        if let Some(message) = existing
            .get("data")
            .and_then(Value::as_array)
            .and_then(|data| (data.len() == 1).then(|| &data[0]))
        {
            return binding_from_message(settings, message);
        }
    }
    let mut query = vec![("direction", "outbound"), ("message_type", message_type)];
    if let Some(warehouse_id) = row.payload.get("warehouse_id").and_then(Value::as_str) {
        query.push(("warehouse_id", warehouse_id));
    }
    let response = control
        .get_query("/api/v1/config/erp-connectors/route-resolve", &query)
        .await?;
    let connector = response.get("connector").ok_or_else(|| {
        WorkerError::new("H8_WORKER_HTTP_INVALID_RESPONSE", "route connector missing")
    })?;
    binding_from_connector(settings, connector)
}

fn binding_from_message(
    settings: &RuntimeSettings,
    message: &Value,
) -> Result<RouteBinding, WorkerError> {
    if message.get("channel").and_then(Value::as_str) != Some("interface_table") {
        return Err(WorkerError::new(
            "H8_WORKER_INTERFACE_CHANNEL_REQUIRED",
            "frozen outbound message is not interface_table",
        ));
    }
    binding_values(
        settings,
        message.get("connector_id").and_then(Value::as_str),
        message.get("connector_code").and_then(Value::as_str),
        message.get("config_version").and_then(Value::as_i64),
    )
}

fn binding_from_connector(
    settings: &RuntimeSettings,
    connector: &Value,
) -> Result<RouteBinding, WorkerError> {
    if connector.get("owner_id").and_then(Value::as_str)
        != Some(settings.owner_id.to_string().as_str())
        || connector.get("channel_mode").and_then(Value::as_str) != Some("interface_table")
    {
        return Err(WorkerError::new(
            "H8_WORKER_ROUTE_CHANGED",
            "outbound route scope or channel changed",
        ));
    }
    binding_values(
        settings,
        connector.get("id").and_then(Value::as_str),
        connector.get("connector_code").and_then(Value::as_str),
        connector.get("config_version").and_then(Value::as_i64),
    )
}

fn binding_values(
    settings: &RuntimeSettings,
    connector_id: Option<&str>,
    connector_code: Option<&str>,
    config_version: Option<i64>,
) -> Result<RouteBinding, WorkerError> {
    if connector_id != Some(settings.bootstrap.connector_id.to_string().as_str())
        || config_version != Some(settings.connector_config_version)
    {
        return Err(WorkerError::new(
            "H8_WORKER_ROUTE_CHANGED",
            "outbound connector binding changed",
        ));
    }
    Ok(RouteBinding {
        connector_id: connector_id.unwrap_or_default().to_owned(),
        connector_code: connector_code
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                WorkerError::new("H8_WORKER_HTTP_INVALID_RESPONSE", "connector code missing")
            })?
            .to_owned(),
        config_version: config_version.unwrap_or_default(),
    })
}

async fn emit_stage(
    control: &ControlPlaneClient,
    row: &OutboxRow,
    message_type: &str,
    stage: &str,
    result: &str,
    message_id: Option<&str>,
    binding: &RouteBinding,
) -> Result<Option<String>, WorkerError> {
    let body = lifecycle_body(row, message_type, stage, result, message_id, binding);
    let response = control
        .post_idempotent(
            "/api/v1/integration/erp-messages/lifecycle",
            &body,
            &format!("h8-life-{}-{stage}-{}", row.id, row.attempt_count),
        )
        .await?;
    Ok(response
        .get("id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| message_id.map(ToOwned::to_owned)))
}

async fn claim_allowed(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
) -> Result<bool, WorkerError> {
    let connector = settings.bootstrap.connector_id.to_string();
    let response = control
        .get_query(
            "/api/v1/integration/erp-messages/worker-runtime/claim-decision",
            &[("connector_id", &connector), ("direction", "outbound")],
        )
        .await?;
    response
        .get("allowed")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            WorkerError::new("H8_WORKER_HTTP_INVALID_RESPONSE", "claim decision missing")
        })
}
