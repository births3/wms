import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  nextDataGridNamedViewDraftName,
  upsertDataGridNamedView,
  type DataGridNamedView,
  type DataGridNamedViewOptions,
} from "../src/business/DataGrid/data-grid-views.ts";

const options: DataGridNamedViewOptions<{ id: string; code: string; status: string }> = {
  columns: [
    { key: "id", hideable: false },
    { key: "code" },
    { key: "status" },
  ],
  pageSizeOptions: [10, 20],
  defaultPageSize: 10,
  now: "2026-07-01T10:00:00.000Z",
};

const first = upsertDataGridNamedView(
  [],
  { name: "收货全字段", state: { visibleColumns: ["id", "code", "status"] } },
  options,
);
assert.equal(first.ok, true);
if (!first.ok) throw new Error(first.error);

const second = upsertDataGridNamedView(
  first.views,
  { name: "收货精简", state: { visibleColumns: ["id", "code"] } },
  { ...options, now: "2026-07-01T10:01:00.000Z" },
);
assert.equal(second.ok, true);
if (!second.ok) throw new Error(second.error);

assert.deepEqual(
  second.views.map((view) => view.name),
  ["收货全字段", "收货精简"],
);
assert.deepEqual(second.views[0]?.state.visibleColumns, ["id", "code", "status"]);
assert.equal(nextDataGridNamedViewDraftName(first.views, "收货精简"), "");

const updated = upsertDataGridNamedView(
  second.views,
  { name: "收货精简", state: { visibleColumns: ["id"] } },
  { ...options, now: "2026-07-01T10:02:00.000Z" },
);
assert.equal(updated.ok, true);
if (!updated.ok) throw new Error(updated.error);

assert.equal(updated.views.length, 2);
assert.deepEqual(updated.views[1]?.state.visibleColumns, ["id"]);
assert.equal(nextDataGridNamedViewDraftName(second.views as DataGridNamedView[], "收货精简"), "收货精简");

const toolbarSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridNamedViewsToolbar.tsx", import.meta.url),
  "utf8",
);
const dataGridSource = readFileSync(
  new URL("../src/business/DataGrid/DataGrid.tsx", import.meta.url),
  "utf8",
);
const headerCellSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridHeaderCell.tsx", import.meta.url),
  "utf8",
);
const namedViewSelectPattern =
  /<select[\s\S]*aria-label="选择命名视图"[\s\S]*onChange=\{\(event\) => setViewName\(event\.target\.value\)\}/;

assert.match(toolbarSource, namedViewSelectPattern);
assert.doesNotMatch(toolbarSource, /<datalist/);
assert.match(toolbarSource, /import \{ Bookmark \} from "lucide-react";/);
assert.match(toolbarSource, /import \{ createPortal \} from "react-dom";/);
assert.match(toolbarSource, /dataGridFloatingPanelPosition\(rect,[\s\S]*320/);
assert.match(toolbarSource, /createPortal\(/);
assert.match(toolbarSource, /className="fixed z-50 w-80/);
assert.doesNotMatch(toolbarSource, /className="absolute right-0 top-full/);
assert.match(dataGridSource, /namedViewsControl=\{\s*<DataGridNamedViewsToolbar/);
assert.doesNotMatch(dataGridSource, /<div className="flex shrink-0 flex-wrap items-center justify-end gap-2 self-end md:ml-auto">\s*<DataGridNamedViewsToolbar/);
assert.match(headerCellSource, /namedViewsControl\?: React\.ReactNode;/);
assert.match(headerCellSource, /\{namedViewsControl\}[\s\S]*aria-label="字段设置"/);
assert.match(headerCellSource, /import \{ createPortal \} from "react-dom";/);
assert.match(headerCellSource, /className="fixed z-50 w-56/);
assert.match(headerCellSource, /createPortal\(/);
assert.doesNotMatch(headerCellSource, /absolute top-full z-30/);
