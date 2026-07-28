import { readFileSync } from "node:fs";

function source(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
}

const page = source("src/pages/print-orchestration/H9PrintDevicePage.tsx");
const dialogs = source("src/pages/print-orchestration/H9PrintDeviceDialogs.tsx");
const feature = source("src/features/print-orchestration/print-device-queries.ts");
const app = source("src/App.tsx");
const renderer = source("src/app-shell/AdminViewRenderer.tsx");
const view = source("src/app-shell/admin-view.ts");
const menu = source("dev-mocks/admin-menu-dev-mock.ts");
const queryConfig = JSON.parse(source("src/pages/page-query-core-fields.json"));

for (const token of ["QueryPanel", "DataGrid", "Dialog", "设备·Print Agent 管理"]) {
  if (!page.includes(token)) throw new Error(`H9 打印设备页面缺少 ${token}`);
}

// US-H9-011：站点/映射/打印机/纸盒/测试打印/租约端点全部接入
for (const path of [
  "/api/v1/print-devices/sites",
  "/api/v1/print-devices/sites/{site_id}/owner-mappings",
  "/api/v1/print-devices/sites/{site_id}/owner-mappings/{mapping_id}/disable",
  "/api/v1/print-devices/printers",
  "/api/v1/print-devices/printers/{printer_id}",
  "/api/v1/print-devices/printers/{printer_id}/trays",
  "/api/v1/print-devices/printers/{printer_id}/trays/{tray_id}",
  "/api/v1/print-devices/printers/{printer_id}/test-print",
  "/api/v1/print-devices/leases",
  "/api/v1/print-devices/leases/{lease_id}/release",
]) {
  if (!feature.includes(path)) throw new Error(`H9 打印设备 feature 缺少 ${path}`);
}

for (const text of [app, renderer, view, menu]) {
  if (!text.includes("h9-print-devices")) {
    throw new Error("H9 打印设备菜单、视图或渲染接线缺失");
  }
}

const config = queryConfig.pages.find((item) => item.id === "h9-print-devices");
if (!config?.required || !config.core.includes("keyword") || !config.core.includes("siteId")) {
  throw new Error("H9 打印设备页面查询分类未登记");
}

// 四个页签：站点 / 打印机 / 纸盒 / 租约
for (const token of ["站点（", "打印机（", "纸盒（", "租约（"]) {
  if (!page.includes(token)) throw new Error(`H9 打印设备页面缺少页签 ${token}`);
}

// AC3：测试打印弹窗（真实硬件回执缺口如实提示）
for (const token of ["TestPrintDialog", "测试打印", "下发测试打印"]) {
  if (!page.includes(token) && !dialogs.includes(token)) {
    throw new Error(`H9 测试打印能力缺少 ${token}`);
  }
}
if (!dialogs.includes("真实硬件回执")) {
  throw new Error("测试打印弹窗必须如实说明真实硬件回执缺口");
}

// AC6/AC7：释放模式覆盖与人工释放（原因 + 二次确认 + 专用权限 + 硬安全条件）
for (const token of ["ReleaseLeaseDialog", "PrinterReleaseModeDialog", "h9.device_lease.release"]) {
  if (!page.includes(token)) throw new Error(`H9 租约能力缺少 ${token}`);
}
for (const token of ["释放原因", "二次确认", "专用权限", "结果不明", "不得释放"]) {
  if (!dialogs.includes(token)) throw new Error(`H9 人工释放弹窗缺少约束文案 ${token}`);
}
if (!dialogs.includes('confirm: true')) {
  throw new Error("人工释放请求必须显式携带 confirm 二次确认字段");
}

// AC5：USB 单机语义与站点边界提示
for (const token of ["USB", "单机", "站点"]) {
  if (!page.includes(token) && !dialogs.includes(token)) {
    throw new Error(`H9 打印设备缺少语义文案 ${token}`);
  }
}

console.log("h9-print-device-slice-self-check: ok");
