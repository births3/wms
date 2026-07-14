import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const appSource = read("src/App.tsx");
const queriesSource = [
  read("src/features/master-data/master-data-queries/types.ts"),
  read("src/features/master-data/master-data-queries/api.ts"),
].join("\n");
const systemDictionaryPageSource = read("src/pages/master-data/SystemDictionaryPage.tsx");
const devMockSource = [
  read("dev-mocks/web-admin-dev-mock-core.ts"),
  read("dev-mocks/web-admin-dev-mock-model.ts"),
].join("\n");
const m1WarehouseStoriesSource = fs.readFileSync(
  path.join(root, "../../docs/domain/user-stories-m1-master-data-warehouse.md"),
  "utf8",
);
const complianceMigration = fs.readFileSync(
  path.join(root, "../../backend/migrations/202607120006_m1_special_drug_compliance_defaults.sql"),
  "utf8",
);

assert.doesNotMatch(appSource, /m1-special-drug-categories/);
assert.equal(fs.existsSync(path.join(root, "src/pages/master-data/SpecialDrugCategoriesPage.tsx")), false);

assert.match(queriesSource, /specialDrugCategoryDictCode = "special_drug_category"/);
assert.match(queriesSource, /api\.GET\("\/api\/v1\/system-dictionaries\/\{dict_code\}\/items"/);
assert.doesNotMatch(queriesSource, /\/api\/v1\/master-data\/special-drug-categories/);

assert.match(systemDictionaryPageSource, /special_drug_category/);
assert.match(devMockSource, /special_drug_category/);
assert.doesNotMatch(devMockSource, /\/api\/v1\/master-data\/special-drug-categories/);
for (const field of [
  "requires_dual_person_matrix",
  "requires_dedicated_ledger",
  "requires_dedicated_storage",
  "requires_qualification",
  "regulation_basis",
]) {
  assert.match(complianceMigration, new RegExp(field));
}

for (const dictCode of ["temperature_zone", "quality_color", "zone_type", "location_type"]) {
  assert.match(queriesSource, new RegExp(`code: "${dictCode}"`));
  assert.match(devMockSource, new RegExp(`${dictCode}: \\[`));
  assert.match(m1WarehouseStoriesSource, new RegExp(`\\\`${dictCode}\\\``));
}

assert.match(queriesSource, /code: "print_template_type"/);
assert.match(devMockSource, /print_template_type: \[/);

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}
