import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const page = readFileSync(resolve(root, "src/pages/outbound/M4OutboundPage.tsx"), "utf8");

const pageHeaderStart = page.indexOf("<PageHeader");
const queryPanelStart = page.indexOf("<QueryPanel", pageHeaderStart);

assert.notEqual(pageHeaderStart, -1, "M4 页应保留 PageHeader");
assert.notEqual(queryPanelStart, -1, "M4 PageHeader 后应进入 QueryPanel");

const pageHeaderBlock = page.slice(pageHeaderStart, queryPanelStart);

assert.doesNotMatch(pageHeaderBlock, /refreshOutbound|globalThis\.print|window\.print|openAction\(meta\.createAction|RefreshCw|Printer|Plus|ArrowLeft|onBack/, "M4 页头不得重复放业务按钮或返回按钮");
assert.match(page, /role="status"/, "M4 页应展示 lastEvent 状态提示");

const dataGridCalls = [...page.matchAll(/<DataGrid\b[\s\S]*?\/>/g)].map((match) => match[0]);

assert.equal(dataGridCalls.length, 3, "M4 orders/waves/returns 三个列表都应使用 DataGrid");

for (const dataGridCall of dataGridCalls) {
  assert.match(dataGridCall, /refreshAction=\{gridRefreshAction\}/, "M4 DataGrid 应传 refreshAction");
  assert.match(dataGridCall, /createAction=\{gridCreateAction\}/, "M4 DataGrid 应传 createAction");
  assert.match(dataGridCall, /detailAction=\{gridDetailAction\}/, "M4 DataGrid 应传 detailAction");
  assert.match(dataGridCall, /printAction=\{gridPrintAction\}/, "M4 DataGrid 应传 printAction");
  assert.match(dataGridCall, /exportAction=\{gridExportAction\}/, "M4 DataGrid 应传 exportAction");
  assert.match(dataGridCall, /toolbarActions=\{gridToolbarActions\}/, "M4 DataGrid 应传 toolbarActions");
  assert.match(dataGridCall, /queryState=\{appliedQuery\}/, "M4 DataGrid 应保留查询状态");
  assert.match(dataGridCall, /querySummaryItems=\{querySummaryItems\}/, "M4 DataGrid 应保留筛选摘要");
  assert.match(dataGridCall, /selectedRowKeys=\{selectedId \? \[selectedId\] : \[\]\}/, "M4 DataGrid 应按 M2 口径传 selectedRowKeys");
  assert.match(dataGridCall, /onSelectedRowKeysChange=\{\(keys\) => setSelectedId\(keys\.at\(-1\) \?\? null\)\}/, "M4 DataGrid 应按 M2 口径更新选择态");
  assert.match(dataGridCall, /\bselectable\b/, "M4 DataGrid 应开启 selectable");
}

assert.doesNotMatch(page, /key:\s*"actions"/, "M4 列定义不得保留行内操作列");
assert.doesNotMatch(page, /function (OrderActions|WaveActions|ReturnActions|ActionButtons)\b/, "M4 行内操作按钮组件应移除");
assert.doesNotMatch(page, /table(Refresh|Create|Detail|Print|Export|Toolbar)Action/, "M4 页面不得保留旧 table* 动作命名");

console.log("m4 outbound DataGrid actions self-check passed");
