use std::{collections::HashMap, fmt, process};

use serde_json::Value;
use uuid::Uuid;

use crate::error::WorkerError;

#[derive(Clone)]
pub struct BootstrapSettings {
    pub api_base: String,
    pub api_token: Option<String>,
    pub api_key: Option<String>,
    pub connector_id: Uuid,
    pub poll_interval_seconds: u64,
    pub max_retry: u32,
    pub batch_size: u32,
    pub lease_minutes: u32,
    pub worker_id: String,
    pub worker_version: String,
    pub heartbeat_ttl_seconds: u32,
    pub owner_code: String,
    pub wms_db_url: Option<String>,
}

impl fmt::Debug for BootstrapSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BootstrapSettings")
            .field("api_base", &self.api_base)
            .field("connector_id", &self.connector_id)
            .field("worker_id", &self.worker_id)
            .finish_non_exhaustive()
    }
}

impl BootstrapSettings {
    pub fn from_env() -> Result<Self, WorkerError> {
        Self::from_map(&std::env::vars().collect())
    }

    pub fn from_map(values: &HashMap<String, String>) -> Result<Self, WorkerError> {
        let connector_id = required(values, "H8_CONNECTOR_ID")?
            .parse::<Uuid>()
            .map_err(|_| {
                WorkerError::new("H8_WORKER_INVALID_CONFIG", "H8_CONNECTOR_ID must be UUID")
            })?;
        let poll_interval_seconds = parse_u64(values, "H8_POLL_INTERVAL_SEC", 5)?;
        let max_retry = parse_u32(values, "H8_MAX_RETRY", 5)?;
        let batch_size = parse_u32(values, "H8_BATCH_SIZE", 10)?;
        let lease_minutes = parse_u32(values, "H8_LEASE_MINUTES", 5)?;
        if poll_interval_seconds == 0 || max_retry == 0 || batch_size == 0 || lease_minutes == 0 {
            return Err(WorkerError::new(
                "H8_WORKER_INVALID_CONFIG",
                "poll, retry, batch and lease values must be positive",
            ));
        }
        let default_ttl =
            15_u32.max(u32::try_from(poll_interval_seconds.saturating_mul(3)).unwrap_or(u32::MAX));
        let worker_id = optional(values, "H8_WORKER_ID").unwrap_or_else(|| {
            let host = optional(values, "HOSTNAME").unwrap_or_else(|| "h8-worker".to_owned());
            format!("{host}-{}", process::id())
        });
        Ok(Self {
            api_base: optional(values, "WMS_API_BASE")
                .unwrap_or_else(|| "http://127.0.0.1:8080".to_owned())
                .trim_end_matches('/')
                .to_owned(),
            api_token: optional(values, "WMS_API_TOKEN"),
            api_key: optional(values, "WMS_API_KEY"),
            connector_id,
            poll_interval_seconds,
            max_retry,
            batch_size,
            lease_minutes,
            worker_id,
            worker_version: optional(values, "H8_WORKER_VERSION").unwrap_or_else(|| "1".to_owned()),
            heartbeat_ttl_seconds: parse_u32(values, "H8_HEARTBEAT_TTL_SEC", default_ttl)?,
            owner_code: optional(values, "H8_OWNER_CODE").unwrap_or_else(|| "ZBPF7".to_owned()),
            wms_db_url: optional(values, "WMS_DB_URL").or_else(|| optional(values, "DATABASE_URL")),
        })
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct MssqlSettings {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl fmt::Debug for MssqlSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MssqlSettings")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database", &self.database)
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct RuntimeSettings {
    pub bootstrap: BootstrapSettings,
    pub owner_id: Uuid,
    pub connector_config_version: i64,
    pub channel_mode: String,
    pub mssql: MssqlSettings,
}

impl fmt::Debug for RuntimeSettings {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeSettings")
            .field("bootstrap", &self.bootstrap)
            .field("owner_id", &self.owner_id)
            .field("connector_config_version", &self.connector_config_version)
            .field("channel_mode", &self.channel_mode)
            .field("mssql", &self.mssql)
            .finish()
    }
}

impl RuntimeSettings {
    pub fn from_snapshot(
        bootstrap: BootstrapSettings,
        expected_version: i64,
        snapshot: &Value,
        secrets: &HashMap<String, String>,
    ) -> Result<Self, WorkerError> {
        if snapshot.get("id").and_then(Value::as_str)
            != Some(bootstrap.connector_id.to_string().as_str())
            || snapshot.get("config_version").and_then(Value::as_i64) != Some(expected_version)
        {
            return Err(WorkerError::new(
                "H8_WORKER_SNAPSHOT_IDENTITY_CHANGED",
                "connector snapshot identity changed",
            ));
        }
        let channel_mode = snapshot
            .get("channel_mode")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(
            channel_mode,
            "interface_table" | "rest_primary_table_fallback"
        ) {
            return Err(WorkerError::new(
                "H8_WORKER_INTERFACE_CHANNEL_REQUIRED",
                "interface table channel required",
            ));
        }
        let password_alias = snapshot_text(snapshot, "interface_db_password_alias")?;
        let password = secrets
            .get(password_alias)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                WorkerError::new("H8_WORKER_SECRET_UNAVAILABLE", "MSSQL secret unavailable")
            })?
            .trim()
            .to_owned();
        let port = snapshot
            .get("interface_db_port")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .filter(|value| *value > 0)
            .ok_or_else(|| WorkerError::new("H8_WORKER_INVALID_SNAPSHOT", "MSSQL port invalid"))?;
        Ok(Self {
            owner_id: snapshot
                .get("owner_id")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    WorkerError::new("H8_WORKER_INVALID_SNAPSHOT", "missing snapshot owner_id")
                })?
                .parse::<Uuid>()
                .map_err(|_| {
                    WorkerError::new("H8_WORKER_INVALID_SNAPSHOT", "snapshot owner_id invalid")
                })?,
            bootstrap,
            connector_config_version: expected_version,
            channel_mode: channel_mode.to_owned(),
            mssql: MssqlSettings {
                host: snapshot_text(snapshot, "interface_db_host")?.to_owned(),
                port,
                database: snapshot_text(snapshot, "interface_db_name")?.to_owned(),
                username: snapshot_text(snapshot, "interface_db_username")?.to_owned(),
                password,
            },
        })
    }
}

pub fn parse_secret_map(raw: Option<&str>) -> Result<HashMap<String, String>, WorkerError> {
    let Some(raw) = raw.filter(|value| !value.trim().is_empty()) else {
        return Ok(HashMap::new());
    };
    serde_json::from_str(raw)
        .map_err(|_| WorkerError::new("H8_WORKER_INVALID_SECRET_MAP", "secrets map invalid"))
}

fn snapshot_text<'a>(snapshot: &'a Value, field: &str) -> Result<&'a str, WorkerError> {
    snapshot
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            WorkerError::new(
                "H8_WORKER_INVALID_SNAPSHOT",
                format!("missing snapshot field {field}"),
            )
        })
}

fn required<'a>(values: &'a HashMap<String, String>, name: &str) -> Result<&'a str, WorkerError> {
    values
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| WorkerError::new("H8_WORKER_INVALID_CONFIG", format!("missing env {name}")))
}

fn optional(values: &HashMap<String, String>, name: &str) -> Option<String> {
    values
        .get(name)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_u32(
    values: &HashMap<String, String>,
    name: &str,
    default: u32,
) -> Result<u32, WorkerError> {
    optional(values, name)
        .map(|value| {
            value.parse::<u32>().map_err(|_| {
                WorkerError::new(
                    "H8_WORKER_INVALID_CONFIG",
                    format!("{name} must be an unsigned integer"),
                )
            })
        })
        .unwrap_or(Ok(default))
}

fn parse_u64(
    values: &HashMap<String, String>,
    name: &str,
    default: u64,
) -> Result<u64, WorkerError> {
    optional(values, name)
        .map(|value| {
            value.parse::<u64>().map_err(|_| {
                WorkerError::new(
                    "H8_WORKER_INVALID_CONFIG",
                    format!("{name} must be an unsigned integer"),
                )
            })
        })
        .unwrap_or(Ok(default))
}
