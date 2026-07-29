import assert from "node:assert/strict";

import {
  defaultInboundDocumentQuery,
  filterInboundDocumentRows,
  validateDrugInspectionFile,
  validateUpstreamDeliveryFiles,
  type InboundDocumentEntryRow,
} from "./inbound-document-entry-model.ts";

const rows: InboundDocumentEntryRow[] = [
  row("ASN-001", "2026-07-25T10:00:00.000Z", "missing", "uploaded"),
  row("ASN-002", "2026-07-24T10:00:00.000Z", "complete", "missing"),
  row("ASN-003", "2026-07-23T10:00:00.000Z", "complete", "uploaded"),
  row("ASN-OLD", "2026-03-01T10:00:00.000Z", "missing", "missing"),
];

const filtered = filterInboundDocumentRows(rows, {
  keyword: "",
  receivedFrom: "2026-04-28",
  receivedTo: "2026-07-26",
  missingDrugInspection: true,
  missingUpstreamDelivery: true,
});

assert.deepEqual(filtered.map((item) => item.receiptNo), ["ASN-001", "ASN-002"]);
assert.deepEqual(
  filterInboundDocumentRows(rows, {
    keyword: "supplier-1",
    receivedFrom: "2026-07-25",
    receivedTo: "2026-07-25",
    missingDrugInspection: false,
    missingUpstreamDelivery: false,
  }).map((item) => item.receiptNo),
  ["ASN-001"],
);
assert.deepEqual(
  filterInboundDocumentRows(
    [row("ASN-TIMEZONE", "2026-07-25T16:30:00.000Z", "missing", "missing")],
    {
      keyword: "",
      receivedFrom: "2026-07-26",
      receivedTo: "2026-07-26",
      missingDrugInspection: false,
      missingUpstreamDelivery: false,
    },
  ).map((item) => item.receiptNo),
  ["ASN-TIMEZONE"],
);
assert.deepEqual(defaultInboundDocumentQuery("2026-07-26"), {
  keyword: "",
  receivedFrom: "2026-04-28",
  receivedTo: "2026-07-26",
  missingDrugInspection: false,
  missingUpstreamDelivery: false,
});

assert.equal(validateUpstreamDeliveryFiles([
  { name: "delivery.pdf", size: 1024, type: "application/pdf" },
]), null);
assert.equal(validateUpstreamDeliveryFiles([
  { name: "a.jpg", size: 1024, type: "image/jpeg" },
  { name: "b.jpg", size: 1024, type: "image/jpeg" },
]), null);
assert.match(validateUpstreamDeliveryFiles([
  { name: "delivery.pdf", size: 1024, type: "application/pdf" },
  { name: "a.jpg", size: 1024, type: "image/jpeg" },
]) ?? "", /PDF/);
assert.equal(validateDrugInspectionFile({
  name: "report.png",
  size: 5 * 1024 * 1024,
  type: "image/png",
}), null);
assert.match(validateDrugInspectionFile({
  name: "report.png",
  size: 5 * 1024 * 1024 + 1,
  type: "image/png",
}) ?? "", /5MB/);

function row(
  receiptNo: string,
  actualReceivedAt: string,
  drugInspectionStatus: InboundDocumentEntryRow["drugInspectionStatus"],
  upstreamDeliveryStatus: InboundDocumentEntryRow["upstreamDeliveryStatus"],
): InboundDocumentEntryRow {
  return {
    id: receiptNo,
    receiptNo,
    purchaseOrderNo: `PO-${receiptNo}`,
    ownerId: "owner-1",
    supplierId: "supplier-1",
    supplierName: "华东医药供应商",
    productCode: "P-001",
    batchNos: ["B-001"],
    actualReceivedAt,
    drugInspectionStatus,
    drugInspectionVersion: drugInspectionStatus === "complete" ? 1 : 0,
    upstreamDeliveryStatus,
    upstreamVersion: upstreamDeliveryStatus === "uploaded" ? 1 : 0,
    createdAt: actualReceivedAt,
  };
}

console.log("inbound document entry model test passed");
