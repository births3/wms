import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  nextDataGridNamedViewDraftName,
  upsertDataGridNamedView,
  type DataGridNamedView,
  type DataGridNamedViewOptions,
} from "../src/business/DataGrid/data-grid-views.ts";

const options: DataGridNamedViewOptions<{
  id: string;
  code: string;
  status: string;
}> = {
  columns: [{ key: "id", hideable: false }, { key: "code" }, { key: "status" }],
  pageSizeOptions: [10, 20],
  defaultPageSize: 10,
  now: "2026-07-01T10:00:00.000Z",
};

const first = upsertDataGridNamedView(
  [],
  { name: "收货全字段", state: { visibleColumns: ["id", "code", "status"] } },
  options,
);
if (!first.ok) throw new Error(first.error);

const second = upsertDataGridNamedView(
  first.views,
  { name: "收货精简", state: { visibleColumns: ["id", "code"] } },
  { ...options, now: "2026-07-01T10:01:00.000Z" },
);
if (!second.ok) throw new Error(second.error);

assert.deepEqual(
  second.views.map((view) => view.name),
  ["收货全字段", "收货精简"],
);
assert.deepEqual(second.views[0]?.state.visibleColumns, [
  "id",
  "code",
  "status",
]);
assert.equal(nextDataGridNamedViewDraftName(first.views, "收货精简"), "");

const updated = upsertDataGridNamedView(
  second.views,
  { name: "收货精简", state: { visibleColumns: ["id"] } },
  { ...options, now: "2026-07-01T10:02:00.000Z" },
);
if (!updated.ok) throw new Error(updated.error);

assert.equal(updated.views.length, 2);
assert.deepEqual(updated.views[1]?.state.visibleColumns, ["id"]);
assert.equal(
  nextDataGridNamedViewDraftName(
    second.views as DataGridNamedView[],
    "收货精简",
  ),
  "收货精简",
);

const toolbarSource = readFileSync(
  new URL(
    "../src/business/DataGrid/DataGridNamedViewsToolbar.tsx",
    import.meta.url,
  ),
  "utf8",
);
const dataGridSource = readFileSync(
  new URL("../src/business/DataGrid/DataGrid.tsx", import.meta.url),
  "utf8",
);
const dataGridToolbarSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridToolbar.tsx", import.meta.url),
  "utf8",
);
const headerCellSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridHeaderCell.tsx", import.meta.url),
  "utf8",
);
const namedViewSelectPattern =
  /<select[\s\S]*aria-label="选择命名视图"[\s\S]*onChange=\{\(event\) => setViewName\(event\.target\.value\)\}/;
const dismissHookSource = readFileSync(
  new URL(
    "../src/business/DataGrid/data-grid-popover-dismiss.ts",
    import.meta.url,
  ),
  "utf8",
);

assert.match(toolbarSource, namedViewSelectPattern);
assert.doesNotMatch(toolbarSource, /<datalist/);
assert.match(toolbarSource, /import \{ Bookmark \} from "lucide-react";/);
assert.match(toolbarSource, /import \{ createPortal \} from "react-dom";/);
assert.match(
  toolbarSource,
  /import \{ useDataGridPopoverDismiss \} from "\.\/data-grid-popover-dismiss";/,
);
assert.match(
  toolbarSource,
  /useDataGridPopoverDismiss\(\{[\s\S]*open,[\s\S]*onDismiss: \(\) => setOpen\(false\),[\s\S]*\}\);/,
);
assert.match(toolbarSource, /dataGridFloatingPanelPosition\(rect,[\s\S]*320/);
assert.match(toolbarSource, /createPortal\(/);
assert.match(toolbarSource, /className="fixed z-50 w-80/);
assert.doesNotMatch(toolbarSource, /className="absolute right-0 top-full/);
assert.match(
  dataGridSource,
  /import \{ useDataGridPopoverDismiss \} from "\.\/data-grid-popover-dismiss";/,
);
assert.match(
  dataGridSource,
  /useDataGridPopoverDismiss\(\{[\s\S]*open: fieldsOpen \|\| actionSettingsOpen \|\| openFilterKey !== null,[\s\S]*setFieldsOpen\(false\);[\s\S]*setActionSettingsOpen\(false\);[\s\S]*setOpenFilterKey\(null\);[\s\S]*\}\);/,
);
assert.match(dataGridSource, /<DataGridToolbar/);
assert.match(dataGridToolbarSource, /<DataGridNamedViewsToolbar/);
assert.match(
  dataGridToolbarSource,
  /<DataGridNamedViewsToolbar[\s\S]*aria-label="字段显示"/,
);
assert.doesNotMatch(headerCellSource, /namedViewsControl/);
assert.match(headerCellSource, /import \{ createPortal \} from "react-dom";/);
assert.match(headerCellSource, /className="fixed z-50 w-56/);
assert.match(headerCellSource, /createPortal\(/);
assert.doesNotMatch(headerCellSource, /absolute top-full z-30/);
assert.match(
  dismissHookSource,
  /document\.addEventListener\("pointerdown", dismissOnOutsidePointer\);/,
);
assert.match(
  dismissHookSource,
  /target\?\.closest\("\[data-datagrid-popover\]"\)/,
);
assert.match(
  dismissHookSource,
  /document\.addEventListener\("keydown", dismissOnEscape\);/,
);
assert.match(dismissHookSource, /event\.key !== "Escape"/);
