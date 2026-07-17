import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const appShell = readFileSync(resolve(root, "src/App.tsx"), "utf8");
const viewRenderer = readFileSync(resolve(root, "src/app-shell/AdminViewRenderer.tsx"), "utf8");
const page = readFileSync(resolve(root, "src/pages/inventory/M3BatchManagementPage.tsx"), "utf8");
const columns = readFileSync(resolve(root, "src/pages/inventory/M3BatchColumns.tsx"), "utf8");
const viewHelpers = readFileSync(resolve(root, "src/pages/inventory/M3BatchViewHelpers.tsx"), "utf8");
const detailDialog = readFileSync(resolve(root, "src/pages/inventory/M3BatchDetailDialog.tsx"), "utf8");
const queries = readFileSync(resolve(root, "src/features/inventory/inventory-queries.ts"), "utf8");
const devMockCore = readFileSync(resolve(root, "dev-mocks/web-admin-dev-mock-core.ts"), "utf8");
const devMockModel = readFileSync(resolve(root, "dev-mocks/web-admin-dev-mock-model.ts"), "utf8");
const devMock = readFileSync(
  resolve(root, "dev-mocks/web-admin-dev-mock-print-inventory.ts"),
  "utf8",
);
const cancelDialog = readFileSync(
  resolve(root, "src/pages/inventory/M3BatchRecallCancelDialog.tsx"),
  "utf8",
);

assert.match(appShell, /id:\s*"m3-batches"/, "库内业务菜单应登记 M3 批号管理视图");
assert.match(viewRenderer, /<M3BatchManagementPage\b/, "视图渲染器应渲染 M3 批号管理页面");
assert.match(page, /<DataGrid\b/, "M3 批号管理页应复用 DataGrid");
assert.match(page, /exportFileBaseName="M3 批号管理"/, "M3 库存查询应使用可识别的 Excel 导出文件名");
assert.doesNotMatch(page, /showExportAction=\{false\}/, "M3 库存查询不得关闭 DataGrid 标准 Excel 导出");
assert.match(page, /detailAction=\{gridDetailAction\}/, "M3 批号管理应提供标准详情动作");
assert.match(page, /toolbarActions=\{\[statusAction, recallAction, cancelRecallAction\]\}/, "M3 批号管理应提供状态、召回和撤回动作");
assert.match(page, /M3BatchRecallDialog/, "M3 召回必须使用独立弹窗");
assert.match(page, /M3BatchRecallCancelDialog/, "M3 取消召回必须使用独立弹窗");
assert.match(page, /变更库存状态|确认变更/, "M3 状态变更必须使用弹窗确认");
assert.match(columns, /onDoubleClick:\s*\(row\)\s*=>\s*onOpenDetail\(row\.id\)/, "M3 批号列应支持双击打开详情");
assert.match(page, /<M3BatchDetailDialog\b/, "M3 页面应挂载批号详情 Dialog");
assert.match(detailDialog, /export function M3BatchDetailDialog\b/, "M3 应提供批号详情 Dialog");
assert.match(detailDialog, /useInventoryBatchTraceQuery/, "M3 详情应读取批次追溯");
assert.match(detailDialog, /title="流转追踪"/, "M3 详情应展示流转追踪分区");
assert.match(page, /useSystemDictionaryItemOptionsQuery\(\s*"inventory_quality_status"\s*\)/, "M3 应查询库存质量状态系统字典");
assert.match(page, /options:\s*qualityStatusOptions/, "M3 质量状态筛选应消费字典选项");
assert.match(page, /statusOptionsFor\([^,]+,\s*qualityStatusOptions\)/, "M3 状态目标应消费字典选项");
assert.match(page, /qualityStatusQuery\.isPending/, "M3 应明确处理质量状态字典加载中");
assert.match(page, /qualityStatusQuery\.error/, "M3 应明确处理质量状态字典加载失败");
assert.match(page, /qualityStatusOptions\.length === 0/, "M3 应明确处理无启用质量状态字典项");
assert.doesNotMatch(page, /const qualityStatusOptions\s*=\s*\[/, "M3 不得把硬编码质量状态选项作为真实来源");
assert.doesNotMatch(viewHelpers, /qualityStatusLabels\s*:/, "M3 状态展示不得依赖本地硬编码标签表");
assert.match(detailDialog, /qualityStatusOptions/, "M3 详情状态展示应消费字典选项");
assert.match(columns, /ExpiryDateCell/, "M3 有效期列应有近效期/过期视觉区分");
assert.match(page, /key:\s*"expiryRisk"/, "M3 应支持按效期风险筛选");
assert.match(page, /expiryRisks\.has\(expiryTone\(batch\.expiry_date, warningDays\)\)/, "效期风险筛选应复用配置阈值判定");
assert.match(page, /useInventoryExpiryPolicyQuery/, "M3 应读取近效期配置中心阈值");
assert.match(page, /DEFAULT_NEAR_EXPIRY_DAYS = 180/, "M3 应保留六个月默认近效期阈值");
assert.match(page, /emptyTitle="暂无库存批次"/, "M3 应保持 emptyTitle");
assert.match(page, /历史追踪/, "库位专项视图应提供跳转到库位历史的入口");
assert.match(page, /rememberLocationHistoryCode/, "跳转库位历史前应写入库位编码");
assert.match(queries, /api\.GET\("\/api\/v1\/inventory\/batches",\s*\{\s*params:\s*\{\s*query\s*\}/, "批号列表应把服务端查询条件传给 inventory batches API");
assert.match(queries, /queryKey:\s*\[\.\.\.inventoryBatchesQueryKey,\s*query\]/, "库存查询条件应进入 Query 缓存键，避免复用旧结果");
assert.match(queries, /useChangeInventoryStatusMutation/, "M3 状态变更必须复用 API mutation");
assert.match(queries, /\/api\/v1\/inventory\/batches\/status/, "M3 状态变更必须接入真实 API");
assert.match(queries, /useMarkInventoryRecallMutation/, "M3 召回必须复用 API mutation");
assert.match(queries, /\/api\/v1\/inventory\/batches\/recall/, "M3 召回必须接入真实 API");
assert.match(queries, /useCancelInventoryRecallMutation/, "M3 取消召回必须复用 API mutation");
assert.match(queries, /\/api\/v1\/inventory\/batches\/recall\/cancel/, "M3 取消召回必须接入真实 API");
assert.match(queries, /inventory_policy/, "M3 应通过系统字典读取近效期配置");
assert.match(queries, /\/api\/v1\/inventory\/batches\/\{id\}\/trace/, "M3 详情必须接入批次追溯 API");
assert.match(devMock, /pathname === "\/api\/v1\/inventory\/batches"/, "开发 mock 应覆盖 M3 批号列表路由");
assert.match(devMockCore, /system-dictionaries[\s\S]*items/, "开发 mock 应保留系统字典 items 路由");
assert.match(devMockModel, /inventory_quality_status:\s*\[/, "开发 mock 应提供库存质量状态字典");
for (const code of ["qualified", "quarantined", "unqualified", "pending_destruction", "loss_deducted"]) {
  assert.match(devMockModel, new RegExp(`"inventory_quality_status"\\s*,\\s*"${code}"`), `开发 mock 应提供 ${code} 默认状态`);
}
assert.match(devMock, /const trace = pathname\.match/, "开发 mock 应覆盖 M3 批次追溯路由");
assert.match(devMock, /pathname === "\/api\/v1\/inventory\/batches\/recall"/, "开发 mock 应覆盖 M3 召回路由");
assert.match(devMock, /pathname === "\/api\/v1\/inventory\/batches\/recall\/cancel"/, "开发 mock 应覆盖 M3 取消召回路由");
assert.match(devMock, /function inventoryBatches\(\)/, "开发 mock 应提供 M3 批号列表数据");
assert.match(cancelDialog, /second_approver_id/, "取消召回弹窗应收集第二审批人");

console.log("m3 batch management slice self-check passed");
