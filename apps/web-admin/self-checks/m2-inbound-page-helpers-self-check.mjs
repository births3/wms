import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const server = await createServer({
  root: fileURLToPath(new URL("..", import.meta.url)),
  logLevel: "silent",
  server: { middlewareMode: true },
  appType: "custom",
});

try {
  const { filterOrders, ownerLabel } = await server.ssrLoadModule("/src/pages/inbound/m2-inbound-page-helpers.ts");
  const { createAsnBatchNo } = await server.ssrLoadModule("/src/pages/inbound/m2-inbound-document-type.ts");

  const ownerA = "00000000-0000-0000-0000-000000000001";
  const ownerB = "11111111-0000-0000-0000-000000000002";
  const ownerContext = { ownerId: ownerA, ownerCode: "PY_OWNER" };
  const orders = [
    orderFixture({ id: "a", owner_id: ownerA, receipt_no: "ASN-A" }),
    orderFixture({ id: "b", owner_id: ownerB, receipt_no: "ASN-B" }),
  ];

  assert.equal(ownerLabel(ownerA, ownerContext), "PY_OWNER");
  assert.equal(ownerLabel(ownerB, ownerContext), "11111111");
  assert.deepEqual(
    filterOrders(orders, "", [], [], "", "", "", "", "PY_OWNER", ownerContext).map((item) => item.receipt_no),
    ["ASN-A"],
  );
  assert.deepEqual(
    filterOrders(orders, "", [], [], "", "", "", "", "11111111", ownerContext).map((item) => item.receipt_no),
    ["ASN-B"],
  );
  assert.equal(createAsnBatchNo("purchase_inbound", "BATCH-001"), null);
  assert.equal(createAsnBatchNo("sales_return", "BATCH-001"), "BATCH-001");
} finally {
  await server.close();
}

function orderFixture(overrides) {
  return {
    id: overrides.id,
    owner_id: overrides.owner_id,
    receipt_no: overrides.receipt_no,
    document_type: "purchase_inbound",
    warehouse_id: "00000000-0000-0000-0000-000000003001",
    status: "released",
    expected_arrival_at: "2026-06-28T10:00:00.000Z",
    external_ref: null,
    supplier_id: "00000000-0000-0000-0000-000000005001",
    created_at: "2026-06-28T09:00:00.000Z",
    updated_at: "2026-06-28T09:00:00.000Z",
    lines: [
      {
        line_no: 1,
        product_code: "P-M2-001",
        product_id: null,
        batch_no: null,
        expected_qty: 10,
        production_date: null,
        expiry_date: null,
      },
    ],
  };
}
