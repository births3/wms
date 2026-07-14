import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const page = readFileSync(resolve(root, "src/pages/billing/BillingRuleConfigPage.tsx"), "utf8");
const queries = readFileSync(resolve(root, "src/features/billing/billing-rule-queries.ts"), "utf8");
const app = readFileSync(resolve(root, "src/App.tsx"), "utf8");
const viewTypes = readFileSync(resolve(root, "src/app-shell/admin-view.ts"), "utf8");
const renderer = readFileSync(resolve(root, "src/app-shell/AdminViewRenderer.tsx"), "utf8");
const devMenu = readFileSync(resolve(root, "dev-mocks/admin-menu-dev-mock.ts"), "utf8");

for (const component of ["Button", "Card", "DataGrid", "Input", "PageHeader"]) {
  assert.match(page, new RegExp(`\\b${component}\\b`), `页面必须复用 @wms/ui ${component}`);
}
for (const field of ["charge_item", "unit", "billing_cycle", "unit_price_cents", "effective_from", "effective_to", "contract_id"]) {
  assert.match(page, new RegExp(`name=\\"${field}\\"`), `页面缺少 ${field} 输入`);
}

assert.match(page, /useCreateBillingRuleMutation/, "页面必须使用计费规则 mutation");
assert.match(page, /setRules\(\(current\) => \[result, \.\.\.current\]\)/, "成功响应必须加入页面列表");
assert.match(page, /role=\{notice\.kind === "error" \? "alert" : "status"\}/, "成功和错误结果必须可见");
assert.match(queries, /api\.POST\("\/api\/v1\/billing\/rules"/, "hook 必须调用真实计费规则 POST");
assert.match(queries, /"Idempotency-Key"/, "计费规则提交必须发送幂等键");
assert.doesNotMatch(queries, /api\.GET\(/, "计费规则 hook 不得伪造 GET 查询");
for (const source of [app, viewTypes, renderer, devMenu]) {
  assert.match(source, /m9-billing-rules/, "M9 计费规则必须有菜单、类型、路由和 dev mock 接线");
}

console.log("m9 billing rule page self-check passed");
