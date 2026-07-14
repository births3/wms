import { readFileSync } from "node:fs";
import { strict as assert } from "node:assert";

const root = new URL("..", import.meta.url);
const read = (relative) => readFileSync(new URL(relative, root), "utf8");
const page = read("src/pages/document-numbering/MCGDocumentNumberingPage.tsx");
const queries = read("src/features/document-numbering/document-numbering-queries.ts");
const app = read("src/App.tsx");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const views = read("src/app-shell/admin-view.ts");
const mockCore = read("dev-mocks/web-admin-dev-mock-core.ts");
const queryConfig = JSON.parse(read("src/pages/page-query-core-fields.json"));

assert.match(app, /id: "mcg-numbering"/, "主菜单必须登记 M-CG");
assert.match(renderer, /MCGDocumentNumberingPage/, "M-CG 必须有页面渲染入口");
assert.match(views, /"mcg-numbering"/, "AdminView 必须登记 M-CG");
assert.match(page, /<QueryPanel/, "M-CG 必须使用公共查询组件");
assert.match(page, /useSystemDictionaryItemOptionsQuery\("document_type"\)/, "M-CG 单据类型必须读取 M1 系统字典");
assert.doesNotMatch(page, /const documentTypeOptions = \[/, "M-CG 不得在页面硬编码单据类型选项");
assert.match(page, /<DataGrid[\s\S]*storageKey="mcg\.document-number-rules"/, "规则列表必须使用 DataGrid");
assert.match(page, /<DataGrid[\s\S]*storageKey="mcg\.document-number-allocations"/, "生成记录必须使用 DataGrid");
assert.match(page, /新增单据号规则|编辑单据号规则/, "规则维护必须使用弹窗");
assert.match(page, /renderPreview/, "规则维护必须提供预览");
assert.match(queries, /Idempotency-Key/, "M-CG 写接口必须携带幂等键");
assert.match(queries, /document-number-allocations/, "前端必须接入生成记录接口");
assert.match(mockCore, /handleDocumentNumberingDevMock/, "9002 dev mock 必须挂载 M-CG 路由");
const config = queryConfig.pages.find((item) => item.id === "mcg-numbering");
assert.deepEqual(config?.core, ["documentType"], "M-CG 核心查询条件必须登记");
console.log("M-CG document numbering slice self-check passed");
