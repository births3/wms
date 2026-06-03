//! Wave 3 PostgreSQL repository, aligned with ADR-0034.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    BillingAccount, BillingContract, BillingRule, ChangeInventoryStatusRequest,
    CreateBillingAccountRequest, CreateBillingContractRequest, CreateBillingRuleRequest,
    CreateReceivingOrderRequest, IngestTemperatureExcursionRequest,
    IngestTemperatureReadingRequest, InspectReceivingOrderRequest, InspectionSignatureRecord,
    InventoryBatch, InventoryMovement, PutawayRecord, PutawayRequest, ReceiveReceivingOrderRequest,
    ReceivingInspectionRecord, ReceivingOrder, ReceivingOrderLine, ReceivingOrderReceipt,
    SignInspectionRequest, TemperatureExcursionEvent, TemperatureReading,
};

use crate::{
    audit::{append_event_in_tx, AuditWriteRequest},
    auth::AuthContext,
    inventory::{allowed_transition, STATUS_QUALIFIED},
};

#[derive(Clone, Debug)]
pub struct PgWave3Repository {
    pool: PgPool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PutawayInventoryCommit {
    pub putaway: PutawayRecord,
    pub inventory_batch: InventoryBatch,
    pub inventory_movement: InventoryMovement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdempotentMutation<T> {
    pub value: T,
    pub replayed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Wave3RepositoryError {
    NotFound,
    DuplicateReceipt,
    DuplicateCode,
    InvalidStatus {
        expected: String,
        actual: String,
    },
    InvalidQuantity,
    InvalidDate(String),
    BatchExpired,
    QuantityClosureMismatch,
    OverReceiptNotAllowed,
    MissingSecondSigner,
    SameSigner,
    MissingApprovalSource,
    InvalidStateTransition {
        from: String,
        to: String,
        approval_source: String,
    },
    IdempotencyConflict,
    BillingRuleConflict,
    InvalidEffectiveWindow,
    InvalidRate,
    FutureTimestamp,
    Audit(String),
    Database(String),
    Serialize(String),
}

#[derive(FromRow)]
struct ReceivingOrderRow {
    id: Uuid,
    owner_id: Uuid,
    receipt_no: String,
    supplier_id: Option<Uuid>,
    warehouse_id: Uuid,
    external_ref: Option<String>,
    status: String,
    expected_arrival_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct ReceivingOrderLineRow {
    line_no: i32,
    product_id: Option<Uuid>,
    product_code: String,
    expected_qty: i64,
    batch_no: Option<String>,
    production_date: Option<NaiveDate>,
    expiry_date: Option<NaiveDate>,
}

#[derive(FromRow)]
struct InventoryBatchRow {
    id: Uuid,
    owner_id: Uuid,
    product_code: String,
    batch_no: String,
    production_date: NaiveDate,
    expiry_date: NaiveDate,
    qty_on_hand: i64,
    qty_locked: i64,
    quality_status: String,
    location_id: Uuid,
    location_code: String,
    recall_flag: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct BillingContractRow {
    id: Uuid,
    owner_id: Uuid,
    account_id: Uuid,
    contract_no: String,
    valid_from: NaiveDate,
    valid_to: NaiveDate,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct TemperatureReadingRow {
    id: Uuid,
    owner_id: Uuid,
    device_code: String,
    temperature_celsius: f64,
    humidity_percent: Option<f64>,
    captured_at: DateTime<Utc>,
    external_report_url: Option<String>,
    out_of_range: bool,
}

#[derive(FromRow)]
struct TemperatureExcursionEventRow {
    id: Uuid,
    owner_id: Uuid,
    external_event_id: String,
    device_code: String,
    location_code: Option<String>,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    min_temperature_celsius: Option<f64>,
    max_temperature_celsius: Option<f64>,
    affected_batch_ids: Vec<Uuid>,
    status: String,
    created_at: DateTime<Utc>,
}

impl PgWave3Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_receiving_order(
        &self,
        ctx: &AuthContext,
        req: CreateReceivingOrderRequest,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        if req.lines.is_empty() {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }

        let mut tx = self.begin().await?;
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO receiving_orders (
                id, owner_id, receipt_no, supplier_id, warehouse_id, external_ref,
                status, expected_arrival_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'draft', $7, $8, $8)
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.receipt_no)
        .bind(req.supplier_id)
        .bind(req.warehouse_id)
        .bind(&req.external_ref)
        .bind(req.expected_arrival_at)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        for line in &req.lines {
            sqlx::query(
                r#"
                INSERT INTO receiving_order_lines (
                    id, receiving_order_id, owner_id, line_no, product_id,
                    product_code, expected_qty, batch_no, production_date, expiry_date
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(id)
            .bind(ctx.owner_id)
            .bind(i32::try_from(line.line_no).map_err(|_| Wave3RepositoryError::InvalidQuantity)?)
            .bind(line.product_id)
            .bind(&line.product_code)
            .bind(line.expected_qty)
            .bind(&line.batch_no)
            .bind(parse_optional_date(line.production_date.as_deref())?)
            .bind(parse_optional_date(line.expiry_date.as_deref())?)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;
        }

        tx.commit().await.map_err(map_db_error)?;
        Ok(ReceivingOrder {
            id,
            owner_id: ctx.owner_id,
            receipt_no: req.receipt_no,
            supplier_id: req.supplier_id,
            warehouse_id: req.warehouse_id,
            external_ref: req.external_ref,
            status: "draft".to_string(),
            expected_arrival_at: req.expected_arrival_at,
            lines: req.lines,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn release_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, Wave3RepositoryError> {
        let updated = sqlx::query_as::<_, ReceivingOrderRow>(
            r#"
            UPDATE receiving_orders
               SET status = 'released',
                   updated_at = $3,
                   version = version + 1
             WHERE id = $1 AND owner_id = $2
            RETURNING id, owner_id, receipt_no, supplier_id, warehouse_id, external_ref,
                      status, expected_arrival_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;

        let lines = self.load_receiving_order_lines(id).await?;
        Ok(map_receiving_order(updated, lines))
    }

    pub async fn receive_receiving_order(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ReceiveReceivingOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<ReceivingOrderReceipt, Wave3RepositoryError> {
        Ok(self
            .receive_receiving_order_with_audit(ctx, id, req, now, idempotency_key, None)
            .await?
            .value)
    }

    pub async fn receive_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: ReceiveReceivingOrderRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ReceivingOrderReceipt>, Wave3RepositoryError> {
        if req.actual_qty < 0 || req.shortage_qty < 0 || req.rejected_qty < 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
        }))?;

        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "released" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "released".to_string(),
                actual: order.status,
            });
        }
        let expected_qty: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(expected_qty), 0)::BIGINT FROM receiving_order_lines WHERE receiving_order_id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if req.actual_qty > expected_qty {
            return Err(Wave3RepositoryError::OverReceiptNotAllowed);
        }
        if req.actual_qty + req.shortage_qty + req.rejected_qty != expected_qty {
            return Err(Wave3RepositoryError::QuantityClosureMismatch);
        }

        let receipt = ReceivingOrderReceipt {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            actual_qty: req.actual_qty,
            shortage_qty: req.shortage_qty,
            rejected_qty: req.rejected_qty,
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_order_receipts (
                id, receiving_order_id, owner_id, actual_qty, shortage_qty,
                rejected_qty, arrival_temperature_celsius, exception_note, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(receipt.id)
        .bind(receipt.receiving_order_id)
        .bind(receipt.owner_id)
        .bind(receipt.actual_qty)
        .bind(receipt.shortage_qty)
        .bind(receipt.rejected_qty)
        .bind(req.arrival_temperature_celsius)
        .bind(&req.exception_note)
        .bind(receipt.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_receipt_insert_error)?;

        sqlx::query(
            "UPDATE receiving_orders SET status = 'inspecting', updated_at = $3, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/receive",
            "receiving_order_receipt",
            receipt.id.to_string(),
            &receipt,
            now,
        )
        .await?;
        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: receipt,
            replayed: false,
        })
    }

    pub async fn putaway_receiving_order_and_inventory(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: PutawayRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
    ) -> Result<PutawayInventoryCommit, Wave3RepositoryError> {
        Ok(self
            .putaway_receiving_order_and_inventory_with_audit(
                ctx,
                id,
                req,
                now,
                idempotency_key,
                None,
            )
            .await?
            .value)
    }

    pub async fn putaway_receiving_order_and_inventory_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: PutawayRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<PutawayInventoryCommit>, Wave3RepositoryError> {
        if req.qty <= 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
        }))?;

        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "putaway" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "putaway".to_string(),
                actual: order.status,
            });
        }
        let line = sqlx::query_as::<_, ReceivingOrderLineRow>(
            r#"
            SELECT line_no, product_id, product_code, expected_qty, batch_no,
                   production_date, expiry_date
              FROM receiving_order_lines
             WHERE receiving_order_id = $1
               AND owner_id = $2
               AND product_code = $3
               AND (batch_no = $4 OR batch_no IS NULL)
             ORDER BY line_no
             LIMIT 1
            "#,
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(&req.product_code)
        .bind(&req.batch_no)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let production_date = line
            .production_date
            .ok_or_else(|| Wave3RepositoryError::InvalidDate("production_date".to_string()))?;
        let expiry_date = line
            .expiry_date
            .ok_or_else(|| Wave3RepositoryError::InvalidDate("expiry_date".to_string()))?;

        let putaway = PutawayRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no.clone(),
            product_code: req.product_code.clone(),
            qty: req.qty,
            location_id: req.location_id,
            location_code: req.location_code.clone(),
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_putaways (
                id, receiving_order_id, owner_id, batch_no, product_code,
                qty, location_id, location_code, quality_status, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(putaway.id)
        .bind(putaway.receiving_order_id)
        .bind(putaway.owner_id)
        .bind(&putaway.batch_no)
        .bind(&putaway.product_code)
        .bind(putaway.qty)
        .bind(putaway.location_id)
        .bind(&putaway.location_code)
        .bind(&req.quality_status)
        .bind(putaway.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let batch_row = sqlx::query_as::<_, InventoryBatchRow>(
            r#"
            INSERT INTO inventory_batches (
                id, owner_id, product_code, batch_no, production_date, expiry_date,
                qty_on_hand, qty_locked, quality_status, location_id, location_code,
                recall_flag, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, $9, $10, FALSE, $11, $11)
            ON CONFLICT (owner_id, product_code, batch_no, location_id, quality_status)
            DO UPDATE SET
                qty_on_hand = inventory_batches.qty_on_hand + EXCLUDED.qty_on_hand,
                updated_at = EXCLUDED.updated_at,
                version = inventory_batches.version + 1
            RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                      qty_on_hand, qty_locked, quality_status, location_id, location_code,
                      recall_flag, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(&req.product_code)
        .bind(&req.batch_no)
        .bind(production_date)
        .bind(expiry_date)
        .bind(req.qty)
        .bind(if req.quality_status.is_empty() {
            STATUS_QUALIFIED
        } else {
            &req.quality_status
        })
        .bind(req.location_id)
        .bind(&req.location_code)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        let inventory_batch = map_inventory_batch(batch_row);

        let movement = InventoryMovement {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            batch_id: inventory_batch.id,
            movement_type: "inbound_putaway".to_string(),
            qty_delta: req.qty,
            source_document_type: "receiving_order".to_string(),
            source_document_id: id,
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO inventory_movements (
                id, owner_id, batch_id, movement_type, qty_delta,
                source_document_type, source_document_id, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(movement.id)
        .bind(movement.owner_id)
        .bind(movement.batch_id)
        .bind(&movement.movement_type)
        .bind(movement.qty_delta)
        .bind(&movement.source_document_type)
        .bind(movement.source_document_id)
        .bind(movement.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        sqlx::query(
            "UPDATE receiving_orders SET status = 'completed', updated_at = $3, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let result = PutawayInventoryCommit {
            putaway,
            inventory_batch,
            inventory_movement: movement,
        };
        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/putaway",
            "receiving_putaway",
            result.putaway.id.to_string(),
            &result,
            now,
        )
        .await?;
        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: result,
            replayed: false,
        })
    }

    pub async fn inspect_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: InspectReceivingOrderRequest,
        today: NaiveDate,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<ReceivingInspectionRecord>, Wave3RepositoryError> {
        if req.accepted_qty < 0 || req.rejected_qty < 0 {
            return Err(Wave3RepositoryError::InvalidQuantity);
        }
        let production_date = parse_date(&req.production_date)?;
        let expiry_date = parse_date(&req.expiry_date)?;
        if expiry_date < today {
            return Err(Wave3RepositoryError::BatchExpired);
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
        }))?;

        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "inspecting" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "inspecting".to_string(),
                actual: order.status,
            });
        }

        let inspection = ReceivingInspectionRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no.clone(),
            accepted_qty: req.accepted_qty,
            rejected_qty: req.rejected_qty,
            quality_status: req.quality_status.clone(),
            occurred_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_inspections (
                id, receiving_order_id, owner_id, batch_no, accepted_qty,
                rejected_qty, production_date, expiry_date, quality_status,
                trace_codes, occurred_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(inspection.id)
        .bind(inspection.receiving_order_id)
        .bind(inspection.owner_id)
        .bind(&inspection.batch_no)
        .bind(inspection.accepted_qty)
        .bind(inspection.rejected_qty)
        .bind(production_date)
        .bind(expiry_date)
        .bind(&inspection.quality_status)
        .bind(&req.trace_codes)
        .bind(inspection.occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/inspect",
            "receiving_inspection",
            inspection.id.to_string(),
            &inspection,
            now,
        )
        .await?;
        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: inspection,
            replayed: false,
        })
    }

    pub async fn sign_receiving_order_with_audit(
        &self,
        ctx: &AuthContext,
        id: Uuid,
        req: SignInspectionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InspectionSignatureRecord>, Wave3RepositoryError> {
        if req.dual_required && req.second_signer_id.is_none() {
            return Err(Wave3RepositoryError::MissingSecondSigner);
        }
        if let Some(second_signer_id) = req.second_signer_id {
            if second_signer_id == req.first_signer_id {
                return Err(Wave3RepositoryError::SameSigner);
            }
        }
        let request_hash = request_hash(&serde_json::json!({
            "receiving_order_id": id,
            "request": req,
        }))?;

        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let order = lock_receiving_order(&mut tx, ctx.owner_id, id).await?;
        if order.status != "inspecting" {
            return Err(Wave3RepositoryError::InvalidStatus {
                expected: "inspecting".to_string(),
                actual: order.status,
            });
        }

        let signature = InspectionSignatureRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            first_signer_id: req.first_signer_id,
            second_signer_id: req.second_signer_id,
            signed_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO receiving_inspection_signatures (
                id, receiving_order_id, owner_id, dual_required,
                first_signer_id, second_signer_id, strategy_rule_id, signed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NULL, $7)
            "#,
        )
        .bind(signature.id)
        .bind(signature.receiving_order_id)
        .bind(signature.owner_id)
        .bind(req.dual_required)
        .bind(signature.first_signer_id)
        .bind(signature.second_signer_id)
        .bind(signature.signed_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        sqlx::query(
            "UPDATE receiving_orders SET status = 'putaway', updated_at = $3, version = version + 1 WHERE id = $1 AND owner_id = $2",
        )
        .bind(id)
        .bind(ctx.owner_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inbound/receiving-orders/{id}/sign",
            "receiving_inspection_signature",
            signature.id.to_string(),
            &signature,
            now,
        )
        .await?;
        if let Some(audit) = audit {
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: signature,
            replayed: false,
        })
    }

    pub async fn list_inventory_batches(
        &self,
        ctx: &AuthContext,
    ) -> Result<Vec<InventoryBatch>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE owner_id = $1
             ORDER BY updated_at DESC, id
            "#,
        )
        .bind(ctx.owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;

        Ok(rows.into_iter().map(map_inventory_batch).collect())
    }

    pub async fn change_inventory_status_with_audit(
        &self,
        ctx: &AuthContext,
        req: ChangeInventoryStatusRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<InventoryBatch>, Wave3RepositoryError> {
        if req.approval_source.trim().is_empty() || req.approval_id.trim().is_empty() {
            return Err(Wave3RepositoryError::MissingApprovalSource);
        }
        let request_hash = request_hash(&serde_json::json!({
            "request": req,
        }))?;

        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }

        let batch_row = sqlx::query_as::<_, InventoryBatchRow>(
            r#"
            SELECT id, owner_id, product_code, batch_no, production_date, expiry_date,
                   qty_on_hand, qty_locked, quality_status, location_id, location_code,
                   recall_flag, created_at, updated_at
              FROM inventory_batches
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE
            "#,
        )
        .bind(req.batch_id)
        .bind(ctx.owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        let from_status = batch_row.quality_status.clone();

        let batch = if from_status == req.target_status {
            map_inventory_batch(batch_row)
        } else {
            if !allowed_transition(&from_status, &req.target_status, &req.approval_source) {
                return Err(Wave3RepositoryError::InvalidStateTransition {
                    from: from_status,
                    to: req.target_status,
                    approval_source: req.approval_source,
                });
            }

            sqlx::query(
                r#"
                INSERT INTO inventory_status_changes (
                    id, owner_id, batch_id, from_status, to_status,
                    reason, approval_source, approval_id, occurred_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(req.batch_id)
            .bind(&from_status)
            .bind(&req.target_status)
            .bind(&req.reason)
            .bind(&req.approval_source)
            .bind(&req.approval_id)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_db_error)?;

            let updated = sqlx::query_as::<_, InventoryBatchRow>(
                r#"
                UPDATE inventory_batches
                   SET quality_status = $3,
                       updated_at = $4,
                       version = version + 1
                 WHERE id = $1 AND owner_id = $2
                RETURNING id, owner_id, product_code, batch_no, production_date, expiry_date,
                          qty_on_hand, qty_locked, quality_status, location_id, location_code,
                          recall_flag, created_at, updated_at
                "#,
            )
            .bind(req.batch_id)
            .bind(ctx.owner_id)
            .bind(&req.target_status)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;

            let batch = map_inventory_batch(updated);
            if let Some(audit) = audit {
                append_event_in_tx(&mut tx, &audit)
                    .await
                    .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
            }
            batch
        };

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/inventory/batches/status",
            "inventory_batch",
            batch.id.to_string(),
            &batch,
            now,
        )
        .await?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: batch,
            replayed: false,
        })
    }

    pub async fn ingest_temperature_reading_with_audit(
        &self,
        ctx: &AuthContext,
        req: IngestTemperatureReadingRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TemperatureReading>, Wave3RepositoryError> {
        if req.captured_at > now {
            return Err(Wave3RepositoryError::FutureTimestamp);
        }
        let request_hash = request_hash(&serde_json::json!({
            "request": req,
        }))?;

        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }
        ensure_cold_chain_device_active(&mut tx, ctx.owner_id, &req.device_code).await?;

        let existing =
            load_temperature_reading(&mut tx, ctx.owner_id, &req.device_code, req.captured_at)
                .await?;
        let (reading, inserted) = if let Some(existing) = existing {
            (existing, false)
        } else {
            let row = sqlx::query_as::<_, TemperatureReadingRow>(
                r#"
                INSERT INTO temperature_readings (
                    id, owner_id, device_code, temperature_celsius, humidity_percent,
                    captured_at, external_report_url, out_of_range, source_system,
                    external_reading_id, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'external_cold_chain', NULL, $9)
                RETURNING id, owner_id, device_code, temperature_celsius, humidity_percent,
                          captured_at, external_report_url, out_of_range
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(&req.device_code)
            .bind(req.temperature_celsius)
            .bind(req.humidity_percent)
            .bind(req.captured_at)
            .bind(&req.external_report_url)
            .bind(req.out_of_range)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            (map_temperature_reading(row), true)
        };

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/cold-chain/readings",
            "temperature_reading",
            reading.id.to_string(),
            &reading,
            now,
        )
        .await?;
        if inserted {
            let mut audit = audit.unwrap_or_else(|| {
                AuditWriteRequest::from_auth_context(
                    ctx,
                    "ingest_reading",
                    "M5",
                    "temperature_reading",
                    reading.id.to_string(),
                    None,
                )
            });
            audit.resource_id = reading.id.to_string();
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: reading,
            replayed: false,
        })
    }

    pub async fn ingest_temperature_excursion_with_audit(
        &self,
        ctx: &AuthContext,
        req: IngestTemperatureExcursionRequest,
        now: DateTime<Utc>,
        idempotency_key: &str,
        audit: Option<AuditWriteRequest>,
    ) -> Result<IdempotentMutation<TemperatureExcursionEvent>, Wave3RepositoryError> {
        if req.started_at > now {
            return Err(Wave3RepositoryError::FutureTimestamp);
        }
        let request_hash = request_hash(&serde_json::json!({
            "request": req,
        }))?;

        let mut tx = self.begin().await?;
        lock_idempotency_key(&mut tx, ctx.owner_id, idempotency_key).await?;
        if let Some(replay) =
            replay_idempotency(&mut tx, ctx.owner_id, idempotency_key, &request_hash, now).await?
        {
            return Ok(IdempotentMutation {
                value: replay,
                replayed: true,
            });
        }
        ensure_cold_chain_device_active(&mut tx, ctx.owner_id, &req.device_code).await?;

        let existing =
            load_temperature_excursion(&mut tx, ctx.owner_id, &req.external_event_id).await?;
        let (event, inserted) = if let Some(existing) = existing {
            (existing, false)
        } else {
            let row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
                r#"
                INSERT INTO temperature_excursion_events (
                    id, owner_id, external_event_id, device_code, location_code,
                    started_at, ended_at, min_temperature_celsius,
                    max_temperature_celsius, affected_batch_ids, status, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending_disposition', $11)
                RETURNING id, owner_id, external_event_id, device_code, location_code,
                          started_at, ended_at, min_temperature_celsius,
                          max_temperature_celsius, affected_batch_ids, status, created_at
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(ctx.owner_id)
            .bind(&req.external_event_id)
            .bind(&req.device_code)
            .bind(&req.location_code)
            .bind(req.started_at)
            .bind(req.ended_at)
            .bind(req.min_temperature_celsius)
            .bind(req.max_temperature_celsius)
            .bind(&req.affected_batch_ids)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(map_db_error)?;
            (map_temperature_excursion(row), true)
        };

        store_idempotency_success(
            &mut tx,
            ctx.owner_id,
            idempotency_key,
            &request_hash,
            "POST",
            "/api/v1/cold-chain/excursions",
            "temperature_excursion",
            event.id.to_string(),
            &event,
            now,
        )
        .await?;
        if inserted {
            let mut audit = audit.unwrap_or_else(|| {
                AuditWriteRequest::from_auth_context(
                    ctx,
                    "ingest_excursion",
                    "M5",
                    "temperature_excursion",
                    event.id.to_string(),
                    None,
                )
            });
            audit.resource_id = event.id.to_string();
            append_event_in_tx(&mut tx, &audit)
                .await
                .map_err(|error| Wave3RepositoryError::Audit(format!("{error:?}")))?;
        }
        tx.commit().await.map_err(map_db_error)?;
        Ok(IdempotentMutation {
            value: event,
            replayed: false,
        })
    }

    pub async fn create_billing_account(
        &self,
        ctx: &AuthContext,
        req: CreateBillingAccountRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingAccount, Wave3RepositoryError> {
        let account = BillingAccount {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            account_code: req.account_code,
            account_name: req.account_name,
            status: "active".to_string(),
            created_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO billing_accounts (
                id, owner_id, account_code, account_name, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            "#,
        )
        .bind(account.id)
        .bind(account.owner_id)
        .bind(&account.account_code)
        .bind(&account.account_name)
        .bind(&account.status)
        .bind(account.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(account)
    }

    pub async fn create_billing_contract(
        &self,
        ctx: &AuthContext,
        req: CreateBillingContractRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingContract, Wave3RepositoryError> {
        let valid_from = parse_date(&req.valid_from)?;
        let valid_to = parse_date(&req.valid_to)?;
        if valid_to < valid_from {
            return Err(Wave3RepositoryError::InvalidEffectiveWindow);
        }
        let row = sqlx::query_as::<_, BillingContractRow>(
            r#"
            INSERT INTO billing_contracts (
                id, owner_id, account_id, contract_no, valid_from, valid_to,
                status, created_at, updated_at
            )
            SELECT $1, $2, $3, $4, $5, $6, 'active', $7, $7
              FROM billing_accounts
             WHERE id = $3 AND owner_id = $2
            RETURNING id, owner_id, account_id, contract_no, valid_from, valid_to,
                      status, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(ctx.owner_id)
        .bind(req.account_id)
        .bind(&req.contract_no)
        .bind(valid_from)
        .bind(valid_to)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(Wave3RepositoryError::NotFound)?;
        Ok(BillingContract {
            id: row.id,
            owner_id: row.owner_id,
            account_id: row.account_id,
            contract_no: row.contract_no,
            valid_from: row.valid_from.to_string(),
            valid_to: row.valid_to.to_string(),
            status: row.status,
            created_at: row.created_at,
        })
    }

    pub async fn create_billing_rule(
        &self,
        ctx: &AuthContext,
        req: CreateBillingRuleRequest,
        now: DateTime<Utc>,
    ) -> Result<BillingRule, Wave3RepositoryError> {
        if req.unit_price_cents < 0 {
            return Err(Wave3RepositoryError::InvalidRate);
        }
        let effective_from = parse_date(&req.effective_from)?;
        let effective_to = parse_date(&req.effective_to)?;
        if effective_to < effective_from {
            return Err(Wave3RepositoryError::InvalidEffectiveWindow);
        }

        let mut tx = self.begin().await?;
        let contract_owner: Option<Uuid> =
            sqlx::query_scalar("SELECT owner_id FROM billing_contracts WHERE id = $1")
                .bind(req.contract_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
        if contract_owner != Some(ctx.owner_id) {
            return Err(Wave3RepositoryError::NotFound);
        }
        let overlap: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1
                  FROM billing_rules
                 WHERE owner_id = $1
                   AND contract_id = $2
                   AND charge_item = $3
                   AND unit = $4
                   AND billing_cycle = $5
                   AND effective_from <= $7
                   AND effective_to >= $6
            )
            "#,
        )
        .bind(ctx.owner_id)
        .bind(req.contract_id)
        .bind(&req.charge_item)
        .bind(&req.unit)
        .bind(&req.billing_cycle)
        .bind(effective_from)
        .bind(effective_to)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;
        if overlap {
            return Err(Wave3RepositoryError::BillingRuleConflict);
        }

        let rule = BillingRule {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            contract_id: req.contract_id,
            charge_item: req.charge_item,
            unit: req.unit,
            unit_price_cents: req.unit_price_cents,
            billing_cycle: req.billing_cycle,
            effective_from: effective_from.to_string(),
            effective_to: effective_to.to_string(),
            created_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO billing_rules (
                id, owner_id, contract_id, charge_item, unit, unit_price_cents,
                billing_cycle, effective_from, effective_to, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(rule.id)
        .bind(rule.owner_id)
        .bind(rule.contract_id)
        .bind(&rule.charge_item)
        .bind(&rule.unit)
        .bind(rule.unit_price_cents)
        .bind(&rule.billing_cycle)
        .bind(effective_from)
        .bind(effective_to)
        .bind(rule.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_db_error)?;
        tx.commit().await.map_err(map_db_error)?;
        Ok(rule)
    }

    async fn begin(&self) -> Result<Transaction<'_, Postgres>, Wave3RepositoryError> {
        self.pool.begin().await.map_err(map_db_error)
    }

    async fn load_receiving_order_lines(
        &self,
        id: Uuid,
    ) -> Result<Vec<ReceivingOrderLine>, Wave3RepositoryError> {
        let rows = sqlx::query_as::<_, ReceivingOrderLineRow>(
            r#"
            SELECT line_no, product_id, product_code, expected_qty, batch_no,
                   production_date, expiry_date
              FROM receiving_order_lines
             WHERE receiving_order_id = $1
             ORDER BY line_no
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        Ok(rows.into_iter().map(map_receiving_order_line).collect())
    }
}

async fn lock_receiving_order(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    id: Uuid,
) -> Result<ReceivingOrderRow, Wave3RepositoryError> {
    sqlx::query_as::<_, ReceivingOrderRow>(
        r#"
        SELECT id, owner_id, receipt_no, supplier_id, warehouse_id, external_ref,
               status, expected_arrival_at, created_at, updated_at
          FROM receiving_orders
         WHERE id = $1 AND owner_id = $2
         FOR UPDATE
        "#,
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?
    .ok_or(Wave3RepositoryError::NotFound)
}

async fn ensure_cold_chain_device_active(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    device_code: &str,
) -> Result<(), Wave3RepositoryError> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
              FROM cold_chain_devices
             WHERE owner_id = $1 AND device_code = $2 AND status = 'active'
        )
        "#,
    )
    .bind(owner_id)
    .bind(device_code)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_db_error)?;
    if exists {
        Ok(())
    } else {
        Err(Wave3RepositoryError::NotFound)
    }
}

async fn load_temperature_reading(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    device_code: &str,
    captured_at: DateTime<Utc>,
) -> Result<Option<TemperatureReading>, Wave3RepositoryError> {
    let row = sqlx::query_as::<_, TemperatureReadingRow>(
        r#"
        SELECT id, owner_id, device_code, temperature_celsius, humidity_percent,
               captured_at, external_report_url, out_of_range
          FROM temperature_readings
         WHERE owner_id = $1 AND device_code = $2 AND captured_at = $3
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(device_code)
    .bind(captured_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(row.map(map_temperature_reading))
}

async fn load_temperature_excursion(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    external_event_id: &str,
) -> Result<Option<TemperatureExcursionEvent>, Wave3RepositoryError> {
    let row = sqlx::query_as::<_, TemperatureExcursionEventRow>(
        r#"
        SELECT id, owner_id, external_event_id, device_code, location_code,
               started_at, ended_at, min_temperature_celsius,
               max_temperature_celsius, affected_batch_ids, status, created_at
          FROM temperature_excursion_events
         WHERE owner_id = $1 AND external_event_id = $2
         FOR UPDATE
        "#,
    )
    .bind(owner_id)
    .bind(external_event_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(row.map(map_temperature_excursion))
}

async fn replay_idempotency<T: DeserializeOwned>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    now: DateTime<Utc>,
) -> Result<Option<T>, Wave3RepositoryError> {
    let row: Option<(String, serde_json::Value, DateTime<Utc>)> = sqlx::query_as(
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
        return Err(Wave3RepositoryError::IdempotencyConflict);
    }
    serde_json::from_value(response_body)
        .map(Some)
        .map_err(|error| Wave3RepositoryError::Serialize(error.to_string()))
}

async fn lock_idempotency_key(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
) -> Result<(), Wave3RepositoryError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(idempotency_lock_id(owner_id, idempotency_key))
        .fetch_one(&mut **tx)
        .await
        .map_err(map_db_error)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn store_idempotency_success<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    idempotency_key: &str,
    request_hash: &str,
    method: &str,
    path: &str,
    resource_type: &str,
    resource_id: String,
    response: &T,
    now: DateTime<Utc>,
) -> Result<(), Wave3RepositoryError> {
    let response_body = serde_json::to_value(response)
        .map_err(|error| Wave3RepositoryError::Serialize(error.to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO idempotency_request (
            id, owner_id, idempotency_key, request_hash, method, path,
            status_code, response_body, resource_type, resource_id, expires_at, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 200, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(method)
    .bind(path)
    .bind(response_body)
    .bind(resource_type)
    .bind(resource_id)
    .bind(now + Duration::hours(24))
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(map_db_error)?;
    Ok(())
}

fn request_hash(value: &serde_json::Value) -> Result<String, Wave3RepositoryError> {
    let text = serde_json::to_string(value)
        .map_err(|error| Wave3RepositoryError::Serialize(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}

fn idempotency_lock_id(owner_id: Uuid, idempotency_key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(owner_id.as_bytes());
    hasher.update([0]);
    hasher.update(idempotency_key.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

fn parse_optional_date(value: Option<&str>) -> Result<Option<NaiveDate>, Wave3RepositoryError> {
    value.map(parse_date).transpose()
}

fn parse_date(value: &str) -> Result<NaiveDate, Wave3RepositoryError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| Wave3RepositoryError::InvalidDate(value.to_string()))
}

fn map_receiving_order(row: ReceivingOrderRow, lines: Vec<ReceivingOrderLine>) -> ReceivingOrder {
    ReceivingOrder {
        id: row.id,
        owner_id: row.owner_id,
        receipt_no: row.receipt_no,
        supplier_id: row.supplier_id,
        warehouse_id: row.warehouse_id,
        external_ref: row.external_ref,
        status: row.status,
        expected_arrival_at: row.expected_arrival_at,
        lines,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_receiving_order_line(row: ReceivingOrderLineRow) -> ReceivingOrderLine {
    ReceivingOrderLine {
        line_no: row.line_no as u32,
        product_id: row.product_id,
        product_code: row.product_code,
        expected_qty: row.expected_qty,
        batch_no: row.batch_no,
        production_date: row.production_date.map(|date| date.to_string()),
        expiry_date: row.expiry_date.map(|date| date.to_string()),
    }
}

fn map_inventory_batch(row: InventoryBatchRow) -> InventoryBatch {
    InventoryBatch {
        id: row.id,
        owner_id: row.owner_id,
        product_code: row.product_code,
        batch_no: row.batch_no,
        production_date: row.production_date.to_string(),
        expiry_date: row.expiry_date.to_string(),
        qty_on_hand: row.qty_on_hand,
        qty_locked: row.qty_locked,
        quality_status: row.quality_status,
        location_id: row.location_id,
        location_code: row.location_code,
        recall_flag: row.recall_flag,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn map_temperature_reading(row: TemperatureReadingRow) -> TemperatureReading {
    TemperatureReading {
        id: row.id,
        owner_id: row.owner_id,
        device_code: row.device_code,
        temperature_celsius: row.temperature_celsius,
        humidity_percent: row.humidity_percent,
        captured_at: row.captured_at,
        external_report_url: row.external_report_url,
        out_of_range: row.out_of_range,
    }
}

fn map_temperature_excursion(row: TemperatureExcursionEventRow) -> TemperatureExcursionEvent {
    TemperatureExcursionEvent {
        id: row.id,
        owner_id: row.owner_id,
        external_event_id: row.external_event_id,
        device_code: row.device_code,
        location_code: row.location_code,
        started_at: row.started_at,
        ended_at: row.ended_at,
        min_temperature_celsius: row.min_temperature_celsius,
        max_temperature_celsius: row.max_temperature_celsius,
        affected_batch_ids: row.affected_batch_ids,
        status: row.status,
        created_at: row.created_at,
    }
}

fn map_db_error(error: sqlx::Error) -> Wave3RepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return Wave3RepositoryError::DuplicateCode;
        }
    }
    Wave3RepositoryError::Database(error.to_string())
}

fn map_receipt_insert_error(error: sqlx::Error) -> Wave3RepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return Wave3RepositoryError::DuplicateReceipt;
        }
    }
    map_db_error(error)
}
