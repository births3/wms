import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  buildDataGridFilterSummaryItems,
  clearDataGridFilterKey,
} from "../src/business/DataGrid/data-grid-filter-summary.ts";

const fields = [
  { key: "code", label: "单号", filter: { type: "text" } },
  {
    key: "status",
    label: "状态",
    filter: {
      type: "select",
      options: [
        { label: "待收货", value: "released" },
        { label: "已完成", value: "completed" },
      ],
    },
  },
  {
    key: "workflowStatus",
    label: "流程状态",
    filter: {
      type: "multiSelect",
      options: [
        { label: "待收货", value: "released" },
        { label: "已完成", value: "completed" },
      ],
    },
  },
  { key: "createdAt", label: "创建时间", filter: { type: "dateRange" } },
  { key: "quantity", label: "数量", filter: { type: "numberRange" } },
  { key: "operation", label: "操作", filter: false },
];

const filters = {
  code: " ASN-001 ",
  status: "released",
  workflowStatus: ["released", "completed"],
  createdAt: { from: "2026-06-01", to: "2026-06-30" },
  quantity: { from: "10" },
  operation: "不应显示",
  ignoredText: "   ",
  ignoredRange: {},
};

assert.deepEqual(buildDataGridFilterSummaryItems(filters, fields), [
  { key: "code", label: "单号", value: "ASN-001", text: "单号：ASN-001" },
  { key: "status", label: "状态", value: "待收货", text: "状态：待收货" },
  {
    key: "workflowStatus",
    label: "流程状态",
    value: "待收货、已完成",
    text: "流程状态：待收货、已完成",
  },
  {
    key: "createdAt",
    label: "创建时间",
    value: "2026-06-01 至 2026-06-30",
    text: "创建时间：2026-06-01 至 2026-06-30",
  },
  { key: "quantity", label: "数量", value: ">= 10", text: "数量：>= 10" },
]);

assert.deepEqual(
  buildDataGridFilterSummaryItems(
    { createdAt: { to: "2026-06-30" }, quantity: { to: "50" } },
    fields,
  ),
  [
    {
      key: "createdAt",
      label: "创建时间",
      value: "<= 2026-06-30",
      text: "创建时间：<= 2026-06-30",
    },
    { key: "quantity", label: "数量", value: "<= 50", text: "数量：<= 50" },
  ],
);

const cleared = clearDataGridFilterKey(filters, "status");
assert.deepEqual(cleared, {
  code: " ASN-001 ",
  workflowStatus: ["released", "completed"],
  createdAt: { from: "2026-06-01", to: "2026-06-30" },
  quantity: { from: "10" },
  operation: "不应显示",
  ignoredText: "   ",
  ignoredRange: {},
});
assert.equal(clearDataGridFilterKey(filters, "missing"), filters);

assert.deepEqual(buildDataGridFilterSummaryItems({}, fields), []);

const dataGridSource = readFileSync(
  new URL("../src/business/DataGrid/DataGrid.tsx", import.meta.url),
  "utf8",
);
assert.match(dataGridSource, /<DataGridFilterChips\b/);
assert.match(dataGridSource, /onClearAll=\{\(\) => setColumnFilters\(\{\}\)\}/);
