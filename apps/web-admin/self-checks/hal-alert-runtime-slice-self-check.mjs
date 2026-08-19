import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url);
const read = (file) => readFileSync(new URL(file, root), "utf8");
const dashboard = read("src/pages/alert-engine/AlertDashboardPage.tsx");
const escalation = read("src/pages/alert-engine/AlertEscalationPage.tsx");
const queries = read("src/features/alert-engine/alert-runtime-queries.ts");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const app = read("src/App.tsx");
const queryGovernance = JSON.parse(read("src/pages/page-query-core-fields.json"));

for (const token of [
  "页面设计契约", "<QueryPanel", "<DataGrid", "alertDashboardQueryFields",
  "alertDashboardCoreQueryFieldKeys", "导出 Excel", "GSP 告警生命周期报表",
]) assert.match(dashboard, new RegExp(token));

for (const token of [
  "页面设计契约", /(?:<QueryPanel|<ListPageTemplate)/, /(?:<DataGrid|<ListPageTemplate)/, "alertEscalationQueryFields",
  "alertEscalationCoreQueryFieldKeys", "最多 3 级", "threshold_seconds", "holiday_dates",
]) assert.match(escalation, typeof token === "string" ? new RegExp(token) : token);

for (const endpoint of [
  "/api/v1/alerts/active", "/api/v1/alerts/statistics", "/api/v1/alerts/gsp-report",
  "/api/v1/alerts/exports", "/api/v1/alerts/{id}/acknowledge",
  "/api/v1/alerts/{id}/handling", "/api/v1/alerts/{id}/close", "/api/v1/alerts/{id}/ignore",
  "/api/v1/alert-escalation-rules",
]) assert.ok(queries.includes(endpoint), `missing real H-AL API ${endpoint}`);
assert.match(queries, /refetchInterval:\s*5_000/);

for (const pageId of ["hal-alert-dashboard", "hal-alert-escalations"]) {
  assert.ok(renderer.includes(pageId), `renderer missing ${pageId}`);
  assert.ok(app.includes(pageId), `menu missing ${pageId}`);
  assert.ok(queryGovernance.pages.some((entry) => entry.id === pageId && entry.required === true), `query governance missing ${pageId}`);
}

console.log("H-AL alert runtime slice self-check passed");
