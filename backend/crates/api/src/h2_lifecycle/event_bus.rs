use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use super::support::{append_system_audit_in_tx, db_error, matches_event_pattern};
use super::types::{
    EventDelivery, EventDeliveryRow, EventEnvelope, EventSubscription, EventSubscriptionRow,
    H2LifecycleError,
};
use super::DEFAULT_EVENT_MAX_ATTEMPTS;

pub async fn upsert_event_subscription(
    pool: &PgPool,
    owner_id: Uuid,
    subscriber_key: &str,
    event_pattern: &str,
    active: bool,
    now: DateTime<Utc>,
) -> Result<EventSubscription, H2LifecycleError> {
    if subscriber_key.trim().is_empty() || event_pattern.trim().is_empty() {
        return Err(H2LifecycleError::InvalidInput(
            "subscriber_key and event_pattern are required".to_string(),
        ));
    }
    let mut tx = pool.begin().await.map_err(db_error)?;
    let row = sqlx::query_as::<_, EventSubscriptionRow>(
        r#"
        INSERT INTO event_bus_subscription (
            id, owner_id, subscriber_key, event_pattern, active, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        ON CONFLICT (owner_id, subscriber_key)
        DO UPDATE SET
            event_pattern = EXCLUDED.event_pattern,
            active = EXCLUDED.active,
            updated_at = EXCLUDED.updated_at
        RETURNING id, owner_id, subscriber_key, event_pattern, active
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(subscriber_key)
    .bind(event_pattern)
    .bind(active)
    .bind(now)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;

    let subscription = EventSubscription::from(row);
    append_system_audit_in_tx(
        &mut tx,
        owner_id,
        "event_bus.subscription.upsert",
        "event_bus_subscription",
        &subscription.id.to_string(),
        serde_json::json!({
            "subscriber_key": subscriber_key,
            "event_pattern": event_pattern,
            "active": active,
        }),
        now,
        "system-event-bus",
    )
    .await?;
    tx.commit().await.map_err(db_error)?;
    Ok(subscription)
}

pub async fn publish_event(
    pool: &PgPool,
    owner_id: Uuid,
    idempotency_key: &str,
    event_type: &str,
    source_module: &str,
    resource_type: &str,
    resource_id: &str,
    payload: Value,
    now: DateTime<Utc>,
) -> Result<EventEnvelope, H2LifecycleError> {
    if idempotency_key.trim().is_empty() || event_type.trim().is_empty() {
        return Err(H2LifecycleError::InvalidInput(
            "idempotency_key and event_type are required".to_string(),
        ));
    }
    if let Some(existing) = load_event_by_idempotency(pool, owner_id, idempotency_key).await? {
        return Ok(existing);
    }

    let mut tx = pool.begin().await.map_err(db_error)?;
    let event_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO event_bus_event (
            id, owner_id, idempotency_key, event_type, source_module,
            resource_type, resource_id, payload, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(event_id)
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(event_type)
    .bind(source_module)
    .bind(resource_type)
    .bind(resource_id)
    .bind(payload)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;

    let subscriptions: Vec<EventSubscriptionRow> = sqlx::query_as(
        r#"
        SELECT id, owner_id, subscriber_key, event_pattern, active
          FROM event_bus_subscription
         WHERE owner_id = $1 AND active = TRUE
        "#,
    )
    .bind(owner_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db_error)?;

    let mut delivery_count = 0_i64;
    for subscription in subscriptions {
        if !matches_event_pattern(&subscription.event_pattern, event_type) {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO event_bus_delivery (
                id, owner_id, event_id, subscription_id, status, attempt_count, next_attempt_at
            )
            VALUES ($1, $2, $3, $4, 'pending', 0, $5)
            ON CONFLICT (event_id, subscription_id) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(event_id)
        .bind(subscription.id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        delivery_count += 1;
    }

    tx.commit().await.map_err(db_error)?;

    Ok(EventEnvelope {
        id: event_id,
        owner_id,
        event_type: event_type.to_string(),
        delivery_count,
    })
}

pub async fn pending_event_deliveries(
    pool: &PgPool,
    owner_id: Uuid,
    limit: i64,
) -> Result<Vec<EventDelivery>, H2LifecycleError> {
    let rows: Vec<EventDeliveryRow> = sqlx::query_as(
        r#"
        SELECT id, event_id, status, attempt_count
          FROM event_bus_delivery
         WHERE owner_id = $1 AND status = 'pending'
         ORDER BY created_at ASC
         LIMIT $2
        "#,
    )
    .bind(owner_id)
    .bind(limit.clamp(1, 1000))
    .fetch_all(pool)
    .await
    .map_err(db_error)?;

    rows.into_iter().map(EventDelivery::try_from).collect()
}

pub async fn record_delivery_failure(
    pool: &PgPool,
    owner_id: Uuid,
    delivery_id: Uuid,
    error: &str,
    now: DateTime<Utc>,
) -> Result<EventDelivery, H2LifecycleError> {
    let mut tx = pool.begin().await.map_err(db_error)?;
    let row = sqlx::query_as::<_, EventDeliveryRow>(
        r#"
        UPDATE event_bus_delivery
           SET attempt_count = attempt_count + 1,
               status = CASE
                   WHEN attempt_count + 1 >= $3 THEN 'dead_letter'
                   ELSE 'pending'
               END,
               last_error = $4,
               updated_at = $5,
               next_attempt_at = $5
         WHERE owner_id = $1 AND id = $2
        RETURNING id, event_id, status, attempt_count
        "#,
    )
    .bind(owner_id)
    .bind(delivery_id)
    .bind(DEFAULT_EVENT_MAX_ATTEMPTS)
    .bind(error)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error)?
    .ok_or(H2LifecycleError::NotFound)?;

    if row.status == "dead_letter" {
        sqlx::query(
            r#"
            INSERT INTO event_bus_dead_letter (id, owner_id, delivery_id, event_id, reason, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (delivery_id) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(owner_id)
        .bind(delivery_id)
        .bind(row.event_id)
        .bind(error)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    }

    tx.commit().await.map_err(db_error)?;
    EventDelivery::try_from(row)
}

async fn load_event_by_idempotency(
    pool: &PgPool,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<EventEnvelope>, H2LifecycleError> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT id, event_type
          FROM event_bus_event
         WHERE owner_id = $1 AND idempotency_key = $2
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await
    .map_err(db_error)?;
    let Some((id, event_type)) = row else {
        return Ok(None);
    };
    let delivery_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM event_bus_delivery WHERE event_id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(db_error)?;
    Ok(Some(EventEnvelope {
        id,
        owner_id,
        event_type,
        delivery_count,
    }))
}
