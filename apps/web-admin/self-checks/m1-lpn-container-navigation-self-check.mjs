import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const nav = readFileSync(new URL("../src/features/master-data/lpn-container-nav.ts", import.meta.url), "utf8");
const page = readFileSync(new URL("../src/pages/master-data/M1LpnContainerPage.tsx", import.meta.url), "utf8");
const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const renderer = readFileSync(new URL("../src/app-shell/AdminViewRenderer.tsx", import.meta.url), "utf8");
const views = readFileSync(new URL("../src/app-shell/admin-view.ts", import.meta.url), "utf8");
const mock = readFileSync(new URL("../dev-mocks/admin-menu-dev-mock.ts", import.meta.url), "utf8");
const lpnMock = readFileSync(new URL("../dev-mocks/lpn-container-dev-mock.ts", import.meta.url), "utf8");

assert.match(nav, /export const LPN_CONTAINER_VIEW_ID = "m1-lpn-containers"/);
assert.match(nav, /title: "M1 容器管理"/);
assert.match(page, /coreQueryFieldKeys=\{lpnCoreQueryFieldKeys\}/);
assert.match(page, /key: "created_at"/);
assert.match(page, /gridProps=\{/);
assert.match(page, /storageKey: "m1-lpn-containers"/);
assert.match(page, /storageKey="m1-lpn-type-policies"/);
assert.match(page, /value: "pallet"/);
assert.match(page, /value: "tote"/);
assert.match(page, /value: "outbound_box"/);
assert.match(page, /value: "insulated_box"/);
assert.match(page, /value: "blind_label"/);
assert.match(page, /FormDialogTemplate/);
assert.match(page, /header=\{\{ title: lpnContainerMenuItem.title/);
assert.match(page, /创建容器/);
assert.match(page, /批量新增容器/);
assert.match(page, /batch-create/);
assert.match(page, /useBatchCreateLpnContainersMutation/);
assert.match(page, /parseBatchCount/);
assert.match(page, /编辑容器/);
assert.match(page, /editAction/);
assert.match(page, /deleteAction/);
assert.match(page, /软删除选中空闲容器/);
assert.match(page, /quality-lock/);
assert.match(page, /LpnQualityLockDialogs/);
assert.match(
  app,
  /\{ id: "m1-lpn-containers", title: "M1 容器管理", subtitle: "LPN \/ 类型策略", icon: PackageCheck \}/,
);
assert.match(renderer, /view === "m1-lpn-containers"/);
assert.match(views, /\| "m1-lpn-containers"/);
assert.match(mock, /\["m1-lpn-containers", "M1 容器管理", "PackageCheck"\]/);
assert.match(lpnMock, /lpn-containers\/batch-create/);
assert.match(lpnMock, /M1_LPN_BATCH_COUNT_INVALID/);

console.log("m1-lpn-container-navigation-self-check ok");
