use std::collections::HashMap;

use reqwest::{Client, Method};
use serde_json::{json, Value};

use crate::{
    config::{BootstrapSettings, RuntimeSettings},
    error::WorkerError,
};

#[derive(Clone)]
pub struct ControlPlaneClient {
    client: Client,
    api_base: String,
    api_token: String,
    api_key: Option<String>,
}

impl ControlPlaneClient {
    pub fn new(settings: &BootstrapSettings) -> Result<Self, WorkerError> {
        let api_token = settings.api_token.clone().ok_or_else(|| {
            WorkerError::new("H8_WORKER_CONTROL_TOKEN_REQUIRED", "WMS_API_TOKEN required")
        })?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|error| WorkerError::new("H8_WORKER_HTTP_CLIENT_FAILED", error.to_string()))?;
        Ok(Self {
            client,
            api_base: settings.api_base.clone(),
            api_token,
            api_key: settings.api_key.clone(),
        })
    }

    pub async fn load_runtime_settings(
        &self,
        bootstrap: BootstrapSettings,
        secrets: &HashMap<String, String>,
    ) -> Result<RuntimeSettings, WorkerError> {
        let connector_path = format!("/api/v1/config/erp-connectors/{}", bootstrap.connector_id);
        let connector = self.get(&connector_path).await?;
        if connector.get("id").and_then(Value::as_str)
            != Some(bootstrap.connector_id.to_string().as_str())
            || connector.get("status").and_then(Value::as_str) != Some("active")
        {
            return Err(WorkerError::new(
                "H8_WORKER_CONNECTOR_NOT_ACTIVE",
                "connector is not active",
            ));
        }
        let config_version = connector
            .get("config_version")
            .and_then(Value::as_i64)
            .filter(|version| *version > 0)
            .ok_or_else(|| {
                WorkerError::new(
                    "H8_WORKER_INVALID_CONNECTOR_VERSION",
                    "invalid config version",
                )
            })?;
        let snapshot_path = format!(
            "/api/v1/config/erp-connectors/{}/versions/{config_version}",
            bootstrap.connector_id
        );
        let snapshot = self.get(&snapshot_path).await?;
        RuntimeSettings::from_snapshot(bootstrap, config_version, &snapshot, secrets)
    }

    pub async fn post_heartbeat(
        &self,
        settings: &RuntimeSettings,
        directions: &[&str],
        current_claims: u32,
    ) -> Result<(), WorkerError> {
        let body = json!({
            "worker_id": settings.bootstrap.worker_id,
            "worker_version": settings.bootstrap.worker_version,
            "connector_id": settings.bootstrap.connector_id,
            "directions": directions,
            "current_claims": current_claims,
            "heartbeat_ttl_seconds": settings.bootstrap.heartbeat_ttl_seconds,
        });
        self.request(
            Method::POST,
            "/api/v1/integration/erp-messages/worker-runtime/heartbeat",
            Some(&body),
        )
        .await?;
        Ok(())
    }

    pub async fn get(&self, path: &str) -> Result<Value, WorkerError> {
        self.request(Method::GET, path, None).await
    }

    pub async fn post(&self, path: &str, body: &Value) -> Result<Value, WorkerError> {
        self.request(Method::POST, path, Some(body)).await
    }

    pub async fn get_query(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, WorkerError> {
        let mut url = reqwest::Url::parse(&format!("{}{path}", self.api_base))
            .map_err(|error| WorkerError::new("H8_WORKER_INVALID_URL", error.to_string()))?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        self.request_url(Method::GET, url, None, None).await
    }

    pub async fn post_idempotent(
        &self,
        path: &str,
        body: &Value,
        idempotency_key: &str,
    ) -> Result<Value, WorkerError> {
        let url = reqwest::Url::parse(&format!("{}{path}", self.api_base))
            .map_err(|error| WorkerError::new("H8_WORKER_INVALID_URL", error.to_string()))?;
        self.request_url(Method::POST, url, Some(body), Some(idempotency_key))
            .await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, WorkerError> {
        let url = reqwest::Url::parse(&format!("{}{path}", self.api_base))
            .map_err(|error| WorkerError::new("H8_WORKER_INVALID_URL", error.to_string()))?;
        self.request_url(method, url, body, None).await
    }

    async fn request_url(
        &self,
        method: Method,
        url: reqwest::Url,
        body: Option<&Value>,
        idempotency_key: Option<&str>,
    ) -> Result<Value, WorkerError> {
        let inbound = url
            .path()
            .starts_with("/api/v1/integration/erp-messages/inbound/");
        let mut request = self
            .client
            .request(method, url)
            .header("Accept", "application/json");
        if inbound {
            if let Some(api_key) = &self.api_key {
                request = request.header("x-wms-api-key", api_key);
            } else {
                request = request.bearer_auth(&self.api_token);
            }
        } else {
            request = request.bearer_auth(&self.api_token);
        }
        if let Some(idempotency_key) = idempotency_key {
            request = request.header("Idempotency-Key", idempotency_key);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|error| WorkerError::new("H8_WORKER_HTTP_UNAVAILABLE", error.to_string()))?;
        let status = response.status();
        let raw = response.text().await.map_err(|error| {
            WorkerError::new("H8_WORKER_HTTP_INVALID_RESPONSE", error.to_string())
        })?;
        if !status.is_success() {
            let code = serde_json::from_str::<Value>(&raw).ok().and_then(|value| {
                value
                    .get("code")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            });
            if code.as_deref() == Some("ORDER_NOT_READY") {
                return Err(WorkerError::new("ORDER_NOT_READY", truncate(&raw)));
            }
            if matches!(status.as_u16(), 408 | 425 | 429) || status.is_server_error() {
                return Err(WorkerError::new(
                    "H8_WORKER_HTTP_RETRYABLE",
                    format!("HTTP {}: {}", status.as_u16(), truncate(&raw)),
                ));
            }
            return Err(WorkerError::new(
                "H8_WORKER_HTTP_REJECTED",
                format!("HTTP {}: {}", status.as_u16(), truncate(&raw)),
            ));
        }
        serde_json::from_str(&raw).map_err(|_| {
            WorkerError::new(
                "H8_WORKER_HTTP_INVALID_RESPONSE",
                "control plane returned invalid JSON",
            )
        })
    }
}

fn truncate(raw: &str) -> String {
    raw.chars().take(500).collect()
}
