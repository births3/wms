import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url);
const read = (file) => readFileSync(new URL(file, root), "utf8");
const page = read("src/pages/alert-engine/AlertDefinitionPage.tsx");
const queries = read("src/features/alert-engine/alert-definition-queries.ts");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const app = read("src/App.tsx");
const queryGovernance = JSON.parse(read("src/pages/page-query-core-fields.json"));

for (const token of [
  "页面设计契约",
  "<QueryPanel",
  "<DataGrid",
  "alertDefinitionQueryFields",
  "alertDefinitionCoreQueryFieldKeys",
  "质量联系单",
  "GSP 强制",
]) assert.match(page, new RegExp(token));

for (const token of ["/api/v1/alert-definitions", "/api/v1/alert-definitions/change-requests", "Idempotency-Key"]) {
  assert.match(queries, new RegExp(token));
}

assert.match(renderer, /hal-alert-definitions/);
assert.match(app, /hal-alert-definitions/);
assert.ok(queryGovernance.pages.some((entry) => entry.id === "hal-alert-definitions" && entry.required === true));
console.log("H-AL alert definition slice self-check passed");
