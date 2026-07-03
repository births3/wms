import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const appSource = read("src/App.tsx");
const queriesSource = read("src/features/master-data/master-data-queries.ts");
const systemDictionaryPageSource = read("src/pages/master-data/SystemDictionaryPage.tsx");
const viteConfigSource = read("vite.config.ts");

assert.doesNotMatch(appSource, /m1-special-drug-categories/);
assert.equal(fs.existsSync(path.join(root, "src/pages/master-data/SpecialDrugCategoriesPage.tsx")), false);

assert.match(queriesSource, /specialDrugCategoryDictCode = "special_drug_category"/);
assert.match(queriesSource, /api\.GET\("\/api\/v1\/system-dictionaries\/\{dict_code\}\/items"/);
assert.doesNotMatch(queriesSource, /\/api\/v1\/master-data\/special-drug-categories/);

assert.match(systemDictionaryPageSource, /special_drug_category/);
assert.match(viteConfigSource, /special_drug_category/);
assert.doesNotMatch(viteConfigSource, /\/api\/v1\/master-data\/special-drug-categories/);

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}
