import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const page = readFileSync(new URL("../src/pages/auth/H1SessionPage.tsx", import.meta.url), "utf8");
const queries = readFileSync(new URL("../src/features/auth/auth-queries.ts", import.meta.url), "utf8");
const apiSchema = readFileSync(new URL("../../../packages/api-client/src/schema.ts", import.meta.url), "utf8");
const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const renderer = readFileSync(new URL("../src/app-shell/AdminViewRenderer.tsx", import.meta.url), "utf8");
const menu = readFileSync(new URL("../dev-mocks/admin-menu-dev-mock.ts", import.meta.url), "utf8");
const devMock = readFileSync(new URL("../dev-mocks/web-admin-dev-mock-core.ts", import.meta.url), "utf8");
const queryConfig = JSON.parse(readFileSync(new URL("../src/pages/page-query-core-fields.json", import.meta.url), "utf8"));

assert.match(app, /"h1-session-management"/, "H1 会话页必须注册到管理端菜单和视图");
assert.match(renderer, /<H1SessionPage currentUser=\{currentUser\}/, "H1 会话页必须由 renderAdminView 渲染");
assert.match(menu, /h1-session-management/, "H1 会话页必须进入菜单 dev mock");
assert.match(page, /<QueryPanel/, "H1 会话页必须使用 QueryPanel");
assert.match(page, /<DataGrid/, "H1 会话页必须使用 DataGrid");
assert.match(page, /<Dialog/, "失效动作必须有确认对话框");
assert.match(page, /失效设备|踢出目标用户|登出其他设备/, "H1 会话页必须提供会话失效动作");
for (const route of [
  "/api/v1/auth/logout",
  "/api/v1/auth/me/password",
  "/api/v1/auth/sessions",
  "/api/v1/auth/sessions/{session_id}/revoke",
  "/api/v1/auth/sessions/revoke-others",
  "/api/v1/auth/users/{user_id}/kick",
  "/api/v1/auth/users/{user_id}/status",
]) {
  assert.ok(queries.includes(route) || apiSchema.includes(route), `auth api-client 缺少 ${route}`);
}
assert.match(devMock, /\/api\/v1\/auth\/logout/);
assert.match(devMock, /\/api\/v1\/auth\/sessions/);
assert.match(devMock, /kickUserMatch/);
const config = queryConfig.pages.find((item) => item.id === "h1-session-management");
assert.equal(config?.required, true, "H1 会话页查询配置必须 required=true");
assert.deepEqual(config?.core, ["targetUserId"], "H1 会话页核心查询条件必须是 targetUserId");

console.log("H1 session page self-check passed");
