import type { ReceivingOrder } from "@/features/inbound/inbound-queries";

export type InboundDocumentType = "purchase_inbound" | "sales_return";
export type InboundDocumentTypeFilter = "all" | InboundDocumentType;

export function inboundDocumentTypeOf(order: ReceivingOrder): InboundDocumentType {
  // ponytail: ReceivingOrder 还没有 document_type；后端补字段后替换前缀判断。
  const marker = [
    order.receipt_no,
    order.external_ref ?? "",
    ...order.lines.map((line) => line.batch_no ?? ""),
  ].join(" ").toUpperCase();
  return marker.includes("SR-") || marker.includes("SALES_RETURN") ? "sales_return" : "purchase_inbound";
}

export function inboundDocumentTypeLabel(type: InboundDocumentType) {
  return type === "sales_return" ? "销售退货" : "采购入库";
}

export function matchesInboundDocumentTypeFilter(order: ReceivingOrder, filter: InboundDocumentTypeFilter) {
  return filter === "all" || inboundDocumentTypeOf(order) === filter;
}
