import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");

const app = read("src/App.tsx");
const view = read("src/app-shell/admin-view.ts");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const menuMock = read("dev-mocks/admin-menu-dev-mock.ts");
const devMock = read("dev-mocks/web-admin-dev-mock-core.ts");
const page = read("src/pages/inbound/M2InboundDocumentsPage.tsx");
const model = read("src/pages/inbound/inbound-document-entry-model.ts");
const documentQueries = read("src/features/drug-inspection/document-queries.ts");
const queryConfig = read("src/pages/page-query-core-fields.json");
const pageId = "m2-inbound-documents";

for (const source of [app, view, renderer, menuMock, queryConfig]) {
  assert.ok(source.includes(pageId), "入库资料录入必须完成菜单、路由、已发布菜单和查询登记");
}
assert.match(renderer, /<M2InboundDocumentsPage/);
assert.match(page, /<QueryPanel/);
assert.match(page, /quickFilters=/);
assert.match(page, /<DataGrid/);
assert.match(page, /<Dialog/);
assert.match(page, /type="file"/);
assert.match(page, /validateUpstreamDeliveryFiles/);
assert.doesNotMatch(`${page}\n${model}`, /开发 Mock|overrides|buildConfirmationDocumentRows/);
assert.match(documentQueries, /\/api\/v1\/drug-inspection\/inbound-documents/);
assert.match(devMock, /\/api\/v1\/drug-inspection\/inbound-documents/);
assert.match(documentQueries, /\/api\/v1\/attachments\/uploads/);
assert.match(documentQueries, /putApiBinary/);
assert.match(documentQueries, /\/api\/v1\/drug-inspection\/report-versions/);
assert.match(documentQueries, /upstream-delivery-document-versions/);

console.log("M2 inbound documents page self-check passed");
