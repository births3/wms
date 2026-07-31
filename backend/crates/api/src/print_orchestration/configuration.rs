use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{PublishRouteBindingRequest, RouteBinding, RouteBindingListResponse};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
};

use super::{
    repository::{
        json_request_hash, lock_idempotency_key, map_db_error, replay_idempotency,
        store_idempotency_success, PgPrintOrchestrationRepository,
    },
    IdempotentMutation, PrintOrchestrationError,
};

#[derive(Debug, FromRow)]
struct RouteBindingRow {
    id: Uuid,
    owner_id: Uuid,
    warehouse_id: Uuid,
    warehouse_code: String,
    warehouse_name: String,
    customer_id: Uuid,
    customer_code: String,
    customer_name: String,
    delivery_address_id: Uuid,
    delivery_address: String,
    route_code: String,
    effective_from: DateTime<Utc>,
    effective_to: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl PgPrintOrchestrationRepository {
    pub(super) async fn list_route_bindings(
        &self,
        ctx: &AuthContext,
        warehouse_id: Option<Uuid>,
    ) -> Result<RouteBindingListResponse, PrintOrchestrationError> {
        let rows = sqlx::query_as::<_, RouteBindingRow>(
            r#"
            SELECT binding.id, binding.owner_id, binding.warehouse_id,
                   warehouse.warehouse_code, warehouse.warehouse_name,
                   binding.customer_id, customer.customer_code, customer.customer_name,
                   binding.delivery_address_id,
                   concat_ws('', address.province, address.city, address.district, address.detail_address)
                       AS delivery_address,
                   binding.route_code, binding.effective_from,
                   binding.effective_to, binding.created_at
              FROM h9_route_bindings binding
              JOIN warehouses warehouse
                ON warehouse.owner_id = binding.owner_id
               AND warehouse.id = binding.warehouse_id
              JOIN customers customer
                ON customer.owner_id = binding.owner_id
               AND customer.id = binding.customer_id
              JOIN customer_addresses address
                ON address.owner_id = binding.owner_id
               AND address.id = binding.delivery_address_id
             WHERE binding.owner_id = $1
               AND ($2::uuid IS NULL OR binding.warehouse_id = $2)
             ORDER BY binding.effective_from DESC, binding.id
            "#,
        )
        .bind(ctx.owner_id)
        .bind(warehouse_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(RouteBindingListResponse {
            data: rows.into_iter().map(RouteBinding::from).collect(),
        })
    }

    pub(super) async fn publish_route_binding(
        &self,
        ctx: &AuthContext,
        request: PublishRouteBindingRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<RouteBinding>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&request)?;
        let path = "/api/v1/print-orchestration/route-bindings";
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(binding) = replay_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            path,
            now,
        )
        .await?
        {
            return Ok(IdempotentMutation {
                value: binding,
                replayed: true,
            });
        }
        lock_route_boundary(&mut tx, ctx.owner_id, &request).await?;
        ensure_route_scope_exists(&mut tx, ctx.owner_id, &request).await?;
        if route_period_overlaps(&mut tx, ctx.owner_id, &request).await? {
            return Err(PrintOrchestrationError::EffectivePeriodOverlap);
        }

        let binding_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO h9_route_bindings (
                id, owner_id, warehouse_id, customer_id, delivery_address_id,
                route_code, effective_from, effective_to, created_by, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(request.warehouse_id)
        .bind(request.customer_id)
        .bind(request.delivery_address_id)
        .bind(request.route_code.trim())
        .bind(request.effective_from)
        .bind(request.effective_to)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let row = sqlx::query_as::<_, RouteBindingRow>(
            r#"
            SELECT binding.id, binding.owner_id, binding.warehouse_id,
                   warehouse.warehouse_code, warehouse.warehouse_name,
                   binding.customer_id, customer.customer_code, customer.customer_name,
                   binding.delivery_address_id,
                   concat_ws('', address.province, address.city, address.district, address.detail_address)
                       AS delivery_address,
                   binding.route_code, binding.effective_from,
                   binding.effective_to, binding.created_at
              FROM h9_route_bindings binding
              JOIN warehouses warehouse
                ON warehouse.owner_id = binding.owner_id
               AND warehouse.id = binding.warehouse_id
              JOIN customers customer
                ON customer.owner_id = binding.owner_id
               AND customer.id = binding.customer_id
              JOIN customer_addresses address
                ON address.owner_id = binding.owner_id
               AND address.id = binding.delivery_address_id
             WHERE binding.owner_id = $1 AND binding.id = $2
            "#,
        )
        .bind(ctx.owner_id)
        .bind(binding_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let binding = RouteBinding::from(row);
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            path,
            "route_binding",
            &binding,
            now,
        )
        .await?;
        append_route_binding_audit(&mut tx, ctx, &binding, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: binding,
            replayed: false,
        })
    }
}

pub(crate) async fn freeze_outbound_route_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    outbound_order_id: Uuid,
    warehouse_id: Uuid,
    customer_id: Uuid,
    delivery_address_id: Uuid,
    now: DateTime<Utc>,
) -> Result<(), PrintOrchestrationError> {
    let route_codes = sqlx::query_scalar::<_, String>(
        r#"
        SELECT route_code
          FROM h9_route_bindings
         WHERE owner_id = $1
           AND warehouse_id = $2
           AND customer_id = $3
           AND delivery_address_id = $4
           AND effective_from <= $5
           AND (effective_to IS NULL OR effective_to > $5)
         ORDER BY effective_from DESC, id
         LIMIT 2
        "#,
    )
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(delivery_address_id)
    .bind(now)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let route_code = match route_codes.as_slice() {
        [route_code] => route_code,
        [] => return Err(PrintOrchestrationError::RouteBindingNotFound),
        _ => return Err(PrintOrchestrationError::EffectivePeriodOverlap),
    };
    sqlx::query(
        r#"
        INSERT INTO h9_outbound_route_snapshots (
            outbound_order_id, owner_id, warehouse_id, customer_id,
            delivery_address_id, route_code, frozen_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        "#,
    )
    .bind(outbound_order_id)
    .bind(owner_id)
    .bind(warehouse_id)
    .bind(customer_id)
    .bind(delivery_address_id)
    .bind(route_code)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

async fn lock_route_boundary(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &PublishRouteBindingRequest,
) -> Result<(), PrintOrchestrationError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('h9-route-binding'), hashtext($1))")
        .bind(format!(
            "{}:{}:{}",
            owner_id, request.warehouse_id, request.delivery_address_id
        ))
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

async fn ensure_route_scope_exists(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &PublishRouteBindingRequest,
) -> Result<(), PrintOrchestrationError> {
    let valid: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM warehouses warehouse
              JOIN customer_addresses address
                ON address.owner_id = warehouse.owner_id
               AND address.id = $4
               AND address.customer_id = $3
             WHERE warehouse.owner_id = $1
               AND warehouse.id = $2
        )
        "#,
    )
    .bind(owner_id)
    .bind(request.warehouse_id)
    .bind(request.customer_id)
    .bind(request.delivery_address_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if valid {
        Ok(())
    } else {
        Err(PrintOrchestrationError::InvalidRequest)
    }
}

async fn route_period_overlaps(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    request: &PublishRouteBindingRequest,
) -> Result<bool, PrintOrchestrationError> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM h9_route_bindings
             WHERE owner_id = $1
               AND warehouse_id = $2
               AND delivery_address_id = $3
               AND effective_from < COALESCE($5, 'infinity'::timestamptz)
               AND COALESCE(effective_to, 'infinity'::timestamptz) > $4
        )
        "#,
    )
    .bind(owner_id)
    .bind(request.warehouse_id)
    .bind(request.delivery_address_id)
    .bind(request.effective_from)
    .bind(request.effective_to)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn append_route_binding_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    binding: &RouteBinding,
    now: DateTime<Utc>,
) -> Result<(), PrintOrchestrationError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        "publish_route_binding",
        "H9",
        "route_binding",
        binding.id.to_string(),
        Some(AuditDiff::compute(Value::Null, json!(binding))),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| PrintOrchestrationError::Audit(format!("{error:?}")))?;
    Ok(())
}

impl From<RouteBindingRow> for RouteBinding {
    fn from(row: RouteBindingRow) -> Self {
        Self {
            id: row.id,
            owner_id: row.owner_id,
            warehouse_id: row.warehouse_id,
            warehouse_code: row.warehouse_code,
            warehouse_name: row.warehouse_name,
            customer_id: row.customer_id,
            customer_code: row.customer_code,
            customer_name: row.customer_name,
            delivery_address_id: row.delivery_address_id,
            delivery_address: row.delivery_address,
            route_code: row.route_code,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            created_at: row.created_at,
        }
    }
}
