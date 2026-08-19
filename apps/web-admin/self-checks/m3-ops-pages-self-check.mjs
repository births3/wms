import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const app = readFileSync(resolve(root, "src/App.tsx"), "utf8");
const view = readFileSync(resolve(root, "src/app-shell/admin-view.ts"), "utf8");
const renderer = readFileSync(resolve(root, "src/app-shell/AdminViewRenderer.tsx"), "utf8");
const countPage = readFileSync(resolve(root, "src/pages/inventory/M3InventoryCountPage.tsx"), "utf8");
const maintPage = readFileSync(resolve(root, "src/pages/inventory/M3MaintenancePage.tsx"), "utf8");
const relocPage = readFileSync(resolve(root, "src/pages/inventory/M3RelocationPage.tsx"), "utf8");
const pageQuery = readFileSync(resolve(root, "src/pages/page-query-core-fields.json"), "utf8");
const menu = readFileSync(resolve(root, "dev-mocks/admin-menu-dev-mock.ts"), "utf8");
const queries = readFileSync(resolve(root, "src/features/inventory/m3-ops-queries.ts"), "utf8");

for (const id of ["m3-counts", "m3-maintenance", "m3-relocations"]) {
  assert.match(app, new RegExp(`id:\\s*"${id}"`), `菜单应登记 ${id}`);
  assert.match(view, new RegExp(`"${id}"`), `AdminView 应包含 ${id}`);
  assert.match(menu, new RegExp(`"${id}"`), `dev menu 应包含 ${id}`);
  assert.match(pageQuery, new RegExp(`"id":\\s*"${id}"`), `查询配置应登记 ${id}`);
}
assert.match(renderer, /M3InventoryCountPage/, "应渲染盘点页");
assert.match(renderer, /M3MaintenancePage/, "应渲染养护页");
assert.match(renderer, /M3RelocationPage/, "应渲染移库页");
assert.match(countPage, /(?:<DataGrid\b|<ListPageTemplate\b)/, "盘点页 DataGrid 或 ListPageTemplate");
assert.match(maintPage, /生成计划/, "养护页生成计划");
assert.match(maintPage, /提交养护结果|提交结果/, "养护页应可提交结果");
assert.match(queries, /\/api\/v1\/inventory\/maintenance\/records/, "养护记录 API");
assert.match(relocPage, /发起移库/, "移库页发起移库");
assert.match(queries, /\/api\/v1\/inventory\/counts/, "盘点 API");
assert.match(queries, /\/lines\/\$\{input\.lineId\}\/submit|\/lines\/.*\/submit/, "盘点实盘提交 API");
assert.match(queries, /\/approve/, "盘点审批 API");
assert.match(countPage, /提交实盘/, "盘点页应有提交实盘");
assert.match(countPage, /审批差异/, "盘点页应有审批差异");
assert.match(queries, /\/api\/v1\/inventory\/maintenance\/tasks/, "养护 API");
assert.match(queries, /\/api\/v1\/inventory\/relocations/, "移库 API");
assert.doesNotMatch(queries, /\bfetch\s*\(/, "M3 查询必须统一使用生成的 api-client");
assert.match(queries, /api\.(GET|POST)\("\/api\/v1\/inventory\//, "M3 查询必须通过 api-client");
console.log("m3 ops pages self-check passed");
