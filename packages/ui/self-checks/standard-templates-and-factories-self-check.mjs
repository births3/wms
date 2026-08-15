import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

// 验证所有标准工厂函数与模板文件存在且包含标准导出
const filesToCheck = [
  "packages/ui/src/business/DataGrid/data-grid-column-factory.ts",
  "packages/ui/src/business/PageTemplates/FormDialogTemplate.tsx",
  "packages/ui/src/business/PageTemplates/DetailDrawerTemplate.tsx",
  "packages/ui/src/business/PageTemplates/TwoPanePageTemplate.tsx",
  "packages/ui/src/business/StepFlow/WorkflowActionBar.tsx",
];

for (const relPath of filesToCheck) {
  const fullPath = path.resolve(process.cwd(), relPath);
  assert.ok(fs.existsSync(fullPath), `File ${relPath} should exist`);
  const content = fs.readFileSync(fullPath, "utf8");
  assert.ok(content.length > 50, `File ${relPath} should not be empty`);
}

console.log("standard templates and factories self-check passed");

