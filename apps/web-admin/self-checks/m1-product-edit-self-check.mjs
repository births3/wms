import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(new URL("..", import.meta.url).pathname);
const page = fs.readFileSync(path.join(root, "src/pages/master-data/M1MasterDataPage.tsx"), "utf8");
const table = fs.readFileSync(path.join(root, "src/pages/master-data/ProductEditTable.tsx"), "utf8");

assert.doesNotMatch(page, /ProductEditDialog/);
assert.doesNotMatch(page, /useProductEditDialog/);
assert.doesNotMatch(table, /onEdit/);
assert.doesNotMatch(table, /编辑商品/);

console.log("m1 read-only product table self-check passed");
