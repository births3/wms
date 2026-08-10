import type * as React from "react";
import type { DataTableColumn, DataTableProps } from "../DataTable";
import type { DataGridContextMenuPosition } from "./DataGridContextMenu";
import type { DataGridFilterConfig } from "./data-grid-logic";

export interface DataGridColumn<T> extends DataTableColumn<T> {
  sortable?: boolean;
  sortValue?: (row: T) => unknown;
  filterValue?: (row: T) => unknown;
  copyValue?: (row: T) => unknown;
  copyable?: boolean;
  onDoubleClick?: (row: T) => void;
  minWidth?: number;
  maxWidth?: number;
  resizable?: boolean;
  hideable?: boolean;
  defaultHidden?: boolean;
  filter?: DataGridFilterConfig | false;
}

export interface DataGridCsvExportState {
  disabled: boolean;
  exportCsv: () => void;
}

export type DataGridActionDisabled = boolean | ((context: DataGridToolbarActionContext) => boolean);

export interface DataGridRefreshAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridQueryAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridCreateAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridDetailAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridEditAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridDeleteAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridDisableAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridPrintAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick?: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridExportAction {
  label?: string;
  description?: string;
  disabled?: DataGridActionDisabled;
  onClick?: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridToolbarAction {
  key: string;
  label: string;
  description?: string;
  icon?: React.ReactNode;
  disabled?: DataGridActionDisabled;
  variant?: "default" | "destructive" | "outline" | "secondary" | "ghost" | "link";
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridToolbarActionContext {
  selectedRowKeys: string[];
}

export interface DataGridSelectedArea<T> {
  rows: T[];
  columns: DataGridColumn<T>[];
  top: number;
  bottom: number;
  left: number;
  right: number;
}

export interface DataGridPasteTarget<T> {
  row: T;
  rowIndex: number;
  column: DataGridColumn<T>;
  columnIndex: number;
  selectedRowKeys: string[];
  selectedArea: DataGridSelectedArea<T> | null;
}

export interface DataGridPasteContext<T> extends DataGridPasteTarget<T> {
  text: string;
  mode: "cell" | "column";
}

export type DataGridPasteDisabled<T> = boolean | ((context: DataGridPasteTarget<T>) => boolean);

export interface DataGridPasteAction<T> {
  label?: string;
  description?: string;
  disabled?: DataGridPasteDisabled<T>;
  onPaste: (context: DataGridPasteContext<T>) => void | Promise<void>;
}

export interface DataGridQuerySummaryItem {
  key: string;
  label: string;
  value: string;
  text: string;
}

export interface DataGridProps<T>
  extends Omit<DataTableProps<T>, "columns" | "data" | "footer">,
    Omit<React.HTMLAttributes<HTMLDivElement>, "onSelect"> {
  columns: DataGridColumn<T>[];
  data: T[];
  /** 表格区域最大高度（自建垂直滚动容器，表头/页脚/横向滚动条常驻）；默认 calc(100vh-15rem) */
  maxHeight?: string | number;
  storageKey?: string;
  pageSizeOptions?: number[];
  defaultPageSize?: number;
  selectable?: boolean;
  selectedRowKeys?: string[];
  onSelectedRowKeysChange?: (keys: string[]) => void;
  csvExportPlacement?: "toolbar" | "external";
  onCsvExportStateChange?: (state: DataGridCsvExportState | null) => void;
  exportFileBaseName?: string;
  refreshAction?: DataGridRefreshAction;
  queryAction?: DataGridQueryAction;
  queryState?: unknown;
  querySummaryItems?: DataGridQuerySummaryItem[];
  onApplyQueryState?: (queryState: unknown) => void;
  onClearQueryState?: () => void;
  createAction?: DataGridCreateAction;
  detailAction?: DataGridDetailAction;
  editAction?: DataGridEditAction;
  deleteAction?: DataGridDeleteAction;
  disableAction?: DataGridDisableAction;
  printAction?: DataGridPrintAction | false;
  exportAction?: DataGridExportAction | false;
  toolbarActions?: DataGridToolbarAction[];
  pasteAction?: DataGridPasteAction<T>;
  columnPasteAction?: DataGridPasteAction<T>;
  showPrintAction?: boolean;
  showExportAction?: boolean;
}

export const defaultPageSizeOptions = [10, 20, 50, 100];
export const defaultColumnWidth = 160;

export interface DataGridActionDescriptor {
  key: string;
  label: string;
  description?: string;
}

export interface DataGridCellPosition {
  rowIndex: number;
  columnIndex: number;
}

export interface DataGridAreaSelection {
  anchor: DataGridCellPosition;
  focus: DataGridCellPosition;
}

export interface DataGridAreaBounds {
  top: number;
  bottom: number;
  left: number;
  right: number;
}

export interface DataGridContextMenuState extends DataGridCellPosition, DataGridContextMenuPosition {}
