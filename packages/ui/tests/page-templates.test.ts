import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const templatesDir = path.resolve(__dirname, "../src/business/PageTemplates");

console.log("Running PageTemplates component contracts test...");

// 1. 验证文件存在
const listPageTemplatePath = path.join(templatesDir, "ListPageTemplate.tsx");
const masterDetailPageTemplatePath = path.join(templatesDir, "MasterDetailPageTemplate.tsx");
const configPageTemplatePath = path.join(templatesDir, "ConfigPageTemplate.tsx");
const dashboardPageTemplatePath = path.join(templatesDir, "DashboardPageTemplate.tsx");
const indexExportPath = path.join(templatesDir, "index.ts");

assert.ok(fs.existsSync(listPageTemplatePath), "ListPageTemplate.tsx 必须存在");
assert.ok(fs.existsSync(masterDetailPageTemplatePath), "MasterDetailPageTemplate.tsx 必须存在");
assert.ok(fs.existsSync(configPageTemplatePath), "ConfigPageTemplate.tsx 必须存在");
assert.ok(fs.existsSync(dashboardPageTemplatePath), "DashboardPageTemplate.tsx 必须存在");
assert.ok(fs.existsSync(indexExportPath), "PageTemplates/index.ts 必须存在");

const listSource = fs.readFileSync(listPageTemplatePath, "utf8");
const masterDetailSource = fs.readFileSync(masterDetailPageTemplatePath, "utf8");
const configSource = fs.readFileSync(configPageTemplatePath, "utf8");
const dashboardSource = fs.readFileSync(dashboardPageTemplatePath, "utf8");
const businessIndexSource = fs.readFileSync(path.resolve(__dirname, "../src/business/index.ts"), "utf8");

// 2. 验证 ListPageTemplate 具备 flex-1 min-h-0 视口撑满能力
assert.match(listSource, /flex-1 min-h-0/, "ListPageTemplate 根 section 必须声明 flex-1 min-h-0");
assert.match(listSource, /<QueryPanel/, "ListPageTemplate 必须内置集成 QueryPanel");
assert.match(listSource, /<DataGrid/, "ListPageTemplate 必须内置集成 DataGrid");
assert.match(listSource, /<PageHeader/, "ListPageTemplate 必须内置集成 PageHeader");

// 3. 验证 MasterDetailPageTemplate 具备双栏独立 flex-1 min-h-0
assert.match(masterDetailSource, /flex-1 min-h-0/, "MasterDetailPageTemplate 根 section 必须声明 flex-1 min-h-0");
assert.match(masterDetailSource, /grid/, "MasterDetailPageTemplate 必须使用响应式 grid 双栏容器");

// 4. 验证 ConfigPageTemplate 与 DashboardPageTemplate
assert.match(configSource, /flex-1 min-h-0/, "ConfigPageTemplate 根 section 必须声明 flex-1 min-h-0");
assert.match(dashboardSource, /flex-1 min-h-0/, "DashboardPageTemplate 根 section 必须声明 flex-1 min-h-0");

// 5. 验证包顶层导出
assert.match(businessIndexSource, /ListPageTemplate/, "@wms/ui business index 必须导出 ListPageTemplate");
assert.match(businessIndexSource, /MasterDetailPageTemplate/, "@wms/ui business index 必须导出 MasterDetailPageTemplate");
assert.match(businessIndexSource, /ConfigPageTemplate/, "@wms/ui business index 必须导出 ConfigPageTemplate");
assert.match(businessIndexSource, /DashboardPageTemplate/, "@wms/ui business index 必须导出 DashboardPageTemplate");

console.log("✅ PageTemplates component contracts test passed!");
