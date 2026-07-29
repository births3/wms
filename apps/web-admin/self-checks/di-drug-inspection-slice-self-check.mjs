import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");
const app = read("src/App.tsx");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const menuMock = read("dev-mocks/admin-menu-dev-mock.ts");
const page = read("src/pages/drug-inspection/DrugInspectionPlatformPage.tsx");
const queries = read("src/features/drug-inspection/drug-inspection-queries.ts");
const pageId = "m-di-platforms";
for (const source of [app, renderer, menuMock]) {
  assert.ok(source.includes(pageId), "M-DI 药检平台必须完成菜单、路由和已发布菜单接线");
}
assert.match(page, /<QueryPanel/);
assert.match(page, /<DataGrid/);
assert.match(page, /<Dialog/);
assert.match(page, /validateDrugInspectionForm/);
assert.match(page, /api_key_configured/);
assert.doesNotMatch(page, /api_key_alias.*row|password_alias.*row/);
assert.match(queries, /api\.GET\("\/api\/v1\/drug-inspection\/platforms"/);
assert.match(queries, /api\.POST\("\/api\/v1\/drug-inspection\/platforms"/);
assert.match(queries, /api\.PATCH\("\/api\/v1\/drug-inspection\/platforms\/\{platform_id\}\/status"/);
console.log("M-DI drug inspection platform slice self-check passed");
