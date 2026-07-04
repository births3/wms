import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  buildDataGridCsv,
  downloadDataGridCsv,
} from "../src/business/DataGrid/data-grid-export.ts";

const columns = [
  { key: "code", header: "单号" },
  {
    key: "status",
    header: "状态",
    filterValue: (row) => row.status,
    copyValue: (row) => row.statusLabel,
  },
  { key: "note", header: "备注" },
  { key: "hidden", header: "隐藏列" },
  { key: "createdAt", header: "创建时间" },
  { key: "tags", header: "标签" },
  { key: "meta", header: "元数据" },
  { key: "empty", header: "空值" },
  { key: "operator", header: { label: "操作人" }, filterValue: (row) => row.operatorName },
];

const rows = [
  {
    code: "ASN-001",
    status: "released",
    statusLabel: '待"收,货',
    note: "冷链\n换行",
    hidden: "不应导出",
    createdAt: new Date("2026-06-30T00:00:00.000Z"),
    tags: ["冷链", "急,件"],
    meta: { owner: "华东", count: 2 },
    empty: null,
    operatorName: "张三",
  },
];

const csv = buildDataGridCsv({
  columns,
  visibleColumnKeys: ["status", "code", "note", "createdAt", "tags", "meta", "empty", "operator"],
  rows,
});

assert.equal(
  csv,
  [
    "状态,单号,备注,创建时间,标签,元数据,空值,operator",
    '"待""收,货",ASN-001,"冷链\n换行",2026-06-30T00:00:00.000Z,' +
      '"冷链 急,件",华东 2,,张三',
  ].join("\n"),
);
assert.equal(csv.includes("隐藏列"), false);
assert.equal(csv.includes("不应导出"), false);
assert.equal(downloadDataGridCsv({ csv, fileName: "data-grid.xls" }), false);

let removed = false;
let revokedHref = "";
const failingLink = {
  href: "",
  download: "",
  click() {
    throw new Error("click failed");
  },
  remove() {
    removed = true;
  },
};
const fakeDocument = {
  body: {
    appendChild(link) {
      assert.equal(link, failingLink);
    },
  },
  createElement(tag) {
    assert.equal(tag, "a");
    return failingLink;
  },
};
const fakeUrl = {
  createObjectURL() {
    return "blob:data-grid";
  },
  revokeObjectURL(href) {
    revokedHref = href;
  },
};

assert.throws(
  () =>
    downloadDataGridCsv({
      csv,
      fileName: "data-grid.xls",
      document: fakeDocument,
      url: fakeUrl,
    }),
  /click failed/,
);
assert.equal(removed, true);
assert.equal(revokedHref, "blob:data-grid");

const dataGridSource = readFileSync(
  new URL("../src/business/DataGrid/DataGrid.tsx", import.meta.url),
  "utf8",
);
const dataGridHeaderSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridHeaderCell.tsx", import.meta.url),
  "utf8",
);
const dataGridNamedViewsSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridNamedViewsToolbar.tsx", import.meta.url),
  "utf8",
);
const inboundPageSource = readFileSync(
  new URL("../../../apps/web-admin/src/pages/inbound/M2InboundPage.tsx", import.meta.url),
  "utf8",
);
const inboundTableSource = readFileSync(
  new URL("../../../apps/web-admin/src/pages/inbound/M2InboundOrderTable.tsx", import.meta.url),
  "utf8",
);
const inboundDetailSource = readFileSync(
  new URL("../../../apps/web-admin/src/pages/inbound/M2InboundDetailDialog.tsx", import.meta.url),
  "utf8",
);
const inboundDetailViewModelSource = readFileSync(
  new URL("../../../apps/web-admin/src/pages/inbound/m2-inbound-detail-view-model.ts", import.meta.url),
  "utf8",
);

assert.match(
  dataGridSource,
  /import \{\s*buildDataGridCsv,\s*downloadDataGridCsv\s*\} from "\.\/data-grid-export";/s,
);
assert.match(dataGridSource, /import \{ Button \} from "\.\.\/\.\.\/ui\/button";/);
assert.match(
  dataGridSource,
  /import \{ Ban, Download, Eye, Pencil, Plus, Printer, RefreshCw, Search, Settings2, Trash2 \} from "lucide-react";/,
);
assert.match(dataGridSource, /export interface DataGridRefreshAction/);
assert.match(dataGridSource, /export interface DataGridQueryAction/);
assert.match(dataGridSource, /export interface DataGridCreateAction/);
assert.match(dataGridSource, /export interface DataGridDetailAction/);
assert.match(dataGridSource, /export interface DataGridEditAction/);
assert.match(dataGridSource, /export interface DataGridDeleteAction/);
assert.match(dataGridSource, /export interface DataGridDisableAction/);
assert.match(dataGridSource, /export interface DataGridPrintAction/);
assert.match(dataGridSource, /export interface DataGridExportAction/);
assert.match(dataGridSource, /refreshAction\?: DataGridRefreshAction;/);
assert.match(dataGridSource, /queryAction\?: DataGridQueryAction;/);
assert.match(dataGridSource, /createAction\?: DataGridCreateAction;/);
assert.match(dataGridSource, /detailAction\?: DataGridDetailAction;/);
assert.match(dataGridSource, /editAction\?: DataGridEditAction;/);
assert.match(dataGridSource, /deleteAction\?: DataGridDeleteAction;/);
assert.match(dataGridSource, /disableAction\?: DataGridDisableAction;/);
assert.match(dataGridSource, /printAction\?: DataGridPrintAction \| false;/);
assert.match(dataGridSource, /exportAction\?: DataGridExportAction \| false;/);
assert.match(
  dataGridSource,
  /const csv = buildDataGridCsv\(\{\s*columns: snapshot\.columns,\s*visibleColumnKeys: snapshot\.visibleColumnKeys,\s*rows: snapshot\.rows,\s*\}\);/s,
);
assert.match(
  dataGridSource,
  /fileName: snapshot\.storageKey \? `\$\{snapshot\.storageKey\}\.xls` : "data-grid\.xls"/,
);
assert.match(
  dataGridSource,
  /document: typeof document === "undefined" \? undefined : document/,
);
assert.match(
  dataGridSource,
  /功能能力[\s\S]*refreshAction[\s\S]*<RefreshCw className="size-4" aria-hidden \/>[\s\S]*refreshAction\.label \?\? "刷新"[\s\S]*queryAction[\s\S]*<Search className="size-4" aria-hidden \/>[\s\S]*queryAction\.label \?\? "查询"[\s\S]*createAction[\s\S]*<Plus className="size-4" aria-hidden \/>[\s\S]*createAction\.label \?\? "新增"[\s\S]*detailAction[\s\S]*<Eye className="size-4" aria-hidden \/>[\s\S]*detailAction\.label \?\? "详情"[\s\S]*editAction[\s\S]*<Pencil className="size-4" aria-hidden \/>[\s\S]*editAction\.label \?\? "修改"[\s\S]*deleteAction[\s\S]*<Trash2 className="size-4" aria-hidden \/>[\s\S]*deleteAction\.label \?\? "删除"[\s\S]*disableAction[\s\S]*<Ban className="size-4" aria-hidden \/>[\s\S]*disableAction\.label \?\? "停用"[\s\S]*<DataGridNamedViewsToolbar[\s\S]*<Settings2 className="size-4" aria-hidden \/>[\s\S]*字段显示[\s\S]*printAction !== false[\s\S]*打印[\s\S]*exportAction !== false[\s\S]*导出 Excel[\s\S]*私有能力[\s\S]*toolbarActions\.map\(\(action\) =>[\s\S]*action\.label/s,
);
assert.doesNotMatch(dataGridHeaderSource, /namedViewsControl|onToggleFields|字段设置|Settings2/);
assert.match(dataGridNamedViewsSource, /aria-label="视图"[\s\S]*<Bookmark className="size-4" aria-hidden \/>[\s\S]*视图/s);
assert.match(dataGridSource, /csvExportPlacement\?: "toolbar" \| "external";/);
assert.match(dataGridSource, /onCsvExportStateChange\?: \(state: DataGridCsvExportState \| null\) => void;/);
assert.doesNotMatch(inboundPageSource, /导出 CSV/);
assert.match(inboundPageSource, /label: "新建 ASN"[\s\S]*onClick: \(\) => setActiveDialog\("create"\)/);
assert.match(inboundPageSource, /label: "刷新"[\s\S]*refreshInbound\(\)/);
assert.doesNotMatch(inboundPageSource, /<RefreshCw className="size-4" aria-hidden \/>/);
assert.match(inboundTableSource, /refreshAction=\{refreshAction\}/);
assert.match(inboundTableSource, /createAction=\{createAction\}/);
assert.match(inboundTableSource, /detailAction=\{detailAction\}/);
assert.match(inboundTableSource, /toolbarActions=\{privateActions\}/);
assert.doesNotMatch(inboundTableSource, /header: "操作"/);
assert.match(inboundDetailViewModelSource, /export const inboundDetailFieldSections/);
assert.match(inboundDetailViewModelSource, /export const productInfoFieldDefinitions/);
assert.match(inboundDetailViewModelSource, /export const batchInfoFieldDefinitions/);
assert.match(inboundDetailSource, /inboundDetailFieldSections\.product\.title/);
assert.match(inboundDetailSource, /inboundDetailFieldSections\.order\.title/);
assert.match(inboundDetailSource, /inboundDetailFieldSections\.batch\.title/);
assert.match(inboundDetailSource, /inboundDetailFieldSections\.process\.title/);
