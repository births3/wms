import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  dataGridNamedViewsStorageKey,
  loadDataGridNamedViewsFromStorage,
  pickDefaultDataGridNamedView,
  removeDataGridNamedView,
  renameDataGridNamedView,
  sanitizeDataGridNamedViews,
  saveDataGridNamedViewsToStorage,
  upsertDataGridNamedView,
} from "../src/business/DataGrid/data-grid-views.ts";

const columns = [
  { key: "code", sortable: true, minWidth: 120, maxWidth: 240 },
  { key: "status", sortable: true },
  { key: "qty", defaultHidden: true },
  { key: "action", hideable: false, copyable: false },
];

const options = {
  columns,
  pageSizeOptions: [10, 20],
  defaultPageSize: 20,
  now: "2026-06-30T12:00:00.000Z",
};
const laterOptions = { ...options, now: "2026-06-30T13:00:00.000Z" };

const created = upsertDataGridNamedView(
  [],
  {
    name: "  收货默认  ",
    state: {
      visibleColumns: ["code", "missing"],
      copyableColumns: ["code", "action"],
      columnWidths: { code: 260, missing: 180 },
      columnOrder: ["status", "code"],
      pageSize: 10,
      sort: { key: "status", direction: "asc" },
    },
  },
  options,
);

assert.equal(created.ok, true);
assert.equal(created.view.name, "收货默认");
assert.equal(created.view.createdAt, options.now);
assert.equal(created.view.updatedAt, options.now);
assert.deepEqual(created.view.state.visibleColumns, ["action", "code"]);
assert.deepEqual(created.view.state.columnWidths, { code: 240 });

const updated = upsertDataGridNamedView(
  created.views,
  {
    name: "收货默认",
    state: {
      visibleColumns: ["status"],
      pageSize: 999,
      sort: { key: "missing", direction: "desc" },
    },
  },
  laterOptions,
);

assert.equal(updated.ok, true);
assert.equal(updated.views.length, 1);
assert.equal(updated.view.createdAt, options.now);
assert.equal(updated.view.updatedAt, laterOptions.now);
assert.deepEqual(updated.view.state.visibleColumns, ["action", "status"]);
assert.equal(updated.view.state.pageSize, 20);
assert.equal(updated.view.state.sort, null);

assert.deepEqual(upsertDataGridNamedView(updated.views, { name: "   ", state: {} }, options), {
  ok: false,
  views: updated.views,
  error: "视图名称不能为空",
});

const renamed = renameDataGridNamedView(
  updated.views,
  "收货默认",
  "已收货视图",
  laterOptions.now,
);
assert.equal(renamed.ok, true);
assert.equal(renamed.view.name, "已收货视图");
assert.equal(renamed.view.createdAt, options.now);
assert.equal(renamed.view.updatedAt, laterOptions.now);

const duplicateRename = renameDataGridNamedView(
  [...renamed.views, { ...renamed.view, name: "另一个视图" }],
  "已收货视图",
  "另一个视图",
  laterOptions.now,
);
assert.equal(duplicateRename.ok, false);
assert.equal(duplicateRename.error, "视图名称已存在");

const badData = sanitizeDataGridNamedViews(
  [
    null,
    { name: "   ", state: {} },
    {
      name: "重复",
      state: { visibleColumns: ["code"], pageSize: 10 },
      createdAt: "bad",
      updatedAt: "bad",
    },
    { name: "重复", state: { visibleColumns: ["status"], pageSize: 10 } },
    { name: "坏状态", state: "nope" },
    { name: "x".repeat(80), state: {} },
  ],
  options,
);

assert.equal(badData.length, 3);
assert.deepEqual(
  badData.map((view) => view.name),
  ["重复", "坏状态", "x".repeat(40)],
);
assert.deepEqual(badData[0].state.visibleColumns, ["action", "code"]);
assert.deepEqual(badData[1].state.visibleColumns, ["action", "code", "status"]);
assert.equal(badData[0].createdAt, options.now);
assert.equal(badData[0].updatedAt, options.now);
assert.equal(pickDefaultDataGridNamedView(badData)?.name, "重复");
assert.equal(pickDefaultDataGridNamedView([]), null);

const removed = removeDataGridNamedView(badData, "重复");
assert.equal(removed.ok, true);
assert.deepEqual(
  removed.views.map((view) => view.name),
  ["坏状态", "x".repeat(40)],
);
assert.equal(removeDataGridNamedView(removed.views, "缺失").error, "视图不存在");

const fakeStorage = new Map();
const storage = {
  getItem(key) {
    return fakeStorage.has(key) ? fakeStorage.get(key) : null;
  },
  setItem(key, value) {
    fakeStorage.set(key, value);
  },
};

const saved = saveDataGridNamedViewsToStorage(storage, "m2.inbound", renamed.views);
assert.deepEqual(saved, { ok: true });
assert.equal(fakeStorage.has(dataGridNamedViewsStorageKey("m2.inbound")), true);
assert.deepEqual(loadDataGridNamedViewsFromStorage(storage, "m2.inbound", options), renamed.views);

fakeStorage.set(dataGridNamedViewsStorageKey("m2.inbound"), "{bad json");
assert.deepEqual(loadDataGridNamedViewsFromStorage(storage, "m2.inbound", options), []);

const dataGridSource = readFileSync(
  new URL("../src/business/DataGrid/DataGrid.tsx", import.meta.url),
  "utf8",
);
const namedViewsToolbarSource = readFileSync(
  new URL("../src/business/DataGrid/DataGridNamedViewsToolbar.tsx", import.meta.url),
  "utf8",
);

assert.match(dataGridSource, /import \{ DataGridNamedViewsToolbar \} from "\.\/DataGridNamedViewsToolbar";/);
assert.match(dataGridSource, /function applyNamedViewState\(state: DataGridLogicState\) \{[\s\S]*setSettings\(state\);[\s\S]*setPageIndex\(0\);[\s\S]*\}/);
assert.match(dataGridSource, /<DataGridNamedViewsToolbar[\s\S]*storageKey=\{storageKey\}[\s\S]*settings=\{settings\}[\s\S]*onApplyView=\{applyNamedViewState\}[\s\S]*\/>/);

for (const symbol of [
  "dataGridNamedViewsStorageKey",
  "loadDataGridNamedViewsFromStorage",
  "pickDefaultDataGridNamedView",
  "removeDataGridNamedView",
  "saveDataGridNamedViewsToStorage",
  "upsertDataGridNamedView",
]) {
  assert.match(namedViewsToolbarSource, new RegExp(`\\b${symbol}\\b`));
}

assert.match(namedViewsToolbarSource, /function getDataGridNamedViewStorage\(storageKey: string \| undefined\): Storage \| null/);
assert.match(namedViewsToolbarSource, /if \(!storageKey \|\| typeof window === "undefined"\) return null;/);
assert.match(namedViewsToolbarSource, /return window\.localStorage;/);
assert.match(namedViewsToolbarSource, /保存视图/);
assert.match(namedViewsToolbarSource, /应用视图/);
assert.match(namedViewsToolbarSource, /删除视图/);
assert.match(namedViewsToolbarSource, /state: settings/);
assert.match(namedViewsToolbarSource, /onApplyView\(selectedView\.state\)/);
assert.match(namedViewsToolbarSource, /saveDataGridNamedViewsToStorage\(storage, storageKey, result\.views\)/);
assert.match(namedViewsToolbarSource, /saveDataGridNamedViewsToStorage\(storage, storageKey, removed\.views\)/);
