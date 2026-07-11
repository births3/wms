import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const schema = readFileSync(new URL("../../../packages/api-client/src/schema.ts", import.meta.url), "utf8");
const glossary = readFileSync(new URL("../../../docs/glossary.md", import.meta.url), "utf8");
const queriesSource = readFileSync(new URL("../src/features/master-data/master-data-queries/queries.ts", import.meta.url), "utf8");
const apiSource = readFileSync(new URL("../src/features/master-data/master-data-queries/api.ts", import.meta.url), "utf8");
const crudDialogSource = readFileSync(new URL("../src/pages/master-data/MasterDataCrudDialog.tsx", import.meta.url), "utf8");
const batchDialogSource = readFileSync(new URL("../src/pages/master-data/LocationBatchDialog.tsx", import.meta.url), "utf8");

for (const field of [
  "zone_id",
  "row_no",
  "column_no",
  "layer_no",
  "max_volume_cm3",
  "used_volume_cm3",
  "max_sku_count",
  "location_type",
  "bound_owner_id",
]) {
  assert.match(schema, new RegExp(`${field}[?:]`), `Location contract should expose ${field}`);
}

assert.match(glossary, /A01-01-02-03/, "glossary should use the full zone-row-column-layer location code");
assert.match(queriesSource, /useSystemDictionaryItemOptionsQuery/, "location UI should read dictionary options through a shared query hook");
assert.doesNotMatch(queriesSource, /function locationTypeLabel/, "location list should not hard-code location type labels");
assert.doesNotMatch(crudDialogSource, /const locationTypeOptions = \[/, "location CRUD should receive dictionary options");
assert.doesNotMatch(batchDialogSource, /const locationTypeOptions/, "location batch dialog should receive dictionary options");
const batchCreateSource = apiSource.slice(
  apiSource.indexOf("export async function batchCreateLocations"),
  apiSource.indexOf("export async function createProduct"),
);
assert.match(
  batchCreateSource,
  /listSystemDictionaryItemOptions/,
  "库位批量创建必须显式等待字典读取",
);
// 允许 await listSystemDictionaryItemOptions(...) 或 await Promise.all([listSystemDictionaryItemOptions(...), ...])
assert.match(
  batchCreateSource,
  /await (?:listSystemDictionaryItemOptions|Promise\.all\([\s\S]*listSystemDictionaryItemOptions)/,
  "库位批量创建必须以 await 等待字典读取完成",
);
const dictReadAt = batchCreateSource.search(/listSystemDictionaryItemOptions/);
const postAt = batchCreateSource.indexOf('api.POST("/api/v1/master-data/locations/batch-create"');
assert.ok(
  dictReadAt >= 0 && postAt >= 0 && dictReadAt < postAt,
  "库位批量创建必须先读取字典，再执行写入，避免写成功后因字典失败误报",
);
