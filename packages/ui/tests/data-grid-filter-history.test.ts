import assert from "node:assert/strict";
import {
  DATA_GRID_FILTER_HISTORY_MAX,
  dataGridColumnFiltersEqual,
  dataGridFilterHistoryStorageKey,
  loadDataGridFilterHistoryFromStorage,
  recordDataGridFilterHistory,
  sanitizeDataGridFilterHistory,
  saveDataGridFilterHistoryToStorage,
  type DataGridFilterHistoryEntry,
  type DataGridFilterHistoryStorage,
} from "../src/business/DataGrid/data-grid-filter-history.ts";

function createMemoryStorage() {
  const store = new Map<string, string>();
  return {
    storage: {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => {
        store.set(key, value);
      },
    },
    store,
  } as { storage: DataGridFilterHistoryStorage; store: Map<string, string> };
}

const filtersA = { code: "ABC", status: ["已收货", "待收货"] };
const filtersB = { code: "XYZ" };
const filtersC = { code: "ABC", status: ["已收货"] };
const filtersD = { createdAt: { from: "2026-07-01", to: "2026-07-31" } };
const filtersE = { qty: { from: "10" } };

// 存储键：storageKey 为空退化到 "default"
assert.equal(dataGridFilterHistoryStorageKey("m2.inbound"), "wms-datagrid-filter-history:m2.inbound");
assert.equal(dataGridFilterHistoryStorageKey(undefined), "wms-datagrid-filter-history:default");
assert.equal(dataGridFilterHistoryStorageKey(""), "wms-datagrid-filter-history:default");
assert.equal(dataGridFilterHistoryStorageKey("  "), "wms-datagrid-filter-history:default");

// 记录：新组合插入头部
let entries = recordDataGridFilterHistory([], filtersA, "2026-07-01T10:00:00.000Z");
assert.equal(entries.length, 1);
assert.deepEqual(entries[0]?.filters, filtersA);

entries = recordDataGridFilterHistory(entries, filtersB, "2026-07-01T10:01:00.000Z");
assert.deepEqual(entries.map((entry) => entry.filters), [filtersB, filtersA]);

// 记录：相同组合仅更新 savedAt 并移到头部（去重）
entries = recordDataGridFilterHistory(entries, filtersA, "2026-07-01T10:02:00.000Z");
assert.equal(entries.length, 2);
assert.deepEqual(entries[0]?.filters, filtersA);
assert.equal(entries[0]?.savedAt, "2026-07-01T10:02:00.000Z");
assert.deepEqual(entries[1]?.filters, filtersB);

// 记录：超上限截断尾部，最多 5 条
entries = recordDataGridFilterHistory(
  [filtersA, filtersB, filtersC, filtersD, filtersE].map((filters, index) => ({
    filters,
    savedAt: `2026-07-01T10:${String(index).padStart(2, "0")}:00.000Z`,
  })),
  { code: "NEW" },
  "2026-07-01T11:00:00.000Z",
);
assert.equal(entries.length, DATA_GRID_FILTER_HISTORY_MAX);
assert.deepEqual(entries[0]?.filters, { code: "NEW" });
assert.deepEqual(entries[1]?.filters, filtersA);
assert.deepEqual(entries[4]?.filters, filtersD);
assert.equal(entries.some((entry) => entry.filters === filtersE), false);

// 相等性：值顺序不同的数组视为不同组合
assert.equal(dataGridColumnFiltersEqual({ status: ["a", "b"] }, { status: ["b", "a"] }), false);
assert.equal(dataGridColumnFiltersEqual({ code: "ABC" }, { code: "AB" }), false);
assert.equal(dataGridColumnFiltersEqual({ code: "ABC" }, {}), false);
assert.equal(
  dataGridColumnFiltersEqual(
    { code: "ABC", status: ["已收货"] },
    { status: ["已收货"], code: "ABC" },
  ),
  true,
);

// 保存/加载回环
const { storage, store } = createMemoryStorage();
saveDataGridFilterHistoryToStorage(storage, "m2.inbound", entries);
assert.ok(store.get("wms-datagrid-filter-history:m2.inbound"));
const loaded = loadDataGridFilterHistoryFromStorage(storage, "m2.inbound");
assert.deepEqual(loaded, entries);

// 加载校验：非数组 / 坏条目 / 空筛选组合被丢弃
assert.deepEqual(loadDataGridFilterHistoryFromStorage(storage, "other"), []);
const { storage: badStorage, store: badStore } = createMemoryStorage();
badStore.set("wms-datagrid-filter-history:default", JSON.stringify([
  { filters: { code: "OK" }, savedAt: "2026-07-01T10:00:00.000Z" },
  { filters: { code: "BAD" }, savedAt: "not-a-date" },
  { filters: {}, savedAt: "2026-07-02T10:00:00.000Z" },
  { filters: { code: "   " }, savedAt: "2026-07-02T10:00:00.000Z" },
  { filters: { code: "BAD2" }, savedAt: 123 },
  "garbage",
  null,
  { filters: { qty: { from: "", to: "" } }, savedAt: "2026-07-02T10:00:00.000Z" },
]));
const sanitized = loadDataGridFilterHistoryFromStorage(badStorage, undefined);
assert.equal(sanitized.length, 1);
assert.deepEqual(sanitized[0]?.filters, { code: "OK" });

// 坏 JSON 不抛错
badStore.set("wms-datagrid-filter-history:default", "{broken");
assert.deepEqual(loadDataGridFilterHistoryFromStorage(badStorage, undefined), []);

// 空 storage 直接返回空
assert.deepEqual(loadDataGridFilterHistoryFromStorage(null, "m2.inbound"), []);

// 类型引用（避免未使用告警）
const typed: DataGridFilterHistoryEntry = { filters: filtersA, savedAt: "2026-07-01T10:00:00.000Z" };
assert.deepEqual(typed.filters, filtersA);

console.log("data-grid-filter-history tests passed");
