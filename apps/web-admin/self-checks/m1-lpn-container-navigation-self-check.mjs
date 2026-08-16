import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const nav = readFileSync(new URL("../src/features/master-data/lpn-container-nav.ts", import.meta.url), "utf8");
const page = readFileSync(new URL("../src/pages/master-data/M1LpnContainerPage.tsx", import.meta.url), "utf8");
const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const renderer = readFileSync(new URL("../src/app-shell/AdminViewRenderer.tsx", import.meta.url), "utf8");
const views = readFileSync(new URL("../src/app-shell/admin-view.ts", import.meta.url), "utf8");
const mock = readFileSync(new URL("../dev-mocks/admin-menu-dev-mock.ts", import.meta.url), "utf8");

assert.match(nav, /export const LPN_CONTAINER_VIEW_ID = "m1-lpn-containers"/);
assert.match(nav, /title: "M1 容器管理"/);
assert.match(page, /coreQueryFieldKeys=\{lpnCoreQueryFieldKeys\}/);
assert.match(page, /key: "created_at"/);
assert.match(page, /gridProps=\{/);
assert.match(page, /storageKey: "m1-lpn-containers"/);
assert.match(page, /FormDialogTemplate/);
assert.match(page, /header=\{\{ title: lpnContainerMenuItem.title/);
assert.match(page, /创建容器/);
assert.match(
  app,
  /\{ id: "m1-lpn-containers", title: "M1 容器管理", subtitle: "LPN \/ 类型策略", icon: PackageCheck \}/,
);
assert.match(renderer, /view === "m1-lpn-containers"/);
assert.match(views, /\| "m1-lpn-containers"/);
assert.match(mock, /\["m1-lpn-containers", "M1 容器管理", "PackageCheck"\]/);

console.log("m1-lpn-container-navigation-self-check ok");
