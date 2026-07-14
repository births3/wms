import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(new URL("..", import.meta.url).pathname);
const page = fs.readFileSync(path.join(root, "src/pages/master-data/M1MasterDataPage.tsx"), "utf8");
const api = fs.readFileSync(
  path.join(root, "src/features/master-data/master-data-queries/api.ts"),
  "utf8",
);
const dialog = fs.readFileSync(
  path.join(root, "src/pages/master-data/ProductBatchImportDialog.tsx"),
  "utf8",
);

assert.match(page, /ProductBatchImportDialog/);
assert.match(page, /setProductBatchImportOpen\(true\)/);
assert.match(api, /export async function batchCreateProducts/);
assert.match(api, /products\/batch-sync/);
assert.match(dialog, /export function parseProductImportText/);
assert.match(dialog, /已解析/);
assert.match(dialog, /storage_condition/);
assert.doesNotMatch(page, /批量导入入口已记录/);

console.log("m1 product batch import self-check passed");
