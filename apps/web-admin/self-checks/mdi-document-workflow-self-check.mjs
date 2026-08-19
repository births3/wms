import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");

const app = read("src/App.tsx");
const view = read("src/app-shell/admin-view.ts");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const menuMock = read("dev-mocks/admin-menu-dev-mock.ts");
const reviewPage = read("src/pages/drug-inspection/DrugInspectionReviewPage.tsx");
const stampPage = read("src/pages/drug-inspection/DrugInspectionStampPage.tsx");
const documentQueries = read("src/features/drug-inspection/document-queries.ts");
const stampQueries = read("src/features/drug-inspection/stamp-queries.ts");

for (const pageId of ["m-di-review", "m-di-stamp"]) {
  for (const source of [app, view, renderer, menuMock]) {
    assert.match(source, new RegExp(pageId), `${pageId} 必须完成菜单、路由和已发布菜单接线`);
  }
}

assert.match(reviewPage, /(?:<QueryPanel|<ListPageTemplate)/);
assert.match(reviewPage, /(?:<DataGrid|<ListPageTemplate)/);
assert.match(reviewPage, /版本与审核时间线/);
assert.match(reviewPage, /退回修改/);
assert.match(documentQueries, /\/api\/v1\/drug-inspection\/review-queue/);
assert.match(documentQueries, /\/api\/v1\/drug-inspection\/report-versions\/\{version_id\}\/review/);

assert.match(stampPage, /透明 PNG 图章/);
assert.match(stampPage, /pointermove/);
assert.match(stampPage, /处理规则应用范围/);
assert.match(stampPage, /rejected: "已退回"/);
assert.match(stampQueries, /\/api\/v1\/drug-inspection\/stamp-versions/);
assert.match(stampQueries, /\/api\/v1\/drug-inspection\/processing-rule-versions/);
assert.match(stampQueries, /oversize-approval/);

console.log("M-DI document review and stamp workflow self-check passed");
