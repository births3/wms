// @governance: skip-page-size 入库与出库共享 OpenAPI DTO 正在随 M4 分组收口，本次仅扩展 H9 标准归集字段，避免在脏工作区扩大跨故事重构。
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::common::PageMeta;
use crate::Quantity;

/// 收货单明细。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrderLine {
    /// 明细行号
    pub line_no: u32,
    /// 商品主数据 ID
    pub product_id: Option<Uuid>,
    /// 商品编码
    pub product_code: String,
    /// 预计数量
    #[schema(value_type = String, format = "decimal")]
    pub expected_qty: Quantity,
    /// 批号
    pub batch_no: Option<String>,
    /// 生产日期
    pub production_date: Option<String>,
    /// 有效期至
    pub expiry_date: Option<String>,
}

pub const RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND: &str = "purchase_inbound";
pub const RECEIVING_DOCUMENT_TYPE_SALES_RETURN: &str = "sales_return";

/// 收货单。
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrder {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub receipt_no: String,
    pub document_type: String,
    pub supplier_id: Option<Uuid>,
    pub warehouse_id: Uuid,
    pub external_ref: Option<String>,
    pub status: String,
    pub expected_arrival_at: Option<DateTime<Utc>>,
    pub lines: Vec<ReceivingOrderLine>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateReceivingOrderRequest {
    /// 收货单号
    pub receipt_no: String,
    /// 单据类型
    pub document_type: String,
    /// 供应商 ID
    pub supplier_id: Option<Uuid>,
    /// 仓库 ID
    pub warehouse_id: Uuid,
    /// 外部来源号
    pub external_ref: Option<String>,
    /// 预计到货时间
    pub expected_arrival_at: Option<DateTime<Utc>>,
    /// 收货明细
    pub lines: Vec<ReceivingOrderLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceivingOrderRequestValidationError {
    MissingSupplier,
    MissingExpectedArrival,
    InvalidExpectedArrival,
    MissingProduct,
    MultipleProducts,
}

/// 校验 ASN 创建时跨内存仓储和 PostgreSQL 都必须满足的请求不变量。
pub fn validate_create_receiving_order_request(
    request: &CreateReceivingOrderRequest,
    now: DateTime<Utc>,
) -> Result<(), ReceivingOrderRequestValidationError> {
    if request.document_type == RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND
        && request.supplier_id.is_none()
    {
        return Err(ReceivingOrderRequestValidationError::MissingSupplier);
    }

    let expected_arrival_at = request
        .expected_arrival_at
        .ok_or(ReceivingOrderRequestValidationError::MissingExpectedArrival)?;
    if expected_arrival_at.date_naive() < now.date_naive() {
        return Err(ReceivingOrderRequestValidationError::InvalidExpectedArrival);
    }

    let Some(first_line) = request.lines.first() else {
        return Err(ReceivingOrderRequestValidationError::MissingProduct);
    };
    if first_line.product_code.trim().is_empty() {
        return Err(ReceivingOrderRequestValidationError::MissingProduct);
    }

    for line in request.lines.iter().skip(1) {
        if line.product_code.trim().is_empty() {
            return Err(ReceivingOrderRequestValidationError::MissingProduct);
        }
        if line.product_code.trim() != first_line.product_code.trim()
            || first_line.product_id != line.product_id
        {
            return Err(ReceivingOrderRequestValidationError::MultipleProducts);
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateReceivingOrderRequest {
    pub supplier_id: Option<Uuid>,
    pub warehouse_id: Option<Uuid>,
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_patch",
        skip_serializing_if = "Option::is_none"
    )]
    pub external_ref: Option<Option<String>>,
    pub expected_arrival_at: Option<DateTime<Utc>>,
    pub lines: Option<Vec<ReceivingOrderLine>>,
}

fn deserialize_nullable_patch<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReceivingOrderListResponse {
    pub data: Vec<ReceivingOrder>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOutboundOrderLineRequest {
    pub line_no: u32,
    pub product_code: String,
    pub batch_no: String,
    #[schema(value_type = String, format = "decimal")]
    pub planned_qty: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundOrderLine {
    pub line_no: u32,
    pub product_code: String,
    pub batch_no: String,
    #[schema(value_type = String, format = "decimal")]
    pub planned_qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub picked_qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub reviewed_qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub shipped_qty: Quantity,
    #[schema(value_type = String, format = "decimal")]
    pub short_pick_qty: Quantity,
}

pub const REVIEW_MODE_PACKING_STATION: &str = "packing_station";
pub const REVIEW_MODE_PDA_FULL_CASE: &str = "pda_full_case";
pub const REVIEW_MODE_PDA_LOOSE: &str = "pda_loose";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReviewOutboundOrderLineRequest {
    pub line_no: u32,
    pub product_code: String,
    #[schema(value_type = String, format = "decimal")]
    pub reviewed_qty: Quantity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewValidationError {
    EmptyOrderLines,
    EmptyReviewLines,
    InvalidReviewMode,
    ReviewerMismatch,
    SameOperator,
    InvalidSecondReviewer,
    DuplicateLine(u32),
    MissingLine(u32),
    UnexpectedLine(u32),
    ProductMismatch(u32),
    InvalidQuantity(u32),
    QuantityMismatch {
        line_no: u32,
        expected: Quantity,
        actual: Quantity,
    },
}

pub fn validate_review_submission(
    order_lines: &[OutboundOrderLine],
    request: &ReviewOutboundOrderRequest,
    actor_id: Uuid,
    pick_operator_ids: &[Uuid],
) -> Result<(), ReviewValidationError> {
    if order_lines.is_empty() {
        return Err(ReviewValidationError::EmptyOrderLines);
    }
    if request.lines.is_empty() {
        return Err(ReviewValidationError::EmptyReviewLines);
    }
    if !matches!(
        request.review_mode.as_str(),
        REVIEW_MODE_PACKING_STATION | REVIEW_MODE_PDA_FULL_CASE | REVIEW_MODE_PDA_LOOSE
    ) {
        return Err(ReviewValidationError::InvalidReviewMode);
    }
    if request.reviewer_id != actor_id {
        return Err(ReviewValidationError::ReviewerMismatch);
    }
    if pick_operator_ids.contains(&request.reviewer_id) {
        return Err(ReviewValidationError::SameOperator);
    }
    if let Some(second_reviewer_id) = request.second_reviewer_id {
        if second_reviewer_id == request.reviewer_id
            || pick_operator_ids.contains(&second_reviewer_id)
        {
            return Err(ReviewValidationError::InvalidSecondReviewer);
        }
    }

    let mut seen = HashSet::with_capacity(request.lines.len());
    for reviewed_line in &request.lines {
        if !seen.insert(reviewed_line.line_no) {
            return Err(ReviewValidationError::DuplicateLine(reviewed_line.line_no));
        }
        let Some(order_line) = order_lines
            .iter()
            .find(|line| line.line_no == reviewed_line.line_no)
        else {
            return Err(ReviewValidationError::UnexpectedLine(reviewed_line.line_no));
        };
        if order_line.product_code != reviewed_line.product_code {
            return Err(ReviewValidationError::ProductMismatch(
                reviewed_line.line_no,
            ));
        }
        if order_line.picked_qty < Quantity::ZERO
            || order_line.picked_qty > order_line.planned_qty
            || order_line.short_pick_qty != order_line.planned_qty - order_line.picked_qty
            || reviewed_line.reviewed_qty < Quantity::ZERO
            || reviewed_line.reviewed_qty > order_line.picked_qty
        {
            return Err(ReviewValidationError::InvalidQuantity(
                reviewed_line.line_no,
            ));
        }
        if reviewed_line.reviewed_qty != order_line.picked_qty {
            return Err(ReviewValidationError::QuantityMismatch {
                line_no: reviewed_line.line_no,
                expected: order_line.picked_qty,
                actual: reviewed_line.reviewed_qty,
            });
        }
    }

    for order_line in order_lines {
        if !seen.contains(&order_line.line_no) {
            return Err(ReviewValidationError::MissingLine(order_line.line_no));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOutboundOrderRequest {
    pub document_type: String,
    pub wms_order_no: String,
    pub erp_order_no: Option<String>,
    pub invoice_no: Option<String>,
    pub transport_mode_code: Option<String>,
    pub department_code: Option<String>,
    pub sales_group_code: Option<String>,
    pub order_group_no: Option<String>,
    pub business_type_code: Option<String>,
    pub customer_id: Uuid,
    pub delivery_address_id: Uuid,
    pub warehouse_id: Uuid,
    pub required_ship_at: Option<DateTime<Utc>>,
    pub lines: Vec<CreateOutboundOrderLineRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundOrder {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub document_type: String,
    pub wms_order_no: String,
    pub erp_order_no: Option<String>,
    pub invoice_no: Option<String>,
    pub transport_mode_code: Option<String>,
    pub department_code: Option<String>,
    pub sales_group_code: Option<String>,
    pub order_group_no: Option<String>,
    pub business_type_code: Option<String>,
    pub customer_id: Uuid,
    pub delivery_address_id: Uuid,
    pub delivery_address_snapshot: serde_json::Value,
    pub warehouse_id: Uuid,
    pub required_ship_at: Option<DateTime<Utc>>,
    pub status: String,
    pub short_pick: bool,
    pub lines: Vec<OutboundOrderLine>,
    pub shipment: Option<OutboundShipment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundOrderListResponse {
    pub data: Vec<OutboundOrder>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateOutboundWaveRequest {
    pub wave_no: String,
    pub order_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundWave {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub wave_no: String,
    pub status: String,
    pub order_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundWaveListResponse {
    pub data: Vec<OutboundWave>,
    pub page: PageMeta,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CompletePickTaskRequest {
    pub line_no: u32,
    #[schema(value_type = String, format = "decimal")]
    pub picked_qty: Quantity,
    pub exception_code: Option<String>,
    pub exception_note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ReviewOutboundOrderRequest {
    pub reviewer_id: Uuid,
    pub review_mode: String,
    pub second_reviewer_id: Option<Uuid>,
    pub lines: Vec<ReviewOutboundOrderLineRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundColdChainPackage {
    pub insulated_container_no: String,
    pub ice_pack_count: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct OutboundShipment {
    pub id: Uuid,
    pub delivery_provider_type: String,
    pub vehicle_no: Option<String>,
    pub plate_no: String,
    pub driver_user_id: Option<Uuid>,
    pub driver_name: Option<String>,
    pub courier_name: Option<String>,
    pub courier_phone: Option<String>,
    pub signature_attachment_id: Option<Uuid>,
    pub cold_chain: bool,
    pub loading_temperature_celsius: Option<f64>,
    pub cold_chain_packages: Vec<OutboundColdChainPackage>,
    pub package_count: u32,
    pub handover_by: Uuid,
    pub shipped_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ShipOutboundOrderRequest {
    pub delivery_provider_type: String,
    pub vehicle_no: Option<String>,
    pub plate_no: String,
    pub driver_user_id: Option<Uuid>,
    pub courier_name: Option<String>,
    pub courier_phone: Option<String>,
    pub signature_attachment_id: Option<Uuid>,
    pub loading_temperature_celsius: Option<f64>,
    #[serde(default)]
    pub cold_chain_packages: Vec<OutboundColdChainPackage>,
    pub package_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShipOutboundValidationError {
    InvalidCarrierType,
    InvalidPackageCount,
    FieldRequired(&'static str),
    FieldForbidden(&'static str),
    FieldTooLong(&'static str),
    InvalidIdentifier(&'static str),
    InvalidTemperature,
    InvalidColdChainPackageCount,
}

pub fn validate_ship_outbound_request(
    request: &ShipOutboundOrderRequest,
    requires_cold_chain: bool,
) -> Result<(), ShipOutboundValidationError> {
    if !matches!(
        request.delivery_provider_type.as_str(),
        "own_fleet" | "third_party_express"
    ) {
        return Err(ShipOutboundValidationError::InvalidCarrierType);
    }
    if request.package_count == 0 {
        return Err(ShipOutboundValidationError::InvalidPackageCount);
    }
    required_text(&request.plate_no, "plate_no", 32)?;
    if request
        .signature_attachment_id
        .is_some_and(|value| value.is_nil())
    {
        return Err(ShipOutboundValidationError::InvalidIdentifier(
            "signature_attachment_id",
        ));
    }

    match request.delivery_provider_type.as_str() {
        "own_fleet" => {
            required_option_text(&request.vehicle_no, "vehicle_no", 64)?;
            if request.driver_user_id.is_none_or(|value| value.is_nil()) {
                return Err(ShipOutboundValidationError::FieldRequired("driver_user_id"));
            }
            forbidden_option(&request.courier_name, "courier_name")?;
            forbidden_option(&request.courier_phone, "courier_phone")?;
        }
        "third_party_express" => {
            if request.vehicle_no.is_some() {
                return Err(ShipOutboundValidationError::FieldForbidden("vehicle_no"));
            }
            if request.driver_user_id.is_some() {
                return Err(ShipOutboundValidationError::FieldForbidden(
                    "driver_user_id",
                ));
            }
            required_option_text(&request.courier_name, "courier_name", 128)?;
            required_option_text(&request.courier_phone, "courier_phone", 32)?;
            if request.signature_attachment_id.is_none() {
                return Err(ShipOutboundValidationError::FieldRequired(
                    "signature_attachment_id",
                ));
            }
        }
        _ => return Err(ShipOutboundValidationError::InvalidCarrierType),
    }

    if requires_cold_chain {
        let temperature = request.loading_temperature_celsius.ok_or(
            ShipOutboundValidationError::FieldRequired("loading_temperature_celsius"),
        )?;
        if !temperature.is_finite() || !(-100.0..=100.0).contains(&temperature) {
            return Err(ShipOutboundValidationError::InvalidTemperature);
        }
        if request.cold_chain_packages.is_empty() {
            return Err(ShipOutboundValidationError::FieldRequired(
                "cold_chain_packages",
            ));
        }
        let package_count = usize::try_from(request.package_count)
            .map_err(|_| ShipOutboundValidationError::InvalidPackageCount)?;
        if request.cold_chain_packages.len() > package_count {
            return Err(ShipOutboundValidationError::InvalidColdChainPackageCount);
        }
        for package in &request.cold_chain_packages {
            required_text(
                &package.insulated_container_no,
                "insulated_container_no",
                64,
            )?;
        }
    } else if request.loading_temperature_celsius.is_some()
        || !request.cold_chain_packages.is_empty()
    {
        return Err(ShipOutboundValidationError::FieldForbidden(
            "cold_chain_fields",
        ));
    }
    Ok(())
}

fn required_option_text(
    value: &Option<String>,
    field: &'static str,
    max_chars: usize,
) -> Result<(), ShipOutboundValidationError> {
    let value = value
        .as_deref()
        .ok_or(ShipOutboundValidationError::FieldRequired(field))?;
    required_text(value, field, max_chars)
}

fn required_text(
    value: &str,
    field: &'static str,
    max_chars: usize,
) -> Result<(), ShipOutboundValidationError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ShipOutboundValidationError::FieldRequired(field));
    }
    if value.chars().count() > max_chars {
        return Err(ShipOutboundValidationError::FieldTooLong(field));
    }
    Ok(())
}

fn forbidden_option(
    value: &Option<String>,
    field: &'static str,
) -> Result<(), ShipOutboundValidationError> {
    if value.is_some() {
        Err(ShipOutboundValidationError::FieldForbidden(field))
    } else {
        Ok(())
    }
}

/// M4 采购退货出库固定单据类型。
pub const PURCHASE_RETURN_DOCUMENT_TYPE: &str = "purchase_return_outbound";
/// M4 采购退货出库固定审批来源。
pub const PURCHASE_RETURN_APPROVAL_SOURCE: &str = "purchase_return_approval";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePurchaseReturnRequest {
    pub return_no: String,
    pub source_purchase_order_no: String,
    pub supplier_id: Option<Uuid>,
    pub supplier_name: String,
    pub reason: String,
    pub warehouse_id: Uuid,
    pub product_code: String,
    #[schema(value_type = String, format = "decimal")]
    pub qty: Quantity,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RejectPurchaseReturnRequest {
    /// 驳回原因（必填，不允许空白）。
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PurchaseReturnOrder {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub warehouse_id: Uuid,
    pub return_no: String,
    pub document_type: String,
    pub source_purchase_order_no: String,
    pub supplier_id: Option<Uuid>,
    pub supplier_name: String,
    pub reason: String,
    pub approval_source: String,
    pub status: String,
    pub product_code: String,
    #[schema(value_type = String, format = "decimal")]
    pub qty: Quantity,
    pub reject_reason: Option<String>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub shipped_by: Option<Uuid>,
    pub shipped_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct PurchaseReturnOrderListResponse {
    pub data: Vec<PurchaseReturnOrder>,
    pub page: PageMeta,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use uuid::Uuid;

    use super::*;

    fn request() -> CreateReceivingOrderRequest {
        CreateReceivingOrderRequest {
            receipt_no: "ASN-TEST-001".to_string(),
            document_type: RECEIVING_DOCUMENT_TYPE_PURCHASE_INBOUND.to_string(),
            supplier_id: Some(Uuid::new_v4()),
            warehouse_id: Uuid::new_v4(),
            external_ref: None,
            expected_arrival_at: Some(
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 14, 9, 0, 0)
                    .single()
                    .expect("valid arrival time"),
            ),
            lines: vec![ReceivingOrderLine {
                line_no: 1,
                product_id: None,
                product_code: "P-001".to_string(),
                expected_qty: 10.into(),
                batch_no: None,
                production_date: None,
                expiry_date: None,
            }],
        }
    }

    #[test]
    fn create_request_requires_supplier() {
        let mut value = request();
        value.supplier_id = None;

        assert_eq!(
            validate_create_receiving_order_request(
                &value,
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 13, 9, 0, 0)
                    .single()
                    .expect("valid current time"),
            ),
            Err(ReceivingOrderRequestValidationError::MissingSupplier)
        );
    }

    #[test]
    fn ship_request_validates_provider_signature_and_cold_chain_fields() {
        let signature_attachment_id = Uuid::new_v4();
        let mut request = ShipOutboundOrderRequest {
            delivery_provider_type: "third_party_express".to_string(),
            vehicle_no: None,
            plate_no: "沪A12345".to_string(),
            driver_user_id: None,
            courier_name: Some("快递员".to_string()),
            courier_phone: Some("13800000000".to_string()),
            signature_attachment_id: Some(signature_attachment_id),
            loading_temperature_celsius: None,
            cold_chain_packages: Vec::new(),
            package_count: 1,
        };
        assert_eq!(validate_ship_outbound_request(&request, false), Ok(()));

        request.signature_attachment_id = None;
        assert_eq!(
            validate_ship_outbound_request(&request, false),
            Err(ShipOutboundValidationError::FieldRequired(
                "signature_attachment_id"
            ))
        );
        request.signature_attachment_id = Some(signature_attachment_id);
        request.loading_temperature_celsius = Some(4.2);
        request.cold_chain_packages = vec![OutboundColdChainPackage {
            insulated_container_no: "BOX-COLD-001".to_string(),
            ice_pack_count: 4,
        }];
        assert_eq!(validate_ship_outbound_request(&request, true), Ok(()));

        request.loading_temperature_celsius = Some(f64::NAN);
        assert_eq!(
            validate_ship_outbound_request(&request, true),
            Err(ShipOutboundValidationError::InvalidTemperature)
        );

        let own_fleet = ShipOutboundOrderRequest {
            delivery_provider_type: "own_fleet".to_string(),
            vehicle_no: Some("VEHICLE-001".to_string()),
            plate_no: "沪A12345".to_string(),
            driver_user_id: Some(Uuid::new_v4()),
            courier_name: None,
            courier_phone: None,
            signature_attachment_id: None,
            loading_temperature_celsius: None,
            cold_chain_packages: Vec::new(),
            package_count: 1,
        };
        assert_eq!(validate_ship_outbound_request(&own_fleet, false), Ok(()));
    }

    #[test]
    fn create_request_requires_expected_arrival() {
        let mut value = request();
        value.expected_arrival_at = None;

        assert_eq!(
            validate_create_receiving_order_request(
                &value,
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 13, 9, 0, 0)
                    .single()
                    .expect("valid current time"),
            ),
            Err(ReceivingOrderRequestValidationError::MissingExpectedArrival)
        );
    }

    #[test]
    fn create_request_rejects_arrival_before_today() {
        let mut value = request();
        value.expected_arrival_at = Some(
            chrono::Utc
                .with_ymd_and_hms(2026, 7, 12, 23, 59, 59)
                .single()
                .expect("valid arrival time"),
        );

        assert_eq!(
            validate_create_receiving_order_request(
                &value,
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 13, 9, 0, 0)
                    .single()
                    .expect("valid current time"),
            ),
            Err(ReceivingOrderRequestValidationError::InvalidExpectedArrival)
        );
    }

    #[test]
    fn create_request_requires_product_code() {
        let mut value = request();
        value.lines[0].product_code = "  ".to_string();

        assert_eq!(
            validate_create_receiving_order_request(
                &value,
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 13, 9, 0, 0)
                    .single()
                    .expect("valid current time"),
            ),
            Err(ReceivingOrderRequestValidationError::MissingProduct)
        );
    }

    #[test]
    fn create_request_accepts_multiple_batches_of_one_product() {
        let mut value = request();
        value.lines.push(ReceivingOrderLine {
            line_no: 2,
            product_id: None,
            product_code: "P-001".to_string(),
            expected_qty: 5.into(),
            batch_no: Some("B-002".to_string()),
            production_date: None,
            expiry_date: None,
        });

        assert_eq!(
            validate_create_receiving_order_request(
                &value,
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 13, 9, 0, 0)
                    .single()
                    .expect("valid current time"),
            ),
            Ok(())
        );
    }

    #[test]
    fn create_request_rejects_multiple_products() {
        let mut value = request();
        value.lines.push(ReceivingOrderLine {
            line_no: 2,
            product_id: None,
            product_code: "P-002".to_string(),
            expected_qty: 5.into(),
            batch_no: None,
            production_date: None,
            expiry_date: None,
        });

        assert_eq!(
            validate_create_receiving_order_request(
                &value,
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 13, 9, 0, 0)
                    .single()
                    .expect("valid current time"),
            ),
            Err(ReceivingOrderRequestValidationError::MultipleProducts)
        );
    }

    #[test]
    fn create_request_rejects_mixed_product_id_presence() {
        let mut value = request();
        value.lines[0].product_id = Some(Uuid::new_v4());
        value.lines.push(ReceivingOrderLine {
            line_no: 2,
            product_id: None,
            product_code: "P-001".to_string(),
            expected_qty: 5.into(),
            batch_no: Some("B-002".to_string()),
            production_date: None,
            expiry_date: None,
        });

        assert_eq!(
            validate_create_receiving_order_request(
                &value,
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 13, 9, 0, 0)
                    .single()
                    .expect("valid current time"),
            ),
            Err(ReceivingOrderRequestValidationError::MultipleProducts)
        );
    }

    #[test]
    fn create_request_accepts_arrival_later_today() {
        let mut value = request();
        value.expected_arrival_at = Some(
            chrono::Utc
                .with_ymd_and_hms(2026, 7, 13, 23, 59, 59)
                .single()
                .expect("valid arrival time"),
        );

        assert_eq!(
            validate_create_receiving_order_request(
                &value,
                chrono::Utc
                    .with_ymd_and_hms(2026, 7, 13, 9, 0, 0)
                    .single()
                    .expect("valid current time"),
            ),
            Ok(())
        );
    }

    #[test]
    fn nullable_patch_distinguishes_missing_null_and_value() {
        let missing: UpdateReceivingOrderRequest =
            serde_json::from_str("{}").expect("missing fields should deserialize");
        let clear: UpdateReceivingOrderRequest = serde_json::from_str(r#"{"external_ref":null}"#)
            .expect("null field should deserialize");
        let value: UpdateReceivingOrderRequest = serde_json::from_str(
            r#"{"external_ref":"ERP-001","expected_arrival_at":"2026-07-11T00:00:00Z"}"#,
        )
        .expect("values should deserialize");

        assert_eq!(missing.external_ref, None);
        assert_eq!(clear.external_ref, Some(None));
        assert_eq!(value.external_ref, Some(Some("ERP-001".to_string())));
        assert!(value.expected_arrival_at.is_some());
    }

    #[test]
    fn receiving_order_patch_rejects_workflow_status_field() {
        let result =
            serde_json::from_str::<UpdateReceivingOrderRequest>(r#"{"status":"completed"}"#);

        assert!(result.is_err());
    }

    fn outbound_line() -> OutboundOrderLine {
        OutboundOrderLine {
            line_no: 1,
            product_code: "P-001".to_string(),
            batch_no: "B-001".to_string(),
            planned_qty: 10.into(),
            picked_qty: 8.into(),
            reviewed_qty: 0.into(),
            shipped_qty: 0.into(),
            short_pick_qty: 2.into(),
        }
    }

    fn review_request(
        reviewer_id: Uuid,
        reviewed_qty: impl Into<Quantity>,
    ) -> ReviewOutboundOrderRequest {
        ReviewOutboundOrderRequest {
            reviewer_id,
            review_mode: REVIEW_MODE_PDA_LOOSE.to_string(),
            second_reviewer_id: None,
            lines: vec![ReviewOutboundOrderLineRequest {
                line_no: 1,
                product_code: "P-001".to_string(),
                reviewed_qty: reviewed_qty.into(),
            }],
        }
    }

    #[test]
    fn review_accepts_short_pick_only_when_scanned_quantity_matches_picked_quantity() {
        let reviewer_id = Uuid::new_v4();
        let request = review_request(reviewer_id, 8);

        assert_eq!(
            validate_review_submission(&[outbound_line()], &request, reviewer_id, &[]),
            Ok(())
        );
    }

    #[test]
    fn review_rejects_product_or_quantity_mismatch() {
        let reviewer_id = Uuid::new_v4();
        let mut request = review_request(reviewer_id, 7);

        assert_eq!(
            validate_review_submission(&[outbound_line()], &request, reviewer_id, &[]),
            Err(ReviewValidationError::QuantityMismatch {
                line_no: 1,
                expected: 8.into(),
                actual: 7.into(),
            })
        );

        request.lines[0].reviewed_qty = 9.into();
        assert_eq!(
            validate_review_submission(&[outbound_line()], &request, reviewer_id, &[]),
            Err(ReviewValidationError::InvalidQuantity(1))
        );

        request.lines[0].reviewed_qty = 8.into();
        request.lines[0].product_code = "P-WRONG".to_string();
        assert_eq!(
            validate_review_submission(&[outbound_line()], &request, reviewer_id, &[]),
            Err(ReviewValidationError::ProductMismatch(1))
        );
    }

    #[test]
    fn review_rejects_the_picker_as_reviewer() {
        let picker_id = Uuid::new_v4();
        let request = review_request(picker_id, 8);

        assert_eq!(
            validate_review_submission(&[outbound_line()], &request, picker_id, &[picker_id]),
            Err(ReviewValidationError::SameOperator)
        );
    }

    #[test]
    fn review_rejects_inconsistent_short_pick_quantity() {
        let reviewer_id = Uuid::new_v4();
        let mut line = outbound_line();
        line.short_pick_qty = 1.into();

        assert_eq!(
            validate_review_submission(&[line], &review_request(reviewer_id, 8), reviewer_id, &[],),
            Err(ReviewValidationError::InvalidQuantity(1))
        );
    }
}
