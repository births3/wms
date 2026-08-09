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
assert.match(pageSource, /title="运单作业"/, "H5 必须提供独立运单作业区");
assert.match(pageSource, /打印面单/, "H5 运单作业区必须提供打印面单");
assert.match(pageSource, /配置与运单作业|运单作业独立分区/, "H5 副标题必须体现配置与运单作业分区");
assert.match(apiSource, /\/api\/v1\/express\/waybills\/\{waybill_no\}\/cancel/, "H5 前端必须调用取消运单 API");
assert.ok(
  devMock.includes("/api\\/v1\\/express\\/waybills\\/([^/]+)\\/cancel"),
  "H5 dev mock 必须覆盖取消运单 API",
);

const carriersGridStart = pageSource.indexOf('storageKey="h5.express.carriers"');
assert.notEqual(carriersGridStart, -1, "H5 快递商配置 DataGrid 必须存在");
const rulesGridStart = pageSource.indexOf('storageKey="h5.express.routing-rules"');
assert.notEqual(rulesGridStart, -1, "H5 快递选择规则 DataGrid 必须存在");
const carriersGrid = pageSource.slice(carriersGridStart, rulesGridStart > carriersGridStart ? rulesGridStart : undefined);
assert.doesNotMatch(carriersGrid, /label:\s*"下单"/, "快递商配置表不得挂载下单动作");
assert.doesNotMatch(carriersGrid, /label:\s*"轨迹"/, "快递商配置表不得挂载轨迹动作");
assert.doesNotMatch(carriersGrid, /label:\s*"取消"/, "快递商配置表不得挂载取消动作");
assert.doesNotMatch(carriersGrid, /printAction=/, "快递商配置表不得挂载打印面单动作");

const rulesGrid = pageSource.slice(rulesGridStart);
assert.match(rulesGrid, /createAction=\{\{ label: BUTTON_ADD/, "快递选择规则表必须保留新增");
assert.match(rulesGrid, /editAction=\{\{/, "快递选择规则表必须保留修改");
assert.match(rulesGrid, /refreshAction=\{\{/, "快递选择规则表必须保留刷新");
assert.doesNotMatch(rulesGrid, /label:\s*"下单"/, "快递选择规则表不得挂载运单作业动作");

console.log("H5 express slice self-check passed");
