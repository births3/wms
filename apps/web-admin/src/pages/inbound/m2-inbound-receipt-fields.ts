import type { ReceivingOrderReceipt } from "@/features/inbound/inbound-queries";
import { receiptOf, type ReceivingOrderListRow } from "@/features/inbound/receiving-order-list-row";

export function receiptDetailsOf(row: ReceivingOrderListRow) {
  const receipt = receiptOf(row);
  return {
    receipt,
    details: receipt?.details,
    batches: receipt?.details?.sales_return_batches ?? [],
  };
}

export function contactLines(details: ReceivingOrderReceipt["details"]) {
  return {
    name: details?.contact_name?.trim() || undefined,
    phone: details?.contact_phone?.trim() || undefined,
    idNo: details?.contact_id_no?.trim() || undefined,
  };
}

export function quantityLabel(value: string | null | undefined) {
  return value != null && value !== "" ? `${value} 件` : "-";
}
