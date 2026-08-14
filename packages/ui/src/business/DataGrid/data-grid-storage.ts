import {
  sanitizeGridState,
  type DataGridLogicColumn,
  type DataGridLogicState,
  type DataGridSortState,
} from "./data-grid-logic";

export function loadGridSettings<T>(
  storageKey: string | undefined,
  columns: DataGridLogicColumn<T>[],
  pageSizeOptions: number[],
  defaultPageSize: number,
  actionKeys: string[] = [],
): DataGridLogicState {
  if (!storageKey || typeof window === "undefined") {
    return sanitizeGridState(null, columns, pageSizeOptions, defaultPageSize, actionKeys);
  }

  try {
    const raw = window.localStorage.getItem(storageKey);
    return sanitizeGridState(parseStoredGridSettings(raw ? JSON.parse(raw) : null), columns, pageSizeOptions, defaultPageSize, actionKeys);
  } catch {
    return sanitizeGridState(null, columns, pageSizeOptions, defaultPageSize, actionKeys);
  }
}

export function saveGridSettings(storageKey: string | undefined, settings: DataGridLogicState) {
  if (!storageKey || typeof window === "undefined") return;
  try {
    window.localStorage.setItem(storageKey, JSON.stringify(settings));
  } catch {
    // localStorage 可能被禁用；表格仍使用当前内存状态。
  }
}

function parseStoredGridSettings(value: unknown): Partial<DataGridLogicState> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  return {
    visibleColumns: Array.isArray(record.visibleColumns) ? record.visibleColumns.filter(isString) : undefined,
    copyableColumns: Array.isArray(record.copyableColumns) ? record.copyableColumns.filter(isString) : undefined,
    frozenColumns: Array.isArray(record.frozenColumns) ? record.frozenColumns.filter(isString) : undefined,
    hiddenActions: Array.isArray(record.hiddenActions) ? record.hiddenActions.filter(isString) : undefined,
    columnFilters: parseStoredColumnFilters(record.columnFilters),
    columnWidths: parseStoredColumnWidths(record.columnWidths),
    columnOrder: Array.isArray(record.columnOrder) ? record.columnOrder.filter(isString) : undefined,
    pageSize: typeof record.pageSize === "number" ? record.pageSize : undefined,
    sort: parseStoredSort(record.sort),
  };
}

function parseStoredColumnFilters(value: unknown): DataGridLogicState["columnFilters"] | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  const filters: DataGridLogicState["columnFilters"] = {};
  for (const [key, filter] of Object.entries(value)) {
    if (typeof filter === "string") {
      filters[key] = filter;
      continue;
    }

    if (Array.isArray(filter)) {
      filters[key] = filter.filter(isString);
      continue;
    }

    if (filter && typeof filter === "object") {
      const record = filter as Record<string, unknown>;
      const range: { from?: string; to?: string } = {};
      if (typeof record.from === "string") range.from = record.from;
      if (typeof record.to === "string") range.to = record.to;
      filters[key] = range;
    }
  }
  return filters;
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
