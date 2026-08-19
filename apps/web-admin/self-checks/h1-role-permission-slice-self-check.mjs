import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url);
const read = (relative) => readFileSync(new URL(relative, root), "utf8");

const page = read("src/pages/auth/H1RolePermissionPage.tsx");
const userDialog = read("src/pages/auth/CreateUserDialog.tsx");
const queries = read("src/features/auth/role-permission-queries.ts");
const app = read("src/App.tsx");
const menuPage = read("src/pages/admin-menu/H1AdminMenuPage.tsx");
const mockCore = read("dev-mocks/web-admin-dev-mock-core.ts");
const mockMenu = read("dev-mocks/admin-menu-dev-mock.ts");
const mockRole = read("dev-mocks/auth-role-permission-dev-mock.ts");

assert.match(page, /<QueryPanel/);
assert.match(page, /<DataGrid/);
assert.match(page, /权限矩阵/);
assert.match(page, /批量授权/);
assert.match(page, /新增用户/);
assert.match(page, /删除角色/);
assert.match(queries, /\/api\/v1\/auth\/roles/);
assert.match(queries, /\/api\/v1\/auth\/permissions/);
assert.match(queries, /\/api\/v1\/auth\/users/);
assert.match(queries, /user-roles\/batch/);
assert.match(queries, /POST\("\/api\/v1\/auth\/users"/);
assert.match(userDialog, /至少一个角色/);
assert.match(app, /h1-role-permission/);
assert.match(menuPage, /"h1-role-permission"/);
assert.match(mockCore, /auth\/roles/);
assert.match(mockMenu, /h1-role-permission/);
assert.match(mockRole, /Array\.from\(\{ length: 8 \}/);
assert.match(mockRole, /auth\/permissions/);
assert.match(mockRole, /auth\/users/);
assert.match(mockRole, /user-roles\/batch/);

console.log("H1 role permission slice self-check passed");
