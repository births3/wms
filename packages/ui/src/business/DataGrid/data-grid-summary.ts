import type { DataGridLogicColumn } from "./data-grid-logic";

export type DataGridSummaryType = "sum" | "avg" | "max" | "min";

type DataGridSummaryColumn<T> = DataGridLogicColumn<T> & { header?: unknown };

export interface DataGridSummarySelection {
  columnKey: string;
  type: DataGridSummaryType;
}

export interface DataGridSummaryResult {
  columnKey: string;
  type: DataGridSummaryType;
  value: string;
  count: number;
}

export interface DataGridSummaryGroupResult {
  key: string;
  label: string;
  rowCount: number;
  results: DataGridSummaryResult[];
}

export interface DataGridSummaryTableColumn {
  key: string;
  label: string;
}

export interface DataGridSummaryTableRow {
  __summaryKey: string;
  [key: string]: string | number;
}

export interface DataGridSummaryTableResult {
  columns: DataGridSummaryTableColumn[];
  rows: DataGridSummaryTableRow[];
}

export function buildDataGridSummaryResults<T>(
  columns: DataGridSummaryColumn<T>[],
  rows: T[],
  selections: DataGridSummarySelection[],
): DataGridSummaryResult[] {
  const columnByKey = new Map(columns.map((column) => [column.key, column]));
  return selections.flatMap((selection) => {
    const column = columnByKey.get(selection.columnKey);
    if (!column) return [];

    const values = rows
      .map((row) => numberValue(summaryValue(row, column)))
      .filter((value): value is number => value !== null);
    const result = summarizeNumbers(values, selection.type);
    return [{
      columnKey: selection.columnKey,
      type: selection.type,
      value: result === null ? "-" : formatSummaryNumber(result),
      count: values.length,
    }];
  });
}

export function buildDataGridSummaryGroups<T>(
  columns: DataGridSummaryColumn<T>[],
  rows: T[],
  groupColumnKeys: string[],
  selections: DataGridSummarySelection[],
): DataGridSummaryGroupResult[] {
  const columnByKey = new Map(columns.map((column) => [column.key, column]));
  const groupColumns = groupColumnKeys.flatMap((key) => {
    const column = columnByKey.get(key);
    return column ? [column] : [];
  });

  if (groupColumns.length === 0) {
    return [{
      key: "all",
      label: "全部数据",
      rowCount: rows.length,
      results: buildDataGridSummaryResults(columns, rows, selections),
    }];
  }

  const groups = new Map<string, { label: string; rows: T[] }>();
  for (const row of rows) {
    const values = groupColumns.map((column) => textValue(summaryValue(row, column)));
    const key = values.join("\u001f");
    const label = groupColumns
      .map((column, index) => `${columnLabel(column)}：${values[index]}`)
      .join(" / ");
    const group = groups.get(key) ?? { label, rows: [] };
    group.rows.push(row);
    groups.set(key, group);
  }

  return Array.from(groups, ([key, group]) => ({
    key,
    label: group.label,
    rowCount: group.rows.length,
    results: buildDataGridSummaryResults(columns, group.rows, selections),
  }));
}

export function buildDataGridSummaryTable<T>(
  columns: DataGridSummaryColumn<T>[],
  rows: T[],
  groupColumnKeys: string[],
  selections: DataGridSummarySelection[],
): DataGridSummaryTableResult {
  const columnByKey = new Map(columns.map((column) => [column.key, column]));
  const groupColumns = groupColumnKeys.flatMap((key) => {
    const column = columnByKey.get(key);
    return column ? [column] : [];
  });
  const validSelections = selections.filter((selection) => columnByKey.has(selection.columnKey));
  const tableColumns: DataGridSummaryTableColumn[] = [
    ...(groupColumns.length > 0
      ? groupColumns.map((column) => ({ key: groupColumnTableKey(column.key), label: columnLabel(column) }))
      : [{ key: "__summaryGroup", label: "分组" }]),
    { key: "__summaryRowCount", label: "行数" },
    ...validSelections.map((selection) => ({
      key: summaryColumnTableKey(selection),
      label: `${columnLabel(columnByKey.get(selection.columnKey) ?? { key: selection.columnKey })} ${summaryTypeLabel(selection.type)}`,
    })),
  ];

  const groups = groupRows(rows, groupColumns);
  return {
    columns: tableColumns,
    rows: groups.map((group) => {
      const row: DataGridSummaryTableRow = {
        __summaryKey: group.key,
        __summaryRowCount: group.rows.length,
      };
      if (groupColumns.length === 0) row.__summaryGroup = "全部数据";
      groupColumns.forEach((column, index) => {
        row[groupColumnTableKey(column.key)] = group.values[index] ?? "空值";
      });
      for (const result of buildDataGridSummaryResults(columns, group.rows, validSelections)) {
        row[summaryColumnTableKey(result)] = result.value;
      }
      return row;
    }),
  };
}

function summarizeNumbers(values: number[], type: DataGridSummaryType): number | null {
  if (values.length === 0) return null;
  if (type === "max") return Math.max(...values);
  if (type === "min") return Math.min(...values);

  const sum = values.reduce((total, value) => total + value, 0);
  return type === "avg" ? sum / values.length : sum;
}

function summaryValue<T>(row: T, column: DataGridLogicColumn<T>): unknown {
  if (column.copyValue) return column.copyValue(row);
  if (column.filterValue) return column.filterValue(row);
  if (!row || typeof row !== "object" || Array.isArray(row)) return undefined;
  return (row as Record<string, unknown>)[column.key];
}

function columnLabel<T>(column: DataGridSummaryColumn<T>): string {
  return typeof column.header === "string" ? column.header : column.key;
}

function textValue(value: unknown): string {
  const text = String(value ?? "").trim();
  return text || "空值";
}

function groupRows<T>(
  rows: T[],
  groupColumns: DataGridSummaryColumn<T>[],
): Array<{ key: string; values: string[]; rows: T[] }> {
  if (groupColumns.length === 0) return [{ key: "all", values: ["全部数据"], rows }];

  const groups = new Map<string, { values: string[]; rows: T[] }>();
  for (const row of rows) {
    const values = groupColumns.map((column) => textValue(summaryValue(row, column)));
    const key = values.join("\u001f");
    const group = groups.get(key) ?? { values, rows: [] };
    group.rows.push(row);
    groups.set(key, group);
  }
  return Array.from(groups, ([key, group]) => ({ key, ...group }));
}

function numberValue(value: unknown): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  const parsed = Number.parseFloat(String(value ?? "").replace(/,/g, ""));
  return Number.isFinite(parsed) ? parsed : null;
}

function formatSummaryNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/\.?0+$/, "");
}

function groupColumnTableKey(key: string): string {
  return `group:${key}`;
}

function summaryColumnTableKey(selection: Pick<DataGridSummarySelection, "columnKey" | "type">): string {
  return `summary:${selection.columnKey}:${selection.type}`;
}

function summaryTypeLabel(type: DataGridSummaryType): string {
  const labels: Record<DataGridSummaryType, string> = {
    sum: "求和",
    avg: "平均",
    max: "最大",
    min: "最小",
  };
  return labels[type];
}
