export interface DataGridCsvColumn<T> {
  key: string;
  header?: unknown;
  filterValue?: (row: T) => unknown;
  copyValue?: (row: T) => unknown;
}

export interface DataGridCsvExportOptions<T> {
  columns: DataGridCsvColumn<T>[];
  visibleColumnKeys: string[];
  rows: T[];
}

export interface DataGridCsvDownloadOptions {
  csv: string;
  fileName: string;
  document?: Document;
  url?: Pick<typeof URL, "createObjectURL" | "revokeObjectURL">;
}

export function buildDataGridCsv<T>({
  columns,
  visibleColumnKeys,
  rows,
}: DataGridCsvExportOptions<T>): string {
  const columnsByKey = new Map(columns.map((column) => [column.key, column]));
  const visibleColumns = visibleColumnKeys
    .map((key) => columnsByKey.get(key))
    .filter(isDataGridCsvColumn);
  const csvRows = [
    visibleColumns.map((column) => csvCell(columnHeader(column))),
    ...rows.map((row) => visibleColumns.map((column) => csvCell(dataGridCsvCellText(row, column)))),
  ];

  return csvRows.map((row) => row.join(",")).join("\n");
}

export function downloadDataGridCsv({
  csv,
  fileName,
  document: targetDocument,
  url = URL,
}: DataGridCsvDownloadOptions): boolean {
  if (!targetDocument || typeof Blob === "undefined") return false;

  const href = url.createObjectURL(new Blob([csv], { type: "text/csv;charset=utf-8" }));
  const link = targetDocument.createElement("a");
  link.href = href;
  link.download = fileName;
  try {
    targetDocument.body.appendChild(link);
    link.click();
  } finally {
    link.remove();
    url.revokeObjectURL(href);
  }
  return true;
}

function columnHeader<T>(column: DataGridCsvColumn<T>): string {
  return typeof column.header === "string" ? column.header : column.key;
}

function dataGridCsvCellText<T>(row: T, column: DataGridCsvColumn<T>): string {
  if (column.copyValue) return valueToText(column.copyValue(row)).trim();
  if (column.filterValue) return valueToText(column.filterValue(row)).trim();
  return valueToText(recordValue(row, column.key)).trim();
}

function csvCell(value: string): string {
  return /[",\r\n]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
}

function recordValue(value: unknown, key: string): unknown {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  return (value as Record<string, unknown>)[key];
}

function valueToText(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (value instanceof Date) return value.toISOString();
  if (Array.isArray(value)) return value.map(valueToText).join(" ");
  if (typeof value === "object") {
    return Object.values(value as Record<string, unknown>).map(valueToText).join(" ");
  }
  return String(value);
}

function isDataGridCsvColumn<T>(
  column: DataGridCsvColumn<T> | undefined,
): column is DataGridCsvColumn<T> {
  return Boolean(column);
}
