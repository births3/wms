import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const appShell = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const viewRenderer = readFileSync(new URL("../src/app-shell/AdminViewRenderer.tsx", import.meta.url), "utf8");
const pageSource = readFileSync(new URL("../src/pages/wechat-notify/H4WechatNotifyPage.tsx", import.meta.url), "utf8");
const dialogsSource = readFileSync(new URL("../src/pages/wechat-notify/H4WechatNotifyDialogs.tsx", import.meta.url), "utf8");
const apiSource = readFileSync(new URL("../src/features/wechat-notify/wechat-notify-queries.ts", import.meta.url), "utf8");
const devMockCore = readFileSync(new URL("../dev-mocks/web-admin-dev-mock-core.ts", import.meta.url), "utf8");
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
  assert.match(viewRenderer, new RegExp(`if \\(view === "${page.id}"\\) return "${page.mode}"`), `${page.id} 必须映射到 H4 页面 mode`);
  assert.match(adminMenuDevMock, new RegExp(`\\["${page.id}", "${page.title}"`), `${page.id} 必须进入 admin-menu dev mock 已发布菜单`);
  assert.ok(queryConfig.pages.some((item) => item.id === page.id), `${page.id} 必须进入页面查询配置`);
}

assert.match(viewRenderer, /<H4WechatNotifyPage mode=\{wechatNotifyMode\}/, "H4 菜单页必须由 renderAdminView 渲染");
assert.match(pageSource, /<QueryPanel[\s\S]*fields=\{h4WechatSettingsQueryFields\}/, "H4 参数设置必须使用公共 QueryPanel");
assert.match(pageSource, /h4WechatSettingsCoreQueryFieldKeys/, "H4 参数设置必须登记核心查询字段常量");
assert.match(pageSource, /filterSettings/, "H4 参数设置必须本地过滤 settingsRows");
assert.match(pageSource, /<QueryPanel[\s\S]*fields=\{h4NotificationConfigQueryFields\}/, "H4 通知配置必须使用公共 QueryPanel");
assert.match(pageSource, /<QueryPanel[\s\S]*fields=\{h4NotificationRecordQueryFields\}/, "H4 发送记录必须使用公共 QueryPanel");
assert.match(pageSource, /mode === "settings"/, "H4 参数设置必须有独立页面模式");
const settingsPageConfig = queryConfig.pages.find((item) => item.id === "h4-wechat-settings");
assert.equal(settingsPageConfig?.required, true, "h4-wechat-settings 查询分类必须 required=true");
assert.equal(settingsPageConfig?.fieldConstant, "h4WechatSettingsQueryFields", "h4-wechat-settings 必须登记 fieldConstant");
assert.deepEqual(settingsPageConfig?.core, ["keyword", "enabled"], "h4-wechat-settings 核心字段必须为 keyword/enabled");
assert.match(pageSource, /<SettingsDialog[\s\S]*onSave=\{saveSettings\}[\s\S]*onTest=\{testSettings\}/, "H4 参数测试必须由维护弹窗触发");
assert.match(pageSource, /<SettingsDialog[\s\S]*notice=\{notice\}/, "H4 参数弹窗必须接收当前错误通知");
assert.match(dialogsSource, /const busy = saving \|\| testing;[\s\S]*<fieldset disabled=\{busy\}/, "H4 参数保存或测试期间必须锁定表单输入");
assert.match(dialogsSource, /notice\?\.type === "error"[\s\S]*<NoticePanel notice=\{notice\}/, "H4 参数保存错误必须显示在当前弹窗内");
assert.match(dialogsSource, /role=\{notice\.type === "error" \? "alert" : "status"\}/, "H4 错误通知必须使用 alert 语义");
assert.match(pageSource, /<DataGrid[\s\S]*storageKey="h4\.wechat-notify\.settings"/, "H4 参数设置必须使用公共 DataGrid");
assert.match(pageSource, /<DataGrid[\s\S]*storageKey="h4\.wechat-notify\.configs"/, "H4 通知配置必须使用公共 DataGrid");
assert.match(pageSource, /<DataGrid[\s\S]*storageKey="h4\.wechat-notify\.records"/, "H4 发送记录必须使用公共 DataGrid");

for (const route of [
  "/api/v1/wechat-notify/configs",
  "/api/v1/wechat-notify/settings",
  "/api/v1/wechat-notify/settings/test",
  "/api/v1/wechat-notify/send",
  "/api/v1/wechat-notify/records",
  "/api/v1/wechat-notify/records/{record_id}/resend",
]) {
  assert.ok(apiSource.includes(route), `H4 api-client 缺少 ${route}`);
}

assert.match(apiSource, /useTestH4WechatSettingsMutation/, "H4 api-client 必须提供参数测试 mutation");
assert.doesNotMatch(pageSource, /key: "test-settings"/, "H4 参数测试不得出现在列表工具栏");
assert.doesNotMatch(pageSource, /SettingsTestDialog/, "H4 参数测试不得使用独立确认弹窗");
assert.doesNotMatch(dialogsSource, /export function SettingsTestDialog/, "H4 参数测试不得提供独立确认弹窗");
assert.match(pageSource, /保存企业微信参数失败[\s\S]*return;[\s\S]*testSettingsMutation\.mutateAsync/, "H4 参数保存失败后不得继续测试或关闭弹窗");
assert.match(devMockCore, /await handleH4WechatNotifyDevMock\(req, res, pathname\)/, "主 dev mock 必须实际调用 H4 通知路由处理器");
assert.match(devMock, /pathname === "\/api\/v1\/wechat-notify\/configs"/, "dev mock 必须覆盖通知配置接口");
assert.match(devMock, /pathname === "\/api\/v1\/wechat-notify\/settings"/, "dev mock 必须覆盖企业微信参数设置接口");
assert.match(devMock, /pathname === "\/api\/v1\/wechat-notify\/settings\/test"/, "dev mock 必须覆盖企业微信参数测试接口");
assert.match(devMock, /validateWechatSettings\(settings\)/, "dev mock 参数测试必须校验已保存参数完整性");
assert.match(devMock, /pathname === "\/api\/v1\/wechat-notify\/records"/, "dev mock 必须覆盖发送记录接口");
assert.ok(devMock.includes("records\\/([^/]+)\\/resend"), "dev mock 必须覆盖发送记录重发接口");
assert.match(devMock, /requireIdempotencyKey\(req, res\)/, "H4 dev mock 写接口必须校验 Idempotency-Key");
assert.match(devMock, /typeof body\.enabled !== "boolean"/, "H4 dev mock 必须校验 enabled 类型");
assert.match(devMock, /Array\.isArray\(body\.channels\)/, "H4 dev mock 必须校验 channels 类型");
assert.match(apiSource, /startOfDayUtc/, "H4 查询起始日期必须转换为 UTC 时间边界");
assert.match(apiSource, /endOfDayUtc/, "H4 查询结束日期必须转换为 UTC 时间边界");
assert.match(pageSource, /!\["failed", "retrying"\]\.includes\(selectedRecord\.status\)/, "H4 重发按钮只能允许失败或重试中状态");
assert.match(pageSource, /localDateKey/, "H4 客户端日期筛选必须按本地日历比较");

console.log("h4 wechat notify slice self-check passed");
