import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const appShell = readFileSync(resolve(root, "src/App.tsx"), "utf8");
const pageFile = readFileSync(resolve(root, "src/pages/outbound/M4OutboundPage.tsx"), "utf8");
const actionDialog = readFileSync(resolve(root, "src/pages/outbound/M4OutboundActionDialog.tsx"), "utf8");
const page = `${pageFile}\n${actionDialog}`;
const parts = readFileSync(resolve(root, "src/pages/outbound/M4OutboundPageParts.tsx"), "utf8");
const detail = readFileSync(resolve(root, "src/pages/outbound/M4OutboundDetailDialog.tsx"), "utf8");
const queries = readFileSync(resolve(root, "src/features/outbound/outbound-queries.ts"), "utf8");
const model = readFileSync(resolve(root, "src/pages/outbound/m4-outbound-page-model.ts"), "utf8");

assert.match(appShell, /id:\s*"m4-review"/, "管理端菜单应登记 m4-review 复核发货视图");
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

const productSummary = sliceBetween(parts, "export function ProductSummary", "export function ReviewSummary");
const reviewSummary = sliceBetween(parts, "export function ReviewSummary", "export function OrderNoSummary");
assert.match(parts, /销售出库/, "OrderNoSummary 类型文案应完整展示「销售出库」");
assert.match(productSummary, /件/, "ProductSummary 应主显件数");
assert.doesNotMatch(productSummary, /校验结果|批号|行\s*\//, "ProductSummary 不得再堆叠校验结果/批号/行数");
assert.match(parts, /function ValidationBadge\b/, "校验应拆为独立 StatusBadge 列组件");
assert.match(parts, /function BatchNoCell\b/, "批号应拆为独立列组件");
assert.match(parts, /function CustomerCell\b/, "客户列应优先可读客户名");
assert.match(reviewSummary, /短拣/, "ReviewSummary 应保留短拣短文案");
assert.doesNotMatch(reviewSummary, /复核模式|计划\s/, "ReviewSummary 不得堆「复核模式」或「计划 N 件 / 短拣」挤乱文案");
assert.match(page, /minWidth:\s*2[24]0/, "采购退货单号或单号列应有足够 minWidth");
assert.match(page, /BatchNoCell|ValidationBadge|CustomerCell/, "M4 列表应使用拆列/降噪单元格组件");

// --- 按状态裁剪私有动作 ---
assert.match(page, /function canValidateOrder\b/, "orders 校验应按状态裁剪");
assert.match(page, /function canVoidOrder\b/, "orders 作废应按状态裁剪");
assert.match(page, /function canReleaseWave\b/, "waves 下发应按状态裁剪");
assert.match(page, /function canCancelWave\b/, "waves 取消应按状态裁剪");
assert.match(page, /function canReviewOrder\b/, "review 复核应按状态裁剪");
assert.match(page, /function canShipOrder\b/, "review 交接应按状态裁剪");
assert.match(page, /function canApproveReturn\b/, "returns 审批应按状态裁剪");
assert.match(page, /function canRejectReturn\b/, "returns 驳回应按状态裁剪");
assert.match(page, /function canPickReturn\b/, "returns 拣货应按状态裁剪");
assert.match(page, /function canReviewReturn\b/, "returns 复核应按状态裁剪");
assert.match(page, /function canShipReturn\b/, "returns 出库应按状态裁剪");

assert.match(page, /pending_validation.*validation_exception.*confirmed|status === "pending_validation".*status === "validation_exception".*status === "confirmed"/s, "校验应允许 pending_validation/validation_exception/confirmed");
assert.match(canValidateOrderBody(), /pending_validation/, "canValidateOrder 应包含 pending_validation");
assert.match(canValidateOrderBody(), /validation_exception/, "canValidateOrder 应包含 validation_exception");
assert.match(canValidateOrderBody(), /confirmed/, "canValidateOrder 应包含 confirmed");
assert.doesNotMatch(canValidateOrderBody(), /shipped/, "canValidateOrder 不得把 shipped 当可校验");
assert.doesNotMatch(canVoidOrderBody(), /shipped|reviewed/, "canVoidOrder 不得允许 reviewed/shipped");
assert.match(canReleaseWaveBody(), /draft/, "下发仅 draft");
assert.doesNotMatch(canReleaseWaveBody(), /released/, "已下发波次不得再下发");
assert.match(canCancelWaveBody(), /draft/, "取消应允许 draft");
assert.match(canCancelWaveBody(), /released/, "取消应允许 released");
assert.doesNotMatch(canCancelWaveBody(), /cancelled/, "已取消波次不得再取消");
assert.match(canReviewOrderBody(), /picked/, "复核应允许 picked");
assert.match(canReviewOrderBody(), /picked_short/, "复核应允许 picked_short");
assert.doesNotMatch(canReviewOrderBody(), /inventory_locked|reviewed|shipped/, "未拣选完成、已复核或已发货订单不得复核");
assert.match(canShipOrderBody(), /reviewed/, "交接应允许 reviewed");
assert.doesNotMatch(canShipOrderBody(), /inventory_locked|shipped/, "未复核/已发货不得交接");
assert.match(
  model,
  /mode === "review"[\s\S]*new Set\(\[[^\]]*"picked"[^\]]*"picked_short"[^\]]*"reviewed"[^\]]*"reviewed_short"[^\]]*"shipped"[^\]]*\]\)/,
  "复核发货列表必须保留已复核与已发货订单，才能闭环真实发货交接并展示结果",
);
assert.doesNotMatch(
  pageFile,
  /mode !== "review" \|\| order\.status === "picked"/,
  "M4 页面不得在统一筛选后再次把复核发货列表裁成仅已拣选订单",
);
assert.match(queries, /useShipOutboundOrderMutation/, "发货交接必须有真实 POST mutation");
assert.match(queries, /\/api\/v1\/outbound\/orders\/\{id\}\/ship/, "发货 mutation 必须调用正式 ship API");
assert.match(pageFile, /shipOutboundOrderMutation\.mutateAsync/, "发货交接不能只改页面内存状态");
assert.doesNotMatch(pageFile, /action\.kind === "ship"\)\s*updateOrder/, "发货交接不得用本地 updateOrder 冒充成功");
assert.match(actionDialog, /客户药检副本.*不.*发货|发货.*不.*客户药检副本/, "发货弹窗应明确客户药检副本不参与发货阻断");
assert.doesNotMatch(pageFile, /配送 第三方快递|包裹数量 1|车牌号 沪A-12345/, "复核发货列表不得展示未由 API 返回的静态交接数据");
assert.match(canApproveReturnBody(), /pending_approval/, "审批仅待审批");
assert.match(canRejectReturnBody(), /pending_approval/, "驳回仅待审批");
assert.match(canPickReturnBody(), /approved/, "拣货仅已审批");
assert.doesNotMatch(canPickReturnBody(), /pending_approval/, "待审批禁止拣货");
assert.match(canReviewReturnBody(), /picking/, "退货复核仅拣货中");
assert.doesNotMatch(canReviewReturnBody(), /pending_approval/, "待审批禁止复核");
assert.match(canShipReturnBody(), /reviewed/, "退货出库仅已复核");
assert.doesNotMatch(canShipReturnBody(), /pending_approval/, "待审批禁止出库/交接");

// --- 采购退货审批可驳回 ---
assert.match(page, /"reject-return"/, "应支持 reject-return 动作 kind");
assert.match(page, /驳回/, "toolbar 应有驳回按钮文案");
assert.match(page, /驳回备注必填|noteRequired|reject-return.*trim|!note\.trim\(\)/, "驳回备注应做必填校验");
assert.match(page, /可选填写审批意见|备注可选/, "审批通过备注应可选");

// --- ActionDialog 业务上下文 ---
assert.match(page, /action-target-context|resolveActionTarget|ActionTargetContext/, "非 create 动作应展示目标业务上下文");
assert.match(page, /目标\$\{target\.kindLabel\}|目标.*docNo|当前状态/, "DialogDescription 或只读区应含单号/状态摘要");
assert.match(page, /titleWithDocNo|meta\.title.*docNo|\$\{meta\.title\} · \$\{target\.docNo\}/, "发货/校验弹窗标题区应可见单号");

// --- 详情去原型说明字段 ---
assert.doesNotMatch(detail, /作废审批入口/, "OrderDetail 不得展示原型说明字段「作废审批入口」");
assert.doesNotMatch(detail, /未进波次订单可申请/, "OrderDetail 不得展示「未进波次订单可申请」说明文案");
assert.doesNotMatch(detail, /沪A-12345|已签字|\["配送方"|\["包裹数量"|\["车牌号"|\["装车温度"|\["签字"/, "订单详情不得展示未由 API 返回的静态交接数据");

function canValidateOrderBody() {
  return functionBody(page, "canValidateOrder");
}
function canVoidOrderBody() {
  return functionBody(page, "canVoidOrder");
}
function canReleaseWaveBody() {
  return functionBody(page, "canReleaseWave");
}
function canCancelWaveBody() {
  return functionBody(page, "canCancelWave");
}
function canReviewOrderBody() {
  return functionBody(page, "canReviewOrder");
}
function canShipOrderBody() {
  return functionBody(page, "canShipOrder");
}
function canApproveReturnBody() {
  return functionBody(page, "canApproveReturn");
}
function canRejectReturnBody() {
  return functionBody(page, "canRejectReturn");
}
function canPickReturnBody() {
  return functionBody(page, "canPickReturn");
}
function canReviewReturnBody() {
  return functionBody(page, "canReviewReturn");
}
function canShipReturnBody() {
  return functionBody(page, "canShipReturn");
}

function functionBody(source, name) {
  const match = source.match(new RegExp(`function ${name}\\b[^{]*\\{([\\s\\S]*?)\\n\\}`));
  assert.ok(match, `应找到函数 ${name}`);
  return match[1];
}

function sliceBetween(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  assert.notEqual(start, -1, `应找到片段起点 ${startMarker}`);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.notEqual(end, -1, `应找到片段终点 ${endMarker}`);
  return source.slice(start, end);
}

console.log("m4 outbound DataGrid actions self-check passed");
