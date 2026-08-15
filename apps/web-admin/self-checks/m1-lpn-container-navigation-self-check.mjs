import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../../..", import.meta.url));
const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const renderer = readFileSync(new URL("../src/app-shell/AdminViewRenderer.tsx", import.meta.url), "utf8");
const views = readFileSync(new URL("../src/app-shell/admin-view.ts", import.meta.url), "utf8");
const mock = readFileSync(new URL("../dev-mocks/admin-menu-dev-mock.ts", import.meta.url), "utf8");
const page = readFileSync(new URL("../src/pages/master-data/M1LpnContainerPage.tsx", import.meta.url), "utf8");

assert.match(views, /m1-lpn-containers/);
assert.match(app, /id: "m1-lpn-containers"/);
assert.match(renderer, /M1LpnContainerPage/);
assert.match(mock, /m1-lpn-containers/);
assert.match(page, /coreQueryFieldKeys=\{lpnCoreQueryFieldKeys\}/);
assert.match(page, /key: "created_at"/);
assert.match(page, /gridProps=\{/);
assert.match(page, /FormDialogTemplate/);
assert.match(page, /header=\{\{ title: "M1 容器管理"/);
assert.match(page, /创建容器/);
console.log("m1-lpn-container-navigation-self-check ok");
