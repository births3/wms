use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{DeliveryNoteGroup, ManualDeliveryNoteCutoffRequest};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    document_numbering::{GenerateDocumentNumberRequest, PgDocumentNumberingService},
};

use super::aggregation_rule::{partition_orders_by_rule_in_tx, resolve_rule_application_in_tx};
use super::{IdempotentMutation, PrintOrchestrationError};

const DELIVERY_NOTE_NUMBERING_SUBJECT: &str = "print_document_category:delivery_note";

#[derive(Clone, Debug)]
pub(super) struct PgPrintOrchestrationRepository {
    pub(super) pool: PgPool,
}

#[derive(Debug, FromRow)]
struct OrderBoundaryRow {
    status: String,
    warehouse_id: Uuid,
    customer_id: Uuid,
    delivery_address_id: Uuid,
    route_code: String,
}

#[derive(Clone, Debug, FromRow)]
pub(super) struct PendingCutoffBoundary {
    pub(super) warehouse_id: Uuid,
    pub(super) customer_id: Uuid,
    pub(super) delivery_address_id: Uuid,
    pub(super) route_code: String,
}

impl PgPrintOrchestrationRepository {
    pub(super) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) async fn manual_cutoff(
        &self,
        ctx: &AuthContext,
        request: ManualDeliveryNoteCutoffRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<DeliveryNoteGroup>, PrintOrchestrationError> {
        let request_hash = json_request_hash(&request)?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(group) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: group,
                replayed: true,
            });
        }

        let rows = lock_order_boundaries(&mut tx, ctx.owner_id, &request.order_ids).await?;
        let boundary = validate_boundary(&request, &rows)?;
        if any_order_already_cutoff(&mut tx, ctx.owner_id, &request.order_ids).await? {
            return Err(PrintOrchestrationError::OrderAlreadyCutoff);
        }
        let rule_application =
            resolve_rule_application_in_tx(&mut tx, ctx.owner_id, &request.order_ids).await?;

        let group_id = Uuid::new_v4();
        let generated = PgDocumentNumberingService::new()
            .generate_in_tx(
                &mut tx,
                ctx,
                GenerateDocumentNumberRequest {
                    document_type: DELIVERY_NOTE_NUMBERING_SUBJECT.to_string(),
                    idempotency_key: format!("h9-delivery-note:{idempotency_key}"),
                    source_module: "H9".to_string(),
                    source_document_id: Some(group_id),
                },
                now,
            )
            .await
            .map_err(PrintOrchestrationError::DocumentNumbering)?;
        let group = DeliveryNoteGroup {
            id: group_id,
            owner_id: ctx.owner_id,
            warehouse_id: boundary.warehouse_id,
            customer_id: boundary.customer_id,
            delivery_address_id: boundary.delivery_address_id,
            route_code: boundary.route_code.clone(),
            delivery_note_no: generated.value.generated_no,
            cutoff_mode: "manual".to_string(),
            cutoff_reason: Some(request.reason.trim().to_string()),
            cutoff_plan_id: None,
            scheduled_cutoff_at: None,
            cutoff_at: now,
            order_ids: request.order_ids.clone(),
            aggregation_rule_version_id: rule_application.version_id,
            aggregation_rule_version_no: rule_application.version_no,
            aggregation_group_key: rule_application.group_key,
        };
        insert_group(&mut tx, ctx, &group, &rule_application.snapshot, now).await?;
        insert_group_orders(&mut tx, &group).await?;
        // US-H9-008: create the frozen suite instance when a published print
        // suite resolves; without one the cutoff behaviour stays unchanged.
        self.create_suite_instance_in_tx(&mut tx, ctx, &group, now)
            .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "/api/v1/print-orchestration/delivery-note-groups/manual-cutoff",
            "delivery_note_group",
            &group,
            now,
        )
        .await?;
        append_cutoff_audit(&mut tx, ctx, "manual_cutoff_delivery_note", &group, now).await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: group,
            replayed: false,
        })
    }

    pub(super) async fn list_pending_cutoff_boundaries(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<PendingCutoffBoundary>, PrintOrchestrationError> {
        sqlx::query_as::<_, PendingCutoffBoundary>(
            r#"
            SELECT DISTINCT snapshot.warehouse_id, snapshot.customer_id,
                   snapshot.delivery_address_id, snapshot.route_code
              FROM outbound_orders order_row
              JOIN h9_outbound_route_snapshots snapshot
                ON snapshot.owner_id = order_row.owner_id
               AND snapshot.outbound_order_id = order_row.id
             WHERE order_row.owner_id = $1
               AND order_row.status = 'confirmed'
               AND NOT EXISTS (
                    SELECT 1
                      FROM h9_delivery_note_group_orders grouped
                     WHERE grouped.owner_id = order_row.owner_id
                       AND grouped.outbound_order_id = order_row.id
               )
             ORDER BY snapshot.warehouse_id, snapshot.customer_id,
                      snapshot.delivery_address_id, snapshot.route_code
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)
    }

    pub(super) async fn scheduled_cutoff(
        &self,
        ctx: &AuthContext,
        plan: &wms_domain::CutoffPlan,
        boundary: &PendingCutoffBoundary,
        scheduled_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<Vec<DeliveryNoteGroup>, PrintOrchestrationError> {
        let idempotency_key = format!(
            "h9-scheduled-cutoff:{}:{}:{}",
            plan.id, boundary.delivery_address_id, scheduled_at
        );
        let request_hash = json_request_hash(&json!({
            "plan_id": plan.id,
            "warehouse_id": boundary.warehouse_id,
            "customer_id": boundary.customer_id,
            "delivery_address_id": boundary.delivery_address_id,
            "route_code": boundary.route_code,
            "scheduled_at": scheduled_at,
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, &idempotency_key).await?;
        if let Some(groups) =
            replay_idempotency(&mut tx, ctx.owner_id, &idempotency_key, &request_hash, now).await?
        {
            return Ok(groups);
        }
        let order_ids =
            lock_scheduled_orders(&mut tx, ctx.owner_id, boundary, scheduled_at).await?;
        if order_ids.is_empty() {
            return Ok(Vec::new());
        }
        let partitions = partition_orders_by_rule_in_tx(&mut tx, ctx.owner_id, &order_ids).await?;
        let mut groups = Vec::with_capacity(partitions.len());
        for (index, partition) in partitions.into_iter().enumerate() {
            let group_id = Uuid::new_v4();
            let generated = PgDocumentNumberingService::new()
                .generate_in_tx(
                    &mut tx,
                    ctx,
                    GenerateDocumentNumberRequest {
                        document_type: DELIVERY_NOTE_NUMBERING_SUBJECT.to_string(),
                        idempotency_key: format!("h9-delivery-note:{idempotency_key}:{index}"),
                        source_module: "H9".to_string(),
                        source_document_id: Some(group_id),
                    },
                    now,
                )
                .await
                .map_err(PrintOrchestrationError::DocumentNumbering)?;
            let group = DeliveryNoteGroup {
                id: group_id,
                owner_id: ctx.owner_id,
                warehouse_id: boundary.warehouse_id,
                customer_id: boundary.customer_id,
                delivery_address_id: boundary.delivery_address_id,
                route_code: boundary.route_code.clone(),
                delivery_note_no: generated.value.generated_no,
                cutoff_mode: "scheduled".to_string(),
                cutoff_reason: None,
                cutoff_plan_id: Some(plan.id),
                scheduled_cutoff_at: Some(scheduled_at),
                cutoff_at: now,
                order_ids: partition.order_ids,
                aggregation_rule_version_id: partition.application.version_id,
                aggregation_rule_version_no: partition.application.version_no,
                aggregation_group_key: partition.application.group_key,
            };
            insert_group(&mut tx, ctx, &group, &partition.application.snapshot, now).await?;
            insert_group_orders(&mut tx, &group).await?;
            self.create_suite_instance_in_tx(&mut tx, ctx, &group, now)
                .await?;
            append_cutoff_audit(&mut tx, ctx, "scheduled_cutoff_delivery_note", &group, now)
                .await?;
            groups.push(group);
        }
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            &idempotency_key,
            &request_hash,
            "/internal/h9/run-scheduled-cutoffs",
            "delivery_note_groups",
            &groups,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(groups)
    }
}

async fn lock_order_boundaries(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_ids: &[Uuid],
) -> Result<Vec<OrderBoundaryRow>, PrintOrchestrationError> {
    sqlx::query_as::<_, OrderBoundaryRow>(
        r#"
        SELECT order_row.status, snapshot.warehouse_id, snapshot.customer_id,
               snapshot.delivery_address_id, snapshot.route_code
          FROM outbound_orders order_row
          JOIN h9_outbound_route_snapshots snapshot
            ON snapshot.owner_id = order_row.owner_id
           AND snapshot.outbound_order_id = order_row.id
         WHERE order_row.owner_id = $1
           AND order_row.id = ANY($2)
         ORDER BY order_row.id
         FOR UPDATE OF order_row
        "#,
    )
    .bind(owner_id)
    .bind(order_ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

fn validate_boundary<'a>(
    request: &ManualDeliveryNoteCutoffRequest,
    rows: &'a [OrderBoundaryRow],
) -> Result<&'a OrderBoundaryRow, PrintOrchestrationError> {
    if rows.len() != request.order_ids.len() {
        return Err(PrintOrchestrationError::OrderNotFound);
    }
    let Some(first) = rows.first() else {
        return Err(PrintOrchestrationError::OrderNotFound);
    };
    if rows.iter().any(|row| row.status != "confirmed") {
        return Err(PrintOrchestrationError::OrderNotEligibleForCutoff);
    }
    if first.warehouse_id != request.warehouse_id
        || first.delivery_address_id != request.delivery_address_id
        || rows.iter().any(|row| {
            row.warehouse_id != first.warehouse_id
                || row.customer_id != first.customer_id
                || row.delivery_address_id != first.delivery_address_id
                || row.route_code != first.route_code
        })
    {
        return Err(PrintOrchestrationError::AggregationBoundaryMismatch);
    }
    Ok(first)
}

async fn any_order_already_cutoff(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    order_ids: &[Uuid],
) -> Result<bool, PrintOrchestrationError> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM h9_delivery_note_group_orders
             WHERE owner_id = $1 AND outbound_order_id = ANY($2)
        )
        "#,
    )
    .bind(owner_id)
    .bind(order_ids)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn lock_scheduled_orders(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    boundary: &PendingCutoffBoundary,
    scheduled_at: DateTime<Utc>,
) -> Result<Vec<Uuid>, PrintOrchestrationError> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT order_row.id
          FROM outbound_orders order_row
          JOIN h9_outbound_route_snapshots snapshot
            ON snapshot.owner_id = order_row.owner_id
           AND snapshot.outbound_order_id = order_row.id
         WHERE order_row.owner_id = $1
           AND snapshot.warehouse_id = $2
           AND snapshot.customer_id = $3
           AND snapshot.delivery_address_id = $4
           AND snapshot.route_code = $5
           AND order_row.status = 'confirmed'
           AND order_row.created_at <= $6
           AND NOT EXISTS (
                SELECT 1
                  FROM h9_delivery_note_group_orders grouped
                 WHERE grouped.owner_id = order_row.owner_id
                   AND grouped.outbound_order_id = order_row.id
           )
         ORDER BY order_row.created_at, order_row.id
         FOR UPDATE OF order_row
        "#,
    )
    .bind(owner_id)
    .bind(boundary.warehouse_id)
    .bind(boundary.customer_id)
    .bind(boundary.delivery_address_id)
    .bind(&boundary.route_code)
    .bind(scheduled_at)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_db_error)
}

async fn insert_group(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    group: &DeliveryNoteGroup,
    rule_snapshot: &Value,
    now: DateTime<Utc>,
) -> Result<(), PrintOrchestrationError> {
    sqlx::query(
        r#"
        INSERT INTO h9_delivery_note_groups (
            id, owner_id, warehouse_id, customer_id, delivery_address_id,
            route_code, delivery_note_no, cutoff_mode, cutoff_reason,
            cutoff_plan_id, scheduled_cutoff_at, cutoff_at,
            aggregation_rule_version_id, aggregation_rule_version_no,
            aggregation_rule_snapshot, aggregation_group_key,
            created_by, created_at
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9,
            $10, $11, $12, $13, $14, $15, $16, $17, $18
        )
        "#,
    )
    .bind(group.id)
    .bind(group.owner_id)
    .bind(group.warehouse_id)
    .bind(group.customer_id)
    .bind(group.delivery_address_id)
    .bind(&group.route_code)
    .bind(&group.delivery_note_no)
    .bind(&group.cutoff_mode)
    .bind(&group.cutoff_reason)
    .bind(group.cutoff_plan_id)
    .bind(group.scheduled_cutoff_at)
    .bind(group.cutoff_at)
    .bind(group.aggregation_rule_version_id)
    .bind(group.aggregation_rule_version_no)
    .bind(rule_snapshot)
    .bind(&group.aggregation_group_key)
    .bind(ctx.user_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map(|_| ())
    .map_err(map_db_error)
}

async fn insert_group_orders(
    tx: &mut Transaction<'_, Postgres>,
    group: &DeliveryNoteGroup,
) -> Result<(), PrintOrchestrationError> {
    for order_id in &group.order_ids {
        sqlx::query(
            r#"
            INSERT INTO h9_delivery_note_group_orders (
                group_id, owner_id, outbound_order_id, warehouse_id,
                customer_id, delivery_address_id, route_code, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(group.id)
        .bind(group.owner_id)
        .bind(order_id)
        .bind(group.warehouse_id)
        .bind(group.customer_id)
        .bind(group.delivery_address_id)
        .bind(&group.route_code)
        .bind(group.cutoff_at)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

async fn append_cutoff_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    group: &DeliveryNoteGroup,
    now: DateTime<Utc>,
) -> Result<(), PrintOrchestrationError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "H9",
        "delivery_note_group",
        group.id.to_string(),
        Some(AuditDiff::compute(Value::Null, json!(group))),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| PrintOrchestrationError::Audit(format!("{error:?}")))?;
    Ok(())
}

pub(super) async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), PrintOrchestrationError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind(owner_id.to_string())
        .bind(idempotency_key)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

pub(super) async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, PrintOrchestrationError> {
    let row: Option<(String, Value, DateTime<Utc>)> = sqlx::query_as(
        r#"
        SELECT request_hash, response_body, expires_at
          FROM idempotency_request
         WHERE owner_id = $1 AND idempotency_key = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(idempotency_key)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    let Some((stored_hash, response_body, expires_at)) = row else {
        return Ok(None);
    };
    if expires_at <= now {
        sqlx::query("DELETE FROM idempotency_request WHERE owner_id = $1 AND idempotency_key = $2")
            .bind(owner_id)
            .bind(idempotency_key)
            .execute(&mut **tx)
            .await
            .map_err(map_db_error)?;
        return Ok(None);
    }
    if stored_hash != request_hash {
        return Err(PrintOrchestrationError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))
}

pub(super) async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    path: &str,
    resource_type: &str,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), PrintOrchestrationError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?;
    let resource_id = response_body
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("delivery_note_group")
        .to_string();
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id,
            expires_at, created_at
        )
        VALUES (
            $1, $2, $3, $4, 'POST', $5,
            200, $6, $7, $8, $9, $10
        )
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map(|_| ())
    .map_err(map_db_error)
}

pub(super) fn json_request_hash<T: Serialize>(
    value: &T,
) -> Result<String, PrintOrchestrationError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| PrintOrchestrationError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

pub(super) fn map_db_error(error: sqlx::Error) -> PrintOrchestrationError {
    PrintOrchestrationError::Database(error.to_string())
}
