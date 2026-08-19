import assert from "node:assert/strict";
import fs from "node:fs";

const root = new URL("../", import.meta.url);
const read = (path) => fs.readFileSync(new URL(path, root), "utf8");

const page = read("src/pages/dock/DockManagementPage.tsx");
const mock = read("dev-mocks/dock-dev-mock.ts");
const queries = read("src/features/dock/dock-queries.ts");
const dialog = read("src/pages/dock/DockAppointmentChangeDialog.tsx");

assert.match(queries, /useUpdateDockAppointmentMutation/);
assert.match(queries, /PATCH\("\/api\/v1\/dock-appointments\/\{id\}"/);
assert.match(queries, /POST\("\/api\/v1\/dock-appointments\/\{id\}\/cancel"/);
assert.match(mock, /version: current\.version \+ 1/);
assert.match(mock, /supersedes_id: current\.id/);
assert.match(mock, /if \(appointment\.status !== "cancelled"\)/);
assert.match(page, /row\.status === "cancelled" \|\| row\.status === "arrived"/);
assert.match(page, /dock_id: changeForm\.dockId/);
assert.match(page, /reason: changeForm\.reason/);
assert.match(dialog, /变更原因/);
assert.match(dialog, /docks\.map/);
assert.match(mock, /DOCK_ALREADY_COMPLETED/);

console.log("h1-dock-appointment-change-self-check: passed");
