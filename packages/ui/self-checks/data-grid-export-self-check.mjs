import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  buildDataGridExport,
  buildDataGridCsv,
  dataGridExportFileName,
  defaultDataGridExportFileName,
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

assert.equal(defaultDataGridExportFileName("M2 收货管理", new Date("2026-07-04T09:34:00+08:00")), "M2收货管理_2607040934");
assert.equal(defaultDataGridExportFileName("M2/收货:管理", new Date("2026-07-04T09:34:00+08:00")), "M2收货管理_2607040934");
assert.equal(dataGridExportFileName("M2收货管理_2607040934.csv", "xlsx"), "M2收货管理_2607040934.xlsx");
const csvExport = buildDataGridExport({
  format: "csv",
  columns,
  visibleColumnKeys: ["status", "code"],
  rows,
});
assert.equal(csvExport.extension, "csv");
assert.match(csvExport.mimeType, /text\/csv/);
assert.equal(String(csvExport.content).startsWith("\uFEFF状态,单号"), true);
const xlsExport = buildDataGridExport({
  format: "xls",
  columns,
  visibleColumnKeys: ["status", "code"],
  rows,
});
assert.equal(xlsExport.extension, "xls");
assert.match(String(xlsExport.content), /<table>/);
const xlsxExport = buildDataGridExport({
  format: "xlsx",
  columns,
  visibleColumnKeys: ["status", "code"],
  rows,
});
assert.equal(xlsxExport.extension, "xlsx");
assert.equal(xlsxExport.content instanceof Uint8Array, true);
assert.deepEqual(Array.from(xlsxExport.content.slice(0, 4)), [0x50, 0x4b, 0x03, 0x04]);

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
const dataGridTypesSource = readFileSync(
  new URL("../src/business/DataGrid/data-grid-types.ts", import.meta.url),
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
const dataGridColumnsSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridColumns.tsx", import.meta.url),
  "utf8",
);
const dataGridExportHookSource = readFileSync(
  new URL("../src/business/DataGrid/use-data-grid-export.ts", import.meta.url),
  "utf8",
);
const dataGridColumnSettingsHookSource = readFileSync(
  new URL("../src/business/DataGrid/use-data-grid-column-settings.ts", import.meta.url),
  "utf8",
);
const dataGridContextMenuHookSource = readFileSync(
  new URL("../src/business/DataGrid/use-data-grid-context-menu.ts", import.meta.url),
  "utf8",
);
const dataGridHeaderSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridHeaderCell.tsx", import.meta.url),
  "utf8",
);
const dataTableSource = readFileSync(
  new URL("../src/business/DataTable/DataTable.tsx", import.meta.url),
  "utf8",
);
const dataGridFieldSettingsSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridFieldSettingsPanel.tsx", import.meta.url),
  "utf8",
);
const dataGridExportDialogSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridExportDialog.tsx", import.meta.url),
  "utf8",
);
const dataGridLogicSource = readFileSync(
  new URL("../src/business/DataGrid/data-grid-logic.ts", import.meta.url),
  "utf8",
);
const dataGridStorageSource = readFileSync(
  new URL("../src/business/DataGrid/data-grid-storage.ts", import.meta.url),
  "utf8",
);
const dataGridViewsSource = readFileSync(
  new URL("../src/business/DataGrid/data-grid-views.ts", import.meta.url),
  "utf8",
);
const dataGridNamedViewsSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridNamedViewsToolbar.tsx", import.meta.url),
  "utf8",
);
const dataGridActionSettingsSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridActionSettingsPanel.tsx", import.meta.url),
  "utf8",
);
const dataGridContextMenuSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridContextMenu.tsx", import.meta.url),
  "utf8",
);
const dataGridSummaryDialogSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridSummaryDialog.tsx", import.meta.url),
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
const masterDataPageSource = readFileSync(
  new URL("../../../apps/web-admin/src/pages/master-data/M1MasterDataPage.tsx", import.meta.url),
  "utf8",
);
const batchManagementPageSource = readFileSync(
  new URL("../../../apps/web-admin/src/pages/inventory/M3BatchManagementPage.tsx", import.meta.url),
  "utf8",
);
const outboundPageSource = readFileSync(
  new URL("../../../apps/web-admin/src/pages/outbound/M4OutboundPage.tsx", import.meta.url),
  "utf8",
);
const featureFlagPageSource = readFileSync(
  new URL("../../../apps/web-admin/src/pages/config-center/FeatureFlagConfigCenterPage.tsx", import.meta.url),
  "utf8",
);

assert.match(dataGridExportHookSource, /import \{[\s\S]*buildDataGridCsv,[\s\S]*buildDataGridExport,[\s\S]*dataGridExportFileName,[\s\S]*defaultDataGridExportFileName,[\s\S]*downloadDataGridCsv,[\s\S]*downloadDataGridExport,[\s\S]*type DataGridExportFormat,[\s\S]*\} from "\.\/data-grid-export";/s);
assert.match(dataGridToolbarSource, /import \{ Button \} from "\.\.\/\.\.\/ui\/button";/);
for (const icon of ["Download", "Printer"]) {
  assert.match(dataGridToolbarSource, new RegExp(`\\b${icon}\\b[\\s\\S]*from "lucide-react"`));
}
assert.match(dataGridContentSource, /\bX\b[\s\S]*from "lucide-react"/);
assert.match(dataGridTypesSource, /export interface DataGridRefreshAction/);
assert.match(dataGridTypesSource, /description\?: string;/);
assert.match(dataGridTypesSource, /export interface DataGridQueryAction/);
assert.match(dataGridTypesSource, /export interface DataGridCreateAction/);
assert.match(dataGridTypesSource, /export interface DataGridDetailAction/);
assert.match(dataGridTypesSource, /export interface DataGridEditAction/);
assert.match(dataGridTypesSource, /export interface DataGridDeleteAction/);
assert.match(dataGridTypesSource, /export interface DataGridDisableAction/);
assert.match(dataGridTypesSource, /export interface DataGridPrintAction/);
assert.match(dataGridTypesSource, /export interface DataGridExportAction/);
assert.match(dataGridTypesSource, /export interface DataGridPasteAction/);
assert.match(dataGridTypesSource, /refreshAction\?: DataGridRefreshAction;/);
assert.match(dataGridTypesSource, /queryAction\?: DataGridQueryAction;/);
assert.match(dataGridTypesSource, /createAction\?: DataGridCreateAction;/);
assert.match(dataGridTypesSource, /detailAction\?: DataGridDetailAction;/);
assert.match(dataGridTypesSource, /editAction\?: DataGridEditAction;/);
assert.match(dataGridTypesSource, /deleteAction\?: DataGridDeleteAction;/);
assert.match(dataGridTypesSource, /disableAction\?: DataGridDisableAction;/);
assert.match(dataGridTypesSource, /printAction\?: DataGridPrintAction \| false;/);
assert.match(dataGridTypesSource, /exportAction\?: DataGridExportAction \| false;/);
assert.match(dataGridTypesSource, /pasteAction\?: DataGridPasteAction<T>;/);
assert.match(dataGridTypesSource, /columnPasteAction\?: DataGridPasteAction<T>;/);
assert.match(dataGridTypesSource, /exportFileBaseName\?: string;/);
assert.match(
  dataGridExportHookSource,
  /const csv = buildDataGridCsv\(\{\s*columns: snapshot\.columns,\s*visibleColumnKeys: snapshot\.visibleColumnKeys,\s*rows: snapshot\.rows,\s*\}\);/s,
);
assert.match(
  dataGridExportHookSource,
  /fileName: snapshot\.storageKey \? `\$\{snapshot\.storageKey\}\.csv` : "data-grid\.csv"/,
);
assert.match(
  dataGridExportHookSource,
  /document: typeof document === "undefined" \? undefined : document/,
);
assert.match(dataGridToolbarSource, /refreshAction && visibleAction\("refresh"\)/);
assert.match(dataGridToolbarSource, /queryAction && visibleAction\("query"\)/);
assert.match(dataGridToolbarSource, /createAction && visibleAction\("create"\)/);
assert.match(dataGridToolbarSource, /detailAction && visibleAction\("detail"\)/);
assert.match(dataGridToolbarSource, /editAction && visibleAction\("edit"\)/);
assert.match(dataGridToolbarSource, /deleteAction && visibleAction\("delete"\)/);
assert.match(dataGridToolbarSource, /disableAction && visibleAction\("disable"\)/);
assert.match(dataGridToolbarSource, /<DataGridNamedViewsToolbar[\s\S]*actionKeys=\{actionKeys\}/);
assert.match(dataGridToolbarSource, /aria-label="字段显示"[\s\S]*字段/);
assert.match(dataGridToolbarSource, /printAction !== false && visibleAction\("print"\)/);
assert.match(dataGridToolbarSource, /exportAction !== false && visibleAction\("export"\)/);
assert.match(dataGridToolbarSource, /onOpenExportDialog\(\);/);
assert.match(dataGridToolbarSource, /toolbarActions\.filter\(\(action\) => visibleAction\(toolbarActionKey\(action\.key\)\)\)\.map/);
assert.match(dataGridContentSource, /<DataGridExportDialog/);
assert.match(dataGridExportHookSource, /defaultDataGridExportFileName\(resolveDataGridExportBaseName\(exportFileBaseName, caption, storageKey\)\)/);
assert.match(dataGridExportHookSource, /buildDataGridExport\(\{/);
assert.match(dataGridExportHookSource, /dataGridExportFileName\(exportFileName, payload\.extension\)/);
assert.match(dataGridExportDialogSource, /<DialogTitle>导出列表<\/DialogTitle>/);
assert.match(dataGridExportDialogSource, /<SelectItem value="xls">xls<\/SelectItem>/);
assert.match(dataGridExportDialogSource, /<SelectItem value="xlsx">xlsx<\/SelectItem>/);
assert.match(dataGridExportDialogSource, /<SelectItem value="csv">csv<\/SelectItem>/);
assert.match(dataGridToolbarSource, /flex min-w-0 flex-1 flex-wrap items-center gap-2 \[\&_svg\]:size-4/);
assert.doesNotMatch(dataGridToolbarSource, /功能能力|私有能力/);
assert.doesNotMatch(dataGridHeaderSource, /namedViewsControl|onToggleFields|字段设置|Settings2/);
assert.match(dataGridSource, /orderedColumnsWithFrozen/);
assert.match(dataGridSource, /dataGridFrozenColumnOffsets/);
assert.match(dataGridColumnsSource, /data-grid-frozen-column/);
assert.match(dataGridColumnsSource, /left: frozenLeft/);
assert.match(dataGridColumnSettingsHookSource, /toggleFrozenColumn/);
assert.match(dataGridSource, /const frozenKeys = new Set\(settings\.frozenColumns\);/);
assert.match(dataGridSource, /frozenKeys=\{frozenKeys\}/);
assert.match(dataGridSource, /onColumnFrozenChange=\{columnSettings\.updateColumnFrozen\}/);
assert.match(dataGridSource, /const hiddenActionKeys = new Set\(settings\.hiddenActions\);/);
assert.match(dataGridSource, /hasHiddenToolbarActions/);
assert.match(dataGridToolbarSource, /aria-label="按钮功能"/);
assert.match(dataGridToolbarSource, /title=\{hasHiddenToolbarActions \? "按钮功能显示设置；有隐藏按钮功能" : "按钮功能显示设置"\}/);
assert.match(dataGridToolbarSource, /absolute -left-1 -top-1 size-2 rounded-full bg-destructive/);
assert.match(dataGridToolbarSource, /visibleAction\("summary"\)/);
assert.match(dataGridToolbarSource, /visibleAction\("export"\)/);
assert.match(dataGridToolbarSource, /visibleAction\(toolbarActionKey\(action\.key\)\)/);
assert.match(dataGridContentSource, /<DataGridContextMenu/);
assert.match(dataGridSource, /onPaste=\{\(\) => void contextMenuState\.pasteFromContext\("cell"\)\}/);
assert.match(dataGridSource, /onColumnPaste=\{\(\) => void contextMenuState\.pasteFromContext\("column"\)\}/);
assert.match(dataGridContextMenuHookSource, /buildDataGridClipboardText/);
assert.match(dataGridColumnsSource, /data-datagrid-cell/);
assert.match(dataGridContentSource, /<DataGridSummaryDialog/);
assert.match(dataGridFieldSettingsSource, /frozenKeys: Set<string>;/);
assert.match(dataGridFieldSettingsSource, /onColumnFrozenChange: \(key: string, frozen: boolean\) => void;/);
assert.match(dataGridFieldSettingsSource, /冻结/);
assert.match(dataGridActionSettingsSource, /DataGridActionSettingsPanel/);
assert.match(dataGridActionSettingsSource, /按钮功能显示设置/);
assert.match(dataGridActionSettingsSource, /全选/);
assert.match(dataGridActionSettingsSource, /取消/);
assert.match(dataGridActionSettingsSource, /aria-label="全选或取消按钮功能"/);
assert.doesNotMatch(dataGridActionSettingsSource, /from "\.\.\/\.\.\/ui\/button"/);
assert.match(dataGridActionSettingsSource, /onActionVisibleChange/);
assert.match(dataGridContextMenuSource, /行复制/);
assert.match(dataGridContextMenuSource, /行复制加表头/);
assert.match(dataGridContextMenuSource, /启动区域选择/);
assert.match(dataGridContextMenuSource, /关闭区域选择/);
assert.match(dataGridContextMenuSource, /复制区域/);
assert.match(dataGridContextMenuSource, /粘贴/);
assert.match(dataGridContextMenuSource, /列粘贴/);
assert.match(dataGridContextMenuSource, /区域求和/);
assert.match(dataGridContextMenuHookSource, /buildDataGridSelectedAreaSumText/);
assert.match(dataGridContextMenuHookSource, /copySelectedAreaSum/);
assert.match(dataGridSummaryDialogSource, /汇总统计/);
assert.match(dataGridSummaryDialogSource, /分组字段/);
assert.match(dataGridSummaryDialogSource, /汇总字段/);
assert.match(dataTableSource, /headerProps\?:/);
assert.match(dataTableSource, /col\.headerProps/);
assert.match(dataGridLogicSource, /frozenColumns: string\[\];/);
assert.match(dataGridLogicSource, /hiddenActions: string\[\];/);
assert.match(dataGridLogicSource, /columnFilters: DataGridColumnFilters;/);
assert.match(dataGridLogicSource, /export function orderedColumnsWithFrozen/);
assert.match(dataGridLogicSource, /export function toggleFrozenColumn/);
assert.match(dataGridLogicSource, /export function toggleHiddenAction/);
assert.match(dataGridStorageSource, /frozenColumns: Array\.isArray\(record\.frozenColumns\)/);
assert.match(dataGridStorageSource, /hiddenActions: Array\.isArray\(record\.hiddenActions\)/);
assert.match(dataGridViewsSource, /frozenColumns: Array\.isArray\(record\.frozenColumns\)/);
assert.match(dataGridViewsSource, /hiddenActions: Array\.isArray\(record\.hiddenActions\)/);
assert.match(dataGridNamedViewsSource, /aria-label="视图"[\s\S]*<Bookmark className="size-4" aria-hidden \/>[\s\S]*视图/s);
assert.match(dataGridNamedViewsSource, /title="视图保存、应用、删除"/);
assert.match(dataGridTypesSource, /csvExportPlacement\?: "toolbar" \| "external";/);
assert.match(dataGridTypesSource, /onCsvExportStateChange\?: \(state: DataGridCsvExportState \| null\) => void;/);
assert.doesNotMatch(inboundPageSource, /导出 CSV/);
assert.match(inboundPageSource, /function openCreateDialog\(\)[\s\S]*setActiveDialog\("create"\)[\s\S]*label: "新增"[\s\S]*description: "新建 ASN"[\s\S]*onClick: openCreateDialog/);
assert.match(inboundPageSource, /label: "刷新"[\s\S]*refreshInbound\(\)/);
assert.match(inboundPageSource, /exportFileBaseName=\{pageMeta\.title\}/);
assert.doesNotMatch(inboundPageSource, /<RefreshCw className="size-4" aria-hidden \/>/);
assert.match(inboundTableSource, /refreshAction=\{refreshAction\}/);
assert.match(inboundTableSource, /createAction=\{createAction\}/);
assert.match(inboundTableSource, /detailAction=\{detailAction\}/);
assert.match(inboundTableSource, /toolbarActions=\{privateActions\}/);
assert.doesNotMatch(inboundTableSource, /header: "操作"/);
assert.match(masterDataPageSource, /exportFileBaseName=\{meta\.title\}/);
assert.match(masterDataPageSource, /refreshAction=\{gridRefreshAction\}/);
assert.match(masterDataPageSource, /createAction=\{gridCreateAction\}/);
assert.match(masterDataPageSource, /editAction=\{gridEditAction\}/);
assert.match(masterDataPageSource, /disableAction=\{gridDisableAction\}/);
assert.match(masterDataPageSource, /toolbarActions=\{gridToolbarActions\}/);
assert.match(masterDataPageSource, /viewId === "m1-business-partners"[\s\S]*key: "supplier-create"[\s\S]*key: "supplier-import"[\s\S]*key: "customer-create"[\s\S]*key: "customer-import"/);
assert.match(masterDataPageSource, /label: "供入"[\s\S]*description: "批量导入供应商"/);
assert.match(masterDataPageSource, /label: "客入"[\s\S]*description: "批量导入客户"/);
assert.doesNotMatch(masterDataPageSource, /导供|导客/);
assert.match(batchManagementPageSource, /exportFileBaseName="M3 批号管理"/);
assert.match(batchManagementPageSource, /refreshAction=\{gridRefreshAction\}/);
assert.match(outboundPageSource, /exportFileBaseName=\{meta\.title\}/);
assert.match(outboundPageSource, /refreshAction=\{gridRefreshAction\}/);
assert.match(outboundPageSource, /createAction=\{gridCreateAction\}/);
assert.match(outboundPageSource, /detailAction=\{gridDetailAction\}/);
assert.match(outboundPageSource, /const gridPrintAction: DataGridPrintAction = \{[\s\S]*label: "打印"[\s\S]*description: `打印\$\{meta\.title\}`/);
assert.match(outboundPageSource, /printAction=\{gridPrintAction\}/);
assert.match(featureFlagPageSource, /exportFileBaseName="功能开关配置中心"/);
assert.match(featureFlagPageSource, /refreshAction=\{gridRefreshAction\}/);
assert.match(featureFlagPageSource, /toolbarActions=\{gridToolbarActions\}/);
assert.match(inboundDetailViewModelSource, /export const inboundDetailFieldSections/);
assert.match(inboundDetailViewModelSource, /export const productInfoFieldDefinitions/);
assert.match(inboundDetailViewModelSource, /export const batchInfoFieldDefinitions/);
assert.match(inboundDetailSource, /inboundDetailFieldSections\.product\.title/);
assert.match(inboundDetailSource, /inboundDetailFieldSections\.order\.title/);
assert.match(inboundDetailSource, /inboundDetailFieldSections\.batch\.title/);
assert.match(inboundDetailSource, /inboundDetailFieldSections\.process\.title/);
