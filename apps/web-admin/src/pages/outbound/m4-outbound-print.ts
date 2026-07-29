import type { OutboundOrder } from "@/features/outbound/outbound-queries";
import type { H9BusinessPrintTarget } from "../print-template/H9BusinessPrintDialog";

export function outboundPrintTarget(order: OutboundOrder): H9BusinessPrintTarget {
  return {
    templateTypeCode: "delivery_note",
    businessModule: "M4",
    businessDocumentType: "delivery_note",
    businessDocumentId: order.id,
    description: `${order.wms_order_no} · 随货同行单`,
    data: order,
  };
}
