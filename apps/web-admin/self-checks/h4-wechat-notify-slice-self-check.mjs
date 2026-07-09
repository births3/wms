import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const pageSource = readFileSync(new URL("../src/pages/wechat-notify/H4WechatNotifyPage.tsx", import.meta.url), "utf8");
const apiSource = readFileSync(new URL("../src/features/wechat-notify/wechat-notify-queries.ts", import.meta.url), "utf8");
const viteConfig = readFileSync(new URL("../vite.config.ts", import.meta.url), "utf8");
const adminMenuDevMock = readFileSync(new URL("../dev-mocks/admin-menu-dev-mock.ts", import.meta.url), "utf8");
const devMock = readFileSync(new URL("../dev-mocks/wechat-notify-dev-mock.ts", import.meta.url), "utf8");
const queryConfig = JSON.parse(readFileSync(new URL("../src/pages/page-query-core-fields.json", import.meta.url), "utf8"));

const h4Pages = [
  { id: "h4-wechat-settings", title: "H4 参数设置", mode: "settings" },
  { id: "h4-notify-configs", title: "H4 通知配置", mode: "configs" },
  { id: "h4-notify-records", title: "H4 发送记录", mode: "records" },
];

for (const page of h4Pages) {
  assert.match(appShell, new RegExp(`\\{ id: "${page.id}", title: "${page.title}"`), `${page.id} 必须进入 menuSections`);
  assert.match(appShell, new RegExp(`menuItem\\("${page.id}"\\)`), `${page.id} 必须进入 defaultMenuTree`);
  assert.match(appShell, new RegExp(`if \\(view === "${page.id}"\\) return "${page.mode}"`), `${page.id} 必须映射到 H4 页面 mode`);
  assert.match(adminMenuDevMock, new RegExp(`\\["${page.id}", "${page.title}"`), `${page.id} 必须进入 admin-menu dev mock 已发布菜单`);
  assert.ok(queryConfig.pages.some((item) => item.id === page.id), `${page.id} 必须进入页面查询配置`);
}

assert.match(appShell, /<H4WechatNotifyPage mode=\{wechatNotifyMode\}/, "H4 菜单页必须由 renderAdminView 渲染");
assert.match(pageSource, /<QueryPanel[\s\S]*fields=\{h4NotificationConfigQueryFields\}/, "H4 通知配置必须使用公共 QueryPanel");
assert.match(pageSource, /<QueryPanel[\s\S]*fields=\{h4NotificationRecordQueryFields\}/, "H4 发送记录必须使用公共 QueryPanel");
assert.match(pageSource, /mode === "settings"/, "H4 参数设置必须有独立页面模式");
assert.match(pageSource, /<SettingsDialog[\s\S]*onSave=\{saveSettings\}/, "H4 参数设置必须有维护弹窗");
assert.match(pageSource, /<DataGrid[\s\S]*storageKey="h4\.wechat-notify\.settings"/, "H4 参数设置必须使用公共 DataGrid");
assert.match(pageSource, /<DataGrid[\s\S]*storageKey="h4\.wechat-notify\.configs"/, "H4 通知配置必须使用公共 DataGrid");
assert.match(pageSource, /<DataGrid[\s\S]*storageKey="h4\.wechat-notify\.records"/, "H4 发送记录必须使用公共 DataGrid");

for (const route of [
  "/api/v1/wechat-notify/configs",
  "/api/v1/wechat-notify/settings",
  "/api/v1/wechat-notify/send",
  "/api/v1/wechat-notify/records",
  "/api/v1/wechat-notify/records/{record_id}/resend",
]) {
  assert.ok(apiSource.includes(route), `H4 api-client 缺少 ${route}`);
}

assert.match(viteConfig, /pathname\.startsWith\("\/api\/v1\/wechat-notify"\)/, "vite dev mock 必须接入 H4 通知路由前缀");
assert.match(devMock, /pathname === "\/api\/v1\/wechat-notify\/configs"/, "dev mock 必须覆盖通知配置接口");
assert.match(devMock, /pathname === "\/api\/v1\/wechat-notify\/settings"/, "dev mock 必须覆盖企业微信参数设置接口");
assert.match(devMock, /pathname === "\/api\/v1\/wechat-notify\/records"/, "dev mock 必须覆盖发送记录接口");
assert.ok(devMock.includes("records\\/([^/]+)\\/resend"), "dev mock 必须覆盖发送记录重发接口");

console.log("h4 wechat notify slice self-check passed");
