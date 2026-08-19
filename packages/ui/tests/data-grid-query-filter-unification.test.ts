import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

console.log("Running DataGrid Query & Filter unification test...");

const chipsSource = fs.readFileSync(
  path.resolve(__dirname, "../src/business/DataGrid/DataGridFilterChips.tsx"),
  "utf8",
);
const gridSource = fs.readFileSync(
  path.resolve(__dirname, "../src/business/DataGrid/DataGrid.tsx"),
  "utf8",
);

// 1. 验证 FilterChips 支持 querySummaryItems 接口与已应用条件文案
assert.match(chipsSource, /querySummaryItems\?: DataGridQuerySummaryItem\[\]/, "DataGridFilterChipsProps 必须包含 querySummaryItems");
assert.match(chipsSource, /已应用条件/, "FilterChips 标签栏标题统一为【已应用条件】");
assert.match(chipsSource, /h-7/, "FilterChips 标签高度统一为紧凑优雅的 h-7");
assert.match(chipsSource, /rounded-md/, "FilterChips 标签圆角统一为 rounded-md");

// 2. 验证 DataGrid 内部桥接 querySummaryItems 到 FilterChips
assert.match(gridSource, /hasActiveQuerySummary\s*=\s*Boolean\(querySummaryItems/, "DataGrid 必须将 querySummaryItems 纳入激活筛选感知");
assert.match(gridSource, /querySummaryItems=\{querySummaryItems\}/, "DataGrid 必须将 querySummaryItems 传递给 DataGridFilterChips");
assert.match(gridSource, /onClearQueryState\?\.\(key\)/, "DataGridFilterChips 点击单个查询标签必须触发 onClearQueryState(key)");
assert.match(gridSource, /onClearQueryState\?\.\(\)/, "DataGridFilterChips 点击清除全部必须同时清空查询状态");

console.log("✅ DataGrid Query & Filter unification test passed!");
