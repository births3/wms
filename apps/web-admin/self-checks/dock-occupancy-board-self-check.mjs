import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const queries = readFileSync(resolve(root, "src/features/dock/dock-queries.ts"), "utf8");
const component = readFileSync(resolve(root, "src/pages/dock/DockOccupancyBoard.tsx"), "utf8");
const page = readFileSync(resolve(root, "src/pages/dock/DockManagementPage.tsx"), "utf8");

assert.match(queries, /useDockAppointmentsQuery/);
assert.match(queries, /api\.GET\("\/api\/v1\/dock-appointments"/);
assert.match(queries, /warehouse_id:\s*warehouseId/);
assert.match(queries, /from:\s*window\.from/);
assert.match(queries, /to:\s*window\.to/);
assert.match(queries, /queryKey:\s*\[\.\.\.dockAppointmentQueryKey,\s*warehouseId\]/);
assert.ok((queries.match(/invalidateQueries\(\{ queryKey: dockAppointmentQueryKey \}/g) ?? []).length >= 3);

assert.match(component, /DockOccupancyBoard/);
assert.match(component, /DockAppointment/);
assert.match(component, /buildDockOccupancyModel/);
assert.match(component, /未来\s*24\s*小时/);
assert.match(component, /接口|真实|全量/);
assert.match(component, /暂无预约|请选择仓库/);
assert.match(component, /loading\??:\s*boolean/);
assert.match(component, /error\??:\s*string/);
assert.match(component, /role="alert"/);
assert.match(component, /预约总数|当前占用|排队/);
assert.match(component, /空闲|预约中|已到达|已取消|超时/);
assert.match(component, /StatusBadge/);
assert.match(component, /warehouseSelected/);
assert.doesNotMatch(component, /appointment_no\s*:/);
assert.doesNotMatch(component, /const\s+appointments\s*=\s*\[/);

assert.match(page, /<DockOccupancyBoard\b/);
assert.match(page, /useDockAppointmentsQuery\(warehouseId\)/);
assert.match(page, /appointments=\{appointmentsQuery\.data\s*\?\?\s*\[\]\}/);
assert.match(page, /docks=\{docksQuery\.data\s*\?\?\s*\[\]\}/);
assert.match(page, /loading=\{docksQuery\.isPending\s*\|\|\s*appointmentsQuery\.isPending\}/);
assert.match(page, /error=\{appointmentsQuery\.error\?\.message\}/);

console.log("dock-occupancy-board-self-check: passed");
