import { maskSensitiveDisplayValue } from "../../lib/mask-sensitive.ts";

import type { ReceivingOrder, ReceivingOrderReceipt } from "./inbound-queries";

/** 列表行与详情共用 ReceivingOrder；列表入口会把联系方式写成脱敏值。 */
export type ReceivingOrderListRow = ReceivingOrder;

export function toReceivingOrderListRow(order: ReceivingOrder): ReceivingOrderListRow {
  const receipt = receiptOf(order);
  if (!receipt?.details) {
    return { ...order, receipt };
  }
  return {
    ...order,
    receipt: {
      ...receipt,
      details: {
        ...receipt.details,
        contact_phone: maskSensitiveDisplayValue(receipt.details.contact_phone) ?? null,
        contact_id_no: maskSensitiveDisplayValue(receipt.details.contact_id_no) ?? null,
      },
    },
  };
}

export function receiptOf(order: ReceivingOrder): ReceivingOrderReceipt | undefined {
  return order.receipt ?? undefined;
}
