import type { QueryPanelRangeValue, QueryPanelValue, StatusKey } from "@wms/ui";

import type { ReceivingOrder } from "@/features/inbound/inbound-queries";
import type { InboundDetailStage } from "./m2-inbound-detail-view-model";
import {
  inboundDocumentTypeLabel,
  inboundDocumentTypeOf,
  matchesInboundDocumentTypeFilter,
  type InboundDocumentTypeFilter,
} from "./m2-inbound-document-type";

export type M2InboundMode = "receiving" | "inspecting" | "putaway";
export type StatusFilterValue = "receiving" | "inspecting" | "putaway" | "completed" | "closed_rejected";
export type StatusFilter = StatusFilterValue[];
interface StatusFilterOption {
  value: StatusFilterValue;
  label: string;
}
export interface OwnerContext {
  ownerId: string;
  ownerCode: string;
}
export interface M2InboundQueryValue extends QueryPanelValue {
  keyword: string;
  ownerKeyword: string;
  documentTypeFilter: InboundDocumentTypeFilter;
  statusFilter: StatusFilter;
  arrivalDate: QueryPanelRangeValue;
  createdAt: QueryPanelRangeValue;
}

export function workFieldText(order: ReceivingOrder, mode: M2InboundMode) {
  const line = order.lines[0];
  return {
    receiving: [`供应商 ${shortId(order.supplier_id ?? "00000000")}`, "承运商 华东冷链 / 车牌沪A-12345"],
    inspecting: [`批号 ${line?.batch_no ?? "-"}`, `效期 ${line?.expiry_date ?? "-"} / 质量待验`],
    putaway: ["推荐 A-01-01 / 实际待录入", "LPN-M2-PC-0001 / 校验待执行"],
  }[mode];
}

export function workFieldHeader(mode: M2InboundMode) {
  const headers: Record<M2InboundMode, string> = {
    receiving: "供应商 / 承运",
    inspecting: "批号 / 质量",
    putaway: "库位 / LPN",
  };
  return headers[mode];
}

export function filterOrders(
  orders: ReceivingOrder[],
  keyword: string,
  documentTypeFilter: InboundDocumentTypeFilter,
  statusFilter: StatusFilter,
  arrivalDateFrom: string,
  arrivalDateTo: string,
  createdAtFrom: string,
  createdAtTo: string,
  ownerKeyword = "",
  ownerContext?: OwnerContext,
) {
  const normalized = keyword.trim().toLowerCase();
  const normalizedOwner = ownerKeyword.trim().toLowerCase();
  return orders.filter((order) => {
    const documentType = inboundDocumentTypeOf(order);
    const matchesDocumentType = matchesInboundDocumentTypeFilter(order, documentTypeFilter);
    const matchesStatus = matchesStatusFilter(order.status, statusFilter);
    const matchesOwner = matchesOwnerFilter(order.owner_id, normalizedOwner, ownerContext);
    const arrivalDate = order.expected_arrival_at?.slice(0, 10) ?? "";
    const matchesDate = (!arrivalDateFrom || arrivalDate >= arrivalDateFrom) && (!arrivalDateTo || arrivalDate <= arrivalDateTo);
    const createdDate = order.created_at.slice(0, 10);
    const matchesCreatedAt = (!createdAtFrom || createdDate >= createdAtFrom) && (!createdAtTo || createdDate <= createdAtTo);
    const searchable = [
      order.receipt_no,
      order.status,
      order.owner_id,
      ownerLabel(order.owner_id, ownerContext),
      inboundDocumentTypeLabel(documentType),
      ...order.lines.flatMap((line) => [line.product_code, line.batch_no ?? ""]),
    ]
      .join(" ")
      .toLowerCase();
    return matchesDocumentType && matchesStatus && matchesOwner && matchesDate && matchesCreatedAt && (!normalized || searchable.includes(normalized));
  });
}

export function defaultCreatedDateRange() {
  const to = new Date();
  const from = new Date(to);
  from.setDate(from.getDate() - 90);
  return { from: dateInputValue(from), to: dateInputValue(to) };
}

export function defaultStatusFilter(mode: M2InboundMode): StatusFilter {
  return [mode];
}

export function statusFilterOptions(mode: M2InboundMode): StatusFilterOption[] {
  const options: Record<M2InboundMode, StatusFilterOption[]> = {
    receiving: [
      { value: "receiving", label: "待收货/收货中" },
      { value: "closed_rejected", label: "已关闭(拒收)" },
    ],
    inspecting: [{ value: "inspecting", label: "验收中" }],
    putaway: [
      { value: "putaway", label: "上架中" },
      { value: "completed", label: "已完成" },
    ],
  };
  return options[mode];
}

export function statusColumnFilterOptions(mode: M2InboundMode): Array<{ value: string; label: string }> {
  const options: Record<M2InboundMode, Array<{ value: string; label: string }>> = {
    receiving: [
      { value: "released", label: "待收货" },
      { value: "receiving", label: "收货中" },
      { value: "closed_rejected", label: "已关闭(拒收)" },
    ],
    inspecting: [{ value: "inspecting", label: "验收中" }],
    putaway: [
      { value: "putaway", label: "上架中" },
      { value: "completed", label: "已完成" },
    ],
  };
  return options[mode];
}

export function detailStageFromMode(mode: M2InboundMode): InboundDetailStage {
  const map = { receiving: "receiving", inspecting: "inspection", putaway: "putaway" } as const;
  return map[mode];
}

export function nextM2InboundSelectedId(
  selectedId: string | null,
  orderIds: readonly string[],
  userClearedSelection: boolean,
) {
  if (selectedId && orderIds.includes(selectedId)) return selectedId;
  if (userClearedSelection) return null;
  return orderIds[0] ?? null;
}

export function inboundPageMeta(mode: M2InboundMode) {
  const meta: Record<M2InboundMode, { title: string; subtitle: string }> = {
    receiving: {
      title: "M2 收货管理",
      subtitle: "ASN 接收 · 到货登记 · 实到/缺货/拒收",
    },
    inspecting: {
      title: "M2 验收管理",
      subtitle: "批号效期验收 · 追溯码 · 双人签字",
    },
    putaway: {
      title: "M2 上架管理",
      subtitle: "库位确认 · 商品批号 · 数量上架",
    },
  };
  return meta[mode];
}

export function canReceiveOrReject(status: string) {
  return status === "released" || status === "receiving";
}

export function statusKey(status: string): StatusKey {
  if (status === "completed") return "completed";
  if (status.includes("receiv") || status.includes("inspect") || status.includes("putaway")) return "in_progress";
  if (status.includes("reject") || status.includes("closed")) return "unqualified";
  return "pending";
}

export function statusLabel(status: string) {
  const labels: Record<string, string> = {
    pending: "待处理",
    released: "待收货",
    receiving: "收货中",
    inspecting: "验收中",
    putaway: "上架中",
    completed: "已完成",
    closed_rejected: "已关闭(拒收)",
  };
  return labels[status] ?? status;
}

export function totalExpectedQty(order: ReceivingOrder) {
  return order.lines.reduce((sum, line) => sum + line.expected_qty, 0);
}

export function productTemperatureAttribute(productCode: string | null | undefined) {
  // ponytail: ReceivingOrderLine 还没有商品温度属性；后端补字段后替换这里。
  if (!productCode) return "常温";
  if (/冻|FROZEN/i.test(productCode)) return "冷冻";
  if (/冷|COLD|P-M2-002/i.test(productCode)) return "冷藏";
  return "常温";
}

export function temperatureControlFromProductAttribute(attribute: string) {
  if (attribute === "冷冻") return "冷冻车";
  if (attribute === "冷藏") return "冷藏车";
  return "常温";
}

export function toInteger(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function splitCodes(value: string) {
  return value.split(/\s+/).map((item) => item.trim()).filter(Boolean);
}

export function dateToIso(value: string) {
  return value ? `${value}T10:00:00.000Z` : null;
}

export function dateInputValue(value: Date) {
  const year = value.getFullYear();
  const month = String(value.getMonth() + 1).padStart(2, "0");
  const day = String(value.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function shortId(value: string) {
  return value.slice(0, 8);
}

export function ownerLabel(value: string | null | undefined, ownerContext?: OwnerContext) {
  if (!value) return "-";
  if (ownerContext && value === ownerContext.ownerId) return ownerContext.ownerCode;
  if (value === "00000000-0000-0000-0000-000000000001") return "PY_OWNER";
  return shortId(value);
}

export function formatDateTime(value: string | null | undefined) {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { hour12: false });
}

function matchesStatusFilter(status: string, filter: StatusFilter) {
  if (filter.length === 0) return true;
  return filter.some((item) => (item === "receiving" ? canReceiveOrReject(status) : status === item));
}

function matchesOwnerFilter(ownerId: string | null | undefined, normalizedOwner: string, ownerContext?: OwnerContext) {
  if (!normalizedOwner) return true;
  const text = [ownerId ?? "", ownerLabel(ownerId, ownerContext)].join(" ").toLowerCase();
  return text.includes(normalizedOwner);
}
