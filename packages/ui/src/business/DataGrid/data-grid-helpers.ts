import type * as React from "react";
import type { DataTableColumn } from "../DataTable";
import type { DataGridContextMenuPosition } from "./DataGridContextMenu";
import { getDataGridCopyText } from "./data-grid-logic";
import type {
  DataGridActionDescriptor,
  DataGridActionDisabled,
  DataGridAreaBounds,
  DataGridAreaSelection,
  DataGridColumn,
  DataGridCreateAction,
  DataGridDeleteAction,
  DataGridDetailAction,
  DataGridDisableAction,
  DataGridEditAction,
  DataGridExportAction,
  DataGridPasteDisabled,
  DataGridPasteTarget,
  DataGridPrintAction,
  DataGridQueryAction,
  DataGridRefreshAction,
  DataGridSelectedArea,
  DataGridToolbarAction,
  DataGridToolbarActionContext,
} from "./data-grid-types";
import type { DataGridSummaryTableColumn, DataGridSummaryTableRow } from "./data-grid-summary";

export function summaryDataTableColumns(
  columns: DataGridSummaryTableColumn[],
): DataTableColumn<DataGridSummaryTableRow>[] {
  return columns.map((column) => ({
    key: column.key,
    header: column.label,
    width: column.key === "__summaryRowCount" ? 100 : 160,
    align: column.key === "__summaryRowCount" || column.key.startsWith("summary:") ? "right" : "left",
    render: (row) => row[column.key],
  }));
}

export function columnLabel<T>(column: DataGridColumn<T>): string {
  return typeof column.header === "string" ? column.header : column.key;
}

export function defaultCellContent<T>(row: T, column: DataGridColumn<T>): React.ReactNode {
  if (!row || typeof row !== "object" || Array.isArray(row)) return null;
  return (row as Record<string, React.ReactNode>)[column.key] ?? null;
}

export function resolveDataGridExportBaseName(
  exportFileBaseName: string | undefined,
  caption: React.ReactNode,
  storageKey: string | undefined,
): string {
  if (exportFileBaseName?.trim()) return exportFileBaseName;
  if (typeof caption === "string" && caption.trim()) return caption;
  if (typeof document !== "undefined" && document.title.trim()) return document.title;
  return storageKey || "data-grid";
}

export function currentColumnWidth<T>(
  handle: HTMLElement,
  column: DataGridColumn<T>,
  savedWidth: number | undefined,
): number {
  if (typeof savedWidth === "number") return savedWidth;
  if (typeof column.width === "number") return column.width;
  return handle.closest("th")?.getBoundingClientRect().width ?? 160;
}

export function resolveDataGridActionDisabled(
  disabled: DataGridActionDisabled | undefined,
  context: DataGridToolbarActionContext,
  fallback = false,
): boolean {
  if (disabled === undefined) return fallback;
  return typeof disabled === "function" ? disabled(context) : disabled;
}

export function resolveDataGridPasteDisabled<T>(
  disabled: DataGridPasteDisabled<T> | undefined,
  context: DataGridPasteTarget<T>,
): boolean {
  if (disabled === undefined) return false;
  return typeof disabled === "function" ? disabled(context) : disabled;
}

export async function readClipboardText(): Promise<string> {
  if (!navigator.clipboard?.readText) return "";
  return navigator.clipboard.readText();
}

export async function writeClipboardText(value: string) {
  if (navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(value);
      return;
    } catch {
      // 兼容非安全上下文或 headless 环境，继续走浏览器原生回退。
    }
  }

  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();
  const copied = document.execCommand("copy");
  textarea.remove();
  if (!copied) throw new Error("copy failed");
}

export function isDataGridColumn<T>(column: DataGridColumn<T> | undefined): column is DataGridColumn<T> {
  return Boolean(column);
}

export function buildDataGridActionDescriptors<T>({
  refreshAction,
  queryAction,
  createAction,
  detailAction,
  editAction,
  deleteAction,
  disableAction,
  printAction,
  exportAction,
  toolbarActions,
  showPrintAction,
  showExportAction,
  csvExportPlacement,
  storageKey,
  hasHideableColumns,
}: {
  refreshAction?: DataGridRefreshAction;
  queryAction?: DataGridQueryAction;
  createAction?: DataGridCreateAction;
  detailAction?: DataGridDetailAction;
  editAction?: DataGridEditAction;
  deleteAction?: DataGridDeleteAction;
  disableAction?: DataGridDisableAction;
  printAction?: DataGridPrintAction | false;
  exportAction?: DataGridExportAction | false;
  toolbarActions: DataGridToolbarAction[];
  showPrintAction: boolean;
  showExportAction: boolean;
  csvExportPlacement: "toolbar" | "external";
  storageKey?: string;
  hasHideableColumns: boolean;
}): DataGridActionDescriptor[] {
  const actions: DataGridActionDescriptor[] = [];
  if (refreshAction) actions.push({ key: "refresh", label: refreshAction.label ?? "刷新", description: refreshAction.description ?? "刷新列表" });
  if (queryAction) actions.push({ key: "query", label: queryAction.label ?? "查询", description: queryAction.description ?? "查询列表" });
  if (createAction) actions.push({ key: "create", label: createAction.label ?? "新增", description: createAction.description ?? "新增记录" });
  if (detailAction) actions.push({ key: "detail", label: detailAction.label ?? "详情", description: detailAction.description ?? "查看详情" });
  if (editAction) actions.push({ key: "edit", label: editAction.label ?? "修改", description: editAction.description ?? "修改记录" });
  if (deleteAction) actions.push({ key: "delete", label: deleteAction.label ?? "删除", description: deleteAction.description ?? "删除记录" });
  if (disableAction) actions.push({ key: "disable", label: disableAction.label ?? "停用", description: disableAction.description ?? "停用记录" });
  if (storageKey) actions.push({ key: "view", label: "视图", description: "视图保存、应用、删除" });
  if (hasHideableColumns) actions.push({ key: "field", label: "字段", description: "字段显示" });
  actions.push({ key: "summary", label: "汇总", description: "汇总统计" });
  if (showPrintAction && printAction !== false) actions.push({ key: "print", label: printAction?.label ?? "打印", description: printAction?.description ?? "打印列表" });
  if (showExportAction && csvExportPlacement === "toolbar" && exportAction !== false) {
    actions.push({ key: "export", label: exportAction?.label ?? "导出", description: exportAction?.description ?? "导出 Excel" });
  }
  for (const action of toolbarActions) {
    actions.push({ key: toolbarActionKey(action.key), label: action.label, description: action.description });
  }
  return actions;
}

export function toolbarActionKey(key: string): string {
  return `toolbar:${key}`;
}

export function contextMenuPosition(x: number, y: number): DataGridContextMenuPosition {
  if (typeof window === "undefined") return { x, y };
  const menuWidth = 192;
  const menuHeight = 240;
  return {
    x: Math.min(x, Math.max(8, window.innerWidth - menuWidth - 8)),
    y: Math.min(y, Math.max(8, window.innerHeight - menuHeight - 8)),
  };
}

export function normalizedAreaBounds(selection: DataGridAreaSelection): DataGridAreaBounds {
  return {
    top: Math.min(selection.anchor.rowIndex, selection.focus.rowIndex),
    bottom: Math.max(selection.anchor.rowIndex, selection.focus.rowIndex),
    left: Math.min(selection.anchor.columnIndex, selection.focus.columnIndex),
    right: Math.max(selection.anchor.columnIndex, selection.focus.columnIndex),
  };
}

export function selectedAreaPayload<T>(
  bounds: DataGridAreaBounds | null,
  rows: T[],
  columns: DataGridColumn<T>[],
): DataGridSelectedArea<T> | null {
  if (!bounds) return null;
  return {
    ...bounds,
    rows: rows.slice(bounds.top, bounds.bottom + 1),
    columns: columns.slice(bounds.left, bounds.right + 1),
  };
}

export function buildDataGridClipboardText<T>(
  columns: DataGridColumn<T>[],
  rows: T[],
  includeHeader: boolean,
): string {
  const lines = rows.map((row) => columns.map((column) => getDataGridCopyText(row, column)).join("\t"));
  if (!includeHeader) return lines.join("\n");
  return [columns.map(columnLabel).join("\t"), ...lines].join("\n");
}

export function buildDataGridSelectedAreaSumText<T>(
  columns: DataGridColumn<T>[],
  rows: T[],
): string | null {
  const values = rows.flatMap((row) =>
    columns.flatMap((column) => {
      const value = dataGridAreaNumberValue(getDataGridCopyText(row, column));
      return value === null ? [] : [value];
    }),
  );
  if (values.length === 0) return null;

  const sum = values.reduce((total, value) => total + value, 0);
  return Number.isInteger(sum) ? String(sum) : sum.toFixed(2).replace(/\.?0+$/, "");
}

function dataGridAreaNumberValue(text: string): number | null {
  const normalized = text.replace(/,/g, "").trim();
  if (!normalized) return null;
  if (/^-?\d+(?:\.\d+)?$/.test(normalized)) return Number(normalized);

  const unitMatch = normalized.match(/^(-?\d+(?:\.\d+)?)\s*\D+$/);
  if (unitMatch) return Number(unitMatch[1]);

  const tokens = normalized.split(/\s+/);
  const lastToken = tokens.at(-1) ?? "";
  if (/^-?\d+(?:\.\d+)?$/.test(lastToken)) return Number(lastToken);

  const lastTokenUnitMatch = lastToken.match(/^(-?\d+(?:\.\d+)?)\D+$/);
  if (lastTokenUnitMatch) return Number(lastTokenUnitMatch[1]);

  const previousToken = tokens.at(-2) ?? "";
  return /^-?\d+(?:\.\d+)?$/.test(previousToken) ? Number(previousToken) : null;
}
