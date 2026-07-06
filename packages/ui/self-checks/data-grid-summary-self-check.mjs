import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  buildDataGridSummaryGroups,
  buildDataGridSummaryResults,
  buildDataGridSummaryTable,
} from "../src/business/DataGrid/data-grid-summary.ts";

const columns = [
  { key: "sku", header: "商品" },
  { key: "qty", header: "数量" },
  { key: "amount", header: "金额", copyValue: (row) => row.amountText },
  { key: "hidden", header: "隐藏列" },
];

const rows = [
  { sku: "A", qty: 10, amountText: "12.50", hidden: 1 },
  { sku: "B", qty: 20, amountText: "7.5", hidden: 2 },
  { sku: "C", qty: "bad", amountText: "", hidden: 3 },
];

assert.deepEqual(
  buildDataGridSummaryResults(columns, rows, [
    { columnKey: "qty", type: "sum" },
    { columnKey: "qty", type: "avg" },
    { columnKey: "amount", type: "max" },
    { columnKey: "amount", type: "min" },
    { columnKey: "missing", type: "sum" },
  ]),
  [
    { columnKey: "qty", type: "sum", value: "30", count: 2 },
    { columnKey: "qty", type: "avg", value: "15", count: 2 },
    { columnKey: "amount", type: "max", value: "12.5", count: 2 },
    { columnKey: "amount", type: "min", value: "7.5", count: 2 },
  ],
);

assert.deepEqual(
  buildDataGridSummaryGroups(columns, rows, ["sku"], [
    { columnKey: "qty", type: "sum" },
    { columnKey: "amount", type: "avg" },
  ]),
  [
    {
      key: "A",
      label: "商品：A",
      rowCount: 1,
      results: [
        { columnKey: "qty", type: "sum", value: "10", count: 1 },
        { columnKey: "amount", type: "avg", value: "12.5", count: 1 },
      ],
    },
    {
      key: "B",
      label: "商品：B",
      rowCount: 1,
      results: [
        { columnKey: "qty", type: "sum", value: "20", count: 1 },
        { columnKey: "amount", type: "avg", value: "7.5", count: 1 },
      ],
    },
    {
      key: "C",
      label: "商品：C",
      rowCount: 1,
      results: [
        { columnKey: "qty", type: "sum", value: "-", count: 0 },
        { columnKey: "amount", type: "avg", value: "-", count: 0 },
      ],
    },
  ],
);

assert.deepEqual(
  buildDataGridSummaryTable(columns, rows, ["sku"], [
    { columnKey: "qty", type: "sum" },
    { columnKey: "amount", type: "avg" },
  ]),
  {
    columns: [
      { key: "group:sku", label: "商品" },
      { key: "__summaryRowCount", label: "行数" },
      { key: "summary:qty:sum", label: "数量 求和" },
      { key: "summary:amount:avg", label: "金额 平均" },
    ],
    rows: [
      { __summaryKey: "A", "group:sku": "A", __summaryRowCount: 1, "summary:qty:sum": "10", "summary:amount:avg": "12.5" },
      { __summaryKey: "B", "group:sku": "B", __summaryRowCount: 1, "summary:qty:sum": "20", "summary:amount:avg": "7.5" },
      { __summaryKey: "C", "group:sku": "C", __summaryRowCount: 1, "summary:qty:sum": "-", "summary:amount:avg": "-" },
    ],
  },
);

const dataGridSource = readFileSync(
  new URL("../src/business/DataGrid/DataGrid.tsx", import.meta.url),
  "utf8",
);
const dataGridToolbarSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridToolbar.tsx", import.meta.url),
  "utf8",
);
const dataGridContentSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridContent.tsx", import.meta.url),
  "utf8",
);
const dataGridHelpersSource = readFileSync(
  new URL("../src/business/DataGrid/data-grid-helpers.ts", import.meta.url),
  "utf8",
);
const summaryDialogSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridSummaryDialog.tsx", import.meta.url),
  "utf8",
);

assert.match(dataGridToolbarSource, /<Calculator className="size-4" aria-hidden \/>[\s\S]*汇总/);
assert.match(dataGridSource, /summaryConfig/);
assert.match(dataGridSource, /summaryTable/);
assert.match(dataGridContentSource, /退出汇总/);
assert.match(dataGridContentSource, /<DataGridSummaryDialog[\s\S]*onApply=\{onApplySummary\}/);
assert.match(dataGridHelpersSource, /actions\.push\(\{ key: "summary", label: "汇总", description: "汇总统计" \}\)/);
assert.match(summaryDialogSource, /<DialogTitle[\s\S]*汇总统计/);
assert.match(summaryDialogSource, /分组字段/);
assert.match(summaryDialogSource, /汇总字段/);
assert.match(summaryDialogSource, /应用/);
assert.doesNotMatch(summaryDialogSource, /统计结果/);
assert.match(summaryDialogSource, /求和/);
assert.match(summaryDialogSource, /平均/);
assert.match(summaryDialogSource, /最大/);
assert.match(summaryDialogSource, /最小/);
