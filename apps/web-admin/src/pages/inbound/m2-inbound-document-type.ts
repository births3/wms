import type { ReceivingOrder } from "@/features/inbound/inbound-queries";

export type InboundDocumentType = "purchase_inbound" | "sales_return";
export type InboundDocumentTypeFilter = "all" | InboundDocumentType;

export function inboundDocumentTypeOf(order: ReceivingOrder): InboundDocumentType {
  return order.document_type as InboundDocumentType;
}

export function inboundDocumentTypeLabel(type: InboundDocumentType) {
  const labels: Record<InboundDocumentType, string> = {
    purchase_inbound: "采购入库",
    sales_return: "销售退货",
  };
  return labels[type];
}

export function matchesInboundDocumentTypeFilter(order: ReceivingOrder, filter: InboundDocumentTypeFilter) {
  return filter === "all" || inboundDocumentTypeOf(order) === filter;
}
