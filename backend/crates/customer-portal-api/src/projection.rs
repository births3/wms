use crate::{
    models::{
        AddressProjection, CustomerOrderSnapshotProjection, CustomerProjection, OrderProjection,
        ProjectionRequest, ProjectionResponse, ReportProjection,
    },
    PortalError, PortalState,
};
use axum::{extract::State, http::HeaderMap, Json};
use sqlx::{Postgres, Transaction};

const MAX_PROJECTION_ATTEMPTS: i32 = 5;

pub async fn ingest_projection(
    State(state): State<PortalState>,
    headers: HeaderMap,
    Json(request): Json<ProjectionRequest>,
) -> Result<Json<ProjectionResponse>, PortalError> {
    let supplied_key = headers
        .get("X-Projection-Key")
        .and_then(|value| value.to_str().ok());
    if supplied_key != Some(state.projection_key.as_ref()) {
        return Err(PortalError::Unauthorized);
    }
    let existing = sqlx::query_as::<_, (String, i32)>(
        "SELECT status, attempt_count
         FROM portal_projection_events
         WHERE event_id = $1",
    )
    .bind(request.event_id)
    .fetch_optional(&state.pool)
    .await?;
    if existing
        .as_ref()
        .is_some_and(|(status, _)| status == "succeeded")
    {
        return Ok(Json(ProjectionResponse {
            event_id: request.event_id,
            duplicate: true,
            status: "succeeded".to_string(),
        }));
    }
    let attempt = existing.map_or(1, |(_, attempt)| attempt + 1);
    if attempt > MAX_PROJECTION_ATTEMPTS {
        return Err(PortalError::Conflict(
            "投影事件已进入死信队列，需人工重放".to_string(),
        ));
    }
    sqlx::query(
        "INSERT INTO portal_projection_events (
            event_id, event_type, occurred_at, payload, status, attempt_count, updated_at
         )
         VALUES ($1, $2, $3, $4, 'processing', $5, now())
         ON CONFLICT (event_id) DO UPDATE
         SET status = 'processing',
             attempt_count = $5,
             last_error = NULL,
             next_attempt_at = NULL,
             updated_at = now()",
    )
    .bind(request.event_id)
    .bind(&request.event_type)
    .bind(request.occurred_at)
    .bind(&request.payload)
    .bind(attempt)
    .execute(&state.pool)
    .await?;

    let mut transaction = state.pool.begin().await?;
    let applied = apply_projection(&mut transaction, &request).await;
    match applied {
        Ok(()) => {
            sqlx::query(
                "UPDATE portal_projection_events
                 SET status = 'succeeded', processed_at = now(), updated_at = now()
                 WHERE event_id = $1",
            )
            .bind(request.event_id)
            .execute(&mut *transaction)
            .await?;
            transaction.commit().await?;
            Ok(Json(ProjectionResponse {
                event_id: request.event_id,
                duplicate: false,
                status: "succeeded".to_string(),
            }))
        }
        Err(error) => {
            transaction.rollback().await?;
            let status = if attempt >= MAX_PROJECTION_ATTEMPTS {
                "dead_letter"
            } else {
                "failed"
            };
            sqlx::query(
                "UPDATE portal_projection_events
                 SET status = $2,
                     last_error = $3,
                     next_attempt_at = CASE
                         WHEN $2 = 'failed' THEN now() + interval '30 seconds'
                         ELSE NULL
                     END,
                     updated_at = now()
                 WHERE event_id = $1",
            )
            .bind(request.event_id)
            .bind(status)
            .bind(error.to_string())
            .execute(&state.pool)
            .await?;
            Err(error)
        }
    }
}

async fn apply_projection(
    transaction: &mut Transaction<'_, Postgres>,
    request: &ProjectionRequest,
) -> Result<(), PortalError> {
    match request.event_type.as_str() {
        "customer_order.snapshot" => {
            let payload: CustomerOrderSnapshotProjection =
                serde_json::from_value(request.payload.clone())?;
            apply_customer(transaction, payload.customer).await?;
            apply_address(transaction, payload.address).await?;
            apply_order(transaction, payload.order).await?;
        }
        "customer.upsert" => {
            let payload: CustomerProjection = serde_json::from_value(request.payload.clone())?;
            apply_customer(transaction, payload).await?;
        }
        "customer_address.upsert" => {
            let payload: AddressProjection = serde_json::from_value(request.payload.clone())?;
            apply_address(transaction, payload).await?;
        }
        "outbound_order.upsert" => {
            let payload: OrderProjection = serde_json::from_value(request.payload.clone())?;
            apply_order(transaction, payload).await?;
        }
        "drug_inspection_report.upsert" => {
            let payload: ReportProjection = serde_json::from_value(request.payload.clone())?;
            let newer_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (
                    SELECT 1 FROM portal_report_versions
                    WHERE report_id = $1 AND updated_at > $2
                 )",
            )
            .bind(payload.report_id)
            .bind(payload.updated_at)
            .fetch_one(&mut **transaction)
            .await?;
            if newer_exists {
                return Ok(());
            }
            if payload.is_current {
                sqlx::query(
                    "UPDATE portal_report_versions
                     SET is_current = FALSE, status = 'superseded'
                     WHERE report_id = $1 AND id <> $2 AND is_current",
                )
                .bind(payload.report_id)
                .bind(payload.id)
                .execute(&mut **transaction)
                .await?;
            }
            sqlx::query(
                "INSERT INTO portal_report_versions (
                    id, report_id, owner_id, product_id, batch_no, version_number,
                    report_no, status, is_current, modification_reason,
                    customer_copy_status, customer_copy_storage_key,
                    customer_copy_file_name, customer_copy_size, customer_copy_hash,
                    digitally_signed_original, confirmed_at, updated_at
                 )
                 VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, $18
                 )
                 ON CONFLICT (id) DO UPDATE
                 SET status = EXCLUDED.status,
                     is_current = EXCLUDED.is_current,
                     modification_reason = EXCLUDED.modification_reason,
                     customer_copy_status = EXCLUDED.customer_copy_status,
                     customer_copy_storage_key = EXCLUDED.customer_copy_storage_key,
                     customer_copy_file_name = EXCLUDED.customer_copy_file_name,
                     customer_copy_size = EXCLUDED.customer_copy_size,
                     customer_copy_hash = EXCLUDED.customer_copy_hash,
                     digitally_signed_original = EXCLUDED.digitally_signed_original,
                     updated_at = EXCLUDED.updated_at
                 WHERE EXCLUDED.updated_at >= portal_report_versions.updated_at",
            )
            .bind(payload.id)
            .bind(payload.report_id)
            .bind(payload.owner_id)
            .bind(payload.product_id)
            .bind(payload.batch_no)
            .bind(payload.version_number)
            .bind(payload.report_no)
            .bind(payload.status)
            .bind(payload.is_current)
            .bind(payload.modification_reason)
            .bind(payload.customer_copy_status)
            .bind(payload.customer_copy_storage_key)
            .bind(payload.customer_copy_file_name)
            .bind(payload.customer_copy_size)
            .bind(payload.customer_copy_hash)
            .bind(payload.digitally_signed_original)
            .bind(payload.confirmed_at)
            .bind(payload.updated_at)
            .execute(&mut **transaction)
            .await?;
        }
        _ => {
            return Err(PortalError::Validation(format!(
                "不支持的投影事件类型：{}",
                request.event_type
            )));
        }
    }
    Ok(())
}

async fn apply_customer(
    transaction: &mut Transaction<'_, Postgres>,
    payload: CustomerProjection,
) -> Result<(), PortalError> {
    sqlx::query(
        "INSERT INTO portal_customers (
            id, customer_code, customer_name, updated_at
         )
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (id) DO UPDATE
         SET customer_code = EXCLUDED.customer_code,
             customer_name = EXCLUDED.customer_name,
             updated_at = EXCLUDED.updated_at
         WHERE EXCLUDED.updated_at >= portal_customers.updated_at",
    )
    .bind(payload.id)
    .bind(payload.customer_code)
    .bind(payload.customer_name)
    .bind(payload.updated_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn apply_address(
    transaction: &mut Transaction<'_, Postgres>,
    payload: AddressProjection,
) -> Result<(), PortalError> {
    sqlx::query(
        "INSERT INTO portal_customer_addresses (
            id, customer_id, address_code, address_name, address_snapshot, updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (id) DO UPDATE
         SET customer_id = EXCLUDED.customer_id,
             address_code = EXCLUDED.address_code,
             address_name = EXCLUDED.address_name,
             address_snapshot = EXCLUDED.address_snapshot,
             updated_at = EXCLUDED.updated_at
         WHERE EXCLUDED.updated_at >= portal_customer_addresses.updated_at",
    )
    .bind(payload.id)
    .bind(payload.customer_id)
    .bind(payload.address_code)
    .bind(payload.address_name)
    .bind(payload.address_snapshot)
    .bind(payload.updated_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn apply_order(
    transaction: &mut Transaction<'_, Postgres>,
    payload: OrderProjection,
) -> Result<(), PortalError> {
    if !matches!(payload.status.as_str(), "shipped" | "signed") {
        sqlx::query(
            "DELETE FROM portal_orders
             WHERE id = $1 AND updated_at <= $2",
        )
        .bind(payload.id)
        .bind(payload.updated_at)
        .execute(&mut **transaction)
        .await?;
        return Ok(());
    }
    let applied = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO portal_orders (
                    id, customer_id, order_no, status, delivery_address_id,
                    address_snapshot, shipped_at, signed_at, updated_at
                 )
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (id) DO UPDATE
                 SET customer_id = EXCLUDED.customer_id,
                     order_no = EXCLUDED.order_no,
                     status = EXCLUDED.status,
                     delivery_address_id = EXCLUDED.delivery_address_id,
                     address_snapshot = EXCLUDED.address_snapshot,
                     shipped_at = EXCLUDED.shipped_at,
                     signed_at = EXCLUDED.signed_at,
                     updated_at = EXCLUDED.updated_at
                 WHERE EXCLUDED.updated_at >= portal_orders.updated_at
                 RETURNING id",
    )
    .bind(payload.id)
    .bind(payload.customer_id)
    .bind(payload.order_no)
    .bind(payload.status)
    .bind(payload.delivery_address_id)
    .bind(payload.address_snapshot)
    .bind(payload.shipped_at)
    .bind(payload.signed_at)
    .bind(payload.updated_at)
    .fetch_optional(&mut **transaction)
    .await?;
    if applied.is_some() {
        sqlx::query("DELETE FROM portal_order_lines WHERE order_id = $1")
            .bind(payload.id)
            .execute(&mut **transaction)
            .await?;
        for line in payload.lines {
            sqlx::query(
                "INSERT INTO portal_order_lines (
                            id, order_id, product_id, product_code, product_name,
                            batch_no, quantity
                         )
                         VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(line.id)
            .bind(payload.id)
            .bind(line.product_id)
            .bind(line.product_code)
            .bind(line.product_name)
            .bind(line.batch_no)
            .bind(line.quantity)
            .execute(&mut **transaction)
            .await?;
        }
    }
    Ok(())
}
