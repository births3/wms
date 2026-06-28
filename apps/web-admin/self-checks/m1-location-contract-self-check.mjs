import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const schema = readFileSync(new URL("../../../packages/api-client/src/schema.ts", import.meta.url), "utf8");
const glossary = readFileSync(new URL("../../../docs/glossary.md", import.meta.url), "utf8");

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
