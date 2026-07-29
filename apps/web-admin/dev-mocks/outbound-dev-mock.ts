import type { IncomingMessage, ServerResponse } from "node:http";

import {
  asNumber,
  asRecord,
  asString,
  readJsonBody,
  sendError,
  sendJson,
} from "./web-admin-dev-mock-core-common";
import { devOwnerId, devWarehouseId } from "./web-admin-dev-mock-model";

const outboundOrders: Record<string, unknown>[] = [];
const outboundWaves: Record<string, unknown>[] = [];
const purchaseReturns: Record<string, unknown>[] = [];

const purchaseReturnTransitions: Record<string, [from: string, to: string]> = {
  approve: ["pending_approval", "approved"],
  reject: ["pending_approval", "cancelled"],
  pick: ["approved", "picking"],
  review: ["picking", "reviewed"],
  ship: ["reviewed", "shipped"],
};

export async function handleOutboundDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (req.method === "GET" && pathname === "/api/v1/outbound/purchase-returns") {
    sendJson(res, 200, { data: purchaseReturns, page: { count: purchaseReturns.length, next_cursor: null } });
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/outbound/purchase-returns") {
    const body = await readJsonBody(req);
    if (!asString(body.return_no, "").trim() || asNumber(body.qty, 0) <= 0) {
      sendError(res, 422, "W4-422", "退货单号和数量不能为空");
      return true;
    }
    const now = new Date().toISOString();
    const item = {
      id: crypto.randomUUID(),
      owner_id: devOwnerId,
      return_no: asString(body.return_no, ""),
      document_type: "purchase_return_outbound",
      source_purchase_order_no: asString(body.source_purchase_order_no, ""),
      supplier_id: typeof body.supplier_id === "string" ? body.supplier_id : null,
      supplier_name: asString(body.supplier_name, ""),
      reason: asString(body.reason, ""),
      approval_source: "purchase_return_approval",
      status: "pending_approval",
      product_code: asString(body.product_code, ""),
      qty: asNumber(body.qty, 0),
      reject_reason: null as string | null,
      shipped_at: null as string | null,
      shipped_by: null,
      shipped_by_name: null,
      warehouse_id: asString(body.warehouse_id, devWarehouseId),
      created_at: now,
      updated_at: now,
    };
    purchaseReturns.unshift(item);
    sendJson(res, 200, item);
    return true;
  }

  const returnActionMatch = pathname.match(/^\/api\/v1\/outbound\/purchase-returns\/([^/]+)\/(approve|reject|pick|review|ship)$/);
  if (req.method === "POST" && returnActionMatch) {
    const item = purchaseReturns.find((entry) => entry.id === returnActionMatch[1]);
    if (!item) {
      sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Purchase return not found");
      return true;
    }
    const [from, to] = purchaseReturnTransitions[returnActionMatch[2]];
    if (item.status !== from) {
      sendError(res, 422, "W4-422", "采购退货单当前状态不允许该操作");
      return true;
    }
    if (returnActionMatch[2] === "reject") {
      const body = await readJsonBody(req);
      const reason = asString(body.reason, "").trim();
      if (!reason) {
        sendError(res, 422, "W4-422", "驳回原因必填");
        return true;
      }
      item.reject_reason = reason;
    }
    item.status = to;
    item.updated_at = new Date().toISOString();
    if (returnActionMatch[2] === "ship") item.shipped_at = item.updated_at;
    sendJson(res, 200, item);
    return true;
  }

  const waveDetailMatch = pathname.match(/^\/api\/v1\/outbound\/waves\/([^/]+)$/);
  if (req.method === "GET" && waveDetailMatch) {
    const wave = outboundWaves.find((item) => item.id === waveDetailMatch[1]);
    if (!wave) {
      sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Outbound wave not found");
      return true;
    }
    sendJson(res, 200, wave);
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/outbound/waves") {
    sendJson(res, 200, { data: outboundWaves, page: { count: outboundWaves.length, next_cursor: null } });
    return true;
  }

  const detailMatch = pathname.match(/^\/api\/v1\/outbound\/orders\/([^/]+)$/);
  if (req.method === "GET" && detailMatch) {
    const order = outboundOrders.find((item) => item.id === detailMatch[1]);
    if (!order) {
      sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Outbound order not found");
      return true;
    }
    sendJson(res, 200, order);
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/outbound/orders") {
    sendJson(res, 200, { data: outboundOrders, page: { count: outboundOrders.length, next_cursor: null } });
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/outbound/waves") {
    const body = await readJsonBody(req);
    const orderIds = Array.isArray(body.order_ids) ? body.order_ids.filter((value): value is string => typeof value === "string") : [];
    if (orderIds.length === 0 || !asString(body.wave_no, "").trim()) {
      sendError(res, 422, "W4-422", "波次号和订单不能为空");
      return true;
    }
    const now = new Date().toISOString();
    const wave = {
      id: crypto.randomUUID(),
      owner_id: devOwnerId,
      wave_no: asString(body.wave_no, ""),
      status: "released",
      order_ids: orderIds,
      created_at: now,
      updated_at: now,
    };
    outboundWaves.unshift(wave);
    sendJson(res, 200, wave);
    return true;
  }

  if (req.method !== "POST" || pathname !== "/api/v1/outbound/orders") return false;

  const body = await readJsonBody(req);
  const documentType = asString(body.document_type, "");
  if (!new Set(["sales_outbound", "purchase_return_outbound"]).has(documentType)) {
    sendError(res, 422, "W4-422", "出库单据类型无效");
    return true;
  }

  const lines = Array.isArray(body.lines) ? body.lines : [];
  const line = asRecord(lines[0]);
  const now = new Date().toISOString();
  const order = {
    id: crypto.randomUUID(),
    owner_id: devOwnerId,
    document_type: documentType,
    wms_order_no: asString(body.wms_order_no, `SO-M4-DEV-${Date.now()}`) || `SO-M4-DEV-${Date.now()}`,
    erp_order_no: typeof body.erp_order_no === "string" && body.erp_order_no.trim() ? body.erp_order_no : null,
    customer_id: asString(body.customer_id, "00000000-0000-0000-0000-000000004001"),
    warehouse_id: asString(body.warehouse_id, devWarehouseId),
    required_ship_at: typeof body.required_ship_at === "string" ? body.required_ship_at : null,
    status: "confirmed",
    short_pick: false,
    lines: [{
      line_no: asNumber(line.line_no, 1),
      product_code: asString(line.product_code, "P-M4-NEW"),
      batch_no: asString(line.batch_no, "BATCH-M4-DEV"),
      planned_qty: asNumber(line.planned_qty, 1),
      picked_qty: 0,
      reviewed_qty: 0,
      shipped_qty: 0,
      short_pick_qty: 0,
    }],
    created_at: now,
    updated_at: now,
  };
  outboundOrders.unshift(order);
  sendJson(res, 200, order);
  return true;
}
