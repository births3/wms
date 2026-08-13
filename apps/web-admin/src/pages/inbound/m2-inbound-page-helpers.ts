import type { QueryPanelRangeValue, QueryPanelValue, StatusKey } from "@wms/ui";

import type { ReceivingOrder } from "@/features/inbound/inbound-queries";
import { queryString, queryStringArray } from "@/lib/query-value";
import { STATUS_COMPLETED, STATUS_DRAFT, STATUS_PENDING, TEMP_AMBIENT, TEMP_COLD } from "@/lib/ui-strings";
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

export function defaultM2InboundQueryValue(
  mode: M2InboundMode,
  currentOwner: OwnerContext,
): M2InboundQueryValue {
  return {
    keyword: "",
    ownerKeyword: currentOwner.ownerCode,
    documentTypeFilter: [],
    statusFilter: defaultStatusFilter(mode),
    arrivalDate: { from: "", to: "" },
    createdAt: defaultCreatedDateRange(),
  };
}

export function normalizeM2InboundQueryValue(
  value: QueryPanelValue,
  fallback: M2InboundQueryValue,
  mode: M2InboundMode,
): M2InboundQueryValue {
  const rawStatusFilter = queryStringArray(value.statusFilter);
  const statusFilter = rawStatusFilter.filter((item): item is StatusFilter[number] =>
    statusFilterOptions(mode).some((option) => option.value === item),
  );
  return {
    keyword: queryString(value.keyword),
    ownerKeyword: queryString(value.ownerKeyword) || fallback.ownerKeyword,
    documentTypeFilter: queryStringArray(value.documentTypeFilter) as InboundDocumentTypeFilter,
    statusFilter: rawStatusFilter.length > 0 && statusFilter.length === 0 ? fallback.statusFilter : statusFilter,
    arrivalDate: queryRange(value.arrivalDate),
    createdAt: queryRange(value.createdAt, fallback.createdAt),
  };
}

export { queryValueFromUnknown } from "@/lib/query-value";

export function workFieldText(order: ReceivingOrder, mode: M2InboundMode) {
  const line = order.lines?.[0];
  return {
    receiving: [
      `供应商 ${order.supplier_id ? shortId(order.supplier_id) : "-"}`,
      "承运商 / 车牌 待录入",
    ],
    inspecting: [`批号 ${line?.batch_no ?? "-"}`, `效期 ${line?.expiry_date ?? "-"} / 质量待验`],
    putaway: ["库位 待录入", "LPN 待录入 / 校验待执行"],
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
    const createdDate = order.created_at?.slice(0, 10) ?? "";
    const matchesCreatedAt = (!createdAtFrom || createdDate >= createdAtFrom) && (!createdAtTo || createdDate <= createdAtTo);
    const lines = order.lines ?? [];
    const searchable = [
      order.receipt_no,
      order.status,
      order.owner_id,
      ownerLabel(order.owner_id, ownerContext),
      inboundDocumentTypeLabel(documentType),
      ...lines.flatMap((line) => [line.product_code, line.batch_no ?? ""]),
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
      { value: "receiving", label: "草稿/待收货/收货中" },
      { value: "closed_rejected", label: "已关闭(拒收)" },
    ],
    inspecting: [{ value: "inspecting", label: "验收中" }],
    putaway: [
      { value: "putaway", label: "上架中" },
      { value: "completed", label: STATUS_COMPLETED },
    ],
  };
  return options[mode];
}

export function statusColumnFilterOptions(mode: M2InboundMode): Array<{ value: string; label: string }> {
  const options: Record<M2InboundMode, Array<{ value: string; label: string }>> = {
    receiving: [
      { value: "draft", label: STATUS_DRAFT },
      { value: "released", label: "待收货" },
      { value: "receiving", label: "收货中" },
      { value: "closed_rejected", label: "已关闭(拒收)" },
    ],
    inspecting: [{ value: "inspecting", label: "验收中" }],
    putaway: [
      { value: "putaway", label: "上架中" },
      { value: "completed", label: STATUS_COMPLETED },
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
  const meta: Record<M2InboundMode, { title: string }> = {
    receiving: {
      title: "M2 收货管理",
    },
    inspecting: {
      title: "M2 验收管理",
    },
    putaway: {
      title: "M2 上架管理",
    },
  };
  return meta[mode];
}

/** 服务端返回双人策略时，作业端必须同步锁定第二签字人。 */
export function dualSignRequiredForPolicy(policy: string | null | undefined) {
  return policy === "dual_scan" || policy === "dual_scan_with_approval";
}

export function canReceiveOrReject(status: string) {
  return status === "released" || status === "receiving";
}

export function canRelease(status: string) {
  return status === "draft";
}

/** 验收：收货后验收；待第二人签字时也可打开签字动作。 */
export function canInspect(status: string) {
  return (
    status === "receiving" ||
    status === "received" ||
    status === "inspecting" ||
    status === "awaiting_second_sign"
  );
}

/** 上架：仅双签完成进入上架中，或历史已验状态。 */
export function canPutaway(status: string) {
  return status === "putaway" || status === "inspected";
}

export function statusKey(status: string | null | undefined): StatusKey {
  if (!status) return "pending";
  if (status === "completed") return "completed";
  if (status.includes("receiv") || status.includes("inspect") || status.includes("putaway")) return "in_progress";
  if (status.includes("reject") || status.includes("closed")) return "unqualified";
  return "pending";
}

export function statusLabel(status: string | null | undefined) {
  if (!status) return "-";
  const labels: Record<string, string> = {
    pending: STATUS_PENDING,
    draft: STATUS_DRAFT,
    released: "待收货",
    receiving: "收货中",
    inspecting: "验收中",
    awaiting_second_sign: "待第二人签字",
    putaway: "上架中",
    completed: STATUS_COMPLETED,
    closed_rejected: "已关闭(拒收)",
    closed_shortage: "已关闭(短少)",
    cancelled: "已作废",
  };
  return labels[status] ?? status;
}

export function totalExpectedQty(order: ReceivingOrder) {
  return (order.lines ?? []).reduce((sum, line) => sum + Number(line.expected_qty), 0);
}

export function productTemperatureAttribute(
  storageCondition: string | null | undefined,
  productCode?: string | null,
) {
  const condition = storageCondition?.trim().toLowerCase();
  if (condition === "frozen" || condition?.includes("冻")) return "冷冻";
  if (
    condition === "cold"
    || condition === "cool"
    || condition === "refrigerated"
    || condition?.includes("冷")
  ) return TEMP_COLD;
  if (condition) return TEMP_AMBIENT;
  // ponytail: 老单据缺商品主数据时才按编码降级；ReceivingOrderLine 补温区后删除。
  if (!productCode) return TEMP_AMBIENT;
  if (/冻|FROZEN/i.test(productCode)) return "冷冻";
  if (/冷|COLD/i.test(productCode)) return TEMP_COLD;
  return TEMP_AMBIENT;
}

export function temperatureControlFromProductAttribute(attribute: string) {
  if (attribute === "冷冻") return "冷冻车";
  if (attribute === TEMP_COLD) return "冷藏车";
  return TEMP_AMBIENT;
}

export function toInteger(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

export function splitCodes(value: string) {
  return value.split(/\s+/).map((item) => item.trim()).filter(Boolean);
}

export function dateToIso(value: string) {
  if (!value) return null;
  const localDate = new Date(`${value}T10:00:00`);
  return Number.isNaN(localDate.getTime()) ? null : localDate.toISOString();
}

export function dateTimeToIso(value: string) {
  if (!value) return null;
  const localDate = new Date(value);
  return Number.isNaN(localDate.getTime()) ? null : localDate.toISOString();
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

export { formatDateTime } from "@/lib/format";

function matchesStatusFilter(status: string | null | undefined, filter: StatusFilter | null | undefined) {
  if (!filter || filter.length === 0) return true;
  if (!status) return false;
  return filter.some((item) => {
    if (item === "receiving") return canRelease(status) || canReceiveOrReject(status);
    if (item === "inspecting") return canInspect(status);
    if (item === "putaway") return canPutaway(status);
    return status === item;
  });
}

/** 与 @/lib/query-value 的 queryRange 相比多一个 fallback（创建时间默认近 90 天需要兜底）。 */
function queryRange(value: QueryPanelValue[string], fallback?: QueryPanelRangeValue): QueryPanelRangeValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) return fallback ?? { from: "", to: "" };
  return {
    from: typeof value.from === "string" ? value.from : fallback?.from ?? "",
    to: typeof value.to === "string" ? value.to : fallback?.to ?? "",
  };
}

function matchesOwnerFilter(ownerId: string | null | undefined, normalizedOwner: string, ownerContext?: OwnerContext) {
  if (!normalizedOwner) return true;
  const text = [ownerId ?? "", ownerLabel(ownerId, ownerContext)].join(" ").toLowerCase();
  return text.includes(normalizedOwner);
}
