import type { DataGridLogicColumn, DataGridLogicState, DataGridSortState } from "./data-grid-logic";

export const DATA_GRID_NAMED_VIEW_NAME_MAX_LENGTH = 40;
const DATA_GRID_NAMED_VIEW_STORAGE_SUFFIX = ":views";

export interface DataGridNamedView {
  name: string;
  state: DataGridLogicState;
  createdAt: string;
  updatedAt: string;
}

export interface DataGridNamedViewInput {
  name: string;
  state: Partial<DataGridLogicState> | null | undefined;
}

export interface DataGridNamedViewOptions<T> {
  columns: DataGridLogicColumn<T>[];
  pageSizeOptions?: number[];
  defaultPageSize?: number;
  now: string;
}

export interface DataGridNamedViewStorage {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
}

export type DataGridNamedViewMutationResult =
  | { ok: true; views: DataGridNamedView[]; view: DataGridNamedView }
  | { ok: false; views: DataGridNamedView[]; error: string };

export type DataGridNamedViewRemoveResult =
  | { ok: true; views: DataGridNamedView[]; removed: DataGridNamedView }
  | { ok: false; views: DataGridNamedView[]; error: string };

export type DataGridNamedViewStorageResult = { ok: true } | { ok: false; error: string };

export function dataGridNamedViewsStorageKey(storageKey: string): string {
  return `${storageKey}${DATA_GRID_NAMED_VIEW_STORAGE_SUFFIX}`;
}

export function sanitizeDataGridNamedViews<T>(
  value: unknown,
  options: DataGridNamedViewOptions<T>,
): DataGridNamedView[] {
  if (!Array.isArray(value)) return [];

  const names = new Set<string>();
  const views: DataGridNamedView[] = [];

  for (const item of value) {
    const view = parseStoredNamedView(item, options);
    if (!view || names.has(view.name)) continue;
    names.add(view.name);
    views.push(view);
  }

  return views;
}

export function upsertDataGridNamedView<T>(
  views: readonly DataGridNamedView[],
  input: DataGridNamedViewInput,
  options: DataGridNamedViewOptions<T>,
): DataGridNamedViewMutationResult {
  const name = validateViewName(input.name);
  if (!name.ok) return { ok: false, views: [...views], error: name.error };

  const index = views.findIndex((view) => view.name === name.value);
  const existing = index >= 0 ? views[index] : null;
  const view: DataGridNamedView = {
    name: name.value,
    state: sanitizeViewState(parseStoredGridSettings(input.state), options),
    createdAt: existing?.createdAt ?? options.now,
    updatedAt: options.now,
  };
  const next = [...views];
  if (index >= 0) next[index] = view;
  else next.push(view);

  return { ok: true, views: next, view };
}

export function nextDataGridNamedViewDraftName(
  previousViews: readonly DataGridNamedView[],
  savedName: string,
): string {
  return previousViews.some((view) => view.name === savedName) ? savedName : "";
}

export function renameDataGridNamedView(
  views: readonly DataGridNamedView[],
  currentName: string,
  nextName: string,
  now: string,
): DataGridNamedViewMutationResult {
  const current = normalizeStoredViewName(currentName);
  const next = validateViewName(nextName);
  if (!next.ok) return { ok: false, views: [...views], error: next.error };

  const index = views.findIndex((view) => view.name === current);
  if (index < 0) return { ok: false, views: [...views], error: "视图不存在" };

  const duplicateIndex = views.findIndex((view) => view.name === next.value);
  if (duplicateIndex >= 0 && duplicateIndex !== index) {
    return { ok: false, views: [...views], error: "视图名称已存在" };
  }

  const view = { ...views[index], name: next.value, updatedAt: now };
  const renamed = [...views];
  renamed[index] = view;
  return { ok: true, views: renamed, view };
}

export function removeDataGridNamedView(
  views: readonly DataGridNamedView[],
  name: string,
): DataGridNamedViewRemoveResult {
  const normalized = normalizeStoredViewName(name);
  const index = views.findIndex((view) => view.name === normalized);
  if (index < 0) return { ok: false, views: [...views], error: "视图不存在" };

  const removed = views[index];
  return { ok: true, views: views.filter((_, itemIndex) => itemIndex !== index), removed };
}

export function pickDefaultDataGridNamedView(
  views: readonly DataGridNamedView[],
): DataGridNamedView | null {
  return views[0] ?? null;
}

export function loadDataGridNamedViewsFromStorage<T>(
  storage: DataGridNamedViewStorage | null | undefined,
  storageKey: string | null | undefined,
  options: DataGridNamedViewOptions<T>,
): DataGridNamedView[] {
  if (!storage || !storageKey) return [];

  try {
    const raw = storage.getItem(dataGridNamedViewsStorageKey(storageKey));
    return sanitizeDataGridNamedViews(raw ? JSON.parse(raw) : [], options);
  } catch {
    return [];
  }
}

export function saveDataGridNamedViewsToStorage(
  storage: DataGridNamedViewStorage | null | undefined,
  storageKey: string | null | undefined,
  views: readonly DataGridNamedView[],
): DataGridNamedViewStorageResult {
  if (!storage || !storageKey) return { ok: false, error: "缺少视图存储键" };

  try {
    storage.setItem(dataGridNamedViewsStorageKey(storageKey), JSON.stringify(views));
    return { ok: true };
  } catch {
    return { ok: false, error: "视图保存失败" };
  }
}

function parseStoredNamedView<T>(
  value: unknown,
  options: DataGridNamedViewOptions<T>,
): DataGridNamedView | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.name !== "string") return null;

  const name = normalizeStoredViewName(record.name);
  if (!name) return null;

  return {
    name,
    state: sanitizeViewState(parseStoredGridSettings(record.state), options),
    createdAt: safeTimestamp(record.createdAt, options.now),
    updatedAt: safeTimestamp(record.updatedAt, options.now),
  };
}

function validateViewName(
  name: string,
): { ok: true; value: string } | { ok: false; error: string } {
  const value = name.trim();
  if (!value) return { ok: false, error: "视图名称不能为空" };
  if (Array.from(value).length > DATA_GRID_NAMED_VIEW_NAME_MAX_LENGTH) {
    return {
      ok: false,
      error: `视图名称不能超过 ${DATA_GRID_NAMED_VIEW_NAME_MAX_LENGTH} 个字符`,
    };
  }
  return { ok: true, value };
}

function normalizeStoredViewName(name: string): string {
  return Array.from(name.trim()).slice(0, DATA_GRID_NAMED_VIEW_NAME_MAX_LENGTH).join("");
}

function safeTimestamp(value: unknown, fallback: string): string {
  if (typeof value !== "string") return fallback;
  return Number.isNaN(Date.parse(value)) ? fallback : value;
}

function parseStoredGridSettings(value: unknown): Partial<DataGridLogicState> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  return {
    visibleColumns: Array.isArray(record.visibleColumns)
      ? record.visibleColumns.filter(isString)
      : undefined,
    copyableColumns: Array.isArray(record.copyableColumns)
      ? record.copyableColumns.filter(isString)
      : undefined,
    columnWidths: parseStoredColumnWidths(record.columnWidths),
    columnOrder: Array.isArray(record.columnOrder)
      ? record.columnOrder.filter(isString)
      : undefined,
    pageSize: typeof record.pageSize === "number" ? record.pageSize : undefined,
    sort: parseStoredSort(record.sort),
  };
}

function parseStoredColumnWidths(value: unknown): Record<string, number> | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  const widths: Record<string, number> = {};
  for (const [key, width] of Object.entries(value)) {
    if (typeof width === "number" && Number.isFinite(width)) widths[key] = width;
  }
  return widths;
}

function parseStoredSort(value: unknown): DataGridSortState | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.key !== "string") return null;
  if (record.direction !== "asc" && record.direction !== "desc") return null;
  return { key: record.key, direction: record.direction };
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

const defaultPageSizeFallback = 20;

function sanitizeViewState<T>(
  state: Partial<DataGridLogicState> | null | undefined,
  options: DataGridNamedViewOptions<T>,
): DataGridLogicState {
  const { columns } = options;
  const pageSizeOptions = options.pageSizeOptions ?? [10, 20, 50, 100];
  const defaultPageSize = options.defaultPageSize ?? defaultPageSizeFallback;
  const columnKeys = new Set(columns.map((column) => column.key));
  const requiredKeys = columns
    .filter((column) => column.hideable === false)
    .map((column) => column.key);
  const requestedVisible =
    state?.visibleColumns?.filter((key) => columnKeys.has(key)) ?? defaultVisibleColumns(columns);
  const visible = Array.from(new Set([...requiredKeys, ...requestedVisible])).filter((key) =>
    columnKeys.has(key),
  );
  const requestedOrder = state?.columnOrder?.filter((key) => columnKeys.has(key)) ?? [];
  const columnOrder = Array.from(
    new Set([...requestedOrder, ...defaultColumnOrder(columns)]),
  ).filter((key) => columnKeys.has(key));
  const copyableColumnKeys = new Set(defaultCopyableColumns(columns));
  const requestedCopyable =
    state?.copyableColumns?.filter((key) => copyableColumnKeys.has(key)) ??
    defaultCopyableColumns(columns);
  const copyableColumns = Array.from(new Set(requestedCopyable)).filter((key) =>
    copyableColumnKeys.has(key),
  );
  const safeDefaultPageSize = pageSizeOptions.includes(defaultPageSize)
    ? defaultPageSize
    : pageSizeOptions[0] ?? defaultPageSizeFallback;
  const pageSize = pageSizeOptions.includes(state?.pageSize ?? 0)
    ? state?.pageSize ?? safeDefaultPageSize
    : safeDefaultPageSize;
  const sortColumn = state?.sort
    ? columns.find((column) => column.key === state.sort?.key && column.sortable)
    : undefined;

  return {
    visibleColumns: visible.length > 0 ? visible : defaultVisibleColumns(columns),
    copyableColumns,
    columnWidths: sanitizeColumnWidths(state?.columnWidths, columns),
    columnOrder,
    pageSize,
    sort: sortColumn && state?.sort ? state.sort : null,
  };
}

function defaultVisibleColumns<T>(columns: DataGridLogicColumn<T>[]): string[] {
  const visible = columns.filter((column) => !column.defaultHidden).map((column) => column.key);
  return visible.length > 0 ? visible : columns.slice(0, 1).map((column) => column.key);
}

function defaultColumnOrder<T>(columns: DataGridLogicColumn<T>[]): string[] {
  return columns.map((column) => column.key);
}

function defaultCopyableColumns<T>(columns: DataGridLogicColumn<T>[]): string[] {
  return columns.filter((column) => column.copyable !== false).map((column) => column.key);
}

function sanitizeColumnWidths<T>(
  columnWidths: Record<string, number> | undefined,
  columns: DataGridLogicColumn<T>[],
): Record<string, number> {
  if (!columnWidths) return {};
  const columnByKey = new Map(columns.map((column) => [column.key, column]));
  const next: Record<string, number> = {};

  for (const [key, width] of Object.entries(columnWidths)) {
    const column = columnByKey.get(key);
    if (!column || !Number.isFinite(width)) continue;
    next[key] = clampColumnWidth(width, column);
  }

  return next;
}

function clampColumnWidth<T>(width: number, column: DataGridLogicColumn<T>): number {
  const minWidth = column.minWidth ?? 80;
  const maxWidth = column.maxWidth ?? 640;
  return Math.min(maxWidth, Math.max(minWidth, Math.round(width)));
}
