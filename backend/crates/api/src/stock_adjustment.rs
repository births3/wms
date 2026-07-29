use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CreateStockLossOrderRequest, DualPersonPolicy, StockAdjustmentSource, StockAdjustmentStatus,
    StockLossOrder, StockLossReason,
};

use crate::{
    auth::AuthContext,
    document_numbering::{GenerateDocumentNumberRequest, PgDocumentNumberingService},
};

mod persistence;
pub(crate) mod quality_liaison;
mod surplus;
mod validation;

use persistence::*;
use validation::*;

const STOCK_LOSS_DOCUMENT_TYPE: &str = "stock_loss";

#[derive(Clone, Debug)]
pub struct PgStockAdjustmentRepository {
    pool: PgPool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StockAdjustmentError {
    NotFound,
    CrossOwner,
    InvalidRequest,
    InvalidStatus { expected: String, actual: String },
    QuantityExceeded,
    InvalidPutawayTarget,
    MissingSecondOperator,
    SameOperator,
    UnqualifiedOperator,
    DifferentFirstOperator,
    DualPersonApprovalRequired,
    IdempotencyConflict,
    DocumentNumbering(String),
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentStockAdjustmentMutation {
    pub value: StockLossOrder,
    pub replayed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentStockSurplusMutation {
    pub value: wms_domain::StockSurplusOrder,
    pub replayed: bool,
}

#[derive(Clone, Debug, FromRow)]
struct StockLossOrderRow {
    id: Uuid,
    owner_id: Uuid,
    warehouse_id: Uuid,
    order_no: String,
    adjustment_type: String,
    batch_id: Uuid,
    product_code: String,
    batch_no: String,
    quantity: i64,
    reason_code: String,
    recall_id: Option<String>,
    source: String,
    external_ref: Option<String>,
    status: String,
    requires_quality_approval: bool,
    quality_liaison_id: Option<String>,
    policy: Option<String>,
    source_rule_id: Option<Uuid>,
    first_operator_id: Option<Uuid>,
    second_operator_id: Option<Uuid>,
    approval_record_id: Option<Uuid>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PgStockAdjustmentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_loss_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
    ) -> Result<StockLossOrder, StockAdjustmentError> {
        load_order_from_pool(&self.pool, ctx.owner_id, order_id).await
    }

    pub async fn create_loss_order(
        &self,
        ctx: &AuthContext,
        request: CreateStockLossOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockAdjustmentMutation, StockAdjustmentError> {
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        let outcome = self
            .create_loss_order_in_tx(&mut tx, ctx, request, now, idempotency_key)
            .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(outcome)
    }

    pub(crate) async fn create_loss_order_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        request: CreateStockLossOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockAdjustmentMutation, StockAdjustmentError> {
        validate_create_request(&request)?;
        let hash = request_hash(&serde_json::json!({"action":"create_loss","request":request}))?;
        lock_idempotency_key(tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentStockAdjustmentMutation {
                value,
                replayed: true,
            });
        }
        let recall_id = normalize_optional(request.recall_id.clone());
        let external_ref = normalize_optional(request.external_ref.clone());
        let requires_quality_approval =
            request.requires_quality_approval || request.reason.is_destruction();
        if request.source == StockAdjustmentSource::Erp {
            let external_ref_value = external_ref
                .as_deref()
                .ok_or(StockAdjustmentError::InvalidRequest)?;
            lock_idempotency_key(
                tx,
                ctx.owner_id,
                &format!("erp-reference:{external_ref_value}"),
            )
            .await?;
            let existing = sqlx::query_as::<_, StockLossOrderRow>(&format!(
                "SELECT {} FROM stock_adjustment_orders WHERE owner_id = $1 AND source = 'erp' AND external_ref = $2 FOR UPDATE",
                order_columns()
            ))
            .bind(ctx.owner_id)
            .bind(external_ref_value)
            .fetch_optional(&mut **tx)
            .await
            .map_err(map_database_error)?;
            if let Some(row) = existing {
                if row.adjustment_type != "loss" {
                    return Err(StockAdjustmentError::IdempotencyConflict);
                }
                let order = row_to_domain(row)?;
                if !same_create_request(
                    &order,
                    &request,
                    recall_id.as_deref(),
                    external_ref.as_deref(),
                    requires_quality_approval,
                ) {
                    return Err(StockAdjustmentError::IdempotencyConflict);
                }
                store_idempotency_success(
                    tx,
                    ctx.owner_id,
                    idempotency_key,
                    &hash,
                    "POST",
                    "/api/v1/stock-adjustments/loss-orders",
                    &order.id.to_string(),
                    &order,
                    now,
                )
                .await?;
                return Ok(IdempotentStockAdjustmentMutation {
                    value: order,
                    replayed: true,
                });
            }
        }

        let warehouse_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM warehouses WHERE owner_id = $1 AND id = $2 AND status = 'active')",
        )
        .bind(ctx.owner_id)
        .bind(request.warehouse_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_database_error)?;
        if !warehouse_exists {
            return Err(StockAdjustmentError::NotFound);
        }
        let batch: Option<(String, String, i64)> = sqlx::query_as(
            "SELECT product_code, batch_no, qty_on_hand - qty_locked FROM inventory_batches WHERE owner_id = $1 AND id = $2",
        )
        .bind(ctx.owner_id)
        .bind(request.batch_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_database_error)?;
        let (product_code, batch_no, available_quantity) =
            batch.ok_or(StockAdjustmentError::NotFound)?;
        if request.quantity > available_quantity {
            return Err(StockAdjustmentError::QuantityExceeded);
        }
        let order_id = Uuid::new_v4();
        let order_no = PgDocumentNumberingService::new()
            .generate_in_tx(
                tx,
                ctx,
                GenerateDocumentNumberRequest {
                    document_type: STOCK_LOSS_DOCUMENT_TYPE.to_string(),
                    idempotency_key: format!("msa-stock-loss:{order_id}"),
                    source_module: "M-SA".to_string(),
                    source_document_id: Some(order_id),
                },
                now,
            )
            .await
            .map_err(|error| StockAdjustmentError::DocumentNumbering(format!("{error:?}")))?
            .value
            .generated_no;
        let status = if requires_quality_approval {
            StockAdjustmentStatus::PendingApproval
        } else {
            StockAdjustmentStatus::PendingExecution
        };
        let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
            r#"
                INSERT INTO stock_adjustment_orders (
                    id, owner_id, warehouse_id, order_no, adjustment_type, batch_id,
                    product_code, batch_no, quantity, reason_code, recall_id, source,
                    external_ref, status, requires_quality_approval, created_by,
                    created_at, updated_at
                )
                VALUES ($1,$2,$3,$4,'loss',$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$16)
                RETURNING {}
                "#,
            order_columns()
        ))
        .bind(order_id)
        .bind(ctx.owner_id)
        .bind(request.warehouse_id)
        .bind(order_no)
        .bind(request.batch_id)
        .bind(product_code)
        .bind(batch_no)
        .bind(request.quantity)
        .bind(request.reason.as_str())
        .bind(recall_id)
        .bind(request.source.as_str())
        .bind(external_ref)
        .bind(status.as_str())
        .bind(requires_quality_approval)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_database_error)?;
        let order = row_to_domain(row)?;
        append_order_audit(
            tx,
            ctx,
            "create_stock_loss_order",
            order.id,
            None,
            &order,
            now,
        )
        .await?;
        store_idempotency_success(
            tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/stock-adjustments/loss-orders",
            &order.id.to_string(),
            &order,
            now,
        )
        .await?;
        Ok(IdempotentStockAdjustmentMutation {
            value: order,
            replayed: false,
        })
    }

    pub async fn record_quality_approval(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        quality_liaison_id: &str,
        approved: bool,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockAdjustmentMutation, StockAdjustmentError> {
        let quality_liaison_id = quality_liaison_id.trim();
        if quality_liaison_id.is_empty() {
            return Err(StockAdjustmentError::InvalidRequest);
        }
        let hash = request_hash(&serde_json::json!({
            "action":"quality_approval",
            "order_id":order_id,
            "quality_liaison_id":quality_liaison_id,
            "approved":approved
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentStockAdjustmentMutation {
                value,
                replayed: true,
            });
        }
        let before = load_order_for_update(&mut tx, ctx.owner_id, order_id).await?;
        ensure_status(&before, StockAdjustmentStatus::PendingApproval)?;
        let next_status = if approved {
            StockAdjustmentStatus::PendingExecution
        } else {
            StockAdjustmentStatus::Rejected
        };
        let order = update_status_and_liaison(
            &mut tx,
            ctx.owner_id,
            order_id,
            next_status,
            quality_liaison_id,
            now,
        )
        .await?;
        crate::reconciliation::advance_from_stock_adjustment_in_tx(
            &mut tx,
            ctx,
            order.id,
            order.status.as_str(),
            now,
        )
        .await
        .map_err(|error| StockAdjustmentError::Database(format!("M-RC 状态推进失败: {error:?}")))?;
        append_order_audit(
            &mut tx,
            ctx,
            "record_stock_loss_quality_approval",
            order.id,
            Some(&before),
            &order,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/stock-adjustments/loss-orders/{id}/quality-approval",
            &order.id.to_string(),
            &order,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentStockAdjustmentMutation {
            value: order,
            replayed: false,
        })
    }

    pub async fn start_loss_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockAdjustmentMutation, StockAdjustmentError> {
        let hash = request_hash(&serde_json::json!({"action":"start_loss","order_id":order_id}))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentStockAdjustmentMutation {
                value,
                replayed: true,
            });
        }
        let before = load_order_for_update(&mut tx, ctx.owner_id, order_id).await?;
        ensure_status(&before, StockAdjustmentStatus::PendingExecution)?;
        ensure_custodian(&mut tx, ctx.owner_id, ctx.user_id).await?;
        let row = sqlx::query_as::<_, StockLossOrderRow>(
            &format!(
                "UPDATE stock_adjustment_orders SET status = 'in_progress', first_operator_id = $3, started_at = $4, updated_at = $4, version = version + 1 WHERE owner_id = $1 AND id = $2 RETURNING {}",
                order_columns()
            ),
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let order = row_to_domain(row)?;
        append_order_audit(
            &mut tx,
            ctx,
            "start_stock_loss_order",
            order.id,
            Some(&before),
            &order,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/stock-adjustments/loss-orders/{id}/start",
            &order.id.to_string(),
            &order,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentStockAdjustmentMutation {
            value: order,
            replayed: false,
        })
    }

    pub async fn execute_loss_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        second_operator_id: Option<Uuid>,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockAdjustmentMutation, StockAdjustmentError> {
        let hash = request_hash(&serde_json::json!({
            "action":"execute_loss",
            "order_id":order_id,
            "second_operator_id":second_operator_id
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentStockAdjustmentMutation {
                value,
                replayed: true,
            });
        }
        let before = load_order_for_update(&mut tx, ctx.owner_id, order_id).await?;
        ensure_status(&before, StockAdjustmentStatus::InProgress)?;
        if before.first_operator_id != Some(ctx.user_id) {
            return Err(StockAdjustmentError::DifferentFirstOperator);
        }
        let (process, node) = if before.reason.is_destruction() {
            ("销毁", "销毁执行")
        } else {
            ("报损", "报损执行")
        };
        let resolved = crate::dual_person_policy::resolve_for_product_codes_in_tx(
            &mut tx,
            ctx.owner_id,
            before.warehouse_id,
            std::slice::from_ref(&before.product_code),
            process,
            node,
        )
        .await
        .map_err(|error| StockAdjustmentError::Database(format!("M-VR 策略解析失败: {error:?}")))?;
        let second_operator_id = validate_second_operator(
            &mut tx,
            ctx.owner_id,
            ctx.user_id,
            second_operator_id,
            resolved.policy,
        )
        .await?;
        let approval_record_id = if resolved.policy == DualPersonPolicy::DualScanWithApproval {
            crate::dual_person_policy::approved_dual_person_record_in_tx(
                &mut tx,
                ctx.owner_id,
                &order_id.to_string(),
            )
            .await
            .map_err(|error| StockAdjustmentError::Database(format!("H4 审批查询失败: {error:?}")))?
            .ok_or(StockAdjustmentError::DualPersonApprovalRequired)?
            .into()
        } else {
            None
        };
        let (approval_source, approval_id) = before.quality_liaison_id.as_ref().map_or_else(
            || ("报损报溢单", before.id.to_string()),
            |id| ("质量联系单", id.clone()),
        );
        crate::inventory::deduct_for_stock_loss_in_tx(
            &mut tx,
            ctx.owner_id,
            before.batch_id,
            before.quantity,
            before.id,
            approval_source,
            &approval_id,
            before.reason == StockLossReason::RecallDestruction,
            now,
        )
        .await
        .map_err(map_database_error)?
        .ok_or(StockAdjustmentError::QuantityExceeded)?;
        sqlx::query(
            r#"
            INSERT INTO stock_adjustment_execution_records (
                id, owner_id, order_id, process_code, node_code, policy,
                source_rule_id, first_operator_id, second_operator_id,
                approval_record_id, quantity, executed_at, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$12)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(process)
        .bind(node)
        .bind(resolved.policy.as_str())
        .bind(resolved.source_rule_id)
        .bind(ctx.user_id)
        .bind(second_operator_id)
        .bind(approval_record_id)
        .bind(before.quantity)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let row = sqlx::query_as::<_, StockLossOrderRow>(
            &format!(
                "UPDATE stock_adjustment_orders SET status = 'completed', policy = $3, source_rule_id = $4, second_operator_id = $5, approval_record_id = $6, completed_at = $7, updated_at = $7, version = version + 1 WHERE owner_id = $1 AND id = $2 RETURNING {}",
                order_columns()
            ),
        )
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(resolved.policy.as_str())
        .bind(resolved.source_rule_id)
        .bind(second_operator_id)
        .bind(approval_record_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let order = row_to_domain(row)?;
        crate::reconciliation::advance_from_stock_adjustment_in_tx(
            &mut tx,
            ctx,
            order.id,
            order.status.as_str(),
            now,
        )
        .await
        .map_err(|error| StockAdjustmentError::Database(format!("M-RC 状态推进失败: {error:?}")))?;
        sqlx::query(
            r#"
            INSERT INTO stock_adjustment_erp_feedback_outbox (
                id, owner_id, order_id, event_type, payload, status,
                attempt_count, next_attempt_at, created_at, updated_at
            ) VALUES ($1,$2,$3,'stock_loss_completed',$4,'pending',0,$5,$5,$5)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(json_value(&order)?)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_database_error)?;
        append_order_audit(
            &mut tx,
            ctx,
            "execute_stock_loss_order",
            order.id,
            Some(&before),
            &order,
            now,
        )
        .await?;
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &hash,
            "POST",
            "/api/v1/stock-adjustments/loss-orders/{id}/execute",
            &order.id.to_string(),
            &order,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentStockAdjustmentMutation {
            value: order,
            replayed: false,
        })
    }
}
