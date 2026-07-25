import type { DualPersonPolicy } from "@/features/validation-rules/dual-person-policy-queries";
import type {
  OutboundOrder,
  OutboundWave,
  PurchaseReturnOrder,
} from "./M4OutboundDetailDialog";

export const outboundOwnerId = "00000000-0000-0000-0000-000000000001";
export const outboundWarehouseId = "00000000-0000-0000-0000-000000003001";
export const outboundCustomerId = "00000000-0000-0000-0000-000000001201";

export function makeOrder(
  id: string,
  wmsNo: string,
  erpNo: string,
  status: string,
  qty: number,
  shortPick: boolean,
  now = "2026-06-27T09:00:00.000Z",
): OutboundOrder {
  return {
    id,
    owner_id: outboundOwnerId,
    document_type: "sales_outbound",
    customer_id: outboundCustomerId,
    delivery_address_id: "00000000-0000-0000-0000-000000001211",
    delivery_address_snapshot: {
      province: "上海市",
      city: "上海市",
      district: "浦东新区",
      detail_address: "示例路 1 号",
      contact_name: "门店收货人",
      contact_phone: "13800000000",
    },
    warehouse_id: outboundWarehouseId,
    wms_order_no: wmsNo,
    erp_order_no: erpNo,
    required_ship_at: "2026-06-28T09:00:00.000Z",
    status,
    short_pick: shortPick,
    created_at: now,
    updated_at: now,
    lines: [
      {
        line_no: 1,
        product_code: "P-M4-001",
        batch_no: "BATCH-OUT-202606",
        planned_qty: qty,
        picked_qty: shortPick ? qty - 2 : qty,
        reviewed_qty:
          status === "reviewed" || status === "shipped" ? qty : 0,
        shipped_qty: status === "shipped" ? qty : 0,
        short_pick_qty: shortPick ? 2 : 0,
      },
    ],
  };
}

export function makeReturn(returnNo: string): PurchaseReturnOrder {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    return_no: returnNo,
    document_type: "purchase_return_outbound",
    source_purchase_order_no: "ASN-M2-PC-0001",
    supplier_name: "华东医药供应商",
    reason: "供应商召回",
    approval_source: "purchase_return_approval",
    status: "pending_approval",
    product_code: "P-M4-001",
    qty: 3,
    created_at: now,
    updated_at: now,
  };
}

export function waveQty(wave: OutboundWave, orders: OutboundOrder[]) {
  const orderIds = wave.order_ids ?? [];
  return orders
    .filter((order) => orderIds.includes(order.id))
    .reduce(
      (sum, order) =>
        sum +
        (order.lines ?? []).reduce(
          (lineSum, line) => lineSum + line.planned_qty,
          0,
        ),
      0,
    );
}

export function waveLineCount(
  wave: OutboundWave,
  orders: OutboundOrder[],
) {
  const orderIds = wave.order_ids ?? [];
  return orders
    .filter((order) => orderIds.includes(order.id))
    .reduce((sum, order) => sum + (order.lines ?? []).length, 0);
}

export function formatDate(value: string | null | undefined) {
  return value ? value.slice(0, 10) : "-";
}

export function toInteger(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function isUuid(value: string) {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
    value,
  );
}

export function strictestDualPersonPolicy(
  policies: DualPersonPolicy[],
): DualPersonPolicy {
  if (policies.includes("dual_scan_with_approval")) {
    return "dual_scan_with_approval";
  }
  if (policies.includes("dual_scan")) return "dual_scan";
  return "single";
}
