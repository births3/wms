//! M-RC 库存对账事务。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::{
    CreateStockLossOrderRequest, CreateStockSurplusOrderRequest, StockAdjustmentSource,
    StockLossReason, StockSurplusReason,
};

use crate::{
    audit::{append_event_in_tx, AuditDiff, AuditWriteRequest},
    auth::AuthContext,
    stock_adjustment::PgStockAdjustmentRepository,
};

mod idempotency;
mod isolation;
mod progression;
mod run_persistence;
mod validation;

pub(crate) use idempotency::{
    db, lock_idempotency, lock_reconciliation_window, replay_idempotency, request_hash,
    store_idempotency,
};
use isolation::{acquire_item_locks, release_item_locks};
pub(crate) use progression::{advance_from_h8_receipt_in_tx, advance_from_stock_adjustment_in_tx};
use run_persistence::load_existing_window;
use validation::{map_stock_adjustment, normalize_request};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ErpInventorySnapshotItem {
    pub product_code: String,
    pub batch_no: String,
    pub qty_on_hand: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RunReconciliationRequest {
    pub claim_id: Uuid,
    pub claim_token: Uuid,
    pub window_key: String,
    pub snapshot_at: DateTime<Utc>,
    pub items: Vec<ErpInventorySnapshotItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconciliationItem {
    pub id: Uuid,
    pub product_code: String,
    pub batch_no: String,
    pub wms_qty: i64,
    pub erp_qty: i64,
    pub difference_qty: i64,
    pub difference_type: String,
    pub resolution_status: String,
    pub stock_adjustment_order_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconciliationInventoryAllocation {
    pub inventory_batch_id: Uuid,
    pub quantity: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReconciliationRun {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub window_key: String,
    pub snapshot_at: DateTime<Utc>,
    pub matched_count: i32,
    pub wms_more_count: i32,
    pub erp_more_count: i32,
    pub items: Vec<ReconciliationItem>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationDisposition {
    WmsTruth,
    ErpTruth,
    KnownDifference,
}

#[derive(Clone, Debug)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReconciliationError {
    InvalidRequest,
    IdempotencyConflict,
    ClaimInvalid,
    ClaimExpired,
    Database(String),
    Serialize(String),
    Audit(String),
    StockAdjustment(String),
}

#[derive(Clone)]
pub struct PgReconciliationRepository {
    pub(crate) pool: PgPool,
}

#[derive(FromRow)]
struct WmsStockRow {
    product_code: String,
    batch_no: String,
    qty: i64,
}

impl PgReconciliationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run(
        &self,
        ctx: &AuthContext,
        req: RunReconciliationRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<ReconciliationRun>, ReconciliationError> {
        let req = normalize_request(req)?;
        let hash = request_hash(&(&req.window_key, req.snapshot_at, &req.items))?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        lock_idempotency(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let claim_id = self
            .validate_claim_for_run(
                &mut tx,
                ctx,
                req.claim_id,
                req.claim_token,
                &req.window_key,
                now,
            )
            .await?;
        lock_reconciliation_window(&mut tx, ctx.owner_id, &req.window_key).await?;
        if let Some(value) =
            load_existing_window(&mut tx, ctx.owner_id, &req.window_key, &hash).await?
        {
            self.complete_claim_for_run(&mut tx, ctx, claim_id, value.id, now)
                .await?;
            store_idempotency(
                &mut tx,
                ctx.owner_id,
                idempotency_key,
                &hash,
                "POST",
                "/api/v1/reconciliation/runs",
                "reconciliation_run",
                value.id.to_string(),
                &value,
                now,
            )
            .await?;
            tx.commit().await.map_err(db)?;
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }

        let wms_rows = sqlx::query_as::<_, WmsStockRow>(
            "SELECT product_code, batch_no, SUM(qty_on_hand)::BIGINT AS qty
               FROM inventory_batches
              WHERE owner_id = $1
              GROUP BY product_code, batch_no",
        )
        .bind(ctx.owner_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut quantities = std::collections::BTreeMap::new();
        for row in wms_rows {
            quantities.insert((row.product_code, row.batch_no), (row.qty, 0));
        }
        for item in &req.items {
            quantities
                .entry((item.product_code.trim().into(), item.batch_no.trim().into()))
                .or_insert((0, 0))
                .1 = item.qty_on_hand;
        }

        let run_id = Uuid::new_v4();
        let mut items = Vec::with_capacity(quantities.len());
        let mut counts = [0_i32; 3];
        for ((product_code, batch_no), (wms_qty, erp_qty)) in quantities {
            let difference_qty = wms_qty - erp_qty;
            let (difference_type, resolution_status, count_index) = match difference_qty.cmp(&0) {
                std::cmp::Ordering::Equal => ("matched", "matched", 0),
                std::cmp::Ordering::Greater => ("wms_more", "open", 1),
                std::cmp::Ordering::Less => ("erp_more", "open", 2),
            };
            counts[count_index] += 1;
            let item = ReconciliationItem {
                id: Uuid::new_v4(),
                product_code,
                batch_no,
                wms_qty,
                erp_qty,
                difference_qty,
                difference_type: difference_type.into(),
                resolution_status: resolution_status.into(),
                stock_adjustment_order_ids: Vec::new(),
                created_at: now,
            };
            items.push(item);
        }
        sqlx::query(
            "INSERT INTO reconciliation_runs
             (id, owner_id, window_key, request_hash, snapshot_at, matched_count,
              wms_more_count, erp_more_count, created_by, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(run_id)
        .bind(ctx.owner_id)
        .bind(&req.window_key)
        .bind(&hash)
        .bind(req.snapshot_at)
        .bind(counts[0])
        .bind(counts[1])
        .bind(counts[2])
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for item in &items {
            sqlx::query(
                "INSERT INTO reconciliation_items
                 (id, owner_id, run_id, product_code, batch_no, wms_qty, erp_qty,
                  difference_qty, difference_type, resolution_status, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$11)",
            )
            .bind(item.id)
            .bind(ctx.owner_id)
            .bind(run_id)
            .bind(&item.product_code)
            .bind(&item.batch_no)
            .bind(item.wms_qty)
            .bind(item.erp_qty)
            .bind(item.difference_qty)
            .bind(&item.difference_type)
            .bind(&item.resolution_status)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        if counts[1] + counts[2] > 0 {
            let content = format!(
                "库存对账发现 {} 条差异：WMS 多 {} 条，ERP 多 {} 条",
                counts[1] + counts[2],
                counts[1],
                counts[2]
            );
            sqlx::query(
                "INSERT INTO h4_notification_records
                 (id, owner_id, event_type, dedupe_key, recipient, channel, content,
                  content_summary, status, failure_reason, created_at, updated_at)
                 VALUES ($1,$2,'rc.reconciliation.difference',$3,'warehouse_manager','wechat',
                         $4,$4,'retrying','awaiting_wechat_delivery',$5,$5)",
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(run_id.to_string())
            .bind(&content)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let value = ReconciliationRun {
            id: run_id,
            owner_id: ctx.owner_id,
            window_key: req.window_key,
            snapshot_at: req.snapshot_at,
            matched_count: counts[0],
            wms_more_count: counts[1],
            erp_more_count: counts[2],
            items,
            created_at: now,
        };
        let mut audit = AuditWriteRequest::from_auth_context(
            ctx,
            "run_reconciliation",
            "M-RC",
            "reconciliation_run",
            run_id.to_string(),
            Some(AuditDiff::compute(
                json!({}),
                json!({
                    "window_key": value.window_key,
                    "matched_count": value.matched_count,
                    "wms_more_count": value.wms_more_count,
                    "erp_more_count": value.erp_more_count,
                }),
            )),
        );
        audit.occurred_at = now;
        append_event_in_tx(&mut tx, &audit)
            .await
            .map_err(|error| ReconciliationError::Audit(format!("{error:?}")))?;
        self.complete_claim_for_run(&mut tx, ctx, claim_id, run_id, now)
            .await?;
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/reconciliation/runs",
            "reconciliation_run",
            run_id.to_string(),
            &value,
            now,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(IdempotentMutation {
            value,
            replayed: false,
        })
    }

    pub async fn set_isolation(
        &self,
        ctx: &AuthContext,
        item_ids: &[Uuid],
        isolate: bool,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<i64>, ReconciliationError> {
        let mut item_ids = item_ids.to_vec();
        item_ids.sort_unstable();
        item_ids.dedup();
        if item_ids.is_empty() {
            return Err(ReconciliationError::InvalidRequest);
        }
        let hash = request_hash(&(&item_ids, isolate))?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        lock_idempotency(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let mut changed = 0_i64;
        for item_id in &item_ids {
            let item: Option<(String, String, String)> = sqlx::query_as(
                "SELECT product_code, batch_no, resolution_status
                   FROM reconciliation_items
                  WHERE owner_id = $1 AND id = $2
                  FOR UPDATE",
            )
            .bind(ctx.owner_id)
            .bind(item_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some((product_code, batch_no, status)) = item else {
                return Err(ReconciliationError::InvalidRequest);
            };
            if status == "matched"
                || (isolate && status != "open")
                || (!isolate && !matches!(status.as_str(), "resolved" | "known_difference"))
            {
                return Err(ReconciliationError::InvalidRequest);
            }
            if isolate {
                changed += acquire_item_locks(
                    &mut tx,
                    ctx.owner_id,
                    *item_id,
                    &product_code,
                    &batch_no,
                    now,
                )
                .await?;
            } else {
                changed += release_item_locks(&mut tx, ctx.owner_id, *item_id, now).await?;
            }
        }
        append_reconciliation_audit(
            &mut tx,
            ctx,
            if isolate {
                "isolate_reconciliation_items"
            } else {
                "release_reconciliation_items"
            },
            item_ids
                .iter()
                .map(Uuid::to_string)
                .collect::<Vec<_>>()
                .join(","),
            json!({"isolate": isolate, "changed_batches": changed}),
            now,
        )
        .await?;
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/reconciliation/items/isolation",
            "reconciliation_item",
            item_ids
                .iter()
                .map(Uuid::to_string)
                .collect::<Vec<_>>()
                .join(","),
            &changed,
            now,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(IdempotentMutation {
            value: changed,
            replayed: false,
        })
    }

    pub async fn resolve(
        &self,
        ctx: &AuthContext,
        item_id: Uuid,
        disposition: ReconciliationDisposition,
        mut allocations: Vec<ReconciliationInventoryAllocation>,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentMutation<ReconciliationItem>, ReconciliationError> {
        allocations.sort_by_key(|allocation| allocation.inventory_batch_id);
        if allocations
            .iter()
            .any(|allocation| allocation.quantity <= 0)
            || allocations
                .windows(2)
                .any(|pair| pair[0].inventory_batch_id == pair[1].inventory_batch_id)
            || matches!(disposition, ReconciliationDisposition::ErpTruth) == allocations.is_empty()
        {
            return Err(ReconciliationError::InvalidRequest);
        }
        let disposition_name = match disposition {
            ReconciliationDisposition::WmsTruth => "wms_truth",
            ReconciliationDisposition::KnownDifference => "known_difference",
            ReconciliationDisposition::ErpTruth => "erp_truth",
        };
        let hash = request_hash(&(item_id, disposition_name, &allocations))?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        lock_idempotency(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentMutation {
                value,
                replayed: true,
            });
        }
        let mut item = load_item_for_update(&mut tx, ctx.owner_id, item_id).await?;
        if item.resolution_status != "open" {
            return Err(ReconciliationError::InvalidRequest);
        }
        if !allocations.is_empty()
            && allocations.iter().try_fold(0_i64, |sum, allocation| {
                sum.checked_add(allocation.quantity)
            }) != Some(item.difference_qty.abs())
        {
            return Err(ReconciliationError::InvalidRequest);
        }
        let mut stock_adjustment_order_ids = Vec::with_capacity(allocations.len());
        for allocation in &allocations {
            let order_id = self
                .create_stock_adjustment_for_erp_truth(
                    &mut tx,
                    ctx,
                    &item,
                    allocation.inventory_batch_id,
                    allocation.quantity,
                    now,
                )
                .await?;
            sqlx::query(
                "INSERT INTO reconciliation_item_adjustments
                 (item_id, owner_id, inventory_batch_id, quantity, adjustment_order_id, created_at)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(item.id)
            .bind(ctx.owner_id)
            .bind(allocation.inventory_batch_id)
            .bind(allocation.quantity)
            .bind(order_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            stock_adjustment_order_ids.push(order_id);
        }
        let released_batches = if matches!(disposition, ReconciliationDisposition::KnownDifference)
        {
            release_item_locks(&mut tx, ctx.owner_id, item_id, now).await?
        } else {
            0
        };
        item.resolution_status = match disposition {
            ReconciliationDisposition::KnownDifference => "known_difference",
            ReconciliationDisposition::WmsTruth => "erp_feedback_pending",
            ReconciliationDisposition::ErpTruth => "adjustment_pending",
        }
        .into();
        item.stock_adjustment_order_ids = stock_adjustment_order_ids;
        sqlx::query(
            "UPDATE reconciliation_items
                SET resolution_status = $3, disposition = $4, resolved_by = $5,
                    resolved_at = CASE WHEN $3 = 'known_difference' THEN $6 ELSE NULL END,
                    updated_at = $6
              WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(item_id)
        .bind(&item.resolution_status)
        .bind(disposition_name)
        .bind(ctx.user_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if matches!(disposition, ReconciliationDisposition::WmsTruth) {
            sqlx::query(
                "INSERT INTO reconciliation_erp_feedback_outbox
                 (id, owner_id, recon_doc_no, payload, created_at, updated_at)
                 VALUES ($1,$2,$3,$4,$5,$5)",
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(item_id.to_string())
            .bind(json!({
                "reconciliation_item_id": item_id,
                "product_code": item.product_code,
                "batch_no": item.batch_no,
                "wms_qty": item.wms_qty,
                "erp_qty": item.erp_qty,
                "difference_qty": item.difference_qty,
                "disposition": disposition_name,
            }))
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        append_reconciliation_audit(
            &mut tx,
            ctx,
            "resolve_reconciliation_item",
            item_id.to_string(),
            json!({
                "disposition": disposition_name,
                "status": item.resolution_status,
                "released_batches": released_batches,
            }),
            now,
        )
        .await?;
        store_idempotency(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/reconciliation/items/{id}/resolve",
            "reconciliation_item",
            item_id.to_string(),
            &item,
            now,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(IdempotentMutation {
            value: item,
            replayed: false,
        })
    }

    async fn create_stock_adjustment_for_erp_truth(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        item: &ReconciliationItem,
        batch_id: Uuid,
        quantity: i64,
        now: DateTime<Utc>,
    ) -> Result<Uuid, ReconciliationError> {
        let warehouse_id: Uuid = sqlx::query_scalar(
            "SELECT location.warehouse_id
               FROM inventory_batches batch
               JOIN warehouse_locations location
                 ON location.owner_id = batch.owner_id
                AND location.id = batch.location_id
              WHERE batch.owner_id = $1
                AND batch.id = $2
                AND batch.product_code = $3
                AND batch.batch_no = $4
              FOR UPDATE",
        )
        .bind(ctx.owner_id)
        .bind(batch_id)
        .bind(&item.product_code)
        .bind(&item.batch_no)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?
        .ok_or(ReconciliationError::InvalidRequest)?;
        let repository = PgStockAdjustmentRepository::new(self.pool.clone());
        let external_ref = format!("reconciliation:{}:{batch_id}", item.id);
        let derived_key = format!("rc-msa:{}:{batch_id}", item.id);
        let order_id = if item.difference_qty > 0 {
            repository
                .create_loss_order_in_tx(
                    tx,
                    ctx,
                    CreateStockLossOrderRequest {
                        warehouse_id,
                        batch_id,
                        quantity,
                        reason: StockLossReason::InventoryLoss,
                        recall_id: None,
                        source: StockAdjustmentSource::Erp,
                        external_ref: Some(external_ref),
                        requires_quality_approval: true,
                    },
                    now,
                    &derived_key,
                )
                .await
                .map_err(map_stock_adjustment)?
                .value
                .id
        } else if item.difference_qty < 0 {
            repository
                .create_surplus_order_in_tx(
                    tx,
                    ctx,
                    CreateStockSurplusOrderRequest {
                        warehouse_id,
                        batch_id,
                        quantity,
                        reason: StockSurplusReason::SystemDifferenceCorrection,
                        source: StockAdjustmentSource::Erp,
                        external_ref: Some(external_ref),
                        requires_quality_approval: true,
                    },
                    now,
                    &derived_key,
                )
                .await
                .map_err(map_stock_adjustment)?
                .value
                .id
        } else {
            return Err(ReconciliationError::InvalidRequest);
        };
        Ok(order_id)
    }
}

async fn load_item_for_update(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    item_id: Uuid,
) -> Result<ReconciliationItem, ReconciliationError> {
    sqlx::query_as::<_, ReconciliationItemRow>(
        "SELECT item.id, item.product_code, item.batch_no, item.wms_qty, item.erp_qty,
                item.difference_qty, item.difference_type, item.resolution_status,
                ARRAY(SELECT link.adjustment_order_id
                        FROM reconciliation_item_adjustments link
                       WHERE link.item_id = item.id
                       ORDER BY link.adjustment_order_id) AS stock_adjustment_order_ids,
                item.created_at
           FROM reconciliation_items item
          WHERE item.owner_id = $1 AND item.id = $2
          FOR UPDATE",
    )
    .bind(owner_id)
    .bind(item_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .map(Into::into)
    .ok_or(ReconciliationError::InvalidRequest)
}

#[derive(FromRow)]
struct ReconciliationItemRow {
    id: Uuid,
    product_code: String,
    batch_no: String,
    wms_qty: i64,
    erp_qty: i64,
    difference_qty: i64,
    difference_type: String,
    resolution_status: String,
    stock_adjustment_order_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
}

impl From<ReconciliationItemRow> for ReconciliationItem {
    fn from(row: ReconciliationItemRow) -> Self {
        Self {
            id: row.id,
            product_code: row.product_code,
            batch_no: row.batch_no,
            wms_qty: row.wms_qty,
            erp_qty: row.erp_qty,
            difference_qty: row.difference_qty,
            difference_type: row.difference_type,
            resolution_status: row.resolution_status,
            stock_adjustment_order_ids: row.stock_adjustment_order_ids,
            created_at: row.created_at,
        }
    }
}

async fn append_reconciliation_audit(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    action: &str,
    resource_id: String,
    after: serde_json::Value,
    now: DateTime<Utc>,
) -> Result<(), ReconciliationError> {
    let mut audit = AuditWriteRequest::from_auth_context(
        ctx,
        action,
        "M-RC",
        "reconciliation_item",
        resource_id,
        Some(AuditDiff::compute(json!({}), after)),
    );
    audit.occurred_at = now;
    append_event_in_tx(tx, &audit)
        .await
        .map_err(|error| ReconciliationError::Audit(format!("{error:?}")))?;
    Ok(())
}
