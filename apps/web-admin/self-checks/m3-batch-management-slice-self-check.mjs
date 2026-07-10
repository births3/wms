import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const appShell = readFileSync(resolve(root, "src/App.tsx"), "utf8");
const page = readFileSync(resolve(root, "src/pages/inventory/M3BatchManagementPage.tsx"), "utf8");
const queries = readFileSync(resolve(root, "src/features/inventory/inventory-queries.ts"), "utf8");
const devMock = readFileSync(
  resolve(root, "dev-mocks/web-admin-dev-mock-print-inventory.ts"),
  "utf8",
);

assert.match(appShell, /id:\s*"m3-batches"/, "库内业务菜单应登记 M3 批号管理视图");
assert.match(appShell, /<M3BatchManagementPage\b/, "App 应渲染 M3 批号管理页面");
assert.match(page, /<DataGrid\b/, "M3 批号管理页应复用 DataGrid");
assert.match(queries, /api\.GET\("\/api\/v1\/inventory\/batches"\)/, "批号列表应通过现有 inventory batches API 读取");
assert.match(devMock, /pathname === "\/api\/v1\/inventory\/batches"/, "开发 mock 应覆盖 M3 批号列表路由");
assert.match(devMock, /function inventoryBatches\(\)/, "开发 mock 应提供 M3 批号列表数据");

console.log("m3 batch management slice self-check passed");
