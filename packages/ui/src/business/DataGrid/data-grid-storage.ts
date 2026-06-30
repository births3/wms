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
): DataGridLogicState {
  if (!storageKey || typeof window === "undefined") {
    return sanitizeGridState(null, columns, pageSizeOptions, defaultPageSize);
  }

  try {
    const raw = window.localStorage.getItem(storageKey);
    return sanitizeGridState(parseStoredGridSettings(raw ? JSON.parse(raw) : null), columns, pageSizeOptions, defaultPageSize);
  } catch {
    return sanitizeGridState(null, columns, pageSizeOptions, defaultPageSize);
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
    columnWidths: parseStoredColumnWidths(record.columnWidths),
    columnOrder: Array.isArray(record.columnOrder) ? record.columnOrder.filter(isString) : undefined,
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
