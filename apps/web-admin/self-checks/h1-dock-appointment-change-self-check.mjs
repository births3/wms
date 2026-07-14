import assert from "node:assert/strict";
import fs from "node:fs";

const page = fs.readFileSync("src/pages/dock/DockManagementPage.tsx", "utf8");
const mock = fs.readFileSync("dev-mocks/dock-dev-mock.ts", "utf8");
const queries = fs.readFileSync("src/features/dock/dock-queries.ts", "utf8");
const dialog = fs.readFileSync("src/pages/dock/DockAppointmentChangeDialog.tsx", "utf8");

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
