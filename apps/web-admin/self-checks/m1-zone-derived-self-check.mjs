import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(fileURLToPath(new URL("../../..", import.meta.url)));

const appSource = read("apps/web-admin/src/App.tsx");
const queriesSource = read("apps/web-admin/src/features/master-data/master-data-queries.ts");
const pageSource = read("apps/web-admin/src/pages/master-data/M1MasterDataPage.tsx");
const navigationCheckSource = read("scripts/governance/check_admin_navigation.py");

assert.match(appSource, /id: "m1-zones", title: "M1 库区管理"/);
assert.match(appSource, /view === "m1-zones"/);
assert.match(pageSource, /"m1-zones": \{\s*title: "M1 库区管理"/);

assert.match(queriesSource, /\|\s*"m1-zones"/);
assert.match(queriesSource, /case "m1-zones":\s*return listWarehouseZones\(\);/);
assert.match(queriesSource, /warehouseZoneRowsFromLocations\(await listLocations\(\)\)/);
assert.match(queriesSource, /statusLabel: "只读派生"/);
assert.doesNotMatch(queriesSource, /api\/v1\/master-data\/(?:zones|warehouse-zones)/);

assert.match(navigationCheckSource, /\("m1-zones", "M1 库区管理"\)/);

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}
