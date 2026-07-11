import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(fileURLToPath(new URL("../../..", import.meta.url)));

const appSource = read("apps/web-admin/src/App.tsx");
const queriesSource = [
  read("apps/web-admin/src/features/master-data/master-data-queries/types.ts"),
  read("apps/web-admin/src/features/master-data/master-data-queries/queries.ts"),
  read("apps/web-admin/src/features/master-data/master-data-queries/api.ts"),
  read("apps/web-admin/src/features/master-data/master-data-queries/mappers.ts"),
].join("\n");
const pageSource = read("apps/web-admin/src/pages/master-data/M1MasterDataPage.tsx");
const dialogSource = read("apps/web-admin/src/pages/master-data/MasterDataCrudDialog.tsx");
const navigationCheckSource = read("scripts/governance/check_admin_navigation.py");

assert.match(appSource, /id: "m1-zones", title: "M1 库区管理"/);
assert.match(appSource, /view === "m1-zones"/);
assert.match(pageSource, /"m1-zones": \{\s*title: "M1 库区管理"/);

assert.match(queriesSource, /\|\s*"m1-zones"/);
assert.match(queriesSource, /case "m1-zones":\s*return listWarehouseZones\(\);/);
assert.match(queriesSource, /api\.GET\("\/api\/v1\/master-data\/warehouse-zones"\)/);
assert.match(queriesSource, /api\.POST\("\/api\/v1\/master-data\/warehouse-zones"/);
assert.match(queriesSource, /api\.PATCH\("\/api\/v1\/master-data\/warehouse-zones\/\{id\}"/);
assert.match(pageSource, /useSystemDictionaryItemOptionsQuery\("temperature_zone"/);
assert.match(pageSource, /useSystemDictionaryItemOptionsQuery\("quality_color"/);
assert.match(pageSource, /useMasterDataRowsQuery\("m1-zones", viewId === "m1-locations"\)/);
assert.match(dialogSource, /kind: "zone"/);
assert.match(dialogSource, /createWarehouseZone/);
assert.match(dialogSource, /updateWarehouseZone/);
assert.doesNotMatch(queriesSource, /warehouseZoneRowsFromLocations/);

assert.match(navigationCheckSource, /\("m1-zones", "M1 库区管理"\)/);

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}
