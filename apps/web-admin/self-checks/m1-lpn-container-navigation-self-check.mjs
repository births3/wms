import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";

const nav = readFileSync(new URL("../src/features/master-data/lpn-container-nav.ts", import.meta.url), "utf8");
const page = readFileSync(new URL("../src/pages/master-data/M1LpnContainerPage.tsx", import.meta.url), "utf8");

assert.match(nav, /export const LPN_CONTAINER_VIEW_ID = "m1-lpn-containers"/);
assert.match(nav, /title: "M1 容器管理"/);
assert.match(page, /coreQueryFieldKeys=\{lpnCoreQueryFieldKeys\}/);
assert.match(page, /key: "created_at"/);
assert.match(page, /gridProps=\{/);
assert.match(page, /FormDialogTemplate/);
assert.match(page, /header=\{\{ title: lpnContainerMenuItem.title/);
assert.match(page, /创建容器/);

function assertIfPresent(relPath, pattern, label) {
  const abs = new URL(relPath, import.meta.url);
  if (!existsSync(abs)) {
    console.log(`m1-lpn-container-navigation-self-check skip missing ${label}`);
    return;
  }
  assert.match(readFileSync(abs, "utf8"), pattern);
}

assertIfPresent("../src/App.tsx", /id: "m1-lpn-containers"/, "App.tsx");
assertIfPresent("../src/app-shell/AdminViewRenderer.tsx", /M1LpnContainerPage/, "AdminViewRenderer.tsx");
assertIfPresent("../src/app-shell/admin-view.ts", /m1-lpn-containers/, "admin-view.ts");
assertIfPresent("../dev-mocks/admin-menu-dev-mock.ts", /m1-lpn-containers/, "admin-menu-dev-mock.ts");

console.log("m1-lpn-container-navigation-self-check ok");
