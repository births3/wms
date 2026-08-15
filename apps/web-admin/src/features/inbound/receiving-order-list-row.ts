import type { ReceivingOrder, ReceivingOrderReceipt } from "./inbound-queries";

export type ReceivingOrderListRow = ReceivingOrder;

export function toReceivingOrderListRow(order: ReceivingOrder): ReceivingOrderListRow {
  return {
    ...order,
    receipt: receiptOf(order),
  };
}

export function receiptOf(order: ReceivingOrder): ReceivingOrderReceipt | undefined {
  return order.receipt ?? undefined;
}
