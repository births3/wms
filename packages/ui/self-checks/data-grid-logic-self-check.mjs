import assert from "node:assert/strict";
import {
  getDataGridCopyText,
  dataGridFloatingPanelPosition,
  dataGridTableWidth,
  dataGridFilterConfigForData,
  getDataGridPage,
  moveColumnBefore,
  nextSortState,
  sanitizeDataGridColumnFiltersForData,
  sanitizeGridState,
  setColumnWidth,
  toggleCopyableColumn,
  toggleVisibleColumn,
} from "../src/business/DataGrid/data-grid-logic.ts";

const columns = [
  { key: "code", sortable: true, minWidth: 120, maxWidth: 240, filter: { type: "text" } },
  {
    key: "status",
    sortable: true,
    filterValue: (row) => row.status,
    filter: { type: "multiSelect", options: [{ label: "待收货", value: "released" }, { label: "已完成", value: "completed" }] },
  },
  { key: "expectedDate", filterValue: (row) => row.expectedDate, filter: { type: "dateRange" } },
  { key: "qty", filterValue: (row) => row.qty, filter: { type: "numberRange" } },
  { key: "action", hideable: false, copyable: false },
];
const rows = [
  { code: "ASN-010", status: "released", statusLabel: "待收货", expectedDate: "2026-06-28T10:00:00.000Z", qty: 10, action: "详情" },
  { code: "ASN-002", status: "completed", statusLabel: "已完成", expectedDate: "2026-06-29T10:00:00.000Z", qty: 20, action: "详情" },
  { code: "ASN-001", status: "putaway", statusLabel: "上架中", expectedDate: "2026-07-01T10:00:00.000Z", qty: 30, action: "详情" },
];

assert.deepEqual(nextSortState(null, "code"), { key: "code", direction: "asc" });
assert.deepEqual(nextSortState({ key: "code", direction: "asc" }, "code"), { key: "code", direction: "desc" });
assert.equal(nextSortState({ key: "code", direction: "desc" }, "code"), null);

const settings = sanitizeGridState({ visibleColumns: ["code"], columnOrder: ["status", "code"], pageSize: 2 }, columns, [2, 20], 20);
assert.deepEqual([...settings.visibleColumns].sort(), ["action", "code"]);
assert.deepEqual(settings.columnOrder, ["status", "code", "expectedDate", "qty", "action"]);
assert.equal(settings.pageSize, 2);

assert.deepEqual(toggleVisibleColumn(["code"], columns, "code", false), ["code"]);
assert.deepEqual(toggleVisibleColumn(["code", "status"], columns, "status", false), ["code"]);
assert.deepEqual(moveColumnBefore(settings.columnOrder, columns, "qty", "code"), ["status", "qty", "code", "expectedDate", "action"]);
assert.deepEqual(moveColumnBefore(settings.columnOrder, columns, "missing", "code"), settings.columnOrder);
assert.deepEqual(settings.copyableColumns, ["code", "status", "expectedDate", "qty"]);

const copySettings = sanitizeGridState({ copyableColumns: ["code", "action", "missing"] }, columns, [2, 20], 20);
assert.deepEqual(copySettings.copyableColumns, ["code"]);
assert.deepEqual(toggleCopyableColumn(["code"], columns, "status", true), ["code", "status"]);
assert.deepEqual(toggleCopyableColumn(["code"], columns, "action", true), ["code"]);
assert.deepEqual(toggleCopyableColumn(["code", "status"], columns, "status", false), ["code"]);
assert.equal(getDataGridCopyText(rows[0], { key: "status", copyValue: (row) => row.statusLabel }), "待收货");
assert.deepEqual(dataGridFilterConfigForData(columns[1], rows.slice(0, 2))?.options, [
  { label: "待收货", value: "released" },
  { label: "已完成", value: "completed" },
]);
assert.deepEqual(
  sanitizeDataGridColumnFiltersForData({ status: ["released", "putaway"], code: "ASN" }, columns, rows.slice(0, 2)),
  { status: ["released"], code: "ASN" },
);

const widthSettings = sanitizeGridState(
  { columnWidths: { code: 260, missing: 180, action: 200 } },
  columns,
  [2, 20],
  20,
);
assert.deepEqual(widthSettings.columnWidths, { code: 240, action: 200 });
assert.deepEqual(setColumnWidth({}, columns, "code", 80), { code: 120 });
assert.deepEqual(setColumnWidth({ code: 160, status: 180 }, columns, "code", null), { status: 180 });
assert.deepEqual(setColumnWidth({}, columns, "missing", 180), {});
assert.equal(dataGridTableWidth([{ key: "select", width: 44 }, { key: "code", width: 120 }, { key: "notes" }]), 324);
assert.deepEqual(
  dataGridFloatingPanelPosition({ top: 120, left: 300, right: 332 }, { width: 640, height: 480 }, 256),
  { top: 120, left: 36, maxHeight: 352 },
);
assert.deepEqual(
  dataGridFloatingPanelPosition({ top: 400, left: 40, right: 72 }, { width: 640, height: 480 }, 256),
  { top: 312, left: 80, maxHeight: 160 },
);

const page = getDataGridPage({
  data: rows,
  columns,
  visibleColumns: ["code", "status", "action"],
  columnFilters: { code: "ASN" },
  sort: { key: "code", direction: "asc" },
  pageIndex: 0,
  pageSize: 2,
});

assert.deepEqual(
  page.rows.map((row) => row.code),
  ["ASN-001", "ASN-002"],
);
assert.equal(page.total, 3);
assert.equal(page.pageCount, 2);
assert.equal(page.rangeStart, 1);
assert.equal(page.rangeEnd, 2);

const filtered = getDataGridPage({
  data: rows,
  columns,
  visibleColumns: ["status"],
  columnFilters: { status: ["released"] },
  sort: null,
  pageIndex: 0,
  pageSize: 20,
});

assert.deepEqual(
  filtered.rows.map((row) => row.code),
  ["ASN-010"],
);

const multiFieldFiltered = getDataGridPage({
  data: rows,
  columns,
  visibleColumns: ["code", "status"],
  columnFilters: { code: "ASN", status: ["completed"] },
  sort: null,
  pageIndex: 0,
  pageSize: 20,
});

assert.deepEqual(
  multiFieldFiltered.rows.map((row) => row.code),
  ["ASN-002"],
);

const multiSelected = getDataGridPage({
  data: rows,
  columns,
  visibleColumns: ["status"],
  columnFilters: { status: ["released", "completed"] },
  sort: { key: "code", direction: "asc" },
  pageIndex: 0,
  pageSize: 20,
});

assert.deepEqual(
  multiSelected.rows.map((row) => row.code),
  ["ASN-002", "ASN-010"],
);

const dateRangeFiltered = getDataGridPage({
  data: rows,
  columns,
  visibleColumns: ["expectedDate"],
  columnFilters: { expectedDate: { from: "2026-06-29", to: "2026-06-30" } },
  sort: null,
  pageIndex: 0,
  pageSize: 20,
});

assert.deepEqual(
  dateRangeFiltered.rows.map((row) => row.code),
  ["ASN-002"],
);

const numberRangeFiltered = getDataGridPage({
  data: rows,
  columns,
  visibleColumns: ["qty"],
  columnFilters: { qty: { from: "15", to: "30" } },
  sort: null,
  pageIndex: 0,
  pageSize: 20,
});

assert.deepEqual(
  numberRangeFiltered.rows.map((row) => row.code),
  ["ASN-002", "ASN-001"],
);
