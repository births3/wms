import type {
  DataGridColumnFilterValue,
  DataGridColumnFilters,
  DataGridRangeFilter,
} from "./data-grid-logic";

export const DATA_GRID_FILTER_HISTORY_MAX = 5;
const DATA_GRID_FILTER_HISTORY_PREFIX = "wms-datagrid-filter-history:";
const DATA_GRID_FILTER_HISTORY_DEFAULT_KEY = "default";

export interface DataGridFilterHistoryEntry {
  filters: DataGridColumnFilters;
  savedAt: string;
}

export interface DataGridFilterHistoryStorage {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
}

/**
 * 最近筛选历史存储键：storageKey 为空时退化为共享的 "default"；
 * 需要独立记录的页面必须传稳定 storageKey。
 */
export function dataGridFilterHistoryStorageKey(storageKey: string | undefined): string {
  return `${DATA_GRID_FILTER_HISTORY_PREFIX}${storageKey?.trim() || DATA_GRID_FILTER_HISTORY_DEFAULT_KEY}`;
}

/**
 * 记录一次筛选组合：与现有条目完全相同时仅更新 savedAt 并移到头部；
 * 新组合插入头部；超出上限截断尾部。
 */
export function recordDataGridFilterHistory(
  entries: readonly DataGridFilterHistoryEntry[],
  filters: DataGridColumnFilters,
  now: string,
): DataGridFilterHistoryEntry[] {
  const entry: DataGridFilterHistoryEntry = { filters: cloneColumnFilters(filters), savedAt: now };
  return [entry, ...entries.filter((existing) => !dataGridColumnFiltersEqual(existing.filters, filters))].slice(
    0,
    DATA_GRID_FILTER_HISTORY_MAX,
  );
}

export function dataGridColumnFiltersEqual(
  left: DataGridColumnFilters,
  right: DataGridColumnFilters,
): boolean {
  const leftKeys = Object.keys(left).sort();
  const rightKeys = Object.keys(right).sort();
  if (leftKeys.length !== rightKeys.length) return false;

  return leftKeys.every((key, index) => {
    if (key !== rightKeys[index]) return false;
    return JSON.stringify(left[key]) === JSON.stringify(right[key]);
  });
}

export function sanitizeDataGridFilterHistory(value: unknown): DataGridFilterHistoryEntry[] {
  if (!Array.isArray(value)) return [];

  const entries: DataGridFilterHistoryEntry[] = [];
  for (const item of value) {
    const entry = parseFilterHistoryEntry(item);
    if (!entry) continue;
    entries.push(entry);
  }
  return entries.slice(0, DATA_GRID_FILTER_HISTORY_MAX);
}

export function loadDataGridFilterHistoryFromStorage(
  storage: DataGridFilterHistoryStorage | null | undefined,
  storageKey: string | undefined,
): DataGridFilterHistoryEntry[] {
  if (!storage) return [];

  try {
    const raw = storage.getItem(dataGridFilterHistoryStorageKey(storageKey));
    return sanitizeDataGridFilterHistory(raw ? JSON.parse(raw) : []);
  } catch {
    return [];
  }
}

export function saveDataGridFilterHistoryToStorage(
  storage: DataGridFilterHistoryStorage | null | undefined,
  storageKey: string | undefined,
  entries: readonly DataGridFilterHistoryEntry[],
): void {
  if (!storage) return;

  try {
    storage.setItem(dataGridFilterHistoryStorageKey(storageKey), JSON.stringify(entries));
  } catch {
    // localStorage 可能被禁用；最近筛选仅保留在当前内存状态。
  }
}

export function getDataGridFilterHistoryStorage(): DataGridFilterHistoryStorage | null {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function parseFilterHistoryEntry(value: unknown): DataGridFilterHistoryEntry | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.savedAt !== "string" || Number.isNaN(Date.parse(record.savedAt))) return null;

  const filters = parseStoredColumnFilters(record.filters);
  if (!filters) return null;
  return { filters, savedAt: record.savedAt };
}

function parseStoredColumnFilters(value: unknown): DataGridColumnFilters | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;

  const filters: DataGridColumnFilters = {};
  for (const [key, filter] of Object.entries(value)) {
    const parsed = parseStoredColumnFilterValue(filter);
    if (parsed !== undefined && filterActive(parsed)) filters[key] = parsed;
  }
  return Object.keys(filters).length > 0 ? filters : null;
}

// 与 data-grid-logic.dataGridFilterActive 语义一致（本文件只做类型导入，保持 node 直跑测试）
function filterActive(value: DataGridColumnFilterValue | undefined): boolean {
  if (typeof value === "string") return value.trim().length > 0;
  if (Array.isArray(value)) return value.length > 0;
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  return Boolean(value.from?.trim() || value.to?.trim());
}

function parseStoredColumnFilterValue(value: unknown): DataGridColumnFilterValue | undefined {
  if (typeof value === "string") return value;

  if (Array.isArray(value)) {
    const items = value.filter(isString).map((item) => item.trim()).filter(Boolean);
    return items.length > 0 ? items : undefined;
  }

  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    const range: DataGridRangeFilter = {};
    if (typeof record.from === "string" && record.from.trim()) range.from = record.from.trim();
    if (typeof record.to === "string" && record.to.trim()) range.to = record.to.trim();
    return range.from || range.to ? range : undefined;
  }

  return undefined;
}

function cloneColumnFilters(filters: DataGridColumnFilters): DataGridColumnFilters {
  try {
    return JSON.parse(JSON.stringify(filters)) as DataGridColumnFilters;
  } catch {
    return { ...filters };
  }
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}
