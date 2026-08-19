import type { IncomingMessage, ServerResponse } from "node:http";

import {
  asNumber,
  asRecord,
  asString,
  readJsonBody,
  sendError,
  sendJson,
} from "./web-admin-dev-mock-core-common";

interface DevReconciliationItem {
  id: string;
  product_code: string;
  batch_no: string;
  wms_qty: number;
  erp_qty: number;
  difference_qty: number;
  difference_type: string;
  resolution_status: string;
  stock_adjustment_order_ids: string[];
  created_at: string;
}

const items: DevReconciliationItem[] = [
  {
    id: "00000000-0000-0000-0000-00000000c101",
    product_code: "P-RC-001",
    batch_no: "B-RC-001",
    wms_qty: 120,
    erp_qty: 116,
    difference_qty: 4,
    difference_type: "wms_more",
    resolution_status: "open",
    stock_adjustment_order_ids: [],
    created_at: "2026-07-23T08:00:00.000Z",
  },
  {
    id: "00000000-0000-0000-0000-00000000c102",
    product_code: "P-RC-002",
    batch_no: "B-RC-002",
    wms_qty: 32,
    erp_qty: 35,
    difference_qty: -3,
    difference_type: "erp_more",
    resolution_status: "open",
    stock_adjustment_order_ids: [],
    created_at: "2026-07-23T08:05:00.000Z",
  },
];

let rule = {
  interval_hours: 24,
  enabled: true,
  updated_at: "2026-07-23T08:00:00.000Z",
};

export async function handleReconciliationDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
) {
  if (pathname === "/api/v1/reconciliation/items" && req.method === "GET") {
    const params = new URL(req.url ?? pathname, "http://wms.local").searchParams;
    const differenceTypes = params.get("difference_type")?.split(",").filter(Boolean) ?? [];
    const resolutionStatuses = params.get("resolution_status")?.split(",").filter(Boolean) ?? [];
    const productCode = params.get("product_code")?.toLocaleLowerCase() ?? "";
    const batchNo = params.get("batch_no")?.toLocaleLowerCase() ?? "";
    const filtered = items.filter((item) =>
      (!productCode || item.product_code.toLocaleLowerCase().includes(productCode)) &&
      (!batchNo || item.batch_no.toLocaleLowerCase().includes(batchNo)) &&
      (!differenceTypes.length || differenceTypes.includes(item.difference_type)) &&
      (!resolutionStatuses.length || resolutionStatuses.includes(item.resolution_status)))
      .sort((left, right) => right.created_at.localeCompare(left.created_at) || right.id.localeCompare(left.id));
    const limit = Math.min(200, Math.max(1, asNumber(params.get("limit"), 50)));
    const cursor = params.get("cursor");
    const start = cursor ? Math.max(0, filtered.findIndex((item) => `${item.created_at},${item.id}` === cursor) + 1) : 0;
    const data = filtered.slice(start, start + limit);
    const last = data.at(-1);
    sendJson(res, 200, {
      data,
      page: {
        count: data.length,
        next_cursor: start + data.length < filtered.length && last ? `${last.created_at},${last.id}` : null,
      },
    });
    return;
  }

  if (pathname === "/api/v1/reconciliation/rule" && req.method === "GET") {
    sendJson(res, 200, rule);
    return;
  }

  if (pathname === "/api/v1/reconciliation/rule" && req.method === "PUT") {
    const body = asRecord(await readJsonBody(req));
    rule = {
      interval_hours: asNumber(body.interval_hours, rule.interval_hours),
      enabled: body.enabled === true,
      updated_at: new Date().toISOString(),
    };
    sendJson(res, 200, rule);
    return;
  }

  if (pathname === "/api/v1/reconciliation/items/isolation" && req.method === "POST") {
    const body = asRecord(await readJsonBody(req));
    const selected = Array.isArray(body.item_ids) ? body.item_ids.length : 0;
    sendJson(res, 200, selected);
    return;
  }

  const resolveMatch = pathname.match(/^\/api\/v1\/reconciliation\/items\/([^/]+)\/resolve$/);
  if (resolveMatch && req.method === "POST") {
    const item = items.find((candidate) => candidate.id === resolveMatch[1]);
    if (!item) {
      sendError(res, 404, "RC_NOT_FOUND", "对账差异不存在");
      return;
    }
    const body = asRecord(await readJsonBody(req));
    const disposition = asString(body.disposition, "");
    const allocations = Array.isArray(body.allocations) ? body.allocations : [];
    if (disposition === "erp_truth" && allocations.length === 0) {
      sendError(res, 422, "RC_INVALID_REQUEST", "以 ERP 为准必须提交库存分配");
      return;
    }
    item.resolution_status = disposition === "known_difference"
      ? "known_difference"
      : disposition === "erp_truth"
        ? "adjustment_pending"
        : "erp_feedback_pending";
    if (disposition === "erp_truth") {
      item.stock_adjustment_order_ids = allocations.map((_, index) =>
        `00000000-0000-0000-0000-${String(201 + index).padStart(12, "0")}`);
    }
    sendJson(res, 200, item);
    return;
  }

  sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Reconciliation dev mock route not found");
}
