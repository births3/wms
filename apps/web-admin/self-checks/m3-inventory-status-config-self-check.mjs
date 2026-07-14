import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const page = readFileSync(resolve(root, "src/pages/inventory/M3InventoryStatusConfigPage.tsx"), "utf8");
const queries = readFileSync(resolve(root, "src/features/inventory/inventory-status-config-queries.ts"), "utf8");
assert.match(page, /<DataGrid\b/, "M3 状态配置必须使用公共 DataGrid");
assert.match(page, /<QueryPanel\b/, "M3 状态配置必须使用公共 QueryPanel");
assert.match(page, /<Dialog\b/, "M3 状态配置写操作必须使用 Dialog");
assert.match(page, /owner_id: form\.scope === "global" \? null : currentUser\.owner_id/, "规则必须区分全局和当前货主");
assert.match(page, /approvalSources/, "页面必须维护原因/审批来源");
assert.match(queries, /api\.GET\("\/api\/v1\/inventory\/status-transitions"\)/, "读取必须使用真实状态转换 API");
assert.match(queries, /api\.PUT\("\/api\/v1\/inventory\/status-transitions\/\{from_status\}\/\{to_status\}"/, "写入必须使用真实状态转换 API");
assert.match(queries, /Idempotency-Key/, "写入必须携带幂等键");
console.log("m3 inventory status config self-check passed");
