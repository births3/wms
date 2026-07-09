import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const pageSource = readFileSync(new URL("../src/pages/express/H5ExpressPage.tsx", import.meta.url), "utf8");
const apiSource = readFileSync(new URL("../src/features/express/express-queries.ts", import.meta.url), "utf8");
const devMock = readFileSync(new URL("../dev-mocks/express-dev-mock.ts", import.meta.url), "utf8");
const appShell = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");

assert.match(appShell, /"h5-express"/, "H5 快递对接必须接入管理端视图");
assert.match(pageSource, /QueryPanel/, "H5 快递对接必须使用公共查询组件");
assert.match(pageSource, /DataGrid/, "H5 快递对接必须使用公共 DataGrid");
assert.match(pageSource, /useCancelExpressWaybillMutation/, "H5 快递对接必须暴露取消运单动作");
assert.match(apiSource, /\/api\/v1\/express\/waybills\/\{waybill_no\}\/cancel/, "H5 前端必须调用取消运单 API");
assert.ok(
  devMock.includes("/api\\/v1\\/express\\/waybills\\/([^/]+)\\/cancel"),
  "H5 dev mock 必须覆盖取消运单 API",
);

console.log("H5 express slice self-check passed");
