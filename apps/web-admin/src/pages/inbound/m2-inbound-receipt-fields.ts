import type { ReceivingOrderReceipt, SalesReturnReceivingBatch } from "@/features/inbound/inbound-queries";
import { receiptOf, type ReceivingOrderListRow } from "@/features/inbound/receiving-order-list-row";
import { maskSensitiveDisplayValue } from "@/lib/mask-sensitive";

export function receiptDetailsOf(row: ReceivingOrderListRow) {
  const receipt = receiptOf(row);
  return {
    receipt,
    details: receipt?.details,
    batches: (receipt?.details?.sales_return_batches ?? []) as SalesReturnReceivingBatch[],
  };
}

export function maskedContactLines(details: ReceivingOrderReceipt["details"]) {
  return {
    name: details?.contact_name?.trim() || undefined,
    phone: maskSensitiveDisplayValue(details?.contact_phone),
    idNo: maskSensitiveDisplayValue(details?.contact_id_no),
  };
}

export function quantityLabel(value: string | null | undefined) {
  return value != null && value !== "" ? `${value} 件` : "-";
}
