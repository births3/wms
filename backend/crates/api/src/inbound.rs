//! Wave 2 M2 receiving-order schema and basic CRUD service.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use wms_domain::{CreateReceivingOrderRequest, ReceivingOrder, UpdateReceivingOrderRequest};

use crate::auth::AuthContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceivingOrderError {
    NotFound,
    DuplicateReceiptNo(String),
    EmptyLines,
}

#[derive(Clone, Debug, Default)]
pub struct ReceivingOrderStore {
    orders: BTreeMap<Uuid, ReceivingOrder>,
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
        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }
        if let Some(value) = req.supplier_id {
            order.supplier_id = Some(value);
        }
        if let Some(value) = req.warehouse_id {
            order.warehouse_id = value;
        }
        if let Some(value) = req.external_ref {
            order.external_ref = Some(value);
        }
        if let Some(value) = req.status {
            order.status = value;
        }
        if let Some(value) = req.expected_arrival_at {
            order.expected_arrival_at = Some(value);
        }
        if let Some(lines) = req.lines {
            if lines.is_empty() {
                return Err(ReceivingOrderError::EmptyLines);
            }
            order.lines = lines;
        }
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
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;
    use wms_domain::{
        CreateReceivingOrderRequest, ReceivingOrderLine, UpdateReceivingOrderRequest,
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
            .update(
                &ctx_a,
                created.id,
                UpdateReceivingOrderRequest {
                    supplier_id: None,
                    warehouse_id: None,
                    external_ref: None,
                    status: Some("released".to_string()),
                    expected_arrival_at: None,
                    lines: None,
                },
                now,
            )
            .expect("update receiving order");
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
}
