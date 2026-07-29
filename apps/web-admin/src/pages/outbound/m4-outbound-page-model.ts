import type {
  QueryPanelRangeValue,
  QueryPanelValue,
  StatusKey,
} from "@wms/ui";

import { queryRange, queryString, queryStringArray } from "@/lib/query-value";
import {
  purchaseReturnApprovalSourceLabel,
  purchaseReturnDocumentTypeLabel,
} from "./M4OutboundPageParts";
import type { OutboundOrder, OutboundWave, PurchaseReturnOrder } from "./M4OutboundDetailDialog";

export type M4OutboundMode = "orders" | "waves" | "review" | "returns";

export function pageMeta(mode: M4OutboundMode) {
  const map = {
    orders: { title: "M4 出库订单管理", subtitle: "订单校验 · 双单号 · 作废申请", createAction: "create-order" as const, createLabel: "新建出库单" },
    waves: { title: "M4 波次规划", subtitle: "波次合并 · 库存锁定 · 路径策略", createAction: "create-wave" as const, createLabel: "新建波次" },
    review: { title: "M4 复核发货", subtitle: "包装站复核 · 打印 · 发货交接", createAction: null, createLabel: "" },
    returns: { title: "M4 采购退货出库", subtitle: "退供应商申请 · 审批 · 拣货复核 · 出库交接", createAction: "create-return" as const, createLabel: "新建采购退货单" },
  };
  return map[mode];
}

/**
 * M4 出库状态 → 中文文案唯一来源。
 * 查询面板选项、表格徽章、详情弹窗共用，避免同一状态多处维护出现文案漂移
 * （历史上 in_wave / picking / void_requested 曾在三个文件里各有一份且互相冲突）。
 */
export const outboundStatusLabels: Record<string, string> = {
  draft: "待下发",
  pending_validation: "待校验",
  validation_exception: "校验异常",
  confirmed: "已确认",
  void_requested: "作废申请中",
  in_wave: "已进波次",
  inventory_locked: "库存锁定",
  picking: "拣货中",
  picked: "已拣选",
  picked_short: "短拣待补齐",
  released: "已下发",
  reviewed: "已复核",
  reviewed_short: "短拣已复核",
  shipped: "已发货",
  signed: "已签收",
  pending_approval: "待审批",
  approved: "已审批",
  pickup: "提货中",
  inspecting: "验收中",
  completed: "已完成",
  cancelled: "已取消",
};

export function statusLabel(status: string | null | undefined) {
  if (!status) return "-";
  return outboundStatusLabels[status] ?? status;
}

export function statusOptions(mode: M4OutboundMode) {
  const keys = mode === "waves"
    ? ["draft", "released", "inventory_locked", "cancelled"]
    : mode === "returns"
      ? ["pending_approval", "approved", "picking", "reviewed", "shipped", "cancelled"]
      : ["pending_validation", "validation_exception", "confirmed", "inventory_locked", "reviewed", "shipped"];
  return keys.map((key) => [key, outboundStatusLabels[key] ?? key]);
}

export function statusKey(status: string | null | undefined): StatusKey {
  if (!status) return "pending";
  if (status.includes("exception") || status === "cancelled") return "unqualified";
  if (status === "completed" || status === "shipped" || status === "signed") return "completed";
  if (status === "inventory_locked" || status === "reviewed" || status === "released" || status === "pickup" || status === "inspecting" || status === "picking") return "in_progress";
  return "pending";
}

export function defaultM4OutboundQueryValue(): QueryPanelValue {
  return { keyword: "", statusFilter: [], businessDate: { from: "", to: "" } };
}

export function normalizeM4OutboundQueryValue(value: QueryPanelValue): QueryPanelValue {
  return {
    keyword: queryString(value.keyword),
    statusFilter: queryStringArray(value.statusFilter),
    businessDate: queryRange(value.businessDate),
  };
}

export function filterOrders(orders: OutboundOrder[], query: QueryPanelValue, mode: M4OutboundMode) {
  const allowed = mode === "review"
    ? new Set(["picked", "picked_short", "reviewed", "reviewed_short", "shipped"])
    : null;
  const keyword = queryString(query.keyword);
  const statuses = new Set(queryStringArray(query.statusFilter));
  const businessDate = queryRange(query.businessDate);
  return orders.filter((order) => {
    const lines = order.lines ?? [];
    const searchable = [order.wms_order_no, order.erp_order_no ?? "", order.customer_id, order.status ?? "", ...lines.flatMap((line) => [line.product_code, line.batch_no])].join(" ").toLowerCase();
    return (!allowed || allowed.has(order.status)) && matches(searchable, keyword) && matchesStatus(order.status, statuses) && dateInRange(order.required_ship_at, businessDate);
  });
}

export function filterWaves(waves: OutboundWave[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword);
  const statuses = new Set(queryStringArray(query.statusFilter));
  const businessDate = queryRange(query.businessDate);
  return waves.filter((wave) => matches(`${wave.wave_no} ${wave.status}`.toLowerCase(), keyword) && matchesStatus(wave.status, statuses) && dateInRange(wave.created_at, businessDate));
}

export function filterReturns(returns: PurchaseReturnOrder[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword);
  const statuses = new Set(queryStringArray(query.statusFilter));
  const businessDate = queryRange(query.businessDate);
  return returns.filter((item) => matches(
    [
      item.return_no,
      item.document_type,
      purchaseReturnDocumentTypeLabel(item.document_type),
      item.source_purchase_order_no,
      item.supplier_name,
      item.reason,
      item.product_code,
      item.approval_source,
      purchaseReturnApprovalSourceLabel(item.approval_source),
    ].join(" ").toLowerCase(),
    keyword,
  ) && matchesStatus(item.status, statuses) && dateInRange(item.created_at, businessDate));
}

function matches(searchable: string, keyword: string) {
  const normalized = keyword.trim().toLowerCase();
  return !normalized || searchable.includes(normalized);
}

function matchesStatus(status: string, statuses: Set<string>) {
  return statuses.size === 0 || statuses.has(status);
}

export { queryValueFromUnknown } from "@/lib/query-value";

function dateInRange(value: string | null | undefined, range: QueryPanelRangeValue) {
  const date = value?.slice(0, 10) ?? "";
  return (!range.from || date >= range.from) && (!range.to || date <= range.to);
}
