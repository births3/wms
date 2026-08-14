import assert from "node:assert/strict";
import {
  getOperatorsForFilterType,
  getDefaultOperatorForFilterType,
  operatorRequiresNoValue,
  getOperatorLabel,
  type DataGridFilterItem,
  type DataGridAdvancedFilterState,
} from "../src/business/DataGrid/data-grid-operators.ts";
import {
  rowMatchesFilterOperator,
  rowMatchesAdvancedFilters,
  type DataGridLogicColumn,
} from "../src/business/DataGrid/data-grid-logic.ts";

interface TestRow {
  id: string;
  code: string;
  count: number;
  date: string;
  status: string;
  category?: string | null;
}

const columns: DataGridLogicColumn<TestRow>[] = [
  { key: "id", filter: { type: "text" } },
  { key: "code", filter: { type: "text" } },
  { key: "count", filter: { type: "numberRange" } },
  { key: "date", filter: { type: "dateRange" } },
  {
    key: "status",
    filter: {
      type: "select",
      options: [
        { label: "合格", value: "pass" },
        { label: "待检", value: "pending" },
        { label: "不合格", value: "reject" },
      ],
    },
  },
  { key: "category", filter: { type: "text" } },
];

const sampleRows: TestRow[] = [
  { id: "1", code: "ASN-2026-001", count: 100, date: "2026-05-10", status: "pass", category: "冷藏" },
  { id: "2", code: "ASN-2026-002", count: 250, date: "2026-06-15", status: "pending", category: "常温" },
  { id: "3", code: "PO-2026-003", count: 50, date: "2026-07-20", status: "reject", category: null },
  { id: "4", code: "PO-2026-004", count: 500, date: "2026-08-01", status: "pass", category: "" },
];

// 1. 算子元数据检查
assert.equal(getDefaultOperatorForFilterType("text"), "contains");
assert.equal(getDefaultOperatorForFilterType("numberRange"), "gte");
assert.equal(getDefaultOperatorForFilterType("dateRange"), "between");
assert.equal(getDefaultOperatorForFilterType("select"), "eq");
assert.equal(getDefaultOperatorForFilterType("multiSelect"), "isAnyOf");

assert.equal(operatorRequiresNoValue("isEmpty"), true);
assert.equal(operatorRequiresNoValue("isNotEmpty"), true);
assert.equal(operatorRequiresNoValue("contains"), false);
assert.equal(getOperatorLabel("contains"), "包含");

// 2. 文本操作符匹配测试
assert.equal(rowMatchesFilterOperator("ASN-2026-001", "contains", "2026", "text"), true);
assert.equal(rowMatchesFilterOperator("ASN-2026-001", "contains", "9999", "text"), false);
assert.equal(rowMatchesFilterOperator("ASN-2026-001", "notContains", "PO", "text"), true);
assert.equal(rowMatchesFilterOperator("ASN-2026-001", "startsWith", "ASN", "text"), true);
assert.equal(rowMatchesFilterOperator("ASN-2026-001", "startsWith", "PO", "text"), false);
assert.equal(rowMatchesFilterOperator("ASN-2026-001", "endsWith", "001", "text"), true);
assert.equal(rowMatchesFilterOperator("ASN-2026-001", "eq", "ASN-2026-001", "text"), true);
assert.equal(rowMatchesFilterOperator("ASN-2026-001", "ne", "ASN-2026-001", "text"), false);

// 3. 空值与非空测试
assert.equal(rowMatchesFilterOperator(null, "isEmpty", undefined, "text"), true);
assert.equal(rowMatchesFilterOperator("", "isEmpty", undefined, "text"), true);
assert.equal(rowMatchesFilterOperator("冷藏", "isEmpty", undefined, "text"), false);
assert.equal(rowMatchesFilterOperator("冷藏", "isNotEmpty", undefined, "text"), true);
assert.equal(rowMatchesFilterOperator(null, "isNotEmpty", undefined, "text"), false);

// 4. 数字操作符测试
assert.equal(rowMatchesFilterOperator(100, "eq", "100", "numberRange"), true);
assert.equal(rowMatchesFilterOperator(100, "gt", "50", "numberRange"), true);
assert.equal(rowMatchesFilterOperator(100, "gt", "100", "numberRange"), false);
assert.equal(rowMatchesFilterOperator(100, "gte", "100", "numberRange"), true);
assert.equal(rowMatchesFilterOperator(100, "lt", "200", "numberRange"), true);
assert.equal(rowMatchesFilterOperator(100, "lte", "100", "numberRange"), true);
assert.equal(rowMatchesFilterOperator(100, "between", { from: "50", to: "150" }, "numberRange"), true);
assert.equal(rowMatchesFilterOperator(100, "between", { from: "120", to: "200" }, "numberRange"), false);

// 5. 日期操作符测试
assert.equal(rowMatchesFilterOperator("2026-06-15", "eq", "2026-06-15", "dateRange"), true);
assert.equal(rowMatchesFilterOperator("2026-06-15", "gt", "2026-06-01", "dateRange"), true);
assert.equal(rowMatchesFilterOperator("2026-06-15", "lt", "2026-06-01", "dateRange"), false);
assert.equal(rowMatchesFilterOperator("2026-06-15", "between", { from: "2026-06-01", to: "2026-06-30" }, "dateRange"), true);

// 6. 枚举/集合操作符测试
assert.equal(rowMatchesFilterOperator("pass", "isAnyOf", ["pass", "pending"], "select"), true);
assert.equal(rowMatchesFilterOperator("reject", "isAnyOf", ["pass", "pending"], "select"), false);
assert.equal(rowMatchesFilterOperator("reject", "isNoneOf", ["pass", "pending"], "select"), true);

// 7. 高级组合筛选 (AND / OR) 测试
const andFilterState: DataGridAdvancedFilterState = {
  joinOperator: "and",
  items: [
    { id: "f1", columnKey: "code", operator: "startsWith", value: "ASN" },
    { id: "f2", columnKey: "count", operator: "gt", value: "150" },
  ],
};
const andResults = sampleRows.filter((row) => rowMatchesAdvancedFilters(row, columns, andFilterState));
assert.equal(andResults.length, 1);
assert.equal(andResults[0]?.code, "ASN-2026-002");

const orFilterState: DataGridAdvancedFilterState = {
  joinOperator: "or",
  items: [
    { id: "f1", columnKey: "status", operator: "eq", value: "reject" },
    { id: "f2", columnKey: "count", operator: "gte", value: "500" },
  ],
};
const orResults = sampleRows.filter((row) => rowMatchesAdvancedFilters(row, columns, orFilterState));
assert.equal(orResults.length, 2);
assert.deepEqual(orResults.map((r) => r.id), ["3", "4"]);

console.log("✓ All data-grid-advanced-filters tests passed!");
