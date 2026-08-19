import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, "../..");

/**
 * 针对标准列表页面的自动重构函数
 */
export function refactorStandardListPage(filePath) {
  let content = fs.readFileSync(filePath, "utf8");
  if (content.includes("ListPageTemplate") || content.includes("MasterDetailPageTemplate") || content.includes("ConfigPageTemplate") || content.includes("DashboardPageTemplate")) {
    console.log(`Skipping already migrated: ${path.basename(filePath)}`);
    return;
  }

  console.log(`Migrating: ${path.basename(filePath)}...`);
  // 检查是否包含标准结构
  // 1. 替换 imports
  if (!content.includes("ListPageTemplate")) {
    content = content.replace(/(import\s*\{[^}]*?)(\bPageHeader\b|\bDataGrid\b|\bQueryPanel\b)([^}]*\}\s*from\s*["']@wms\/ui["'];?)/g, (match, p1, p2, p3) => {
      let merged = p1 + p2 + p3;
      if (!merged.includes("ListPageTemplate")) {
        merged = merged.replace(/import\s*\{/, "import {\n  ListPageTemplate,");
      }
      return merged;
    });
  }

  fs.writeFileSync(filePath, content, "utf8");
}
