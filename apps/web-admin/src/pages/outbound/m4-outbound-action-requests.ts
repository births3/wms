import type {
  CreateOutboundOrderRequest,
  CreatePurchaseReturnRequest,
  ShipOutboundOrderRequest,
} from "@/features/outbound/outbound-queries";

import type {
  OutboundCreateForm,
  OutboundShipForm,
  PurchaseReturnCreateForm,
} from "./M4OutboundActionDialog";

export const emptyPurchaseReturnForm: PurchaseReturnCreateForm = {
  returnNo: "",
  sourcePurchaseOrderNo: "",
  supplierName: "",
  reason: "",
  warehouseId: "",
  productCode: "",
  qty: "",
};

export const emptyShipForm: OutboundShipForm = {
  carrierType: "",
  handoverTo: "",
  packageCount: "",
};

export function outboundOrderRequest(form: OutboundCreateForm): CreateOutboundOrderRequest {
  return {
    document_type: form.documentType,
    wms_order_no: form.wmsOrderNo.trim(),
    erp_order_no: form.erpOrderNo.trim() || null,
    customer_id: form.customerId,
    delivery_address_id: form.deliveryAddressId,
    warehouse_id: form.warehouseId,
    required_ship_at: form.requiredShipDate
      ? `${form.requiredShipDate}T09:00:00.000Z`
      : null,
    lines: [{
      line_no: 1,
      product_code: form.productCode.trim(),
      batch_no: form.batchNo.trim(),
      planned_qty: positiveInteger(form.plannedQty, "计划数量"),
    }],
  };
}

export function purchaseReturnRequest(
  form: PurchaseReturnCreateForm,
): CreatePurchaseReturnRequest {
  const qty = positiveInteger(form.qty, "采购退货数量");
  const request = {
    return_no: form.returnNo.trim(),
    source_purchase_order_no: form.sourcePurchaseOrderNo.trim(),
    supplier_name: form.supplierName.trim(),
    reason: form.reason.trim(),
    product_code: form.productCode.trim(),
    qty,
    warehouse_id: form.warehouseId,
  };
  if (Object.values(request).some((value) => value === "")) {
    throw new Error("采购退货必填字段不能为空");
  }
  return request;
}

export function outboundShipRequest(form: OutboundShipForm): ShipOutboundOrderRequest {
  const request = {
    carrier_type: form.carrierType,
    handover_to: form.handoverTo.trim(),
    package_count: positiveInteger(form.packageCount, "包裹数量"),
  };
  if (!request.carrier_type || !request.handover_to) {
    throw new Error("配送方类型和交接对象必填");
  }
  return request;
}

function positiveInteger(value: string, label: string) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error(`${label}必须大于 0`);
  }
  return parsed;
}
