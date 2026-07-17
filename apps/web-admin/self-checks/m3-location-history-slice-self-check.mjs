import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const appShell = readFileSync(resolve(root, "src/App.tsx"), "utf8");
const viewRenderer = readFileSync(resolve(root, "src/app-shell/AdminViewRenderer.tsx"), "utf8");
const adminView = readFileSync(resolve(root, "src/app-shell/admin-view.ts"), "utf8");
const page = readFileSync(resolve(root, "src/pages/inventory/M3LocationHistoryPage.tsx"), "utf8");
const batches = readFileSync(resolve(root, "src/pages/inventory/M3BatchManagementPage.tsx"), "utf8");
const queries = readFileSync(resolve(root, "src/features/inventory/inventory-queries.ts"), "utf8");
const pageQuery = readFileSync(resolve(root, "src/pages/page-query-core-fields.json"), "utf8");
const devMock = readFileSync(resolve(root, "dev-mocks/web-admin-dev-mock-print-inventory.ts"), "utf8");
const menuMock = readFileSync(resolve(root, "dev-mocks/admin-menu-dev-mock.ts"), "utf8");

assert.match(adminView, /"m3-location-history"/, "AdminView 应登记库位历史视图");
assert.match(appShell, /id:\s*"m3-location-history"/, "菜单应登记 M3 库位历史");
assert.match(viewRenderer, /<M3LocationHistoryPage\b/, "视图渲染器应渲染库位历史页");
assert.match(viewRenderer, /onOpenLocationHistory=\{\(\) => navigateTo\("m3-location-history"\)\}/, "批号页应能跳转库位历史");
assert.match(page, /export function M3LocationHistoryPage\b/, "应提供库位历史页面");
assert.match(page, /<DataGrid\b/, "库位历史应复用 DataGrid");
assert.match(page, /exportFileBaseName="M3-库位历史"/, "库位历史应支持 Excel 导出");
assert.match(page, /m3LocationHistoryQueryFields/, "库位历史应声明查询字段");
assert.match(page, /aria-label="库位风险识别"/, "库位历史应展示风险识别区");
assert.match(batches, /历史追踪/, "库位专项视图应提供历史追踪入口");
assert.match(batches, /rememberLocationHistoryCode/, "跳转前应记住库位编码");
assert.match(queries, /useLocationHistoryQuery/, "应提供库位历史 query hook");
assert.match(queries, /\/api\/v1\/inventory\/locations\/history/, "库位历史应请求真实 API 路径");
assert.match(pageQuery, /"id":\s*"m3-location-history"/, "页面查询配置应登记库位历史");
assert.match(devMock, /pathname === "\/api\/v1\/inventory\/locations\/history"/, "dev mock 应覆盖库位历史");
assert.match(menuMock, /"m3-location-history"/, "已发布菜单 mock 应包含库位历史");

console.log("m3 location history slice self-check passed");
