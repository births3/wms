use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::{
    config::RuntimeSettings,
    contract::validate_published_unit,
    control_plane::ControlPlaneClient,
    error::WorkerError,
    inbound::{contracts, request_body, InboundContract},
    mssql::{table_contract, MarkOutcome, MarkStatus, MssqlRepository, TableContract},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailureDecision {
    pub status: MarkStatus,
    pub retry_count: u32,
}

pub fn failure_decision(
    error: &WorkerError,
    retry_count: u32,
    max_retry: u32,
    inserted_at: DateTime<Utc>,
) -> FailureDecision {
    if error.code() == "ORDER_NOT_READY" {
        let status = if Utc::now().signed_duration_since(inserted_at).num_minutes() >= 30 {
            MarkStatus::Dead
        } else {
            MarkStatus::Retry
        };
        return FailureDecision {
            status,
            retry_count,
        };
    }
    let retryable = matches!(
        error.code(),
        "H8_WORKER_HTTP_RETRYABLE" | "H8_WORKER_HTTP_UNAVAILABLE"
    );
    if retryable {
        let next = retry_count.saturating_add(1);
        return FailureDecision {
            status: if next >= max_retry {
                MarkStatus::Dead
            } else {
                MarkStatus::Retry
            },
            retry_count: next,
        };
    }
    FailureDecision {
        status: MarkStatus::Dead,
        retry_count,
    }
}

pub async fn run_once(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    mssql: &MssqlRepository,
) -> Result<u32, WorkerError> {
    let _ = control
        .post_heartbeat(settings, &["inbound", "outbound"], 0)
        .await;
    if !claim_allowed(settings, control).await? {
        return Ok(0);
    }
    let mut processed = 0_u32;
    for inbound in contracts() {
        let table = table_contract(inbound.table)
            .ok_or_else(|| WorkerError::new("H8_WORKER_UNSUPPORTED_TABLE", inbound.table))?;
        prepare_manual_replays(settings, control, mssql, inbound.message_type, table).await?;
        let units = mssql
            .claim(
                table,
                settings.bootstrap.batch_size,
                &settings.bootstrap.worker_id,
                settings.bootstrap.lease_minutes,
                &settings.bootstrap.owner_code,
            )
            .await?;
        let _ = control
            .post_heartbeat(settings, &["inbound", "outbound"], units.len() as u32)
            .await;
        for unit in units {
            processed = processed.saturating_add(1);
            process_unit(
                settings,
                control,
                mssql,
                inbound,
                table,
                unit.row,
                unit.children,
            )
            .await?;
        }
    }
    let _ = control
        .post_heartbeat(settings, &["inbound", "outbound"], 0)
        .await;
    Ok(processed)
}

async fn prepare_manual_replays(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    mssql: &MssqlRepository,
    message_type: &str,
    table: &TableContract,
) -> Result<(), WorkerError> {
    let connector = settings.bootstrap.connector_id.to_string();
    let response = control
        .get_query(
            "/api/v1/integration/erp-messages",
            &[
                ("direction", "inbound"),
                ("message_type", message_type),
                ("status", "processing"),
                ("connector_id", &connector),
                ("channel", "interface_table"),
                ("replay_requested", "true"),
                ("created_from", "1970-01-01T00:00:00Z"),
                ("limit", "200"),
            ],
        )
        .await?;
    let messages = response
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            WorkerError::new(
                "H8_WORKER_HTTP_INVALID_RESPONSE",
                "manual replay list missing",
            )
        })?;
    for message in messages {
        let id = text(message, "id")?;
        let idempotency = text(message, "idempotency_key")?;
        if message
            .get("claimed_by")
            .and_then(Value::as_str)
            .is_none_or(|value| !value.starts_with("replay:"))
        {
            return Err(WorkerError::new(
                "H8_WORKER_REPLAY_SCOPE_CHANGED",
                "manual replay marker missing",
            ));
        }
        if !mssql
            .requeue_manual_replay(table, &settings.bootstrap.owner_code, idempotency)
            .await?
        {
            continue;
        }
        let response = control
            .post_idempotent(
                &format!("/api/v1/integration/erp-messages/{id}/claim"),
                &serde_json::json!({
                    "worker_id": settings.bootstrap.worker_id,
                    "lease_seconds": 300
                }),
                &format!("manual-replay-claim-{id}-{}", settings.bootstrap.worker_id),
            )
            .await?;
        if response.get("claimed_by").and_then(Value::as_str)
            != Some(settings.bootstrap.worker_id.as_str())
        {
            return Err(WorkerError::new(
                "H8_WORKER_REPLAY_CLAIM_FAILED",
                "manual replay claim failed",
            ));
        }
    }
    Ok(())
}

async fn process_unit(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    mssql: &MssqlRepository,
    inbound: &InboundContract,
    table: &TableContract,
    row: Value,
    children: Vec<Value>,
) -> Result<(), WorkerError> {
    let row_id = row
        .get(table.primary_key)
        .cloned()
        .ok_or_else(|| WorkerError::new("INVALID_DATA", "missing primary key"))?;
    let retry_count = unsigned(&row, "retry_count").unwrap_or(0);
    let inserted_at = datetime(&row, "inserttime").unwrap_or_else(|_| Utc::now());
    let result = async {
        if row.get("OwnerCode").and_then(Value::as_str)
            != Some(settings.bootstrap.owner_code.as_str())
        {
            return Err(WorkerError::new("INVALID_DATA", "OwnerCode mismatch"));
        }
        if row.get("SchemaVersion").and_then(Value::as_str) != Some("1") {
            return Err(WorkerError::new(
                "INVALID_DATA",
                "unsupported SchemaVersion",
            ));
        }
        validate_published_unit(table.table, &row, &children)?;
        resolve_inbound_route(settings, control, inbound.message_type, &row).await?;
        let body = request_body(inbound.message_type, &row, &children)?;
        let idempotency_key = text(&row, "IdempotencyKey")?;
        let response = control
            .post_idempotent(
                &format!(
                    "/api/v1/integration/erp-messages/inbound/{}",
                    inbound.message_type
                ),
                &body,
                idempotency_key,
            )
            .await?;
        if response.get("wms_resource_id").is_none() && response.get("id").is_none() {
            return Err(WorkerError::new(
                "H8_WORKER_HTTP_INVALID_RESPONSE",
                "inbound response missing resource id",
            ));
        }
        Ok(())
    }
    .await;

    match result {
        Ok(()) => {
            mssql
                .mark(
                    table,
                    &settings.bootstrap.owner_code,
                    &row_id,
                    MarkOutcome {
                        status: MarkStatus::Success,
                        message: None,
                        error_code: None,
                        retry_count: Some(retry_count),
                    },
                )
                .await
        }
        Err(error) => {
            let decision = failure_decision(
                &error,
                retry_count,
                settings.bootstrap.max_retry,
                inserted_at,
            );
            let message = truncate(error.message(), 200);
            mssql
                .mark(
                    table,
                    &settings.bootstrap.owner_code,
                    &row_id,
                    MarkOutcome {
                        status: decision.status,
                        message: Some(&message),
                        error_code: Some(error.code()),
                        retry_count: Some(decision.retry_count),
                    },
                )
                .await
        }
    }
}

async fn claim_allowed(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
) -> Result<bool, WorkerError> {
    let connector = settings.bootstrap.connector_id.to_string();
    let response = control
        .get_query(
            "/api/v1/integration/erp-messages/worker-runtime/claim-decision",
            &[("connector_id", &connector), ("direction", "inbound")],
        )
        .await?;
    response
        .get("allowed")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            WorkerError::new("H8_WORKER_HTTP_INVALID_RESPONSE", "claim decision missing")
        })
}

async fn resolve_inbound_route(
    settings: &RuntimeSettings,
    control: &ControlPlaneClient,
    message_type: &str,
    row: &Value,
) -> Result<(), WorkerError> {
    let mut query = vec![("direction", "inbound"), ("message_type", message_type)];
    if let Some(depot_code) = row.get("DepotCode").and_then(Value::as_str) {
        query.push(("warehouse_code", depot_code));
    }
    let response = control
        .get_query("/api/v1/config/erp-connectors/route-resolve", &query)
        .await?;
    let connector = response.get("connector").ok_or_else(|| {
        WorkerError::new("H8_WORKER_HTTP_INVALID_RESPONSE", "route connector missing")
    })?;
    if connector.get("id").and_then(Value::as_str)
        != Some(settings.bootstrap.connector_id.to_string().as_str())
        || connector.get("config_version").and_then(Value::as_i64)
            != Some(settings.connector_config_version)
        || !matches!(
            connector.get("channel_mode").and_then(Value::as_str),
            Some("interface_table" | "rest_primary_table_fallback")
        )
    {
        return Err(WorkerError::new(
            "H8_WORKER_ROUTE_CHANGED",
            "route does not match frozen connector snapshot",
        ));
    }
    Ok(())
}

fn text<'a>(row: &'a Value, field: &str) -> Result<&'a str, WorkerError> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| WorkerError::new("INVALID_DATA", format!("missing {field}")))
}

fn unsigned(row: &Value, field: &str) -> Option<u32> {
    row.get(field)
        .and_then(Value::as_i64)
        .and_then(|value| u32::try_from(value).ok())
}

fn datetime(row: &Value, field: &str) -> Result<DateTime<Utc>, WorkerError> {
    DateTime::parse_from_rfc3339(text(row, field)?)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| WorkerError::new("INVALID_DATA", format!("invalid {field}")))
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}
