//! Wave 2 M2 receiving-order schema and basic CRUD service.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;
use wms_domain::{
    CreateReceivingOrderRequest, InspectReceivingOrderRequest, InspectionSignatureRecord,
    PutawayRecord, PutawayRequest, ReceiveReceivingOrderRequest, ReceivingInspectionRecord,
    ReceivingOrder, ReceivingOrderReceipt, RejectReceivingOrderRequest,
    UpdateReceivingOrderRequest, RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND,
    RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
};

use crate::auth::AuthContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceivingOrderError {
    NotFound,
    DuplicateReceiptNo(String),
    EmptyLines,
    InvalidStatus {
        expected: &'static str,
        actual: String,
    },
    QuantityClosureMismatch,
    OverReceiptNotAllowed,
    InvalidQuantity,
    InvalidReason,
    InvalidDocumentType,
    BatchExpired,
    SameSigner,
    MissingSecondSigner,
}

#[derive(Clone, Debug, Default)]
pub struct ReceivingOrderStore {
    orders: BTreeMap<Uuid, ReceivingOrder>,
    receipts: BTreeMap<Uuid, ReceivingOrderReceipt>,
    inspections: BTreeMap<Uuid, ReceivingInspectionRecord>,
    signatures: BTreeMap<Uuid, InspectionSignatureRecord>,
    putaways: BTreeMap<Uuid, PutawayRecord>,
}

impl ReceivingOrderStore {
    pub fn create(
        &mut self,
        ctx: &AuthContext,
        req: CreateReceivingOrderRequest,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, ReceivingOrderError> {
        if req.lines.is_empty() {
            return Err(ReceivingOrderError::EmptyLines);
        }
        validate_document_type(&req.document_type)?;
        if self
            .orders
            .values()
            .any(|order| order.owner_id == ctx.owner_id && order.receipt_no == req.receipt_no)
        {
            return Err(ReceivingOrderError::DuplicateReceiptNo(req.receipt_no));
        }

        let order = ReceivingOrder {
            id: Uuid::new_v4(),
            owner_id: ctx.owner_id,
            receipt_no: req.receipt_no,
            document_type: req.document_type,
            supplier_id: req.supplier_id,
            warehouse_id: req.warehouse_id,
            external_ref: req.external_ref,
            status: "draft".to_string(),
            expected_arrival_at: req.expected_arrival_at,
            lines: req.lines,
            created_at: now,
            updated_at: now,
        };
        self.orders.insert(order.id, order.clone());
        Ok(order)
    }

    pub fn list(&self, ctx: &AuthContext) -> Vec<ReceivingOrder> {
        self.orders
            .values()
            .filter(|order| order.owner_id == ctx.owner_id)
            .cloned()
            .collect()
    }

    pub fn get(&self, ctx: &AuthContext, id: Uuid) -> Result<ReceivingOrder, ReceivingOrderError> {
        self.orders
            .get(&id)
            .filter(|order| order.owner_id == ctx.owner_id)
            .cloned()
            .ok_or(ReceivingOrderError::NotFound)
    }

    pub fn update(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: UpdateReceivingOrderRequest,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, ReceivingOrderError> {
        if req.lines.as_ref().is_some_and(Vec::is_empty) {
            return Err(ReceivingOrderError::EmptyLines);
        }
        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }
        if order.status != "draft" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "draft",
                actual: order.status.clone(),
            });
        }
        if let Some(value) = req.supplier_id {
            order.supplier_id = Some(value);
        }
        if let Some(value) = req.warehouse_id {
            order.warehouse_id = value;
        }
        if let Some(value) = req.external_ref {
            order.external_ref = value;
        }
        if let Some(value) = req.expected_arrival_at {
            order.expected_arrival_at = Some(value);
        }
        if let Some(lines) = req.lines {
            order.lines = lines;
        }
        order.updated_at = now;
        Ok(order.clone())
    }

    pub fn release(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrder, ReceivingOrderError> {
        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }
        if order.status != "draft" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "draft",
                actual: order.status.clone(),
            });
        }
        order.status = "released".to_string();
        order.updated_at = now;
        Ok(order.clone())
    }

    pub fn delete(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
    ) -> Result<ReceivingOrder, ReceivingOrderError> {
        let order = self.get(ctx, id)?;
        self.orders.remove(&id);
        Ok(order)
    }

    pub fn receive(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: ReceiveReceivingOrderRequest,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrderReceipt, ReceivingOrderError> {
        if req.actual_qty < 0 || req.shortage_qty < 0 || req.rejected_qty < 0 {
            return Err(ReceivingOrderError::InvalidQuantity);
        }

        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }
        if order.status != "released" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "released",
                actual: order.status.clone(),
            });
        }

        let expected_qty = order
            .lines
            .iter()
            .map(|line| line.expected_qty)
            .sum::<i64>();
        if req.actual_qty > expected_qty {
            return Err(ReceivingOrderError::OverReceiptNotAllowed);
        }
        if req.actual_qty + req.shortage_qty + req.rejected_qty != expected_qty {
            return Err(ReceivingOrderError::QuantityClosureMismatch);
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
        order.status = "inspecting".to_string();
        order.updated_at = now;
        self.receipts.insert(receipt.id, receipt.clone());
        Ok(receipt)
    }

    pub fn reject(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: RejectReceivingOrderRequest,
        now: DateTime<Utc>,
    ) -> Result<ReceivingOrderReceipt, ReceivingOrderError> {
        if req.reason.trim().is_empty() {
            return Err(ReceivingOrderError::InvalidReason);
        }

        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }
        if order.status != "released" && order.status != "receiving" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "released/receiving",
                actual: order.status.clone(),
            });
        }

        let expected_qty = order
            .lines
            .iter()
            .map(|line| line.expected_qty)
            .sum::<i64>();
        let receipt = ReceivingOrderReceipt {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            actual_qty: 0,
            shortage_qty: 0,
            rejected_qty: expected_qty,
            occurred_at: now,
        };
        order.status = "closed_rejected".to_string();
        order.updated_at = now;
        self.receipts.insert(receipt.id, receipt.clone());
        Ok(receipt)
    }

    pub fn inspect(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: InspectReceivingOrderRequest,
        today: NaiveDate,
        now: DateTime<Utc>,
    ) -> Result<ReceivingInspectionRecord, ReceivingOrderError> {
        if req.accepted_qty < 0 || req.rejected_qty < 0 {
            return Err(ReceivingOrderError::InvalidQuantity);
        }
        let expiry_date = NaiveDate::parse_from_str(&req.expiry_date, "%Y-%m-%d")
            .map_err(|_| ReceivingOrderError::BatchExpired)?;
        if expiry_date < today {
            return Err(ReceivingOrderError::BatchExpired);
        }

        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }
        if order.status != "inspecting" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "inspecting",
                actual: order.status.clone(),
            });
        }

        let inspection = ReceivingInspectionRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no,
            accepted_qty: req.accepted_qty,
            rejected_qty: req.rejected_qty,
            quality_status: req.quality_status,
            occurred_at: now,
        };
        self.inspections.insert(inspection.id, inspection.clone());
        Ok(inspection)
    }

    pub fn sign_inspection(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: wms_domain::SignInspectionRequest,
        now: DateTime<Utc>,
    ) -> Result<InspectionSignatureRecord, ReceivingOrderError> {
        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }
        if order.status != "inspecting" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "inspecting",
                actual: order.status.clone(),
            });
        }
        if req.dual_required {
            let second = req
                .second_signer_id
                .ok_or(ReceivingOrderError::MissingSecondSigner)?;
            if second == req.first_signer_id {
                return Err(ReceivingOrderError::SameSigner);
            }
        }

        let signature = InspectionSignatureRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            first_signer_id: req.first_signer_id,
            second_signer_id: req.second_signer_id,
            signed_at: now,
        };
        order.status = "putaway".to_string();
        order.updated_at = now;
        self.signatures.insert(signature.id, signature.clone());
        Ok(signature)
    }

    pub fn putaway(
        &mut self,
        ctx: &AuthContext,
        id: Uuid,
        req: PutawayRequest,
        now: DateTime<Utc>,
    ) -> Result<PutawayRecord, ReceivingOrderError> {
        if req.qty <= 0 {
            return Err(ReceivingOrderError::InvalidQuantity);
        }
        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }
        if order.status != "putaway" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "putaway",
                actual: order.status.clone(),
            });
        }

        let record = PutawayRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no,
            product_code: req.product_code,
            qty: req.qty,
            location_id: req.location_id,
            location_code: req.location_code,
            occurred_at: now,
        };
        order.status = "completed".to_string();
        order.updated_at = now;
        self.putaways.insert(record.id, record.clone());
        Ok(record)
    }
}

fn validate_document_type(value: &str) -> Result<(), ReceivingOrderError> {
    match value {
        RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND | RECEIVING_DOCUMENT_TYPE_SALES_RETURN => Ok(()),
        _ => Err(ReceivingOrderError::InvalidDocumentType),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{
        CreateReceivingOrderRequest, InspectReceivingOrderRequest, PutawayRequest,
        ReceiveReceivingOrderRequest, ReceivingOrderLine, RejectReceivingOrderRequest,
        UpdateReceivingOrderRequest,
    };

    use super::{ReceivingOrderError, ReceivingOrderStore};
    use crate::auth::AuthContext;

    fn ctx(owner_id: Uuid) -> AuthContext {
        AuthContext {
            user_id: Uuid::new_v4(),
            owner_id,
            actor_name: "tester".to_string(),
            permissions: vec!["m2.write".to_string()],
            jti: Uuid::new_v4().to_string(),
        }
    }

    fn line() -> ReceivingOrderLine {
        ReceivingOrderLine {
            line_no: 1,
            product_id: None,
            product_code: "P-001".to_string(),
            expected_qty: 10,
            batch_no: Some("B202606".to_string()),
            production_date: Some("2026-01-01".to_string()),
            expiry_date: Some("2028-01-01".to_string()),
        }
    }

    #[test]
    fn receiving_order_crud_is_owner_scoped() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let ctx_a = ctx(Uuid::new_v4());
        let ctx_b = ctx(Uuid::new_v4());
        let mut store = ReceivingOrderStore::default();

        let created = store
            .create(
                &ctx_a,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-001".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: Some("ERP-ASN-001".to_string()),
                    expected_arrival_at: None,
                    lines: vec![line()],
                },
                now,
            )
            .expect("create receiving order");

        assert_eq!(store.list(&ctx_a).len(), 1);
        assert!(matches!(
            store.get(&ctx_b, created.id),
            Err(ReceivingOrderError::NotFound)
        ));

        let updated = store
            .release(&ctx_a, created.id, now)
            .expect("release receiving order");
        assert_eq!(updated.status, "released");

        store
            .delete(&ctx_a, created.id)
            .expect("delete receiving order");
        assert!(store.list(&ctx_a).is_empty());
    }

    #[test]
    fn receiving_order_requires_lines() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = ReceivingOrderStore::default();

        let result = store.create(
            &ctx,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-EMPTY".to_string(),
                document_type: "purchase_inbound".to_string(),
                supplier_id: None,
                warehouse_id: Uuid::new_v4(),
                external_ref: None,
                expected_arrival_at: None,
                lines: vec![],
            },
            now,
        );

        assert!(matches!(result, Err(ReceivingOrderError::EmptyLines)));
    }

    #[test]
    fn receiving_order_update_validation_is_atomic() {
        let now = Utc::now();
        let ctx = ctx(Uuid::new_v4());
        let mut store = ReceivingOrderStore::default();
        let created = store
            .create(
                &ctx,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-ATOMIC-001".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![line()],
                },
                now,
            )
            .expect("create order");

        let result = store.update(
            &ctx,
            created.id,
            UpdateReceivingOrderRequest {
                supplier_id: Some(Uuid::new_v4()),
                warehouse_id: Some(Uuid::new_v4()),
                external_ref: Some(Some("MUST-NOT-PERSIST".to_string())),
                expected_arrival_at: None,
                lines: Some(Vec::new()),
            },
            now,
        );

        assert!(matches!(result, Err(ReceivingOrderError::EmptyLines)));
        assert_eq!(
            store.get(&ctx, created.id).expect("order").external_ref,
            None
        );
    }

    #[test]
    fn receiving_order_update_can_clear_external_reference() {
        let now = Utc::now();
        let ctx = ctx(Uuid::new_v4());
        let mut store = ReceivingOrderStore::default();
        let created = store
            .create(
                &ctx,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-CLEAR-001".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: Some(Uuid::new_v4()),
                    warehouse_id: Uuid::new_v4(),
                    external_ref: Some("ERP-CLEAR-001".to_string()),
                    expected_arrival_at: Some(now),
                    lines: vec![line()],
                },
                now,
            )
            .expect("create order");

        let updated = store
            .update(
                &ctx,
                created.id,
                UpdateReceivingOrderRequest {
                    supplier_id: None,
                    warehouse_id: None,
                    external_ref: Some(None),
                    expected_arrival_at: None,
                    lines: None,
                },
                now,
            )
            .expect("clear nullable fields");

        assert_eq!(updated.supplier_id, created.supplier_id);
        assert_eq!(updated.external_ref, None);
        assert_eq!(updated.expected_arrival_at, created.expected_arrival_at);
    }

    #[test]
    fn receiving_order_rejects_invalid_document_type() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = ReceivingOrderStore::default();

        let result = store.create(
            &ctx,
            CreateReceivingOrderRequest {
                receipt_no: "ASN-BAD-TYPE".to_string(),
                document_type: "purchase_return".to_string(),
                supplier_id: None,
                warehouse_id: Uuid::new_v4(),
                external_ref: None,
                expected_arrival_at: None,
                lines: vec![line()],
            },
            now,
        );

        assert!(matches!(
            result,
            Err(ReceivingOrderError::InvalidDocumentType)
        ));
    }

    #[test]
    fn receiving_workflow_enforces_quantity_closure_and_dual_signature() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = ReceivingOrderStore::default();
        let created = store
            .create(
                &ctx,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-W3-001".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![line()],
                },
                now,
            )
            .expect("create order");
        store.release(&ctx, created.id, now).expect("release order");

        let mismatch = store.receive(
            &ctx,
            created.id,
            ReceiveReceivingOrderRequest {
                actual_qty: 8,
                shortage_qty: 1,
                rejected_qty: 0,
                arrival_temperature_celsius: None,
                exception_note: None,
            },
            now,
        );
        assert!(matches!(
            mismatch,
            Err(ReceivingOrderError::QuantityClosureMismatch)
        ));

        let receipt = store
            .receive(
                &ctx,
                created.id,
                ReceiveReceivingOrderRequest {
                    actual_qty: 8,
                    shortage_qty: 2,
                    rejected_qty: 0,
                    arrival_temperature_celsius: None,
                    exception_note: None,
                },
                now,
            )
            .expect("closed receipt");
        assert_eq!(receipt.actual_qty, 8);

        store
            .inspect(
                &ctx,
                created.id,
                InspectReceivingOrderRequest {
                    batch_no: "B202606".to_string(),
                    accepted_qty: 8,
                    rejected_qty: 0,
                    production_date: "2026-01-01".to_string(),
                    expiry_date: "2028-01-01".to_string(),
                    quality_status: "qualified".to_string(),
                    trace_codes: vec![],
                },
                chrono::NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date"),
                now,
            )
            .expect("inspect");

        let same_signer = store.sign_inspection(
            &ctx,
            created.id,
            wms_domain::SignInspectionRequest {
                first_signer_id: ctx.user_id,
                second_signer_id: Some(ctx.user_id),
                dual_required: true,
            },
            now,
        );
        assert!(matches!(same_signer, Err(ReceivingOrderError::SameSigner)));

        let signature = store
            .sign_inspection(
                &ctx,
                created.id,
                wms_domain::SignInspectionRequest {
                    first_signer_id: ctx.user_id,
                    second_signer_id: Some(Uuid::new_v4()),
                    dual_required: true,
                },
                now,
            )
            .expect("sign");
        assert_eq!(signature.owner_id, ctx.owner_id);

        let putaway = store
            .putaway(
                &ctx,
                created.id,
                PutawayRequest {
                    batch_no: "B202606".to_string(),
                    product_code: "P-001".to_string(),
                    qty: 8,
                    location_id: Uuid::new_v4(),
                    location_code: "A-01-01".to_string(),
                    quality_status: "qualified".to_string(),
                },
                now,
            )
            .expect("putaway");
        assert_eq!(putaway.qty, 8);
        assert_eq!(
            store.get(&ctx, created.id).expect("get").status,
            "completed"
        );
    }

    #[test]
    fn receiving_order_reject_accepts_receiving_status_and_closes_order() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = ReceivingOrderStore::default();
        let created = store
            .create(
                &ctx,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-W3-REJECT".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![line()],
                },
                now,
            )
            .expect("create order");
        store.release(&ctx, created.id, now).expect("release order");

        let receipt = store
            .reject(
                &ctx,
                created.id,
                RejectReceivingOrderRequest {
                    reason: "外包装严重破损".to_string(),
                },
                now,
            )
            .expect("reject receiving order");

        assert_eq!(receipt.rejected_qty, 10);
        assert_eq!(
            store.get(&ctx, created.id).expect("get").status,
            "closed_rejected"
        );
    }

    #[test]
    fn receiving_inspection_rejects_expired_batch() {
        let now = Utc
            .with_ymd_and_hms(2026, 6, 4, 10, 0, 0)
            .single()
            .expect("valid time");
        let ctx = ctx(Uuid::new_v4());
        let mut store = ReceivingOrderStore::default();
        let created = store
            .create(
                &ctx,
                CreateReceivingOrderRequest {
                    receipt_no: "ASN-W3-002".to_string(),
                    document_type: "purchase_inbound".to_string(),
                    supplier_id: None,
                    warehouse_id: Uuid::new_v4(),
                    external_ref: None,
                    expected_arrival_at: None,
                    lines: vec![line()],
                },
                now,
            )
            .expect("create order");
        store.release(&ctx, created.id, now).expect("release order");
        store
            .receive(
                &ctx,
                created.id,
                ReceiveReceivingOrderRequest {
                    actual_qty: 10,
                    shortage_qty: 0,
                    rejected_qty: 0,
                    arrival_temperature_celsius: None,
                    exception_note: None,
                },
                now,
            )
            .expect("receive");

        let result = store.inspect(
            &ctx,
            created.id,
            InspectReceivingOrderRequest {
                batch_no: "B-EXPIRED".to_string(),
                accepted_qty: 1,
                rejected_qty: 0,
                production_date: "2025-01-01".to_string(),
                expiry_date: "2026-01-01".to_string(),
                quality_status: "qualified".to_string(),
                trace_codes: vec![],
            },
            chrono::NaiveDate::from_ymd_opt(2026, 6, 4).expect("valid date"),
            now,
        );

        assert!(matches!(result, Err(ReceivingOrderError::BatchExpired)));
    }
}
