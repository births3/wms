import assert from "node:assert/strict";

import { receiptOf, toReceivingOrderListRow } from "./receiving-order-list-row.ts";
import type { ReceivingOrder, ReceivingOrderReceipt } from "./inbound-queries.ts";

const order = {
  id: "00000000-0000-0000-0000-000000000001",
  owner_id: "00000000-0000-0000-0000-000000000010",
  receipt_no: "ASN-001",
  document_type: "purchase_inbound",
  warehouse_id: "00000000-0000-0000-0000-000000000020",
  status: "receiving",
  expected_arrival_at: null,
  external_ref: null,
  supplier_id: null,
  created_at: "2026-08-13T00:00:00.000Z",
  updated_at: "2026-08-13T00:00:00.000Z",
  lines: [],
} satisfies ReceivingOrder;

assert.equal(receiptOf(order), undefined);

const receipt = {
  id: "00000000-0000-0000-0000-000000000030",
  receiving_order_id: order.id,
  owner_id: order.owner_id,
  actual_qty: "10",
  shortage_qty: "0",
  rejected_qty: "0",
  arrival_temperature_celsius: 5,
  exception_note: null,
  occurred_at: "2026-08-13T02:00:00.000Z",
  details: {
    delivery_qty: "10",
    contact_name: "张三",
    contact_phone: "13800000000",
    contact_id_no: "320101199001011234",
    carrier: "华东",
    vehicle_no: "苏A12345",
    origin: "南京",
    departure_at: null,
    arrival_at: null,
    storage_at: null,
    transport_mode: "公路",
    seal_checked: "已核对",
    filing_checked: "已核对",
    temperature_control_method: "冷藏车",
    second_receiver_id: null,
    sales_return_batches: [],
  },
} satisfies ReceivingOrderReceipt;

const row = toReceivingOrderListRow({ ...order, receipt });
assert.equal(receiptOf(row)?.actual_qty, "10");
assert.equal(receiptOf(row)?.details?.contact_name, "张三");
