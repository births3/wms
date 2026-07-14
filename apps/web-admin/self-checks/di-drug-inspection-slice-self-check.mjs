import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const root = new URL("../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");
const page = read("src/pages/drug-inspection/DrugInspectionPlatformPage.tsx");
const queries = read("src/features/drug-inspection/drug-inspection-queries.ts");
assert.match(page, /<QueryPanel/);
assert.match(page, /<DataGrid/);
assert.match(page, /<Dialog/);
assert.match(page, /validateDrugInspectionForm/);
assert.match(page, /api_key_configured/);
assert.doesNotMatch(page, /api_key_alias.*row|password_alias.*row/);
assert.match(queries, /api\.GET\("\/api\/v1\/drug-inspection\/platforms"/);
assert.match(queries, /api\.POST\("\/api\/v1\/drug-inspection\/platforms"/);
assert.match(queries, /api\.PATCH\("\/api\/v1\/drug-inspection\/platforms\/\{platform_id\}\/status"/);
console.log("M-DI drug inspection platform slice self-check passed");
