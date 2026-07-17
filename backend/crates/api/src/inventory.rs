//! Wave 3 M3 inventory domain service.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
use wms_domain::{
    CancelInventoryRecallRequest, ChangeInventoryStatusRequest, InventoryBatch,
    InventoryBatchTrace, InventoryMovement, MarkInventoryRecallRequest, PutawayInventoryRequest,
};

use crate::auth::AuthContext;

mod stock_adjustment;

pub(crate) use stock_adjustment::add_for_stock_surplus_in_tx;

pub const STATUS_QUALIFIED: &str = "qualified";
pub const STATUS_QUARANTINED: &str = "quarantined";
pub const STATUS_UNQUALIFIED: &str = "unqualified";
pub const STATUS_PENDING_DESTRUCTION: &str = "pending_destruction";
pub const STATUS_LOSS_DEDUCTED: &str = "loss_deducted";
pub const APPROVAL_SOURCE_EXPIRY: &str = "M3-002-EXPIRY";

pub(crate) async fn deduct_for_stock_loss_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    batch_id: Uuid,
    quantity: i64,
    source_document_id: Uuid,
    approval_source: &str,
    approval_id: &str,
    clear_recall: bool,
    now: DateTime<Utc>,
) -> Result<Option<i64>, sqlx::Error> {
    let remaining = sqlx::query_scalar::<_, i64>(
        r#"
        UPDATE inventory_batches
           SET qty_on_hand = qty_on_hand - $3,
               recall_flag = CASE WHEN $4 THEN FALSE ELSE recall_flag END,
               updated_at = $5,
               version = version + 1
         WHERE owner_id = $1
           AND id = $2
           AND $3 > 0
           AND qty_on_hand - qty_locked >= $3
        RETURNING qty_on_hand
        "#,
    )
    .bind(owner_id)
    .bind(batch_id)
    .bind(quantity)
    .bind(clear_recall)
    .bind(now)
    .fetch_optional(&mut **tx)
    .await?;
    if remaining.is_none() {
        return Ok(None);
    }
    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, owner_id, batch_id, movement_type, qty_delta,
            source_document_type, source_document_id, approval_source,
            approval_id, occurred_at
        ) VALUES ($1,$2,$3,'stock_loss',$4,'stock_loss_order',$5,$6,$7,$8)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(owner_id)
    .bind(batch_id)
    .bind(-quantity)
    .bind(source_document_id)
    .bind(approval_source)
    .bind(approval_id)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(remaining)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryError {
    NotFound,
    InvalidQuantity,
    ExpiredBatch,
    InvalidReason,
    MissingApprovalSource,
    RecallAlreadyActive,
    RecallNotActive,
    RecallStateChanged,
    SameApprover,
    InvalidStateTransition {
        from: String,
        to: String,
        approval_source: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct InventoryStore {
    batches: BTreeMap<Uuid, InventoryBatch>,
    movements: BTreeMap<Uuid, InventoryMovement>,
    recall_previous_status: BTreeMap<Uuid, String>,
}

impl InventoryStore {
    pub fn putaway_from_inbound(
        &mut self,
        ctx: &AuthContext,
        req: PutawayInventoryRequest,
        today: NaiveDate,
        now: DateTime<Utc>,
    ) -> Result<InventoryBatch, InventoryError> {
        if req.qty <= 0 {
            return Err(InventoryError::InvalidQuantity);
        }
        let expiry = NaiveDate::parse_from_str(&req.expiry_date, "%Y-%m-%d")
            .map_err(|_| InventoryError::ExpiredBatch)?;
        if expiry < today {
            return Err(InventoryError::ExpiredBatch);
        }

        let existing_id = self
            .batches
            .values()
            .find(|batch| {
                batch.owner_id == ctx.owner_id
                    && batch.product_code == req.product_code
                    && batch.batch_no == req.batch_no
                    && batch.location_id == req.location_id
                    && batch.quality_status == req.quality_status
            })
            .map(|batch| batch.id);

        let batch = if let Some(id) = existing_id {
            let batch = self.batches.get_mut(&id).ok_or(InventoryError::NotFound)?;
            batch.qty_on_hand += req.qty;
            batch.updated_at = now;
            batch.clone()
        } else {
            let batch = InventoryBatch {
                id: Uuid::new_v4(),
                owner_id: ctx.owner_id,
                product_code: req.product_code.clone(),
                product_name: None,
                specification: None,
                manufacturer: None,
                batch_no: req.batch_no.clone(),
                production_date: req.production_date.clone(),
                expiry_date: req.expiry_date.clone(),
                qty_on_hand: req.qty,
                qty_locked: 0,
                quality_status: req.quality_status.clone(),
                location_id: req.location_id,
                location_code: req.location_code.clone(),
                row_no: None,
                column_no: None,
                layer_no: None,
                zone_code: None,
                temperature_zone: None,
                quality_color: None,
                max_volume_cm3: None,
                used_volume_cm3: None,
                remaining_volume_cm3: None,
                max_sku_count: None,
                current_sku_count: None,
                container_lpn: None,
                recall_flag: false,
                created_at: now,
                updated_at: now,
            };
            self.batches.insert(batch.id, batch.clone());
            batch
        };

        let movement = InventoryMovement {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            batch_id: batch.id,
            movement_type: "inbound_putaway".to_string(),
            qty_delta: req.qty,
            source_document_type: "receiving_order".to_string(),
            source_document_id: req.source_receiving_order_id,
            occurred_at: now,
            location_code: Some(batch.location_code.clone()),
            from_location_code: None,
            to_location_code: Some(batch.location_code.clone()),
            lpn_code: None,
            operator_user_id: Some(ctx.user_id),
            operator_name: Some(ctx.actor_name.clone()),
            volume_delta_cm3: None,
            product_code: Some(batch.product_code.clone()),
            product_name: None,
            batch_no: Some(batch.batch_no.clone()),
            expiry_date: Some(batch.expiry_date.clone()),
        };
        self.movements.insert(movement.id, movement);

        Ok(batch)
    }

    pub fn list_batches(&self, ctx: &AuthContext) -> Vec<InventoryBatch> {
        self.batches
            .values()
            .filter(|batch| batch.owner_id == ctx.owner_id)
            .cloned()
            .collect()
    }

    pub fn trace_batch(
        &self,
        ctx: &AuthContext,
        batch_id: Uuid,
    ) -> Result<InventoryBatchTrace, InventoryError> {
        let batch = self
            .batches
            .get(&batch_id)
            .filter(|batch| batch.owner_id == ctx.owner_id)
            .cloned()
            .ok_or(InventoryError::NotFound)?;
        let movements = self
            .movements
            .values()
            .filter(|movement| movement.owner_id == ctx.owner_id && movement.batch_id == batch_id)
            .cloned()
            .collect();
        Ok(InventoryBatchTrace {
            batch,
            movements,
            status_changes: Vec::new(),
        })
    }

    pub fn available_qty(&self, ctx: &AuthContext, batch_id: Uuid) -> Result<i64, InventoryError> {
        let batch = self
            .batches
            .get(&batch_id)
            .filter(|batch| batch.owner_id == ctx.owner_id)
            .ok_or(InventoryError::NotFound)?;
        if batch.quality_status != STATUS_QUALIFIED || batch.recall_flag {
            return Ok(0);
        }
        Ok(batch.qty_on_hand - batch.qty_locked)
    }

    pub fn change_status(
        &mut self,
        ctx: &AuthContext,
        req: ChangeInventoryStatusRequest,
        now: DateTime<Utc>,
    ) -> Result<InventoryBatch, InventoryError> {
        if req.reason.trim().is_empty() {
            return Err(InventoryError::InvalidReason);
        }
        if req.approval_source.trim().is_empty() || req.approval_id.trim().is_empty() {
            return Err(InventoryError::MissingApprovalSource);
        }
        let batch = self
            .batches
            .get_mut(&req.batch_id)
            .filter(|batch| batch.owner_id == ctx.owner_id)
            .ok_or(InventoryError::NotFound)?;

        if batch.quality_status == req.target_status {
            return Ok(batch.clone());
        }

        if !allowed_transition(
            &batch.quality_status,
            &req.target_status,
            &req.approval_source,
        ) {
            return Err(InventoryError::InvalidStateTransition {
                from: batch.quality_status.clone(),
                to: req.target_status,
                approval_source: req.approval_source,
            });
        }

        batch.quality_status = req.target_status;
        batch.updated_at = now;
        Ok(batch.clone())
    }

    pub fn mark_recall(
        &mut self,
        ctx: &AuthContext,
        req: MarkInventoryRecallRequest,
        now: DateTime<Utc>,
    ) -> Result<InventoryBatch, InventoryError> {
        if req.reason.trim().is_empty()
            || req.approval_id.trim().is_empty()
            || !matches!(req.approval_source.as_str(), "M-QL" | "M-TC")
        {
            return Err(InventoryError::MissingApprovalSource);
        }
        let (batch, previous_status) = {
            let batch = self
                .batches
                .get_mut(&req.batch_id)
                .filter(|batch| batch.owner_id == ctx.owner_id)
                .ok_or(InventoryError::NotFound)?;
            if batch.recall_flag {
                return Err(InventoryError::RecallAlreadyActive);
            }
            if batch.quality_status == STATUS_QUALIFIED
                && !allowed_transition(STATUS_QUALIFIED, STATUS_QUARANTINED, &req.approval_source)
            {
                return Err(InventoryError::InvalidStateTransition {
                    from: batch.quality_status.clone(),
                    to: STATUS_QUARANTINED.to_string(),
                    approval_source: req.approval_source,
                });
            }
            let previous_status = batch.quality_status.clone();
            if previous_status == STATUS_QUALIFIED {
                batch.quality_status = STATUS_QUARANTINED.to_string();
            }
            batch.recall_flag = true;
            batch.updated_at = now;
            (batch.clone(), previous_status)
        };
        self.recall_previous_status
            .insert(batch.id, previous_status);
        Ok(batch)
    }

    pub fn cancel_recall(
        &mut self,
        ctx: &AuthContext,
        req: CancelInventoryRecallRequest,
        now: DateTime<Utc>,
    ) -> Result<InventoryBatch, InventoryError> {
        if req.reason.trim().is_empty() || req.approval_id.trim().is_empty() {
            return Err(InventoryError::MissingApprovalSource);
        }
        if req.second_approver_id == ctx.user_id {
            return Err(InventoryError::SameApprover);
        }
        let previous_status = self
            .recall_previous_status
            .get(&req.batch_id)
            .cloned()
            .ok_or(InventoryError::RecallNotActive)?;
        let batch = {
            let batch = self
                .batches
                .get_mut(&req.batch_id)
                .filter(|batch| batch.owner_id == ctx.owner_id)
                .ok_or(InventoryError::NotFound)?;
            if !batch.recall_flag {
                return Err(InventoryError::RecallNotActive);
            }
            let expected_status = if previous_status == STATUS_QUALIFIED {
                STATUS_QUARANTINED
            } else {
                previous_status.as_str()
            };
            if batch.quality_status != expected_status {
                return Err(InventoryError::RecallStateChanged);
            }
            batch.quality_status = previous_status;
            batch.recall_flag = false;
            batch.updated_at = now;
            batch.clone()
        };
        self.recall_previous_status.remove(&batch.id);
        Ok(batch)
    }

    pub fn isolate_expired_batches(
        &mut self,
        ctx: &AuthContext,
        as_of: NaiveDate,
        now: DateTime<Utc>,
    ) -> Result<Vec<InventoryBatch>, InventoryError> {
        let ids = self
            .batches
            .values()
            .filter(|batch| {
                batch.owner_id == ctx.owner_id
                    && batch.quality_status == STATUS_QUALIFIED
                    && NaiveDate::parse_from_str(&batch.expiry_date, "%Y-%m-%d")
                        .is_ok_and(|expiry| expiry <= as_of)
            })
            .map(|batch| batch.id)
            .collect::<Vec<_>>();
        let mut isolated = Vec::with_capacity(ids.len());
        for id in ids {
            let batch = self.batches.get_mut(&id).ok_or(InventoryError::NotFound)?;
            batch.quality_status = STATUS_UNQUALIFIED.to_string();
            batch.updated_at = now;
            isolated.push(batch.clone());
        }
        Ok(isolated)
    }
}

pub(crate) fn allowed_transition(from: &str, to: &str, source: &str) -> bool {
    let normalized_source = source.trim();
    match (from, to) {
        (STATUS_QUALIFIED, STATUS_QUARANTINED) => matches!(
            normalized_source,
            "质量联系单"
                | "对账差异"
                | "养护异常"
                | "温度超标事件"
                | "M-QL"
                | "M-RC"
                | "M3-MAINT"
                | "M5-TEMP_EXCURSION"
                | "M-TC"
        ),
        (STATUS_QUALIFIED, STATUS_UNQUALIFIED) => normalized_source == APPROVAL_SOURCE_EXPIRY,
        (STATUS_QUARANTINED, STATUS_QUALIFIED | STATUS_UNQUALIFIED) => {
            matches!(normalized_source, "验收结论" | "M2-INSPECTION")
        }
        (STATUS_UNQUALIFIED, STATUS_PENDING_DESTRUCTION) => {
            matches!(normalized_source, "质量联系单" | "M-QL")
        }
        (_, STATUS_LOSS_DEDUCTED) => {
            matches!(
                normalized_source,
                "报损报溢单" | "质量联系单" | "M-SA" | "M-QL"
            )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{ChangeInventoryStatusRequest, PutawayInventoryRequest};

    use super::{
        InventoryError, InventoryStore, STATUS_QUALIFIED, STATUS_QUARANTINED, STATUS_UNQUALIFIED,
    };
    use crate::auth::AuthContext;

    fn ctx(owner_id: Uuid) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: vec!["m3.write".to_string()],
            jti: Uuid::new_v4().to_string(),
        }
    }

    fn putaway_req() -> PutawayInventoryRequest {
        PutawayInventoryRequest {
            product_code: "P-001".to_string(),
            batch_no: "B202606".to_string(),
            production_date: "2026-01-01".to_string(),
            expiry_date: "2028-01-01".to_string(),
            qty: 10,
            quality_status: STATUS_QUALIFIED.to_string(),
            location_id: Uuid::new_v4(),
            location_code: "A-01-01".to_string(),
            source_receiving_order_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn inbound_putaway_increases_owner_scoped_available_inventory() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 13, 0, 0)
            .single()
            .expect("valid time");
        let today = NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date");
        let ctx_a = ctx(Uuid::new_v4());
        let ctx_b = ctx(Uuid::new_v4());
        let mut store = InventoryStore::default();

        let batch = store
            .putaway_from_inbound(&ctx_a, putaway_req(), today, now)
            .expect("putaway");

        assert_eq!(store.available_qty(&ctx_a, batch.id), Ok(10));
        assert!(matches!(
            store.available_qty(&ctx_b, batch.id),
            Err(InventoryError::NotFound)
        ));
    }

    #[test]
    fn inventory_status_transition_requires_allowed_approval_source() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 13, 0, 0)
            .single()
            .expect("valid time");
        let today = NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date");
        let ctx = ctx(Uuid::new_v4());
        let mut store = InventoryStore::default();
        let batch = store
            .putaway_from_inbound(&ctx, putaway_req(), today, now)
            .expect("putaway");

        let missing = store.change_status(
            &ctx,
            ChangeInventoryStatusRequest {
                batch_id: batch.id,
                target_status: STATUS_QUARANTINED.to_string(),
                reason: "temperature exception".to_string(),
                approval_source: "".to_string(),
                approval_id: "".to_string(),
            },
            now,
        );
        assert!(matches!(
            missing,
            Err(InventoryError::MissingApprovalSource)
        ));

        let quarantined = store
            .change_status(
                &ctx,
                ChangeInventoryStatusRequest {
                    batch_id: batch.id,
                    target_status: STATUS_QUARANTINED.to_string(),
                    reason: "temperature exception".to_string(),
                    approval_source: "温度超标事件".to_string(),
                    approval_id: "TEMP-001".to_string(),
                },
                now,
            )
            .expect("status change");

        assert_eq!(quarantined.quality_status, STATUS_QUARANTINED);
        assert_eq!(store.available_qty(&ctx, batch.id), Ok(0));
    }

    #[test]
    fn inventory_status_transition_rejects_blank_reason() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 13, 0, 0)
            .single()
            .expect("valid time");
        let today = NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date");
        let ctx = ctx(Uuid::new_v4());
        let mut store = InventoryStore::default();
        let batch = store
            .putaway_from_inbound(&ctx, putaway_req(), today, now)
            .expect("putaway");

        let error = store
            .change_status(
                &ctx,
                ChangeInventoryStatusRequest {
                    batch_id: batch.id,
                    target_status: STATUS_QUARANTINED.to_string(),
                    reason: " \t".to_string(),
                    approval_source: "M-QL".to_string(),
                    approval_id: "QL-001".to_string(),
                },
                now,
            )
            .expect_err("blank status reason must be rejected");

        assert_eq!(error, InventoryError::InvalidReason);
        assert_eq!(store.available_qty(&ctx, batch.id), Ok(10));
    }

    #[test]
    fn expired_batch_cannot_be_putaway() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 13, 0, 0)
            .single()
            .expect("valid time");
        let today = NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date");
        let ctx = ctx(Uuid::new_v4());
        let mut store = InventoryStore::default();
        let mut req = putaway_req();
        req.expiry_date = "2026-01-01".to_string();

        let result = store.putaway_from_inbound(&ctx, req, today, now);

        assert!(matches!(result, Err(InventoryError::ExpiredBatch)));
    }

    #[test]
    fn expiry_isolation_is_owner_scoped_and_idempotent_in_memory() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 13, 0, 0)
            .single()
            .expect("valid time");
        let today = NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date");
        let owner = ctx(Uuid::new_v4());
        let other = ctx(Uuid::new_v4());
        let mut store = InventoryStore::default();
        let mut expired = putaway_req();
        expired.expiry_date = "2026-06-04".to_string();
        let mut other_expired = expired.clone();
        other_expired.source_receiving_order_id = Uuid::new_v4();
        store
            .putaway_from_inbound(&owner, expired, today, now)
            .expect("putaway");
        store
            .putaway_from_inbound(&other, other_expired, today, now)
            .expect("putaway");

        let isolated = store
            .isolate_expired_batches(&owner, today, now)
            .expect("isolate");
        assert_eq!(isolated.len(), 1);
        assert_eq!(isolated[0].quality_status, STATUS_UNQUALIFIED);
        assert!(store
            .isolate_expired_batches(&owner, today, now)
            .expect("replay")
            .is_empty());
        assert_eq!(store.list_batches(&other).len(), 1);
        assert_eq!(
            store.list_batches(&other)[0].quality_status,
            STATUS_QUALIFIED
        );
    }
}
