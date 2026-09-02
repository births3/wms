import type {
  DataGridAdvancedFilterState,
  DataGridFilterOperator,
} from "./data-grid-operators";

export type DataGridSortDirection = "asc" | "desc";

export interface DataGridSortState {
  key: string;
  direction: DataGridSortDirection;
}

export type DataGridFilterType = "text" | "select" | "multiSelect" | "dateRange" | "numberRange";

export interface DataGridFilterOption {
  label: string;
  value: string;
}

export interface DataGridFilterConfig {
  type: DataGridFilterType;
  options?: DataGridFilterOption[];
}

export interface DataGridRangeFilter {
  from?: string;
  to?: string;
}

export type DataGridColumnFilterValue = string | string[] | DataGridRangeFilter;
export type DataGridColumnFilters = Record<string, DataGridColumnFilterValue>;

export interface DataGridLogicColumn<T> {
  key: string;
  width?: string | number;
  minWidth?: number;
  maxWidth?: number;
  sortable?: boolean;
  sortValue?: (row: T) => unknown;
  filterValue?: (row: T) => unknown;
  copyValue?: (row: T) => unknown;
  copyable?: boolean;
  filter?: DataGridFilterConfig | false;
  hideable?: boolean;
  defaultHidden?: boolean;
}

export interface DataGridLogicState {
  visibleColumns: string[];
  copyableColumns: string[];
  frozenColumns: string[];
  hiddenActions: string[];
  columnFilters: DataGridColumnFilters;
  advancedFilters?: DataGridAdvancedFilterState;
  columnWidths: Record<string, number>;
  columnOrder: string[];
  pageSize: number;
  sort: DataGridSortState | null;
}

export interface DataGridPageResult<T> {
  rows: T[];
  filteredRows: T[];
  total: number;
  pageCount: number;
  pageIndex: number;
  rangeStart: number;
  rangeEnd: number;
}

export interface DataGridFloatingPanelRect {
  top: number;
  left: number;
  right: number;
}

export interface DataGridFloatingPanelViewport {
  width: number;
  height: number;
}

export interface DataGridFloatingPanelPosition {
  top: number;
  left: number;
  maxHeight: number;
}

const defaultPageSizeFallback = 20;
const resizeMinWidthFallback = 80;

export function defaultVisibleColumns<T>(columns: DataGridLogicColumn<T>[]): string[] {
  const visible = columns.filter((column) => !column.defaultHidden).map((column) => column.key);
  return visible.length > 0 ? visible : columns.slice(0, 1).map((column) => column.key);
}

export function defaultColumnOrder<T>(columns: DataGridLogicColumn<T>[]): string[] {
  return columns.map((column) => column.key);
}

export function defaultCopyableColumns<T>(columns: DataGridLogicColumn<T>[]): string[] {
  return columns.filter((column) => column.copyable !== false).map((column) => column.key);
}

export function sanitizeGridState<T>(
  state: Partial<DataGridLogicState> | null | undefined,
  columns: DataGridLogicColumn<T>[],
  pageSizeOptions: number[] = [10, 20, 50, 100],
  defaultPageSize = defaultPageSizeFallback,
  actionKeys: string[] = [],
): DataGridLogicState {
  const columnKeys = new Set(columns.map((column) => column.key));
  const requiredKeys = columns.filter((column) => column.hideable === false).map((column) => column.key);
  const requestedVisible = state?.visibleColumns?.filter((key) => columnKeys.has(key)) ?? defaultVisibleColumns(columns);
  const visible = Array.from(new Set([...requiredKeys, ...requestedVisible])).filter((key) => columnKeys.has(key));
  const requestedOrder = state?.columnOrder?.filter((key) => columnKeys.has(key)) ?? [];
  const columnOrder = Array.from(new Set([...requestedOrder, ...defaultColumnOrder(columns)])).filter((key) => columnKeys.has(key));
  const frozenColumns = Array.from(new Set(state?.frozenColumns?.filter((key) => columnKeys.has(key)) ?? []));
  const hiddenActions = sanitizeHiddenActions(state?.hiddenActions, actionKeys);
  const columnFilters = sanitizeDataGridColumnFiltersForColumns(state?.columnFilters, columns);
  const copyableColumnKeys = new Set(defaultCopyableColumns(columns));
  const requestedCopyable = state?.copyableColumns?.filter((key) => copyableColumnKeys.has(key)) ?? defaultCopyableColumns(columns);
  const copyableColumns = Array.from(new Set(requestedCopyable)).filter((key) => copyableColumnKeys.has(key));
  const columnWidths = sanitizeColumnWidths(state?.columnWidths, columns);
  const safeDefaultPageSize = pageSizeOptions.includes(defaultPageSize) ? defaultPageSize : pageSizeOptions[0] ?? defaultPageSizeFallback;
  const pageSize = pageSizeOptions.includes(state?.pageSize ?? 0) ? state?.pageSize ?? safeDefaultPageSize : safeDefaultPageSize;
  const sortColumn = state?.sort ? columns.find((column) => column.key === state.sort?.key && column.sortable) : undefined;

  return {
    visibleColumns: visible.length > 0 ? visible : defaultVisibleColumns(columns),
    copyableColumns,
    frozenColumns,
    hiddenActions,
    columnFilters,
    columnWidths,
    columnOrder,
    pageSize,
    sort: sortColumn && state?.sort ? state.sort : null,
  };
}

export function orderedColumnsWithFrozen<T>(
  columnOrder: string[],
  frozenColumns: string[],
  columns: DataGridLogicColumn<T>[],
): string[] {
  const columnKeys = new Set(columns.map((column) => column.key));
  const ordered = Array.from(new Set([...columnOrder, ...defaultColumnOrder(columns)])).filter((key) => columnKeys.has(key));
  const frozen = Array.from(new Set(frozenColumns)).filter((key) => columnKeys.has(key));
  const frozenSet = new Set(frozen);
  return [...frozen, ...ordered.filter((key) => !frozenSet.has(key))];
}

export function moveColumnBefore<T>(
  columnOrder: string[],
  columns: DataGridLogicColumn<T>[],
  key: string,
  beforeKey: string,
): string[] {
  const columnKeys = new Set(columns.map((column) => column.key));
  if (key === beforeKey || !columnKeys.has(key) || !columnKeys.has(beforeKey)) return columnOrder;

  const ordered = Array.from(new Set([...columnOrder, ...defaultColumnOrder(columns)])).filter((item) => columnKeys.has(item) && item !== key);
  const beforeIndex = ordered.indexOf(beforeKey);
  if (beforeIndex < 0) return columnOrder;
  ordered.splice(beforeIndex, 0, key);
  return ordered;
}

export function toggleVisibleColumn<T>(
  visibleColumns: string[],
  columns: DataGridLogicColumn<T>[],
  key: string,
  visible: boolean,
): string[] {
  const column = columns.find((item) => item.key === key);
  if (!column || column.hideable === false) return visibleColumns;

  const current = new Set(visibleColumns);
  if (visible) {
    current.add(key);
    return Array.from(current);
  }

  if (current.size <= 1) return visibleColumns;
  current.delete(key);
  return current.size > 0 ? Array.from(current) : visibleColumns;
}

export function toggleCopyableColumn<T>(
  copyableColumns: string[],
  columns: DataGridLogicColumn<T>[],
  key: string,
  copyable: boolean,
): string[] {
  const column = columns.find((item) => item.key === key && item.copyable !== false);
  if (!column) return copyableColumns;

  const current = new Set(copyableColumns);
  if (copyable) current.add(key);
  else current.delete(key);
  return Array.from(current);
}

export function toggleFrozenColumn<T>(
  frozenColumns: string[],
  columns: DataGridLogicColumn<T>[],
  key: string,
  frozen: boolean,
): string[] {
  const column = columns.find((item) => item.key === key);
  if (!column) return frozenColumns;

  const current = frozenColumns.filter((item, index, source) => source.indexOf(item) === index && columns.some((column) => column.key === item));
  if (frozen) return current.includes(key) ? current : [...current, key];
  return current.filter((item) => item !== key);
}

export function toggleHiddenAction(
  hiddenActions: string[],
  actionKeys: string[],
  key: string,
  visible: boolean,
): string[] {
  if (!actionKeys.includes(key)) return sanitizeHiddenActions(hiddenActions, actionKeys);

  const current = sanitizeHiddenActions(hiddenActions, actionKeys);
  if (visible) return current.filter((item) => item !== key);
  return current.includes(key) ? current : [...current, key];
}

export function setColumnWidth<T>(
  columnWidths: Record<string, number>,
  columns: DataGridLogicColumn<T>[],
  key: string,
  width: number | null,
): Record<string, number> {
  const column = columns.find((item) => item.key === key);
  if (!column) return columnWidths;

  const next = { ...columnWidths };
  if (width === null) {
    delete next[key];
    return next;
  }

  next[key] = clampColumnWidth(width, column);
  return next;
}

export function dataGridTableWidth<T>(columns: DataGridLogicColumn<T>[], fallbackWidth = 160): number | string {
  const parts = columns.map((column) => columnWidthPart(column.width, fallbackWidth));
  if (parts.every((part): part is number => typeof part === "number")) {
    return parts.reduce((total, width) => total + width, 0);
  }

  return `calc(${parts.map((part) => (typeof part === "number" ? `${part}px` : part)).join(" + ")})`;
}

export function dataGridFrozenColumnOffsets<T>(
  columns: DataGridLogicColumn<T>[],
  frozenKeys: Set<string>,
  fallbackWidth = 160,
): Record<string, number | string> {
  const offsets: Record<string, number | string> = {};
  const parts: Array<number | string> = [];

  for (const column of columns) {
    if (!frozenKeys.has(column.key)) continue;
    offsets[column.key] = dataGridWidthOffset(parts);
    parts.push(columnWidthPart(column.width, fallbackWidth));
  }

  return offsets;
}

export function reconcileDataGridSelectedRowKeys(selectedKeys: string[], availableKeys: string[]): string[] {
  const available = new Set(availableKeys);
  return selectedKeys.filter((key) => available.has(key));
}

export function dataGridFloatingPanelPosition(
  anchor: DataGridFloatingPanelRect,
  viewport: DataGridFloatingPanelViewport,
  panelWidth: number,
  minHeight = 160,
  gap = 8,
  margin = 8,
): DataGridFloatingPanelPosition {
  const maxLeft = Math.max(margin, viewport.width - panelWidth - margin);
  const preferredLeft = anchor.left - panelWidth - gap;
  const fallbackLeft = anchor.right + gap;
  const left = preferredLeft >= margin ? preferredLeft : Math.min(Math.max(margin, fallbackLeft), maxLeft);
  const top = Math.min(Math.max(margin, anchor.top), Math.max(margin, viewport.height - minHeight - margin));
  return {
    top,
    left,
    maxHeight: Math.max(minHeight, viewport.height - top - margin),
  };
}

export function nextSortState(current: DataGridSortState | null, key: string): DataGridSortState | null {
  if (!current || current.key !== key) return { key, direction: "asc" };
  if (current.direction === "asc") return { key, direction: "desc" };
  return null;
}

export function getDataGridPage<T>({
  data,
  columns,
  visibleColumns,
  columnFilters,
  advancedFilters,
  sort,
  pageIndex,
  pageSize,
}: {
  data: T[];
  columns: DataGridLogicColumn<T>[];
  visibleColumns: string[];
  columnFilters: DataGridColumnFilters;
  advancedFilters?: DataGridAdvancedFilterState;
  sort: DataGridSortState | null;
  pageIndex: number;
  pageSize: number;
}): DataGridPageResult<T> {
  const visibleColumnSet = new Set(visibleColumns);
  const searchableColumns = columns.filter((column) => visibleColumnSet.has(column.key));
  const filtered = filterRows(data, searchableColumns, columnFilters, advancedFilters);
  const sorted = sortRows(filtered, columns, sort);
  const total = sorted.length;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const safePageIndex = Math.min(Math.max(pageIndex, 0), pageCount - 1);
  const start = safePageIndex * pageSize;
  const rows = sorted.slice(start, start + pageSize);

  return {
    rows,
    filteredRows: sorted,
    total,
    pageCount,
    pageIndex: safePageIndex,
    rangeStart: total === 0 ? 0 : start + 1,
    rangeEnd: Math.min(start + rows.length, total),
  };
}

function filterRows<T>(
  data: T[],
  columns: DataGridLogicColumn<T>[],
  columnFilters: DataGridColumnFilters,
  advancedFilters?: DataGridAdvancedFilterState,
): T[] {
  const columnByKey = new Map(columns.map((column) => [column.key, column]));
  const activeFilters = Object.entries(columnFilters)
    .map(([key, value]) => ({ column: columnByKey.get(key), value }))
    .filter((filter): filter is { column: DataGridLogicColumn<T>; value: DataGridColumnFilterValue } =>
      Boolean(filter.column && dataGridFilterActive(filter.value)),
    );

  const hasAdvanced = advancedFilters && advancedFilters.items && advancedFilters.items.length > 0;

  if (activeFilters.length === 0 && !hasAdvanced) return data;

  return data.filter((row) => {
    const matchesColumn =
      activeFilters.length === 0 ||
      activeFilters.every(({ column, value }) => rowMatchesColumnFilter(row, column, value));
    const matchesAdvanced = !hasAdvanced || rowMatchesAdvancedFilters(row, columns, advancedFilters);
    return matchesColumn && matchesAdvanced;
  });
}

export function rowMatchesFilterOperator(
  rawValue: unknown,
  operator: DataGridFilterOperator,
  filterValue: unknown,
  filterType: DataGridFilterType | undefined,
): boolean {
  if (operator === "isEmpty") {
    if (rawValue === null || rawValue === undefined) return true;
    if (typeof rawValue === "string") return rawValue.trim().length === 0;
    if (Array.isArray(rawValue)) return rawValue.length === 0;
    return false;
  }

  if (operator === "isNotEmpty") {
    return !rowMatchesFilterOperator(rawValue, "isEmpty", undefined, filterType);
  }

  const type = filterType ?? "text";

  if (type === "numberRange" || typeof rawValue === "number") {
    const num = numberValue(rawValue);
    if (num === null) return false;

    if (operator === "between") {
      const range = (typeof filterValue === "object" && filterValue !== null ? filterValue : {}) as DataGridRangeFilter;
      return rangeIncludesNumber(num, range);
    }

    const target = typeof filterValue === "number" ? filterValue : Number(filterValue);
    if (Number.isNaN(target)) return false;

    switch (operator) {
      case "eq":
        return num === target;
      case "ne":
        return num !== target;
      case "gt":
        return num > target;
      case "gte":
        return num >= target;
      case "lt":
        return num < target;
      case "lte":
        return num <= target;
      default:
        return num === target;
    }
  }

  if (type === "dateRange") {
    const dateStr = dateOnly(rawValue);
    if (!dateStr) return false;

    if (operator === "between") {
      const range = (typeof filterValue === "object" && filterValue !== null ? filterValue : {}) as DataGridRangeFilter;
      return rangeIncludes(dateStr, range);
    }

    if (operator === "today") {
      const today = new Date().toISOString().slice(0, 10);
      return dateStr === today;
    }

    const targetDate = dateOnly(filterValue);
    if (!targetDate) return false;

    switch (operator) {
      case "eq":
        return dateStr === targetDate;
      case "ne":
        return dateStr !== targetDate;
      case "gt":
        return dateStr > targetDate;
      case "gte":
        return dateStr >= targetDate;
      case "lt":
        return dateStr < targetDate;
      case "lte":
        return dateStr <= targetDate;
      default:
        return dateStr === targetDate;
    }
  }

  if (type === "select" || type === "multiSelect") {
    const textVal = valueToText(rawValue);
    if (operator === "isAnyOf") {
      if (Array.isArray(filterValue)) return filterValue.includes(textVal);
      return textVal === String(filterValue);
    }
    if (operator === "isNoneOf") {
      if (Array.isArray(filterValue)) return !filterValue.includes(textVal);
      return textVal !== String(filterValue);
    }
    if (operator === "eq") {
      return textVal === String(filterValue);
    }
    if (operator === "ne") {
      return textVal !== String(filterValue);
    }
  }

  const text = valueToText(rawValue).toLowerCase();
  const target = valueToText(filterValue).trim().toLowerCase();

  switch (operator) {
    case "contains":
      return text.includes(target);
    case "notContains":
      return !text.includes(target);
    case "eq":
      return text === target;
    case "ne":
      return text !== target;
    case "startsWith":
      return text.startsWith(target);
    case "endsWith":
      return text.endsWith(target);
    default:
      return text.includes(target);
  }
}

export function rowMatchesAdvancedFilters<T>(
  row: T,
  columns: DataGridLogicColumn<T>[],
  filterState: DataGridAdvancedFilterState | undefined,
): boolean {
  if (!filterState || !filterState.items || filterState.items.length === 0) return true;
  const columnByKey = new Map(columns.map((col) => [col.key, col]));

  const validItems = filterState.items.filter((item) => {
    const col = columnByKey.get(item.columnKey);
    if (!col) return false;
    if (item.operator === "isEmpty" || item.operator === "isNotEmpty" || item.operator === "today" || item.operator === "thisWeek" || item.operator === "thisMonth") {
      return true;
    }
    return dataGridFilterActive(item.value as DataGridColumnFilterValue);
  });

  if (validItems.length === 0) return true;

  if (filterState.joinOperator === "or") {
    return validItems.some((item) => {
      const col = columnByKey.get(item.columnKey)!;
      const rawValue = columnFilterValue(row, col);
      const type = col.filter === false ? "text" : col.filter?.type ?? "text";
      return rowMatchesFilterOperator(rawValue, item.operator, item.value, type);
    });
  }

  return validItems.every((item) => {
    const col = columnByKey.get(item.columnKey)!;
    const rawValue = columnFilterValue(row, col);
    const type = col.filter === false ? "text" : col.filter?.type ?? "text";
    return rowMatchesFilterOperator(rawValue, item.operator, item.value, type);
  });
}

export function dataGridFilterActive(value: DataGridColumnFilterValue | undefined): boolean {
  if (typeof value === "string") return value.trim().length > 0;
  if (Array.isArray(value)) return value.length > 0;
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  return Boolean(value.from?.trim() || value.to?.trim());
}

export function dataGridFilterConfigForData<T>(
  column: DataGridLogicColumn<T>,
  data: T[],
): DataGridFilterConfig | false | undefined {
  const filter = column.filter;
  if (!filter || (filter.type !== "select" && filter.type !== "multiSelect")) return filter;

  const values = new Set(data.map((row) => valueToText(columnFilterValue(row, column))).filter(Boolean));
  const options = filter.options ?? Array.from(values).map((value) => ({ label: value, value }));
  return { ...filter, options: options.filter((option) => values.has(option.value)) };
}

export function sanitizeDataGridColumnFiltersForData<T>(
  filters: DataGridColumnFilters,
  columns: DataGridLogicColumn<T>[],
  data: T[],
): DataGridColumnFilters {
  let changed = false;
  const next: DataGridColumnFilters = {};
  const safeFilters = sanitizeDataGridColumnFiltersForColumns(filters, columns);

  for (const [key, value] of Object.entries(safeFilters)) {
    const column = columns.find((item) => item.key === key);
    const filter = column ? dataGridFilterConfigForData(column, data) : undefined;
    if (!column) {
      changed = true;
      continue;
    }

    if (!filter || (filter.type !== "select" && filter.type !== "multiSelect")) {
      next[key] = value;
      continue;
    }

    const validValues = new Set((filter.options ?? []).map((option) => option.value));
    if (filter.type === "select") {
      if (typeof value === "string" && validValues.has(value)) next[key] = value;
      else changed = true;
      continue;
    }

    if (Array.isArray(value)) {
      const validSelected = value.filter((item) => validValues.has(item));
      if (validSelected.length > 0) next[key] = validSelected;
      if (validSelected.length !== value.length) changed = true;
    } else {
      changed = true;
    }
  }

  return changed || safeFilters !== filters ? next : filters;
}

export function sanitizeDataGridColumnFiltersForColumns<T>(
  filters: DataGridColumnFilters | undefined,
  columns: DataGridLogicColumn<T>[],
): DataGridColumnFilters {
  if (!filters) return {};

  const columnByKey = new Map(columns.map((column) => [column.key, column]));
  const next: DataGridColumnFilters = {};

  for (const [key, value] of Object.entries(filters)) {
    const column = columnByKey.get(key);
    if (!column || column.hideable === false || column.filter === false) continue;

    const sanitized = sanitizeColumnFilterValue(value, column.filter);
    if (sanitized !== undefined && dataGridFilterActive(sanitized)) next[key] = sanitized;
  }

  return next;
}

function rowMatchesColumnFilter<T>(row: T, column: DataGridLogicColumn<T>, value: DataGridColumnFilterValue): boolean {
  const rawValue = columnFilterValue(row, column);
  const filter = column.filter === false ? { type: "text" as const } : column.filter ?? { type: "text" as const };

  if (filter.type === "select") {
    return typeof value === "string" && valueToText(rawValue) === value;
  }

  if (filter.type === "multiSelect") {
    return Array.isArray(value) && value.includes(valueToText(rawValue));
  }

  if (filter.type === "dateRange") {
    return rangeIncludes(dateOnly(rawValue), value);
  }

  if (filter.type === "numberRange") {
    return rangeIncludesNumber(numberValue(rawValue), value);
  }

  return typeof value === "string" && valueToText(rawValue).toLowerCase().includes(value.trim().toLowerCase());
}

function sortRows<T>(data: T[], columns: DataGridLogicColumn<T>[], sort: DataGridSortState | null): T[] {
  if (!sort) return data;
  const column = columns.find((item) => item.key === sort.key && item.sortable);
  if (!column) return data;
  const direction = sort.direction === "asc" ? 1 : -1;

  return data
    .map((row, index) => ({ row, index }))
    .sort((left, right) => {
      const compared = compareValues(sortValue(left.row, column), sortValue(right.row, column));
      return compared === 0 ? left.index - right.index : compared * direction;
    })
    .map((item) => item.row);
}

function sortValue<T>(row: T, column: DataGridLogicColumn<T>): unknown {
  if (column.sortValue) return column.sortValue(row);
  return recordValue(row, column.key);
}

function columnFilterValue<T>(row: T, column: DataGridLogicColumn<T>): unknown {
  if (column.filterValue) return column.filterValue(row);
  return recordValue(row, column.key);
}

function sanitizeColumnFilterValue(
  value: DataGridColumnFilterValue,
  filter: DataGridFilterConfig | false | undefined,
): DataGridColumnFilterValue | undefined {
  const type = filter === false ? "text" : filter?.type ?? "text";
  if (type === "multiSelect") {
    if (!Array.isArray(value)) return undefined;
    const selected = value.map((item) => item.trim()).filter(Boolean);
    return selected.length > 0 ? Array.from(new Set(selected)) : undefined;
  }

  if (type === "dateRange" || type === "numberRange") {
    if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
    const from = value.from?.trim();
    const to = value.to?.trim();
    const range: DataGridRangeFilter = {};
    if (from) range.from = from;
    if (to) range.to = to;
    return dataGridFilterActive(range) ? range : undefined;
  }

  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

export function getDataGridCopyText<T>(row: T, column: DataGridLogicColumn<T>): string {
  if (column.copyValue) return valueToText(column.copyValue(row)).trim();
  if (column.filterValue) return valueToText(column.filterValue(row)).trim();
  return valueToText(recordValue(row, column.key)).trim();
}

function recordValue(value: unknown, key: string): unknown {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  return (value as Record<string, unknown>)[key];
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
  // ponytail: minWidth 保留给初始布局；用户拖动统一允许压到紧凑下限。需要分列下限时再加 resizeMinWidth。
  const minWidth = Math.min(column.minWidth ?? resizeMinWidthFallback, resizeMinWidthFallback);
  const maxWidth = column.maxWidth ?? 640;
  return Math.min(maxWidth, Math.max(minWidth, Math.round(width)));
}

function sanitizeHiddenActions(hiddenActions: string[] | undefined, actionKeys: string[]): string[] {
  const available = new Set(actionKeys);
  return Array.from(new Set(hiddenActions?.filter((key) => available.has(key)) ?? []));
}

function valueToText(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (value instanceof Date) return value.toISOString();
  if (Array.isArray(value)) return value.map(valueToText).join(" ");
  if (typeof value === "object") return Object.values(value as Record<string, unknown>).map(valueToText).join(" ");
  return String(value);
}

function dateOnly(value: unknown): string | null {
  if (value instanceof Date) return value.toISOString().slice(0, 10);
  const text = valueToText(value);
  if (!text) return null;
  if (/^\d{4}-\d{2}-\d{2}/.test(text)) return text.slice(0, 10);
  const parsed = new Date(text);
  return Number.isNaN(parsed.getTime()) ? null : parsed.toISOString().slice(0, 10);
}

function numberValue(value: unknown): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  const parsed = Number.parseFloat(valueToText(value));
  return Number.isFinite(parsed) ? parsed : null;
}

function rangeIncludes(value: string | null, filter: DataGridColumnFilterValue): boolean {
  if (!isRangeFilter(filter) || !value) return false;
  const from = filter.from?.trim();
  const to = filter.to?.trim();
  return (!from || value >= from) && (!to || value <= to);
}

function rangeIncludesNumber(value: number | null, filter: DataGridColumnFilterValue): boolean {
  if (!isRangeFilter(filter) || value === null) return false;
  const from = numberValue(filter.from);
  const to = numberValue(filter.to);
  return (from === null || value >= from) && (to === null || value <= to);
}

function isRangeFilter(value: DataGridColumnFilterValue): value is DataGridRangeFilter {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function compareValues(left: unknown, right: unknown): number {
  const leftRank = valueRank(left);
  const rightRank = valueRank(right);
  if (leftRank !== rightRank) return leftRank - rightRank;
  if (leftRank === 1) return 0;

  if (typeof left === "number" && typeof right === "number") return left - right;
  if (left instanceof Date && right instanceof Date) return left.getTime() - right.getTime();
  return valueToText(left).localeCompare(valueToText(right), "zh-CN", { numeric: true });
}

function valueRank(value: unknown): number {
  return value === null || value === undefined || value === "" ? 1 : 0;
}

function columnWidthPart(width: string | number | undefined, fallbackWidth: number): string | number {
  if (typeof width === "number" && Number.isFinite(width)) return width;
  if (typeof width === "string" && width.trim()) return width.trim();
  return fallbackWidth;
}

function dataGridWidthOffset(parts: Array<number | string>): number | string {
  if (parts.length === 0) return 0;
  if (parts.every((part): part is number => typeof part === "number")) {
    return parts.reduce((total, width) => total + width, 0);
  }

  return `calc(${parts.map((part) => (typeof part === "number" ? `${part}px` : part)).join(" + ")})`;
}
