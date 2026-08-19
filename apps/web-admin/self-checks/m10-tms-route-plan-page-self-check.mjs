import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const page = readFileSync(resolve(root, "src/pages/tms/TmsRoutePlanPage.tsx"), "utf8");
const queries = readFileSync(resolve(root, "src/features/tms/tms-route-plan-queries.ts"), "utf8");
const ui = readFileSync(resolve(root, "../../packages/ui/src/index.ts"), "utf8");
const app = readFileSync(resolve(root, "src/App.tsx"), "utf8");
const viewTypes = readFileSync(resolve(root, "src/app-shell/admin-view.ts"), "utf8");
const renderer = readFileSync(resolve(root, "src/app-shell/AdminViewRenderer.tsx"), "utf8");
const devMenu = readFileSync(resolve(root, "dev-mocks/admin-menu-dev-mock.ts"), "utf8");

assert.match(ui, /export \* from "\.\/ui"/, "页面必须使用公共 @wms/ui 导出");
assert.match(page, /from "@wms\/ui"/, "页面必须引用 @wms/ui");
assert.match(page, /useReceiveTmsRoutePlanMutation/, "页面必须接入接收 mutation");
for (const field of ["出库订单 ID", "司机 user_id", "规划版本", "路线站点 JSON"]) {
  assert.match(page, new RegExp(field), `页面必须包含${field}输入`);
}
assert.match(page, /outbound_order_ids/, "页面必须组装出库订单 ID 列表");
assert.match(page, /driver_user_id/, "页面必须提交司机 ID");
assert.match(page, /version/, "页面必须提交规划版本");
assert.match(page, /stops/, "页面必须提交路线站点");
assert.match(queries, /api\.POST\("\/api\/v1\/tms\/route-plans"/, "hook 必须调用真实 TMS 路径规划接收接口");
assert.match(queries, /Idempotency-Key/, "hook 必须生成幂等键");
assert.doesNotMatch(queries, /api\.(GET|PUT|PATCH|DELETE)\(/, "hook 不得凭空添加查询或更新接口");
assert.match(page, /mutation\.error/, "页面必须显示接收错误");
assert.match(page, /receivedPlan/, "页面必须显示接收后的路线信息");
for (const source of [app, viewTypes, renderer, devMenu]) {
  assert.match(source, /m10-route-plans/, "M10 路径规划必须有菜单、类型、路由和 dev mock 接线");
}

console.log("m10 TMS route plan page self-check passed");
