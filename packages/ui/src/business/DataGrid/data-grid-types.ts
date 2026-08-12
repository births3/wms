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

/** 服务端分页受控模式配置：提供时页脚按服务端值展示，翻页/改每页条数回调服务端 */
export interface DataGridServerPagination {
  /** 服务端页码（0 基） */
  pageIndex: number;
  /** 服务端每页条数 */
  pageSize: number;
  /** 服务端记录总数 */
  total: number;
  /** 翻页回调（0 基页码，由服务端返回当前页数据） */
  onPageChange: (pageIndex: number) => void;
  /** 每页条数变更回调（可选；未提供时回退内部 onPageSizeChange） */
  onPageSizeChange?: (pageSize: number) => void;
}

export interface DataGridProps<T>
  extends Omit<DataTableProps<T>, "columns" | "data" | "footer">,
    Omit<React.HTMLAttributes<HTMLDivElement>, "onSelect"> {
  columns: DataGridColumn<T>[];
  data: T[];
  /** 默认显示序号列（行号，跨页连续）；传 false 关闭 */
  showRowNumber?: boolean;
  // maxHeight 继承自 DataTableProps：默认按视口测量（悬停表格滚轮滚动列表数据）；传值覆盖测量
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
  /** 服务端分页受控模式：提供时页脚按 serverPagination 的值展示（pageCount/rangeStart/rangeEnd 由 pageIndex/pageSize/total 计算），翻页与改每页条数回调服务端；data 需为当前页数据（DataGrid 不自行切页）。未提供时保持现有内存分页行为不变 */
  serverPagination?: DataGridServerPagination;
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
