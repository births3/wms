use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{
    CreateStockSurplusOrderRequest, DualPersonPolicy, StockAdjustmentSource, StockAdjustmentStatus,
    StockSurplusOrder,
};

use super::*;

mod persistence;

use persistence::*;

const STOCK_SURPLUS_DOCUMENT_TYPE: &str = "stock_surplus";

impl PgStockAdjustmentRepository {
    pub async fn get_surplus_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
    ) -> Result<StockSurplusOrder, StockAdjustmentError> {
        load_surplus_order_from_pool(&self.pool, ctx.owner_id, order_id).await
    }

    pub async fn create_surplus_order(
        &self,
        ctx: &AuthContext,
        request: CreateStockSurplusOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockSurplusMutation, StockAdjustmentError> {
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        let outcome = self
            .create_surplus_order_in_tx(&mut tx, ctx, request, now, idempotency_key)
            .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(outcome)
    }

    pub(crate) async fn create_surplus_order_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ctx: &AuthContext,
        request: CreateStockSurplusOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockSurplusMutation, StockAdjustmentError> {
        validate_surplus_create_request(&request)?;
        let hash = request_hash(&serde_json::json!({"action":"create_surplus","request":request}))?;
        lock_idempotency_key(tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentStockSurplusMutation {
                value,
                replayed: true,
            });
        }
        let external_ref = normalize_optional(request.external_ref.clone());
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
                if row.adjustment_type != "surplus" {
                    return Err(StockAdjustmentError::IdempotencyConflict);
                }
                let order = row_to_surplus_domain(row)?;
                if !same_surplus_create_request(&order, &request, external_ref.as_deref()) {
                    return Err(StockAdjustmentError::IdempotencyConflict);
                }
                store_idempotency_success(
                    tx,
                    ctx.owner_id,
                    idempotency_key,
                    &hash,
                    "POST",
                    "/api/v1/stock-adjustments/surplus-orders",
                    &order.id.to_string(),
                    &order,
                    now,
                )
                .await?;
                return Ok(IdempotentStockSurplusMutation {
                    value: order,
                    replayed: true,
                });
            }
        }

        let batch: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT batch.product_code, batch.batch_no
              FROM inventory_batches batch
              JOIN products product
                ON product.owner_id = batch.owner_id
               AND product.product_code = batch.product_code
               AND product.status = 'active'
              JOIN warehouses warehouse
                ON warehouse.owner_id = batch.owner_id
               AND warehouse.id = $2
               AND warehouse.status = 'active'
              JOIN warehouse_locations location
                ON location.owner_id = batch.owner_id
               AND location.id = batch.location_id
               AND location.warehouse_id = warehouse.id
             WHERE batch.owner_id = $1 AND batch.id = $3
            "#,
        )
        .bind(ctx.owner_id)
        .bind(request.warehouse_id)
        .bind(request.batch_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_database_error)?;
        let (product_code, batch_no) = batch.ok_or(StockAdjustmentError::NotFound)?;
        let order_id = Uuid::new_v4();
        let order_no = PgDocumentNumberingService::new()
            .generate_in_tx(
                tx,
                ctx,
                GenerateDocumentNumberRequest {
                    document_type: STOCK_SURPLUS_DOCUMENT_TYPE.to_string(),
                    idempotency_key: format!("msa-stock-surplus:{order_id}"),
                    source_module: "M-SA".to_string(),
                    source_document_id: Some(order_id),
                },
                now,
            )
            .await
            .map_err(|error| StockAdjustmentError::DocumentNumbering(format!("{error:?}")))?
            .value
            .generated_no;
        let status = if request.requires_quality_approval {
            StockAdjustmentStatus::PendingApproval
        } else {
            StockAdjustmentStatus::PendingExecution
        };
        let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
            r#"
            INSERT INTO stock_adjustment_orders (
                id, owner_id, warehouse_id, order_no, adjustment_type, batch_id,
                product_code, batch_no, quantity, reason_code, source, external_ref,
                status, requires_quality_approval, created_by, created_at, updated_at
            )
            VALUES ($1,$2,$3,$4,'surplus',$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$15)
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
        .bind(request.source.as_str())
        .bind(external_ref)
        .bind(status.as_str())
        .bind(request.requires_quality_approval)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_database_error)?;
        let order = row_to_surplus_domain(row)?;
        append_order_audit(
            tx,
            ctx,
            "create_stock_surplus_order",
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
            "/api/v1/stock-adjustments/surplus-orders",
            &order.id.to_string(),
            &order,
            now,
        )
        .await?;
        Ok(IdempotentStockSurplusMutation {
            value: order,
            replayed: false,
        })
    }

    pub async fn start_surplus_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockSurplusMutation, StockAdjustmentError> {
        let hash =
            request_hash(&serde_json::json!({"action":"start_surplus","order_id":order_id}))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentStockSurplusMutation {
                value,
                replayed: true,
            });
        }
        let before = load_surplus_order_for_update(&mut tx, ctx.owner_id, order_id).await?;
        ensure_surplus_status(&before, StockAdjustmentStatus::PendingExecution)?;
        ensure_custodian(&mut tx, ctx.owner_id, ctx.user_id).await?;
        let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
            "UPDATE stock_adjustment_orders SET status = 'in_progress', first_operator_id = $3, started_at = $4, updated_at = $4, version = version + 1 WHERE owner_id = $1 AND id = $2 AND adjustment_type = 'surplus' RETURNING {}",
            order_columns()
        ))
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(ctx.user_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let order = row_to_surplus_domain(row)?;
        append_order_audit(
            &mut tx,
            ctx,
            "start_stock_surplus_order",
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
            "/api/v1/stock-adjustments/surplus-orders/{id}/start",
            &order.id.to_string(),
            &order,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentStockSurplusMutation {
            value: order,
            replayed: false,
        })
    }

    pub async fn record_surplus_quality_approval(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        quality_liaison_id: &str,
        approved: bool,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockSurplusMutation, StockAdjustmentError> {
        let quality_liaison_id = quality_liaison_id.trim();
        if quality_liaison_id.is_empty() {
            return Err(StockAdjustmentError::InvalidRequest);
        }
        let hash = request_hash(&serde_json::json!({
            "action":"surplus_quality_approval",
            "order_id":order_id,
            "quality_liaison_id":quality_liaison_id,
            "approved":approved
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentStockSurplusMutation {
                value,
                replayed: true,
            });
        }
        let before = load_surplus_order_for_update(&mut tx, ctx.owner_id, order_id).await?;
        ensure_surplus_status(&before, StockAdjustmentStatus::PendingApproval)?;
        let next_status = if approved {
            StockAdjustmentStatus::PendingExecution
        } else {
            StockAdjustmentStatus::Rejected
        };
        let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
            "UPDATE stock_adjustment_orders SET status = $3, quality_liaison_id = $4, updated_at = $5, version = version + 1 WHERE owner_id = $1 AND id = $2 AND adjustment_type = 'surplus' RETURNING {}",
            order_columns()
        ))
        .bind(ctx.owner_id)
        .bind(order_id)
        .bind(next_status.as_str())
        .bind(quality_liaison_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_database_error)?;
        let order = row_to_surplus_domain(row)?;
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
            "record_stock_surplus_quality_approval",
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
            "/api/v1/stock-adjustments/surplus-orders/{id}/quality-approval",
            &order.id.to_string(),
            &order,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentStockSurplusMutation {
            value: order,
            replayed: false,
        })
    }

    pub async fn execute_surplus_order(
        &self,
        ctx: &AuthContext,
        order_id: Uuid,
        second_operator_id: Option<Uuid>,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<IdempotentStockSurplusMutation, StockAdjustmentError> {
        let hash = request_hash(&serde_json::json!({
            "action":"execute_surplus",
            "order_id":order_id,
            "second_operator_id":second_operator_id
        }))?;
        let mut tx = self.pool.begin().await.map_err(map_database_error)?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(value) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &hash, now).await?
        {
            return Ok(IdempotentStockSurplusMutation {
                value,
                replayed: true,
            });
        }
        let before = load_surplus_order_for_update(&mut tx, ctx.owner_id, order_id).await?;
        ensure_surplus_status(&before, StockAdjustmentStatus::InProgress)?;
        if before.first_operator_id != Some(ctx.user_id) {
            return Err(StockAdjustmentError::DifferentFirstOperator);
        }
        let resolved = crate::dual_person_policy::resolve_for_product_codes_in_tx(
            &mut tx,
            ctx.owner_id,
            before.warehouse_id,
            std::slice::from_ref(&before.product_code),
            "报溢",
            "报溢执行",
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
            Some(
                crate::dual_person_policy::approved_dual_person_record_in_tx(
                    &mut tx,
                    ctx.owner_id,
                    &order_id.to_string(),
                )
                .await
                .map_err(|error| {
                    StockAdjustmentError::Database(format!("H4 审批查询失败: {error:?}"))
                })?
                .ok_or(StockAdjustmentError::DualPersonApprovalRequired)?,
            )
        } else {
            None
        };
        let (approval_source, approval_id) = before.quality_liaison_id.as_ref().map_or_else(
            || ("报损报溢单", before.id.to_string()),
            |id| ("质量联系单", id.clone()),
        );
        crate::inventory::add_for_stock_surplus_in_tx(
            &mut tx,
            ctx.owner_id,
            before.batch_id,
            before.warehouse_id,
            before.quantity,
            before.id,
            approval_source,
            &approval_id,
            now,
        )
        .await
        .map_err(map_database_error)?
        .ok_or(StockAdjustmentError::InvalidPutawayTarget)?;
        sqlx::query(
            r#"
            INSERT INTO stock_adjustment_execution_records (
                id, owner_id, order_id, process_code, node_code, policy,
                source_rule_id, first_operator_id, second_operator_id,
                approval_record_id, quantity, executed_at, created_at
            ) VALUES ($1,$2,$3,'报溢','报溢执行',$4,$5,$6,$7,$8,$9,$10,$10)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(order_id)
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
        let row = sqlx::query_as::<_, StockLossOrderRow>(&format!(
            "UPDATE stock_adjustment_orders SET status = 'completed', policy = $3, source_rule_id = $4, second_operator_id = $5, approval_record_id = $6, completed_at = $7, updated_at = $7, version = version + 1 WHERE owner_id = $1 AND id = $2 AND adjustment_type = 'surplus' RETURNING {}",
            order_columns()
        ))
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
        let order = row_to_surplus_domain(row)?;
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
            ) VALUES ($1,$2,$3,'stock_surplus_completed',$4,'pending',0,$5,$5,$5)
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
            "execute_stock_surplus_order",
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
            "/api/v1/stock-adjustments/surplus-orders/{id}/execute",
            &order.id.to_string(),
            &order,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_database_error)?;
        Ok(IdempotentStockSurplusMutation {
            value: order,
            replayed: false,
        })
    }
}

fn validate_surplus_create_request(
    request: &CreateStockSurplusOrderRequest,
) -> Result<(), StockAdjustmentError> {
    if request.quantity <= 0
        || request.source == StockAdjustmentSource::Erp
            && request
                .external_ref
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
    {
        return Err(StockAdjustmentError::InvalidRequest);
    }
    Ok(())
}

fn same_surplus_create_request(
    order: &StockSurplusOrder,
    request: &CreateStockSurplusOrderRequest,
    external_ref: Option<&str>,
) -> bool {
    order.warehouse_id == request.warehouse_id
        && order.batch_id == request.batch_id
        && order.quantity == request.quantity
        && order.reason == request.reason
        && order.source == request.source
        && order.external_ref.as_deref() == external_ref
        && order.requires_quality_approval == request.requires_quality_approval
}

fn ensure_surplus_status(
    order: &StockSurplusOrder,
    expected: StockAdjustmentStatus,
) -> Result<(), StockAdjustmentError> {
    if order.status == expected {
        Ok(())
    } else {
        Err(StockAdjustmentError::InvalidStatus {
            expected: expected.as_str().to_string(),
            actual: order.status.as_str().to_string(),
        })
    }
}
