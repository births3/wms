import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const appShell = readFileSync(resolve(__dirname, "../src/App.tsx"), "utf8");
const adminSidebarMenu = readFileSync(resolve(__dirname, "../src/app-shell/AdminSidebarMenu.tsx"), "utf8");
const workspaceTabsPath = resolve(__dirname, "../../../packages/ui/src/business/WorkspaceTabs/WorkspaceTabs.tsx");
const componentRegistry = readFileSync(resolve(__dirname, "../../../docs/prototypes/component-registry.md"), "utf8");
const menuPages = [
  "../src/App.tsx",
  "../src/pages/admin-menu/H1AdminMenuPage.tsx",
  "../src/pages/config-center/FeatureFlagConfigCenterPage.tsx",
  "../src/pages/inbound/M2InboundPage.tsx",
  "../src/pages/inventory/M3BatchManagementPage.tsx",
  "../src/pages/master-data/M1MasterDataPage.tsx",
  "../src/pages/master-data/SystemDictionaryPage.tsx",
  "../src/pages/outbound/M4OutboundPage.tsx",
];

assert.match(appShell, /lg:grid-cols-\[14rem_1fr\]/, "桌面菜单栏宽度应为 14rem");
assert.doesNotMatch(appShell, /lg:grid-cols-\[16rem_1fr\]/, "桌面菜单栏不能回退到 16rem");
assert.match(appShell, /WorkspaceTabs/, "AppShell 应接入公共 WorkspaceTabs");
assert.match(appShell, /WORKSPACE_TABS_STORAGE_KEY/, "工作台多页签必须有稳定 localStorage key");
assert.match(appShell, /readWorkspaceTabs/, "工作台多页签刷新后应从 localStorage 恢复");
assert.match(appShell, /writeWorkspaceTabs/, "工作台多页签变更后应写入 localStorage");
assert.match(appShell, /openTabs\.map/, "工作台多页签应按已打开页签 keep-alive 渲染页面");
assert.match(appShell, /hidden=\{tab\.view !== view\}/, "非激活页签应隐藏但不卸载");
assert.match(appShell, /lg:grid-rows-\[3\.5rem_1fr\]/, "桌面端应有顶部导航栏和下方工作区两行布局");
assert.match(appShell, /lg:col-span-2/, "顶部导航栏应横跨左侧菜单和主内容区");
assert.match(appShell, /<WorkspaceTabs\b[\s\S]*tabs=\{openTabs\.map/, "WorkspaceTabs 应融入顶部导航栏并消费 openTabs");
assert.match(appShell, /<Warehouse className="size-5"/, "顶部导航栏左侧应显示系统 Logo");
assert.match(appShell, /currentUser\.display_name/, "顶部导航栏右侧应显示用户信息");
assert.match(appShell, /menuKeysForActiveView/, "菜单切换页面时应自动展开当前路径");
assert.doesNotMatch(adminSidebarMenu, /\|\|\s*hasActive/, "当前页面所在菜单不能用 hasActive 强制展开，否则用户无法折叠");

assert.ok(existsSync(workspaceTabsPath), "必须新增公共 WorkspaceTabs 组件");
const workspaceTabs = readFileSync(workspaceTabsPath, "utf8");
assert.doesNotMatch(workspaceTabs, /lg:sticky|lg:top-0|border-b bg-muted\/20/, "WorkspaceTabs 不应作为内容区独立横条");
assert.match(workspaceTabs, /rounded-full/, "激活页签应使用圆角胶囊样式");
assert.match(workspaceTabs, /bg-primary\/10 text-primary/, "激活页签应使用浅蓝底和蓝色文字");
assert.match(workspaceTabs, /h-8/, "WorkspaceTabs 页签应使用紧凑高度");
assert.match(workspaceTabs, /whitespace-nowrap/, "WorkspaceTabs 页签文案应单行展示");
assert.doesNotMatch(workspaceTabs, /tab\.subtitle &&/, "WorkspaceTabs 页签不能默认展示第二行副标题");
assert.match(workspaceTabs, /onContextMenu/, "WorkspaceTabs 应支持右键菜单");
assert.match(workspaceTabs, /关闭其他/, "WorkspaceTabs 右键菜单应支持关闭其他");
assert.match(componentRegistry, /\*\*WorkspaceTabs\*\*/, "WorkspaceTabs 必须登记到组件注册表");

for (const relativePath of menuPages) {
  const source = readFileSync(resolve(__dirname, relativePath), "utf8");
  assert.doesNotMatch(source, /mx-auto flex w-full max-w/, `${relativePath} 不能使用居中窄版页面容器`);
  assert.match(source, /lg:px-8/, `${relativePath} 桌面页面左侧留白应为 32px`);
}

console.log("app shell layout self-check passed");
