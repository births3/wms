import { readFileSync } from "node:fs";

function source(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
}

const page = source("src/pages/print-orchestration/H9DeliveryNoteAggregationPage.tsx");
const feature = source("src/features/print-orchestration/print-orchestration-queries.ts");
const app = source("src/App.tsx");
const renderer = source("src/app-shell/AdminViewRenderer.tsx");
const view = source("src/app-shell/admin-view.ts");
const menu = source("dev-mocks/admin-menu-dev-mock.ts");
const queryConfig = JSON.parse(source("src/pages/page-query-core-fields.json"));

for (const token of ["QueryPanel", "DataGrid", "Dialog", "作业·随货同行单归集"]) {
  if (!page.includes(token)) throw new Error(`H9 归集页面缺少 ${token}`);
}
for (const path of [
  "/api/v1/print-orchestration/delivery-note-candidates",
  "/api/v1/print-orchestration/delivery-note-groups",
  "/api/v1/print-orchestration/route-bindings",
  "/api/v1/print-orchestration/cutoff-plans",
]) {
  if (!feature.includes(path)) throw new Error(`H9 归集 feature 缺少 ${path}`);
}
for (const text of [app, renderer, view, menu]) {
  if (!text.includes("h9-delivery-note-aggregation")) {
    throw new Error("H9 归集菜单、视图或渲染接线缺失");
  }
}
const config = queryConfig.pages.find((item) => item.id === "h9-delivery-note-aggregation");
if (!config?.required || !config.core.includes("warehouseId")) {
  throw new Error("H9 归集页面查询分类未登记");
}

console.log("h9-delivery-note-aggregation-self-check: ok");
