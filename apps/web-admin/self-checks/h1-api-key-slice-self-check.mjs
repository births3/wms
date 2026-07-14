import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url);
const read = (relative) => readFileSync(new URL(relative, root), "utf8");
const page = read("src/pages/api-key/H1ApiKeyPage.tsx");
const queries = read("src/features/api-key/api-key-queries.ts");
const schema = read("../../packages/api-client/src/schema.ts");
const app = read("src/App.tsx");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const menu = read("dev-mocks/admin-menu-dev-mock.ts");
const config = JSON.parse(read("src/pages/page-query-core-fields.json"));

assert.match(app, /h1-api-keys/);
assert.match(renderer, /<H1ApiKeyPage currentUser=\{currentUser\}/);
assert.match(menu, /h1-api-keys/);
assert.match(page, /<QueryPanel/);
assert.match(page, /<DataGrid/);
assert.match(page, /<Dialog/);
assert.match(page, /创建 Key/);
assert.match(page, /轮换/);
assert.match(page, /吊销/);
for (const route of [
  "/api/v1/auth/api-keys",
  "/api/v1/auth/api-keys/{api_key_id}/rotate",
  "/api/v1/auth/api-keys/{api_key_id}/revoke",
]) {
  assert.ok(queries.includes(route) || schema.includes(route), `api-client 缺少 ${route}`);
}
const pageConfig = config.pages.find((item) => item.id === "h1-api-keys");
assert.equal(pageConfig?.required, true);
assert.deepEqual(pageConfig?.core, ["keyword", "status"]);

console.log("H1 API key slice self-check passed");
