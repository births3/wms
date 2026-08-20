import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const appShell = readFileSync(resolve(root, "src/App.tsx"), "utf8");
const viewRenderer = readFileSync(resolve(root, "src/app-shell/AdminViewRenderer.tsx"), "utf8");
const adminView = readFileSync(resolve(root, "src/app-shell/admin-view.ts"), "utf8");
const page = readFileSync(resolve(root, "src/pages/master/M1DevicePage.tsx"), "utf8");
const queries = readFileSync(resolve(root, "src/features/device/device-queries.ts"), "utf8");
const devMock = readFileSync(resolve(root, "dev-mocks/admin-menu-dev-mock.ts"), "utf8");
const pageQuery = readFileSync(resolve(root, "src/pages/page-query-core-fields.json"), "utf8");

assert.match(adminView, /"m1-devices"/, "AdminView 应包含 m1-devices");
assert.match(appShell, /id:\s*"m1-devices"/, "菜单应登记 m1-devices");
assert.match(viewRenderer, /m1-devices/, "视图渲染器应覆盖 m1-devices");
assert.match(viewRenderer, /M1DevicePage/, "视图渲染器应挂载 M1DevicePage");
assert.match(devMock, /m1-devices/, "dev mock 菜单应包含 m1-devices");
assert.match(pageQuery, /"id":\s*"m1-devices"/, "查询面板配置应登记 m1-devices");
assert.match(page, /<Dialog\b/, "注册/绑定必须使用 Dialog");
assert.match(page, /设备编码/, "固定列应含设备编码");
assert.match(page, /设备类型/, "固定列应含设备类型");
assert.match(page, /在线状态/, "固定列应含在线状态");
assert.match(page, /注册设备/, "私有动作应含注册设备");
assert.match(page, /库位绑定/, "私有动作应含库位绑定");
assert.match(page, /解绑/, "私有动作应含解绑");
assert.match(queries, /api\.POST\("\/api\/v1\/location-device-bindings\/\{id\}\/unbind"/, "解绑必须使用解绑 API");
assert.match(page, /启停|停用/, "私有动作应含启停");
assert.doesNotMatch(queries, /\bfetch\s*\(/, "设备查询必须统一使用生成的 api-client");
assert.match(queries, /api\.GET\("\/api\/v1\/iot-devices"/, "列表必须使用设备列表 API");
assert.match(queries, /api\.POST\("\/api\/v1\/iot-devices"/, "注册必须使用设备注册 API");
assert.match(queries, /api\.POST\("\/api\/v1\/location-device-bindings"/, "绑定必须使用绑定 API");
assert.match(queries, /Idempotency-Key/, "写入必须携带幂等键");

console.log("m1 device self-check passed");
