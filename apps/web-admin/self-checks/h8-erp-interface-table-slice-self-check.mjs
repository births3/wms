import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const root = new URL("..", import.meta.url);
const read = (file) => readFileSync(new URL(file, root), "utf8");
const page = read("src/pages/config-center/ErpInterfaceTablePage.tsx");
const connectorPage = read("src/pages/config-center/ErpConnectorConfigPage.tsx");
const queries = read("src/features/config-center/erp-interface-table-queries.ts");
const renderer = read("src/app-shell/AdminViewRenderer.tsx");
const app = read("src/App.tsx");
const menuMock = read("dev-mocks/admin-menu-dev-mock.ts");
const routeMock = read("dev-mocks/erp-interface-table-dev-mock.ts");
const permissionMigration = read("../../backend/migrations/202607210002_h8_erp_interface_table_permissions_menu.sql");
const latestMenuRepair = read("../../backend/migrations/202607210003_h8_erp_interface_table_menu_latest_version.sql");
const e2eApi = read("../../backend/crates/api/examples/wms_api_e2e.rs");
const e2eSeed = read("../../backend/crates/api/examples/support/wms_api_e2e_seed_data.rs");
const probeInit = read("../../deploy/h8-erp-if/init/00_probe_account.sql");
const readonlyCheck = read("../../scripts/h8_erp_interface_sync/check_probe_readonly.sh");
const queryGovernance = JSON.parse(read("src/pages/page-query-core-fields.json"));

for (const token of [
  "US-H8-004",
  "<QueryPanel",
  "<DataGrid",
  "h8ErpInterfaceTableQueryFields",
  "h8ErpInterfaceTableCoreQueryFieldKeys",
  "最近 7 天",
  "最大跨度 31 天",
  "无写操作",
  "payload_summary",
  "external_ref",
  "probe_credentials_configured",
  "disabled: !connector.probe_credentials_configured",
]) assert.match(page, new RegExp(token));
assert.doesNotMatch(page, /payload_json/);
assert.match(connectorPage, /接口表探查账号（只读）/);
assert.match(connectorPage, /expected_probe_config_version/);

assert.match(queries, /erp-interface-tables\/rows/);
assert.match(queries, /erp-interface-tables\/connectors/);
assert.match(queries, /api\.GET/);
assert.doesNotMatch(queries, /api\.(POST|PATCH|PUT|DELETE)/);
assert.match(renderer, /h8-erp-interface-tables/);
assert.match(app, /h8-erp-interface-tables/);
assert.match(menuMock, /h8\.erp_interface_table\.read/);
assert.match(routeMock, /仅支持 GET/);
assert.match(permissionMigration, /auth_role_permissions/);
assert.match(permissionMigration, /system_admin/);
assert.match(latestMenuRepair, /ORDER BY version_no DESC/);
assert.match(latestMenuRepair, /admin_menu_version_nodes/);
assert.match(e2eApi, /h8_erp_interface_tables::\{h8_erp_interface_table_router, H8ErpInterfaceTableAppState\}/);
assert.match(e2eApi, /\.merge\(h8_erp_interface_table_router\(/);
assert.match(e2eSeed, /h8\.erp_interface_table\.read/);
assert.match(probeInit, /USE wms_erp_if/);
assert.match(readonlyCheck, /-d wms_erp_if/);
assert.match(readonlyCheck, /DEMO-ASN-001/);
assert.match(readonlyCheck, /DEMO-PM-001/);
assert.ok(queryGovernance.pages.some((entry) => entry.id === "h8-erp-interface-tables" && entry.required === true));

console.log("H8 ERP interface table slice self-check passed");
