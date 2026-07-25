use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::h2_lifecycle::{acknowledge_event_delivery, record_delivery_failure};

#[derive(Clone)]
pub struct DrugInspectionPortalBridge {
    pool: PgPool,
    client: reqwest::Client,
    projection_url: String,
    projection_key: String,
}

#[derive(Debug, FromRow)]
struct PendingProjection {
    delivery_id: Uuid,
    owner_id: Uuid,
    event_id: Uuid,
    event_type: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

impl DrugInspectionPortalBridge {
    pub fn new(pool: PgPool, portal_base_url: &str, projection_key: String) -> Self {
        Self {
            pool,
            client: reqwest::Client::new(),
            projection_url: format!(
                "{}/api/v1/internal/projections",
                portal_base_url.trim_end_matches('/')
            ),
            projection_key,
        }
    }

    pub async fn deliver_next(&self) -> Result<bool, String> {
        let pending = sqlx::query_as::<_, PendingProjection>(
            r#"
            SELECT delivery.id AS delivery_id, delivery.owner_id,
                   event.id AS event_id, event.event_type, event.payload, event.created_at
              FROM event_bus_delivery delivery
              JOIN event_bus_subscription subscription
                ON subscription.id = delivery.subscription_id
               AND subscription.owner_id = delivery.owner_id
              JOIN event_bus_event event
                ON event.id = delivery.event_id
               AND event.owner_id = delivery.owner_id
             WHERE subscription.subscriber_key = 'mdi-customer-portal'
               AND subscription.active
               AND delivery.status = 'pending'
               AND (delivery.next_attempt_at IS NULL OR delivery.next_attempt_at <= now())
             ORDER BY delivery.created_at
             LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        let Some(pending) = pending else {
            return Ok(false);
        };
        let projection_event_type = pending
            .payload
            .get("projection_event_type")
            .and_then(Value::as_str)
            .ok_or_else(|| "portal projection event is missing projection_event_type".to_string());
        let response = match projection_event_type {
            Ok(event_type) => self
                .client
                .post(&self.projection_url)
                .header("X-Projection-Key", &self.projection_key)
                .json(&json!({
                    "event_id": pending.event_id,
                    "event_type": event_type,
                    "occurred_at": pending.created_at,
                    "payload": pending.payload,
                }))
                .send()
                .await
                .map_err(|error| error.to_string()),
            Err(error) => Err(error),
        };
        match response {
            Ok(response) if response.status().is_success() => {
                acknowledge_event_delivery(
                    &self.pool,
                    pending.owner_id,
                    pending.delivery_id,
                    Utc::now(),
                )
                .await
                .map_err(|error| format!("{error:?}"))?;
                Ok(true)
            }
            Ok(response) => {
                let error = format!(
                    "portal projection rejected event {} with HTTP {}",
                    pending.event_type,
                    response.status()
                );
                self.record_failure(&pending, &error).await?;
                Err(error)
            }
            Err(error) => {
                self.record_failure(&pending, &error).await?;
                Err(error)
            }
        }
    }

    async fn record_failure(&self, pending: &PendingProjection, error: &str) -> Result<(), String> {
        record_delivery_failure(
            &self.pool,
            pending.owner_id,
            pending.delivery_id,
            &error.chars().take(500).collect::<String>(),
            Utc::now(),
        )
        .await
        .map(|_| ())
        .map_err(|failure| format!("{failure:?}"))
    }
}

pub fn spawn_drug_inspection_portal_bridge(
    pool: PgPool,
    portal_base_url: String,
    projection_key: String,
) {
    tokio::spawn(async move {
        let bridge = DrugInspectionPortalBridge::new(pool, &portal_base_url, projection_key);
        loop {
            match bridge.deliver_next().await {
                Ok(true) => continue,
                Ok(false) | Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    });
}
