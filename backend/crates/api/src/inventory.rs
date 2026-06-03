//! Wave 3 M3 inventory domain service.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;
use wms_domain::{
    ChangeInventoryStatusRequest, InventoryBatch, InventoryMovement, PutawayInventoryRequest,
};

use crate::auth::AuthContext;

pub const STATUS_QUALIFIED: &str = "qualified";
pub const STATUS_QUARANTINED: &str = "quarantined";
pub const STATUS_UNQUALIFIED: &str = "unqualified";
pub const STATUS_PENDING_DESTRUCTION: &str = "pending_destruction";
pub const STATUS_LOSS_DEDUCTED: &str = "loss_deducted";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryError {
    NotFound,
    InvalidQuantity,
    ExpiredBatch,
    MissingApprovalSource,
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
            let batch = self.batches.get_mut(&id).expect("existing batch id");
            batch.qty_on_hand += req.qty;
            batch.updated_at = now;
            batch.clone()
        } else {
            let batch = InventoryBatch {
                id: Uuid::new_v4(),
                owner_id: ctx.owner_id,
                product_code: req.product_code.clone(),
                batch_no: req.batch_no.clone(),
                production_date: req.production_date,
                expiry_date: req.expiry_date,
                qty_on_hand: req.qty,
                qty_locked: 0,
                quality_status: req.quality_status,
                location_id: req.location_id,
                location_code: req.location_code,
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
        ),
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

    use super::{InventoryError, InventoryStore, STATUS_QUALIFIED, STATUS_QUARANTINED};
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
}
