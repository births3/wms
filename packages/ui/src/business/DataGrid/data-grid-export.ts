export interface DataGridCsvColumn<T> {
  key: string;
  header?: unknown;
  filterValue?: (row: T) => unknown;
  copyValue?: (row: T) => unknown;
}

export type DataGridExportFormat = "xls" | "xlsx" | "csv";

export interface DataGridCsvExportOptions<T> {
  columns: DataGridCsvColumn<T>[];
  visibleColumnKeys: string[];
  rows: T[];
}

export interface DataGridExportOptions<T> extends DataGridCsvExportOptions<T> {
  format: DataGridExportFormat;
}

export interface DataGridExportPayload {
  content: string | Uint8Array;
  extension: DataGridExportFormat;
  mimeType: string;
}

export interface DataGridCsvDownloadOptions {
  csv: string;
  fileName: string;
  document?: Document;
  url?: Pick<typeof URL, "createObjectURL" | "revokeObjectURL">;
}

export interface DataGridExportDownloadOptions {
  content: string | Uint8Array;
  fileName: string;
  mimeType: string;
  document?: Document;
  url?: Pick<typeof URL, "createObjectURL" | "revokeObjectURL">;
}

export function buildDataGridCsv<T>({
  columns,
  visibleColumnKeys,
  rows,
}: DataGridCsvExportOptions<T>): string {
  const csvRows = dataGridExportRows({ columns, visibleColumnKeys, rows }).map((row) => row.map(csvCell));

  return csvRows.map((row) => row.join(",")).join("\n");
}

export function buildDataGridExport<T>(options: DataGridExportOptions<T>): DataGridExportPayload {
  if (options.format === "csv") {
    return {
      content: `\uFEFF${buildDataGridCsv(options)}`,
      extension: "csv",
      mimeType: "text/csv;charset=utf-8",
    };
  }

  const rows = dataGridExportRows(options);
  if (options.format === "xls") {
    return {
      content: buildDataGridHtmlTable(rows),
      extension: "xls",
      mimeType: "application/vnd.ms-excel;charset=utf-8",
    };
  }

  return {
    content: buildDataGridXlsx(rows),
    extension: "xlsx",
    mimeType: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  };
}

export function defaultDataGridExportFileName(menuName: string | null | undefined, now = new Date()): string {
  return `${sanitizeExportFileNameBase(menuName || "data-grid")}_${formatDataGridExportTimestamp(now)}`;
}

export function dataGridExportFileName(fileName: string, format: DataGridExportFormat): string {
  const base = sanitizeExportFileNameBase(fileName.replace(/\.(xls|xlsx|csv)$/i, ""));
  return `${base || "data-grid"}.${format}`;
}

export function downloadDataGridExport({
  content,
  fileName,
  mimeType,
  document: targetDocument,
  url = URL,
}: DataGridExportDownloadOptions): boolean {
  if (!targetDocument || typeof Blob === "undefined") return false;

  const href = url.createObjectURL(new Blob([content], { type: mimeType }));
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

export function downloadDataGridCsv({
  csv,
  fileName,
  document: targetDocument,
  url = URL,
}: DataGridCsvDownloadOptions): boolean {
  return downloadDataGridExport({ content: csv, fileName, mimeType: "text/csv;charset=utf-8", document: targetDocument, url });
}

function dataGridExportRows<T>({
  columns,
  visibleColumnKeys,
  rows,
}: DataGridCsvExportOptions<T>): string[][] {
  const columnsByKey = new Map(columns.map((column) => [column.key, column]));
  const visibleColumns = visibleColumnKeys.map((key) => columnsByKey.get(key)).filter(isDataGridCsvColumn);
  return [
    visibleColumns.map((column) => columnHeader(column)),
    ...rows.map((row) => visibleColumns.map((column) => dataGridCsvCellText(row, column))),
  ];
}

function buildDataGridHtmlTable(rows: string[][]): string {
  const tableRows = rows
    .map((row, index) => {
      const tag = index === 0 ? "th" : "td";
      return `<tr>${row.map((cell) => `<${tag}>${escapeHtml(cell)}</${tag}>`).join("")}</tr>`;
    })
    .join("");
  return `<!doctype html><html><head><meta charset="utf-8"></head><body><table>${tableRows}</table></body></html>`;
}

function buildDataGridXlsx(rows: string[][]): Uint8Array {
  const files = [
    {
      name: "[Content_Types].xml",
      content:
        '<?xml version="1.0" encoding="UTF-8"?>' +
        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">' +
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>' +
        '<Default Extension="xml" ContentType="application/xml"/>' +
        '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>' +
        '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>' +
        "</Types>",
    },
    {
      name: "_rels/.rels",
      content:
        '<?xml version="1.0" encoding="UTF-8"?>' +
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' +
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>' +
        "</Relationships>",
    },
    {
      name: "xl/workbook.xml",
      content:
        '<?xml version="1.0" encoding="UTF-8"?>' +
        '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">' +
        '<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>' +
        "</workbook>",
    },
    {
      name: "xl/_rels/workbook.xml.rels",
      content:
        '<?xml version="1.0" encoding="UTF-8"?>' +
        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' +
        '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>' +
        "</Relationships>",
    },
    {
      name: "xl/worksheets/sheet1.xml",
      content: buildDataGridXlsxSheet(rows),
    },
  ];
  return buildZip(files);
}

function buildDataGridXlsxSheet(rows: string[][]): string {
  const sheetRows = rows
    .map((row, rowIndex) => {
      const rowNumber = rowIndex + 1;
      const cells = row
        .map((cell, columnIndex) => {
          const reference = `${xlsxColumnName(columnIndex + 1)}${rowNumber}`;
          return `<c r="${reference}" t="inlineStr"><is><t xml:space="preserve">${escapeXml(cell)}</t></is></c>`;
        })
        .join("");
      return `<row r="${rowNumber}">${cells}</row>`;
    })
    .join("");
  const lastColumn = xlsxColumnName(Math.max(1, rows[0]?.length ?? 1));
  const lastRow = Math.max(1, rows.length);
  return (
    '<?xml version="1.0" encoding="UTF-8"?>' +
    '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">' +
    `<dimension ref="A1:${lastColumn}${lastRow}"/>` +
    `<sheetData>${sheetRows}</sheetData>` +
    "</worksheet>"
  );
}

function buildZip(files: Array<{ name: string; content: string }>): Uint8Array {
  const encoder = new TextEncoder();
  const entries = files.map((file) => {
    const name = encoder.encode(file.name);
    const content = encoder.encode(file.content);
    return { name, content, crc: crc32(content), offset: 0 };
  });
  let offset = 0;
  const localParts: Uint8Array[] = [];
  for (const entry of entries) {
    entry.offset = offset;
    const header = zipLocalFileHeader(entry.name, entry.content, entry.crc);
    localParts.push(header, entry.content);
    offset += header.length + entry.content.length;
  }

  const centralDirectoryOffset = offset;
  const centralParts = entries.map((entry) => zipCentralDirectoryHeader(entry.name, entry.content, entry.crc, entry.offset));
  const centralDirectorySize = centralParts.reduce((total, part) => total + part.length, 0);
  const end = zipEndOfCentralDirectory(entries.length, centralDirectorySize, centralDirectoryOffset);
  return concatBytes([...localParts, ...centralParts, end]);
}

function zipLocalFileHeader(name: Uint8Array, content: Uint8Array, crc: number): Uint8Array {
  const header = new Uint8Array(30 + name.length);
  const view = new DataView(header.buffer);
  view.setUint32(0, 0x04034b50, true);
  view.setUint16(4, 20, true);
  view.setUint16(6, 0, true);
  view.setUint16(8, 0, true);
  view.setUint16(10, 0, true);
  view.setUint16(12, 0, true);
  view.setUint32(14, crc, true);
  view.setUint32(18, content.length, true);
  view.setUint32(22, content.length, true);
  view.setUint16(26, name.length, true);
  view.setUint16(28, 0, true);
  header.set(name, 30);
  return header;
}

function zipCentralDirectoryHeader(name: Uint8Array, content: Uint8Array, crc: number, offset: number): Uint8Array {
  const header = new Uint8Array(46 + name.length);
  const view = new DataView(header.buffer);
  view.setUint32(0, 0x02014b50, true);
  view.setUint16(4, 20, true);
  view.setUint16(6, 20, true);
  view.setUint16(8, 0, true);
  view.setUint16(10, 0, true);
  view.setUint16(12, 0, true);
  view.setUint16(14, 0, true);
  view.setUint32(16, crc, true);
  view.setUint32(20, content.length, true);
  view.setUint32(24, content.length, true);
  view.setUint16(28, name.length, true);
  view.setUint16(30, 0, true);
  view.setUint16(32, 0, true);
  view.setUint16(34, 0, true);
  view.setUint16(36, 0, true);
  view.setUint32(38, 0, true);
  view.setUint32(42, offset, true);
  header.set(name, 46);
  return header;
}

function zipEndOfCentralDirectory(entries: number, centralDirectorySize: number, centralDirectoryOffset: number): Uint8Array {
  const header = new Uint8Array(22);
  const view = new DataView(header.buffer);
  view.setUint32(0, 0x06054b50, true);
  view.setUint16(4, 0, true);
  view.setUint16(6, 0, true);
  view.setUint16(8, entries, true);
  view.setUint16(10, entries, true);
  view.setUint32(12, centralDirectorySize, true);
  view.setUint32(16, centralDirectoryOffset, true);
  view.setUint16(20, 0, true);
  return header;
}

function concatBytes(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((sum, part) => sum + part.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}

function crc32(content: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of content) {
    crc = (crc >>> 8) ^ crc32Table[(crc ^ byte) & 0xff];
  }
  return (crc ^ 0xffffffff) >>> 0;
}

const crc32Table = Array.from({ length: 256 }, (_, index) => {
  let value = index;
  for (let bit = 0; bit < 8; bit += 1) {
    value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
  }
  return value >>> 0;
});

function xlsxColumnName(index: number): string {
  let value = index;
  let name = "";
  while (value > 0) {
    value -= 1;
    name = String.fromCharCode(65 + (value % 26)) + name;
    value = Math.floor(value / 26);
  }
  return name || "A";
}

function formatDataGridExportTimestamp(date: Date): string {
  return [
    String(date.getFullYear()).slice(-2),
    pad2(date.getMonth() + 1),
    pad2(date.getDate()),
    pad2(date.getHours()),
    pad2(date.getMinutes()),
  ].join("");
}

function sanitizeExportFileNameBase(value: string): string {
  const sanitized = value
    .trim()
    .replace(/[\\/:*?"<>|]/g, "")
    .replace(/\s+/g, "");
  return sanitized || "data-grid";
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

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function escapeXml(value: string): string {
  return escapeHtml(value);
}

function pad2(value: number): string {
  return String(value).padStart(2, "0");
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
