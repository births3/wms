//! Wave 2 M2 receiving-order schema and basic CRUD service.

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;
use wms_domain::{
    validate_create_receiving_order_request, CreateReceivingOrderRequest,
    InspectReceivingOrderRequest, InspectionSignatureRecord, PutawayRecord, PutawayRequest,
    ReceiveReceivingOrderRequest, ReceivingInspectionRecord, ReceivingOrder,
    ReceivingOrderPrintData, ReceivingOrderReceipt, ReceivingOrderRequestValidationError,
    RejectReceivingOrderRequest, UpdateReceivingOrderRequest,
    RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND, RECEIVING_DOCUMENT_TYPE_SALES_RETURN,
};

use crate::auth::AuthContext;

mod batch_policy;
use batch_policy::validate_receiving_order_lines;
mod print_data;
#[cfg(test)]
mod print_data_tests;

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
    MissingSupplier,
    MissingExpectedArrival,
    InvalidExpectedArrival,
    MissingProduct,
    MultipleProducts,
    InvalidReason,
    InvalidDocumentType,
    InvalidBatchPolicy,
    BatchExpired,
    SameSigner,
    UnauthorizedSigner,
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
        validate_receiving_order_lines(&req.document_type, &req.lines)?;
        validate_create_receiving_order_request(&req, now).map_err(map_request_validation_error)?;
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
        // G0-F：放行收货前必须绑定供应商（内存路径无法校验资质，仅要求字段存在）
        if order.supplier_id.is_none() {
            return Err(ReceivingOrderError::InvalidReason);
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
        if order.status != "draft" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "draft",
                actual: order.status,
            });
        }
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
        if req.actual_qty < wms_domain::Quantity::ZERO
            || req.shortage_qty < wms_domain::Quantity::ZERO
            || req.rejected_qty < wms_domain::Quantity::ZERO
        {
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
            .sum::<wms_domain::Quantity>();
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
            arrival_temperature_celsius: req.arrival_temperature_celsius,
            exception_note: req.exception_note,
            details: req.details,
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
            .sum::<wms_domain::Quantity>();
        let receipt = ReceivingOrderReceipt {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            actual_qty: wms_domain::Quantity::ZERO,
            shortage_qty: wms_domain::Quantity::ZERO,
            rejected_qty: expected_qty,
            arrival_temperature_celsius: None,
            exception_note: Some(req.reason),
            details: None,
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
        if req.accepted_qty < wms_domain::Quantity::ZERO
            || req.rejected_qty < wms_domain::Quantity::ZERO
        {
            return Err(ReceivingOrderError::InvalidQuantity);
        }
        let inspected_qty = req
            .accepted_qty
            .checked_add(req.rejected_qty)
            .filter(|qty| *qty > wms_domain::Quantity::ZERO)
            .ok_or(ReceivingOrderError::InvalidQuantity)?;
        let expiry_date = NaiveDate::parse_from_str(&req.expiry_date, "%Y-%m-%d")
            .map_err(|_| ReceivingOrderError::BatchExpired)?;
        if expiry_date < today {
            return Err(ReceivingOrderError::BatchExpired);
        }
        if req.batch_no.trim().is_empty() {
            return Err(ReceivingOrderError::InvalidBatchPolicy);
        }

        let received_qty = self
            .receipts
            .values()
            .filter(|receipt| receipt.receiving_order_id == id && receipt.owner_id == ctx.owner_id)
            .try_fold(wms_domain::Quantity::ZERO, |total, receipt| {
                total.checked_add(receipt.actual_qty)
            })
            .ok_or(ReceivingOrderError::InvalidQuantity)?;
        let previous_inspected_qty = self
            .inspections
            .values()
            .filter(|inspection| {
                inspection.receiving_order_id == id && inspection.owner_id == ctx.owner_id
            })
            .try_fold(wms_domain::Quantity::ZERO, |total, inspection| {
                total
                    .checked_add(inspection.accepted_qty)
                    .and_then(|total| total.checked_add(inspection.rejected_qty))
            })
            .ok_or(ReceivingOrderError::InvalidQuantity)?;
        if previous_inspected_qty
            .checked_add(inspected_qty)
            .is_none_or(|qty| qty > received_qty)
        {
            return Err(ReceivingOrderError::QuantityClosureMismatch);
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
        let line = order
            .lines
            .iter_mut()
            .find(|line| match order.document_type.as_str() {
                RECEIVING_DOCUMENT_TYPE_SALES_RETURN => {
                    line.batch_no.as_deref() == Some(req.batch_no.as_str())
                }
                _ => line.batch_no.is_none(),
            })
            .ok_or(ReceivingOrderError::InvalidBatchPolicy)?;
        let line_inspected_qty = self
            .inspections
            .values()
            .filter(|inspection| {
                inspection.receiving_order_id == id
                    && inspection.owner_id == ctx.owner_id
                    && inspection.batch_no == req.batch_no
            })
            .try_fold(wms_domain::Quantity::ZERO, |total, inspection| {
                total
                    .checked_add(inspection.accepted_qty)
                    .and_then(|total| total.checked_add(inspection.rejected_qty))
            })
            .ok_or(ReceivingOrderError::InvalidQuantity)?;
        if line_inspected_qty
            .checked_add(inspected_qty)
            .is_none_or(|qty| qty > line.expected_qty)
        {
            return Err(ReceivingOrderError::QuantityClosureMismatch);
        }
        if order.document_type == RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND {
            line.batch_no = Some(req.batch_no.clone());
        }

        let inspection = ReceivingInspectionRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            batch_no: req.batch_no,
            accepted_qty: req.accepted_qty,
            rejected_qty: req.rejected_qty,
            quality_status: req.quality_status,
            quality_checks: Some(serde_json::json!({
                "appearance": req.appearance_check,
                "package": req.package_check,
                "instruction": req.instruction_check,
                "label": req.label_check,
            })),
            sampling_qty: req.sampling_qty,
            approval_no: req.approval_no,
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
        let order = self.orders.get(&id).ok_or(ReceivingOrderError::NotFound)?;
        if order.owner_id != ctx.owner_id {
            return Err(ReceivingOrderError::NotFound);
        }

        // 第二人独立签字（append-only：追加完整双签记录）
        if order.status == "awaiting_second_sign" {
            let second = req.second_signer_id.unwrap_or(ctx.user_id);
            if second != ctx.user_id {
                return Err(ReceivingOrderError::UnauthorizedSigner);
            }
            let first_record = self
                .signatures
                .values()
                .filter(|item| item.receiving_order_id == id && item.owner_id == ctx.owner_id)
                .max_by_key(|item| item.signed_at)
                .cloned()
                .ok_or(ReceivingOrderError::NotFound)?;
            if first_record.second_signer_id.is_some() {
                return Err(ReceivingOrderError::InvalidStatus {
                    expected: "awaiting_second_sign",
                    actual: "already_fully_signed".to_string(),
                });
            }
            if second == first_record.first_signer_id {
                return Err(ReceivingOrderError::SameSigner);
            }
            let complete = InspectionSignatureRecord {
                id: Uuid::new_v4(),
                receiving_order_id: id,
                owner_id: ctx.owner_id,
                first_signer_id: first_record.first_signer_id,
                second_signer_id: Some(second),
                strategy_rule_id: first_record.strategy_rule_id,
                approval_record_id: first_record.approval_record_id,
                signed_at: now,
            };
            self.signatures.insert(complete.id, complete.clone());
            let order = self
                .orders
                .get_mut(&id)
                .ok_or(ReceivingOrderError::NotFound)?;
            order.status = "putaway".to_string();
            order.updated_at = now;
            return Ok(complete);
        }

        if req.first_signer_id != ctx.user_id {
            return Err(ReceivingOrderError::UnauthorizedSigner);
        }
        if let Some(second) = req.second_signer_id {
            if second == req.first_signer_id {
                return Err(ReceivingOrderError::SameSigner);
            }
        }
        let received_qty = self
            .receipts
            .values()
            .filter(|receipt| receipt.receiving_order_id == id && receipt.owner_id == ctx.owner_id)
            .try_fold(wms_domain::Quantity::ZERO, |total, receipt| {
                total.checked_add(receipt.actual_qty)
            })
            .ok_or(ReceivingOrderError::InvalidQuantity)?;
        let inspected_qty = self
            .inspections
            .values()
            .filter(|inspection| {
                inspection.receiving_order_id == id && inspection.owner_id == ctx.owner_id
            })
            .try_fold(wms_domain::Quantity::ZERO, |total, inspection| {
                total
                    .checked_add(inspection.accepted_qty)
                    .and_then(|total| total.checked_add(inspection.rejected_qty))
            })
            .ok_or(ReceivingOrderError::InvalidQuantity)?;
        if received_qty <= wms_domain::Quantity::ZERO || inspected_qty != received_qty {
            return Err(ReceivingOrderError::QuantityClosureMismatch);
        }
        let all_lines_inspected = self
            .orders
            .get(&id)
            .filter(|order| order.owner_id == ctx.owner_id)
            .map(|order| {
                order.lines.iter().all(|line| {
                    line.batch_no.as_deref().is_some_and(|batch_no| {
                        self.inspections.values().any(|inspection| {
                            inspection.receiving_order_id == id
                                && inspection.owner_id == ctx.owner_id
                                && inspection.batch_no == batch_no
                        })
                    })
                })
            })
            .ok_or(ReceivingOrderError::NotFound)?;
        if !all_lines_inspected {
            return Err(ReceivingOrderError::QuantityClosureMismatch);
        }

        let order = self
            .orders
            .get_mut(&id)
            .ok_or(ReceivingOrderError::NotFound)?;
        if order.status != "inspecting" {
            return Err(ReceivingOrderError::InvalidStatus {
                expected: "inspecting",
                actual: order.status.clone(),
            });
        }
        // 双人策略禁止一次提交两名签字人。
        if req.dual_required && req.second_signer_id.is_some() {
            return Err(ReceivingOrderError::UnauthorizedSigner);
        }

        let signature = InspectionSignatureRecord {
            id: Uuid::new_v4(),
            receiving_order_id: id,
            owner_id: ctx.owner_id,
            first_signer_id: req.first_signer_id,
            second_signer_id: None,
            strategy_rule_id: None,
            approval_record_id: None,
            signed_at: now,
        };
        order.status = if req.dual_required {
            "awaiting_second_sign".to_string()
        } else {
            "putaway".to_string()
        };
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
        if req.qty <= wms_domain::Quantity::ZERO {
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
            lpn_code: None,
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

fn map_request_validation_error(
    error: ReceivingOrderRequestValidationError,
) -> ReceivingOrderError {
    match error {
        ReceivingOrderRequestValidationError::MissingSupplier => {
            ReceivingOrderError::MissingSupplier
        }
        ReceivingOrderRequestValidationError::MissingExpectedArrival => {
            ReceivingOrderError::MissingExpectedArrival
        }
        ReceivingOrderRequestValidationError::InvalidExpectedArrival => {
            ReceivingOrderError::InvalidExpectedArrival
        }
        ReceivingOrderRequestValidationError::MissingProduct => ReceivingOrderError::MissingProduct,
        ReceivingOrderRequestValidationError::MultipleProducts => {
            ReceivingOrderError::MultipleProducts
        }
    }
}

#[cfg(test)]
mod tests;
