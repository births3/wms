import { readFileSync } from "node:fs";

function source(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
}

const page = source("src/pages/print-orchestration/H9DeliveryNoteAggregationPage.tsx");
const feature = source("src/features/print-orchestration/print-orchestration-queries.ts");
const app = source("src/App.tsx");
const renderer = source("src/app-shell/AdminViewRenderer.tsx");
const view = source("src/app-shell/admin-view.ts");
const menu = source("dev-mocks/admin-menu-dev-mock.ts");
const queryConfig = JSON.parse(source("src/pages/page-query-core-fields.json"));

for (const token of ["QueryPanel", "DataGrid", "Dialog", "作业·随货同行单归集"]) {
  if (!page.includes(token)) throw new Error(`H9 归集页面缺少 ${token}`);
}
for (const path of [
  "/api/v1/print-orchestration/delivery-note-candidates",
  "/api/v1/print-orchestration/delivery-note-groups",
  "/api/v1/print-orchestration/route-bindings",
  "/api/v1/print-orchestration/cutoff-plans",
]) {
  if (!feature.includes(path)) throw new Error(`H9 归集 feature 缺少 ${path}`);
}
for (const text of [app, renderer, view, menu]) {
  if (!text.includes("h9-delivery-note-aggregation")) {
    throw new Error("H9 归集菜单、视图或渲染接线缺失");
  }
}
const config = queryConfig.pages.find((item) => item.id === "h9-delivery-note-aggregation");
if (!config?.required || !config.core.includes("warehouseId")) {
  throw new Error("H9 归集页面查询分类未登记");
}

// US-H9-007：归集维度规则配置
const dialogs = source("src/pages/print-orchestration/H9DeliveryNoteAggregationDialogs.tsx");
for (const path of [
  "/api/v1/print-orchestration/aggregation-fields",
  "/api/v1/print-orchestration/aggregation-rules/versions",
  "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/test",
  "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/publish",
  "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/disable",
]) {
  if (!feature.includes(path)) throw new Error(`H9 归集规则 feature 缺少 ${path}`);
}
for (const token of ["归集规则", "AggregationRuleDialog", "AggregationRuleTestDialog", "新建规则版本", "测试规则", "发布规则", "停用规则"]) {
  if (!page.includes(token)) throw new Error(`H9 归集规则页面缺少 ${token}`);
}
for (const token of ["等值", "样本订单测试", "不可覆盖", "硬边界"]) {
  if (!dialogs.includes(token) && !page.includes(token)) {
    throw new Error(`H9 归集规则缺少受控约束文案 ${token}`);
  }
}
// AC2：不得提供自由 SQL / 脚本 / 正则输入口
for (const forbidden of ["textarea", "SELECT ", "regex", "RegExp("]) {
  if (dialogs.includes(forbidden)) {
    throw new Error(`H9 归集规则弹窗不得包含自由表达式输入痕迹：${forbidden}`);
  }
}

// US-H9-008：打印组套配置与就绪策略
const suitePanel = source("src/pages/print-orchestration/H9PrintSuitePanel.tsx");
for (const path of [
  "/api/v1/print-orchestration/print-document-categories",
  "/api/v1/print-orchestration/print-suites/versions",
  "/api/v1/print-orchestration/print-suites/versions/{version_id}/test",
  "/api/v1/print-orchestration/print-suites/versions/{version_id}/publish",
  "/api/v1/print-orchestration/print-suites/versions/{version_id}/disable",
  "/api/v1/print-orchestration/suite-instances",
]) {
  if (!feature.includes(path)) throw new Error(`H9 打印组套 feature 缺少 ${path}`);
}
for (const token of ["打印组套", "H9PrintSuitePanel"]) {
  if (!page.includes(token)) throw new Error(`H9 归集页面缺少打印组套页签接线 ${token}`);
}
for (const token of [
  "新建组套版本",
  "测试组套",
  "发布组套",
  "停用组套",
  "送货地址",
  "仓库默认",
  "wait_hold_instance",
  "pause_agent_queue",
  "h-file:",
  "usePrintTemplatesQuery",
]) {
  if (!suitePanel.includes(token)) throw new Error(`H9 打印组套面板缺少 ${token}`);
}
// AC3/AC7：必需项不可跳过；rendered 绑定模板版本、external_file 绑定稳定文件引用
for (const token of [
  'item.required && item.failurePolicy !== "pause_suite"',
  "latestVersionStatus === \"published\"",
  'externalFileRef.trim().startsWith("h-file:")',
]) {
  if (!suitePanel.includes(token)) {
    throw new Error(`H9 打印组套受控约束缺失：${token}`);
  }
}

// US-H9-009：分类 PDF、H-FILE 留存、选择下载与应急打印
const categoryPdfPanel = source("src/pages/print-orchestration/H9CategoryPdfPanel.tsx");
for (const path of [
  "/api/v1/print-orchestration/suite-instances/{instance_id}/category-pdfs",
  "/api/v1/print-orchestration/suite-instances/{instance_id}/category-pdfs/prepare",
  "/api/v1/print-orchestration/suite-instances/{instance_id}/category-pdfs/download",
  "/api/v1/print-orchestration/suite-instances/{instance_id}/category-pdfs/emergency-print",
]) {
  if (!feature.includes(path)) throw new Error(`H9 分类 PDF feature 缺少 ${path}`);
}
for (const token of [
  "H9CategoryPdfPanel",
  "生成分类 PDF",
  "下载所选分类",
  "应急打印所选",
  "源数据版本 / 权威文件",
  "模板版本 ID",
  "内容 SHA-256",
  "GSP 五年归档",
  "短期缓存",
]) {
  if (!categoryPdfPanel.includes(token) && !suitePanel.includes(token)) {
    throw new Error(`H9 分类 PDF 面板缺少 ${token}`);
  }
}
for (const permission of [
  "h9.print_pdf.read",
  "h9.print_pdf.prepare",
  "h9.print_pdf.download",
  "h9.print_pdf.emergency_print",
]) {
  if (!page.includes(permission)) throw new Error(`H9 分类 PDF 页面缺少权限 ${permission}`);
}

console.log("h9-delivery-note-aggregation-self-check: ok");
