import * as React from "react";
import { Ban, Calculator, Download, Eye, ListChecks, Pencil, Plus, Printer, RefreshCw, Search, Settings2, Trash2, X } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Checkbox } from "../../ui/checkbox";
import { DataTable, type DataTableColumn, type DataTableProps } from "../DataTable";
import { DataGridActionSettingsPanel, type DataGridActionSettingItem } from "./DataGridActionSettingsPanel";
import { DataGridCellContent } from "./DataGridCellContent";
import { DataGridContextMenu, type DataGridContextMenuPosition } from "./DataGridContextMenu";
import { DataGridExportDialog } from "./DataGridExportDialog";
import { DataGridFieldSettingsPanel } from "./DataGridFieldSettingsPanel";
import { DataGridFilterChips } from "./DataGridFilterChips";
import { DataGridHeaderCell } from "./DataGridHeaderCell";
import { DataGridNamedViewsToolbar } from "./DataGridNamedViewsToolbar";
import { DataGridPaginationFooter } from "./DataGridPaginationFooter";
import { DataGridSummaryDialog, type DataGridSummaryConfig } from "./DataGridSummaryDialog";
import {
  buildDataGridCsv,
  buildDataGridExport,
  dataGridExportFileName,
  defaultDataGridExportFileName,
  downloadDataGridCsv,
  downloadDataGridExport,
  type DataGridExportFormat,
} from "./data-grid-export";
import { clearDataGridFilterKey } from "./data-grid-filter-summary";
import { useDataGridPopoverDismiss } from "./data-grid-popover-dismiss";
import {
  buildDataGridSummaryTable,
  type DataGridSummaryTableColumn,
  type DataGridSummaryTableRow,
} from "./data-grid-summary";
import {
  getDataGridCopyText,
  dataGridFloatingPanelPosition,
  dataGridFrozenColumnOffsets,
  dataGridTableWidth,
  dataGridFilterConfigForData,
  getDataGridPage,
  moveColumnBefore,
  nextSortState,
  orderedColumnsWithFrozen,
  reconcileDataGridSelectedRowKeys,
  sanitizeGridState,
  sanitizeDataGridColumnFiltersForData,
  setColumnWidth,
  toggleHiddenAction,
  toggleCopyableColumn,
  toggleFrozenColumn,
  toggleVisibleColumn,
  dataGridFilterActive,
  type DataGridColumnFilterValue,
  type DataGridColumnFilters,
  type DataGridFloatingPanelPosition,
  type DataGridFilterConfig,
  type DataGridLogicState,
} from "./data-grid-logic";
import { loadGridSettings, saveGridSettings } from "./data-grid-storage";

/**
 * DataGrid — 管理页薄数据网格（DataTable + 客户端分页/排序/列视图/字段筛选）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2 收货管理列表
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：只做客户端能力；视图偏好通过 storageKey 保存到 localStorage
 *
 * @example
 *   <DataGrid storageKey="m2.inbound" columns={columns} data={rows} rowKey={(row) => row.id} />
 */
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

const defaultPageSizeOptions = [10, 20, 50, 100];
const defaultColumnWidth = 160;

interface DataGridActionDescriptor {
  key: string;
  label: string;
  description?: string;
}

interface DataGridCellPosition {
  rowIndex: number;
  columnIndex: number;
}

interface DataGridAreaSelection {
  anchor: DataGridCellPosition;
  focus: DataGridCellPosition;
}

interface DataGridContextMenuState extends DataGridCellPosition, DataGridContextMenuPosition {}

function DataGridInner<T>(
  {
    columns,
    data,
    rowKey,
    selectedKey,
    onRowClick,
    caption,
    emptyTitle,
    emptyDescription,
    storageKey,
    pageSizeOptions = defaultPageSizeOptions,
    defaultPageSize = 20,
    selectable = false,
    selectedRowKeys,
    onSelectedRowKeysChange,
    csvExportPlacement = "toolbar",
    onCsvExportStateChange,
    exportFileBaseName,
    refreshAction,
    queryAction,
    queryState,
    querySummaryItems = [],
    onApplyQueryState,
    onClearQueryState,
    createAction,
    detailAction,
    editAction,
    deleteAction,
    disableAction,
    printAction,
    exportAction,
    toolbarActions = [],
    pasteAction,
    columnPasteAction,
    showPrintAction = true,
    showExportAction = true,
    className,
    tableClassName,
    ...rest
  }: DataGridProps<T>,
  ref: React.ForwardedRef<HTMLDivElement>,
) {
  const rootRef = React.useRef<HTMLDivElement | null>(null);
  const fieldButtonRef = React.useRef<HTMLButtonElement | null>(null);
  const actionSettingsButtonRef = React.useRef<HTMLButtonElement | null>(null);
  const columnsRef = React.useRef(columns);
  const fieldListId = React.useId();
  const actionSettingsPanelId = React.useId();
  const pageSizeSignature = pageSizeOptions.join("|");
  const columnSignature = columns
    .map(
      (column) =>
        `${column.key}:${column.hideable === false ? "fixed" : "hideable"}:${column.defaultHidden ? "0" : "1"}:${column.copyable === false ? "copy-off" : "copy-on"}`,
    )
    .join("|");
  const safePageSizeOptions = React.useMemo(() => {
    const values = pageSizeOptions.filter((value, index, source) => value > 0 && source.indexOf(value) === index);
    return values.length > 0 ? values : defaultPageSizeOptions;
  }, [pageSizeSignature]);
  const actionDescriptors = React.useMemo(
    () =>
      buildDataGridActionDescriptors({
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
        hasHideableColumns: columns.some((column) => column.hideable !== false),
      }),
    [
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
      columnSignature,
    ],
  );
  const actionKeys = React.useMemo(() => actionDescriptors.map((action) => action.key), [actionDescriptors]);
  const actionSignature = actionKeys.join("|");
  const [settings, setSettings] = React.useState<DataGridLogicState>(() =>
    loadGridSettings(storageKey, columns, safePageSizeOptions, defaultPageSize, actionKeys),
  );
  const [pageIndex, setPageIndex] = React.useState(0);
  const [fieldsOpen, setFieldsOpen] = React.useState(false);
  const [fieldsPanelPosition, setFieldsPanelPosition] = React.useState<DataGridFloatingPanelPosition | null>(null);
  const [actionSettingsOpen, setActionSettingsOpen] = React.useState(false);
  const [actionSettingsPanelPosition, setActionSettingsPanelPosition] = React.useState<DataGridFloatingPanelPosition | null>(null);
  const [summaryOpen, setSummaryOpen] = React.useState(false);
  const [summaryConfig, setSummaryConfig] = React.useState<DataGridSummaryConfig | null>(null);
  const [openFilterKey, setOpenFilterKey] = React.useState<string | null>(null);
  const [exportOpen, setExportOpen] = React.useState(false);
  const [exportFormat, setExportFormat] = React.useState<DataGridExportFormat>("xlsx");
  const [exportFileName, setExportFileName] = React.useState("");
  const [internalSelectedRowKeys, setInternalSelectedRowKeys] = React.useState<string[]>([]);
  const [areaSelectionEnabled, setAreaSelectionEnabled] = React.useState(false);
  const [areaSelection, setAreaSelection] = React.useState<DataGridAreaSelection | null>(null);
  const [areaSelecting, setAreaSelecting] = React.useState(false);
  const [contextMenu, setContextMenu] = React.useState<DataGridContextMenuState | null>(null);
  const [draggingColumnKey, setDraggingColumnKey] = React.useState<string | null>(null);
  const [resizingColumn, setResizingColumn] = React.useState<{ key: string; startX: number; startWidth: number } | null>(null);
  const [copyNotice, setCopyNotice] = React.useState<{ cellKey: string; text: string } | null>(null);

  React.useImperativeHandle(ref, () => rootRef.current as HTMLDivElement);

  React.useEffect(() => {
    columnsRef.current = columns;
  }, [columns]);

  React.useEffect(() => {
    setSettings((current) => sanitizeGridState(current, columns, safePageSizeOptions, defaultPageSize, actionKeys));
  }, [actionSignature, columnSignature, defaultPageSize, safePageSizeOptions]);

  React.useEffect(() => {
    saveGridSettings(storageKey, settings);
  }, [settings, storageKey]);

  React.useEffect(() => {
    setPageIndex(0);
  }, [settings.columnFilters, data, settings.pageSize, settings.sort?.key, settings.sort?.direction]);

  React.useEffect(() => {
    setSettings((current) => {
      const nextFilters = sanitizeDataGridColumnFiltersForData(current.columnFilters, columns, data);
      return nextFilters === current.columnFilters ? current : { ...current, columnFilters: nextFilters };
    });
  }, [columns, data]);

  React.useEffect(() => {
    if (!copyNotice) return;
    const timer = window.setTimeout(() => setCopyNotice(null), 2000);
    return () => window.clearTimeout(timer);
  }, [copyNotice]);

  React.useEffect(() => {
    if (!resizingColumn) return;
    const resize = resizingColumn;
    const previousCursor = document.body.style.cursor;
    const previousUserSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    function resizeColumn(event: PointerEvent) {
      const nextWidth = resize.startWidth + event.clientX - resize.startX;
      setSettings((current) => ({
        ...current,
        columnWidths: setColumnWidth(current.columnWidths, columnsRef.current, resize.key, nextWidth),
      }));
    }

    function stopResize() {
      setResizingColumn(null);
    }

    document.addEventListener("pointermove", resizeColumn);
    document.addEventListener("pointerup", stopResize, { once: true });
    return () => {
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousUserSelect;
      document.removeEventListener("pointermove", resizeColumn);
      document.removeEventListener("pointerup", stopResize);
    };
  }, [resizingColumn]);

  useDataGridPopoverDismiss({
    open: fieldsOpen || actionSettingsOpen || openFilterKey !== null,
    onDismiss: () => {
      setFieldsOpen(false);
      setActionSettingsOpen(false);
      setOpenFilterKey(null);
    },
  });

  React.useEffect(() => {
    if (!fieldsOpen) return;

    function updatePosition() {
      const rect = fieldButtonRef.current?.getBoundingClientRect();
      if (!rect) return;
      setFieldsPanelPosition(
        dataGridFloatingPanelPosition(rect, { width: window.innerWidth, height: window.innerHeight }, 320),
      );
    }

    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [fieldsOpen]);

  React.useEffect(() => {
    if (!actionSettingsOpen) return;

    function updatePosition() {
      const rect = actionSettingsButtonRef.current?.getBoundingClientRect();
      if (!rect) return;
      setActionSettingsPanelPosition(
        dataGridFloatingPanelPosition(rect, { width: window.innerWidth, height: window.innerHeight }, 288),
      );
    }

    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [actionSettingsOpen]);

  React.useEffect(() => {
    if (!areaSelecting) return;
    const stopSelecting = () => setAreaSelecting(false);
    window.addEventListener("mouseup", stopSelecting, { once: true });
    return () => window.removeEventListener("mouseup", stopSelecting);
  }, [areaSelecting]);

  const page = getDataGridPage({
    data,
    columns,
    visibleColumns: settings.visibleColumns,
    columnFilters: settings.columnFilters,
    sort: settings.sort,
    pageIndex,
    pageSize: settings.pageSize,
  });
  const columnFilters = settings.columnFilters;
  const visibleKeys = new Set(settings.visibleColumns);
  const copyableKeys = new Set(settings.copyableColumns);
  const frozenKeys = new Set(settings.frozenColumns);
  const hiddenActionKeys = new Set(settings.hiddenActions);
  const columnsByKey = new Map(columns.map((column) => [column.key, column]));
  const orderedColumnKeys = orderedColumnsWithFrozen(settings.columnOrder, settings.frozenColumns, columns);
  const orderedColumns = orderedColumnKeys.map((key) => columnsByKey.get(key)).filter(isDataGridColumn);
  const orderedHideableColumns = orderedColumns.filter((column) => column.hideable !== false);
  const fixedColumns = orderedColumns.filter((column) => column.hideable === false);
  const visibleColumns = [...orderedHideableColumns.filter((column) => visibleKeys.has(column.key)), ...fixedColumns];
  const frozenColumnOffsets = dataGridFrozenColumnOffsets(visibleColumns, frozenKeys, defaultColumnWidth);
  const summaryTable = summaryConfig
    ? buildDataGridSummaryTable(visibleColumns, page.filteredRows, summaryConfig.groupColumnKeys, summaryConfig.selections)
    : null;
  const summaryColumns = summaryTable ? summaryDataTableColumns(summaryTable.columns) : [];
  const csvExportSnapshotRef = React.useRef<{
    columns: DataGridColumn<T>[];
    visibleColumnKeys: string[];
    rows: T[];
    storageKey: string | undefined;
  } | null>(null);
  csvExportSnapshotRef.current = {
    columns,
    visibleColumnKeys: visibleColumns.map((column) => column.key),
    rows: page.filteredRows,
    storageKey,
  };
  const hideableColumns = orderedHideableColumns;
  const visibleHideableCount = hideableColumns.filter((column) => visibleKeys.has(column.key)).length;
  const selectedKeys = selectedRowKeys ?? internalSelectedRowKeys;
  const selectedKeySet = new Set(selectedKeys);
  const toolbarActionContext = React.useMemo(() => ({ selectedRowKeys: selectedKeys }), [selectedKeys]);
  const pageRowKeys = page.rows.map(rowKey);
  const filteredRowKeys = React.useMemo(() => page.filteredRows.map(rowKey), [page.filteredRows, rowKey]);
  const selectedPageCount = pageRowKeys.filter((key) => selectedKeySet.has(key)).length;
  const allPageSelected = pageRowKeys.length > 0 && selectedPageCount === pageRowKeys.length;
  const filterSummaryFields = visibleColumns.map((column) => ({
    key: column.key,
    label: columnLabel(column),
    filter: dataGridFilterConfigForData(column, data),
  }));
  const visibleAction = React.useCallback((key: string) => !hiddenActionKeys.has(key), [settings.hiddenActions]);
  const hasHiddenToolbarActions = actionDescriptors.some((action) => hiddenActionKeys.has(action.key));
  const actionSettingItems: DataGridActionSettingItem[] = actionDescriptors.map((action) => ({
    ...action,
    visible: visibleAction(action.key),
  }));
  const selectedAreaBounds = areaSelection ? normalizedAreaBounds(areaSelection) : null;
  const selectedAreaSumText = selectedAreaBounds
    ? buildDataGridSelectedAreaSumText(
        visibleColumns.slice(selectedAreaBounds.left, selectedAreaBounds.right + 1),
        page.rows.slice(selectedAreaBounds.top, selectedAreaBounds.bottom + 1),
      )
    : null;

  React.useEffect(() => {
    if (!selectable || selectedKeys.length === 0) return;
    const nextKeys = reconcileDataGridSelectedRowKeys(selectedKeys, filteredRowKeys);
    const changed = nextKeys.length !== selectedKeys.length || nextKeys.some((key, index) => key !== selectedKeys[index]);
    if (changed) setSelectedKeys(nextKeys);
  }, [selectable, selectedKeys, filteredRowKeys]);

  function updateSort(column: DataGridColumn<T>) {
    if (!column.sortable) return;
    setSettings((current) => ({ ...current, sort: nextSortState(current.sort, column.key) }));
  }

  function updateColumnVisible(key: string, visible: boolean) {
    setSettings((current) => {
      const nextVisible = toggleVisibleColumn(current.visibleColumns, columns, key, visible);
      const sortStillVisible = !current.sort || nextVisible.includes(current.sort.key);
      return { ...current, visibleColumns: nextVisible, sort: sortStillVisible ? current.sort : null };
    });
  }

  function updateColumnCopyable(key: string, copyable: boolean) {
    setSettings((current) => ({
      ...current,
      copyableColumns: toggleCopyableColumn(current.copyableColumns, columns, key, copyable),
    }));
  }

  function updateColumnFrozen(key: string, frozen: boolean) {
    setSettings((current) => ({
      ...current,
      frozenColumns: toggleFrozenColumn(current.frozenColumns, columns, key, frozen),
    }));
  }

  function updateActionVisible(key: string, visible: boolean) {
    setSettings((current) => ({
      ...current,
      hiddenActions: toggleHiddenAction(current.hiddenActions, actionKeys, key, visible),
    }));
  }

  function moveColumn(key: string, beforeKey: string) {
    setSettings((current) => ({ ...current, columnOrder: moveColumnBefore(current.columnOrder, columns, key, beforeKey) }));
  }

  function moveColumnByStep(key: string, step: -1 | 1) {
    const index = hideableColumns.findIndex((column) => column.key === key);
    const target = hideableColumns[index + step];
    if (!target) return;

    if (step < 0) {
      moveColumn(key, target.key);
      return;
    }

    const afterTarget = hideableColumns[index + 2];
    setSettings((current) => {
      const moved = moveColumnBefore(current.columnOrder, columns, key, target.key);
      const columnOrder = afterTarget ? moveColumnBefore(moved, columns, key, afterTarget.key) : [...moved.filter((item) => item !== key), key];
      return { ...current, columnOrder };
    });
  }

  function resetColumnWidth(key: string) {
    setSettings((current) => ({ ...current, columnWidths: setColumnWidth(current.columnWidths, columns, key, null) }));
  }

  function startColumnResize(handle: HTMLElement, column: DataGridColumn<T>, clientX: number) {
    setResizingColumn({
      key: column.key,
      startX: clientX,
      startWidth: currentColumnWidth(handle, column, settings.columnWidths[column.key]),
    });
  }

  function nudgeColumnWidth(handle: HTMLElement, column: DataGridColumn<T>, delta: number) {
    const nextWidth = currentColumnWidth(handle, column, settings.columnWidths[column.key]) + delta;
    setSettings((current) => ({
      ...current,
      columnWidths: setColumnWidth(current.columnWidths, columns, column.key, nextWidth),
    }));
  }

  function updateColumnFilterValue(key: string, value: DataGridColumnFilterValue) {
    setSettings((current) => {
      const nextFilters = { ...current.columnFilters };
      if (dataGridFilterActive(value)) nextFilters[key] = value;
      else delete nextFilters[key];
      return { ...current, columnFilters: nextFilters };
    });
  }

  function setSelectedKeys(keys: string[]) {
    if (!selectedRowKeys) setInternalSelectedRowKeys(keys);
    onSelectedRowKeysChange?.(keys);
  }

  function updateRowSelected(key: string, selected: boolean) {
    const next = new Set(selectedKeys);
    if (selected) next.add(key);
    else next.delete(key);
    setSelectedKeys(Array.from(next));
  }

  function updatePageSelected(selected: boolean) {
    const next = new Set(selectedKeys);
    for (const key of pageRowKeys) {
      if (selected) next.add(key);
      else next.delete(key);
    }
    setSelectedKeys(Array.from(next));
  }

  async function copyCellValue(row: T, column: DataGridColumn<T>, cellKey: string) {
    const text = getDataGridCopyText(row, column);
    if (!text) return;

    try {
      await writeClipboardText(text);
      setCopyNotice({ cellKey, text: "已复制" });
    } catch {
      setCopyNotice({ cellKey, text: "复制失败" });
    }
  }

  const exportCsv = React.useCallback(() => {
    const snapshot = csvExportSnapshotRef.current;
    if (!snapshot) return;

    const csv = buildDataGridCsv({
      columns: snapshot.columns,
      visibleColumnKeys: snapshot.visibleColumnKeys,
      rows: snapshot.rows,
    });

    downloadDataGridCsv({
      csv,
      fileName: snapshot.storageKey ? `${snapshot.storageKey}.csv` : "data-grid.csv",
      document: typeof document === "undefined" ? undefined : document,
    });
  }, []);

  React.useEffect(() => {
    if (csvExportPlacement !== "external" || !onCsvExportStateChange) return;
    onCsvExportStateChange({ disabled: page.filteredRows.length === 0, exportCsv });
    return () => onCsvExportStateChange(null);
  }, [csvExportPlacement, exportCsv, onCsvExportStateChange, page.filteredRows.length]);

  function applyNamedViewState(state: DataGridLogicState, nextQueryState?: unknown) {
    setSettings(state);
    if (nextQueryState !== undefined) onApplyQueryState?.(nextQueryState);
    setPageIndex(0);
  }

  function openExportDialog() {
    setExportFileName(defaultDataGridExportFileName(resolveDataGridExportBaseName(exportFileBaseName, caption, storageKey)));
    setExportFormat("xlsx");
    setExportOpen(true);
  }

  function confirmExport() {
    const snapshot = csvExportSnapshotRef.current;
    if (!snapshot) return;

    const payload = buildDataGridExport({
      format: exportFormat,
      columns: snapshot.columns,
      visibleColumnKeys: snapshot.visibleColumnKeys,
      rows: snapshot.rows,
    });
    downloadDataGridExport({
      content: payload.content,
      mimeType: payload.mimeType,
      fileName: dataGridExportFileName(exportFileName, payload.extension),
      document: typeof document === "undefined" ? undefined : document,
    });
    setExportOpen(false);
  }

  function openContextMenu(event: React.MouseEvent, rowIndex: number, columnIndex: number) {
    event.preventDefault();
    setContextMenu({ ...contextMenuPosition(event.clientX, event.clientY), rowIndex, columnIndex });
  }

  function startCellAreaSelection(event: React.MouseEvent, rowIndex: number, columnIndex: number) {
    if (!areaSelectionEnabled || event.button !== 0) return;
    event.preventDefault();
    setAreaSelection({ anchor: { rowIndex, columnIndex }, focus: { rowIndex, columnIndex } });
    setAreaSelecting(true);
  }

  function updateCellAreaSelection(rowIndex: number, columnIndex: number) {
    if (!areaSelectionEnabled || !areaSelecting) return;
    setAreaSelection((current) =>
      current ? { ...current, focus: { rowIndex, columnIndex } } : current,
    );
  }

  function copyContextRow(includeHeader: boolean) {
    if (!contextMenu) return;
    const row = page.rows[contextMenu.rowIndex];
    if (!row) return;

    void writeClipboardText(buildDataGridClipboardText(visibleColumns, [row], includeHeader));
    setContextMenu(null);
  }

  function startAreaSelectionFromContext() {
    setAreaSelectionEnabled(true);
    if (contextMenu) {
      const cell = { rowIndex: contextMenu.rowIndex, columnIndex: contextMenu.columnIndex };
      setAreaSelection({ anchor: cell, focus: cell });
    }
    setContextMenu(null);
  }

  function closeAreaSelectionFromContext() {
    setAreaSelectionEnabled(false);
    setAreaSelection(null);
    setAreaSelecting(false);
    setContextMenu(null);
  }

  function copySelectedArea(includeHeader: boolean) {
    if (!selectedAreaBounds) return;
    const selectedColumns = visibleColumns.slice(selectedAreaBounds.left, selectedAreaBounds.right + 1);
    const selectedRows = page.rows.slice(selectedAreaBounds.top, selectedAreaBounds.bottom + 1);
    if (selectedColumns.length === 0 || selectedRows.length === 0) return;

    void writeClipboardText(buildDataGridClipboardText(selectedColumns, selectedRows, includeHeader));
    setContextMenu(null);
  }

  function copySelectedAreaSum() {
    if (!selectedAreaSumText) return;
    void writeClipboardText(selectedAreaSumText);
    setContextMenu(null);
  }

  async function pasteFromContext(mode: "cell" | "column") {
    const action = mode === "column" ? columnPasteAction : pasteAction;
    const target = buildPasteTarget();
    if (!action || !target || resolveDataGridPasteDisabled(action.disabled, target)) return;

    const text = await readClipboardText();
    await action.onPaste({ ...target, text, mode });
    setContextMenu(null);
  }

  function buildPasteTarget(): DataGridPasteTarget<T> | null {
    if (!contextMenu) return null;
    const row = page.rows[contextMenu.rowIndex];
    const column = visibleColumns[contextMenu.columnIndex];
    if (!row || !column) return null;
    return {
      row,
      rowIndex: contextMenu.rowIndex,
      column,
      columnIndex: contextMenu.columnIndex,
      selectedRowKeys: selectedKeys,
      selectedArea: selectedAreaPayload(selectedAreaBounds, page.rows, visibleColumns),
    };
  }

  const contextPasteTarget = buildPasteTarget();
  const canPaste = Boolean(pasteAction && contextPasteTarget && !resolveDataGridPasteDisabled(pasteAction.disabled, contextPasteTarget));
  const canColumnPaste = Boolean(
    columnPasteAction &&
      contextPasteTarget &&
      !resolveDataGridPasteDisabled(columnPasteAction.disabled, contextPasteTarget),
  );

  const tableColumns: DataTableColumn<T>[] = visibleColumns.map((column, columnIndex) => {
    const sourceRender = column.render;
    const columnCanCopy = column.copyable !== false && copyableKeys.has(column.key);
    const columnWidth = settings.columnWidths[column.key] ?? column.width ?? defaultColumnWidth;
    const frozenLeft = frozenColumnOffsets[column.key];
    const frozen = frozenLeft !== undefined;
    const frozenBaseClassName = "sticky shadow-[1px_0_0_hsl(var(--border))]";

    return {
      ...column,
      width: columnWidth,
      headerProps: frozen
        ? {
            className: cn("z-30 bg-muted/40", frozenBaseClassName),
            style: { left: frozenLeft },
            "data-grid-frozen-column": "true",
          }
        : undefined,
      cellProps: (_row, rowIndex) => ({
        onContextMenu: (event) => openContextMenu(event, rowIndex, columnIndex),
        onMouseDown: (event) => startCellAreaSelection(event, rowIndex, columnIndex),
        onMouseEnter: () => updateCellAreaSelection(rowIndex, columnIndex),
        style: frozen ? { left: frozenLeft } : undefined,
        className: cn(
          frozen && "z-20 bg-background",
          frozen && frozenBaseClassName,
          areaSelectionEnabled && "select-none",
          selectedAreaBounds &&
            rowIndex >= selectedAreaBounds.top &&
            rowIndex <= selectedAreaBounds.bottom &&
            columnIndex >= selectedAreaBounds.left &&
            columnIndex <= selectedAreaBounds.right &&
            "bg-primary/10 ring-1 ring-inset ring-primary/30",
        ),
        "data-datagrid-cell": `${rowIndex}:${column.key}`,
        "data-grid-frozen-column": frozen ? "true" : undefined,
      }),
      className: cn(column.className, "max-w-0 overflow-hidden"),
      render: (row, index) => {
        const content = sourceRender ? sourceRender(row, index) : defaultCellContent(row, column);
        const copyText = columnCanCopy ? getDataGridCopyText(row, column) : "";
        const cellKey = `${rowKey(row)}:${column.key}`;
        const cellNotice = copyNotice?.cellKey === cellKey ? copyNotice.text : null;

        return (
          <DataGridCellContent
            row={row}
            column={column}
            content={content}
            copyText={copyText}
            cellNotice={cellNotice}
            label={columnLabel(column)}
            onCopy={() => void copyCellValue(row, column, cellKey)}
            onDoubleClick={column.onDoubleClick}
          />
        );
      },
      header: (
        <DataGridHeaderCell
          column={column}
          sort={settings.sort}
          filter={dataGridFilterConfigForData(column, data)}
          filterValue={columnFilters[column.key]}
          filterOpen={openFilterKey === column.key}
          onSort={updateSort}
          onToggleFilter={(key) => {
            setFieldsOpen(false);
            setOpenFilterKey((current) => (current === key ? null : key));
          }}
          onFilterChange={updateColumnFilterValue}
          onCloseFilter={() => setOpenFilterKey(null)}
          onResetColumnWidth={resetColumnWidth}
          onStartResize={startColumnResize}
          onNudgeColumnWidth={nudgeColumnWidth}
        />
      ),
    };
  });
  const finalColumns: DataTableColumn<T>[] = selectable
    ? [
        {
          key: "__select",
          header: (
            <Checkbox
              checked={allPageSelected || (selectedPageCount > 0 ? "indeterminate" : false)}
              disabled={pageRowKeys.length === 0}
              aria-label="选择当前页"
              onCheckedChange={(value) => updatePageSelected(value === true)}
            />
          ),
          width: 44,
          render: (row) => {
            const key = rowKey(row);
            return (
              <Checkbox
                checked={selectedKeySet.has(key)}
                aria-label="选择此行"
                onClick={(event) => event.stopPropagation()}
                onCheckedChange={(value) => updateRowSelected(key, value === true)}
              />
            );
          },
        },
        ...tableColumns,
      ]
    : tableColumns;
  const finalTableWidth = dataGridTableWidth(finalColumns);
  const summaryTableWidth = summaryColumns.length > 0 ? dataGridTableWidth(summaryColumns) : finalTableWidth;
  // 动态：表格宽度由当前显示列、用户拖动列宽和字符串列宽共同计算。
  const tableStyle = { width: finalTableWidth, minWidth: finalTableWidth };
  const summaryTableStyle = { width: summaryTableWidth, minWidth: summaryTableWidth };

  return (
    <div ref={rootRef} className={cn("space-y-3", className)} {...rest}>
      <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2 [&_svg]:size-4">
          {refreshAction && visibleAction("refresh") && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              title={refreshAction.description ?? refreshAction.label ?? "刷新列表"}
              disabled={resolveDataGridActionDisabled(refreshAction.disabled, toolbarActionContext)}
              onClick={() => refreshAction.onClick(toolbarActionContext)}
            >
              <RefreshCw className="size-4" aria-hidden />
              {refreshAction.label ?? "刷新"}
            </Button>
          )}
          {queryAction && visibleAction("query") && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              title={queryAction.description ?? queryAction.label ?? "查询列表"}
              disabled={resolveDataGridActionDisabled(queryAction.disabled, toolbarActionContext)}
              onClick={() => queryAction.onClick(toolbarActionContext)}
            >
              <Search className="size-4" aria-hidden />
              {queryAction.label ?? "查询"}
            </Button>
          )}
          {createAction && visibleAction("create") && (
            <Button
              type="button"
              variant="default"
              size="sm"
              className="h-8 shrink-0"
              title={createAction.description ?? createAction.label ?? "新增记录"}
              disabled={resolveDataGridActionDisabled(createAction.disabled, toolbarActionContext)}
              onClick={() => createAction.onClick(toolbarActionContext)}
            >
              <Plus className="size-4" aria-hidden />
              {createAction.label ?? "新增"}
            </Button>
          )}
          {detailAction && visibleAction("detail") && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              title={detailAction.description ?? detailAction.label ?? "查看详情"}
              disabled={resolveDataGridActionDisabled(detailAction.disabled, toolbarActionContext, selectedKeys.length !== 1)}
              onClick={() => detailAction.onClick(toolbarActionContext)}
            >
              <Eye className="size-4" aria-hidden />
              {detailAction.label ?? "详情"}
            </Button>
          )}
          {editAction && visibleAction("edit") && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              title={editAction.description ?? editAction.label ?? "修改记录"}
              disabled={resolveDataGridActionDisabled(editAction.disabled, toolbarActionContext, selectedKeys.length !== 1)}
              onClick={() => editAction.onClick(toolbarActionContext)}
            >
              <Pencil className="size-4" aria-hidden />
              {editAction.label ?? "修改"}
            </Button>
          )}
          {deleteAction && visibleAction("delete") && (
            <Button
              type="button"
              variant="destructive"
              size="sm"
              className="h-8 shrink-0"
              title={deleteAction.description ?? deleteAction.label ?? "删除记录"}
              disabled={resolveDataGridActionDisabled(deleteAction.disabled, toolbarActionContext, selectedKeys.length === 0)}
              onClick={() => deleteAction.onClick(toolbarActionContext)}
            >
              <Trash2 className="size-4" aria-hidden />
              {deleteAction.label ?? "删除"}
            </Button>
          )}
          {disableAction && visibleAction("disable") && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              title={disableAction.description ?? disableAction.label ?? "停用记录"}
              disabled={resolveDataGridActionDisabled(disableAction.disabled, toolbarActionContext, selectedKeys.length === 0)}
              onClick={() => disableAction.onClick(toolbarActionContext)}
            >
              <Ban className="size-4" aria-hidden />
              {disableAction.label ?? "停用"}
            </Button>
          )}
          {visibleAction("view") && (
            <DataGridNamedViewsToolbar
              storageKey={storageKey}
              columns={columns}
              actionKeys={actionKeys}
              pageSizeOptions={safePageSizeOptions}
              defaultPageSize={defaultPageSize}
              settings={settings}
              queryState={queryState}
              onApplyView={applyNamedViewState}
            />
          )}
          {visibleAction("field") && (
            <Button
              ref={fieldButtonRef}
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              aria-label="字段显示"
              title="字段显示"
              aria-expanded={fieldsOpen}
              aria-controls={fieldListId}
              disabled={hideableColumns.length === 0}
              onClick={() => {
                setActionSettingsOpen(false);
                setOpenFilterKey(null);
                setFieldsOpen((open) => !open);
              }}
              data-datagrid-popover
            >
              <Settings2 className="size-4" aria-hidden />
              字段
            </Button>
          )}
          {visibleAction("summary") && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              title="汇总统计"
              disabled={page.filteredRows.length === 0}
              onClick={() => setSummaryOpen(true)}
            >
              <Calculator className="size-4" aria-hidden />
              汇总
            </Button>
          )}
          {showPrintAction && printAction !== false && visibleAction("print") && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              title={printAction?.description ?? printAction?.label ?? "打印列表"}
              disabled={resolveDataGridActionDisabled(printAction?.disabled, toolbarActionContext)}
              onClick={() => {
                if (printAction?.onClick) {
                  printAction.onClick(toolbarActionContext);
                  return;
                }
                if (typeof window !== "undefined") window.print();
              }}
            >
              <Printer className="size-4" aria-hidden />
              {printAction?.label ?? "打印"}
            </Button>
          )}
          {showExportAction && csvExportPlacement === "toolbar" && exportAction !== false && visibleAction("export") && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              title={exportAction?.description ?? exportAction?.label ?? "导出 Excel"}
              disabled={resolveDataGridActionDisabled(exportAction?.disabled, toolbarActionContext, page.filteredRows.length === 0)}
              onClick={() => {
                if (exportAction?.onClick) {
                  exportAction.onClick(toolbarActionContext);
                  return;
                }
                openExportDialog();
              }}
            >
              <Download className="size-4" aria-hidden />
              {exportAction?.label ?? "导出"}
            </Button>
          )}
          {toolbarActions.some((action) => visibleAction(toolbarActionKey(action.key))) && (
            <>
              <span className="mx-1 h-5 w-px bg-border" aria-hidden />
              {toolbarActions.filter((action) => visibleAction(toolbarActionKey(action.key))).map((action) => (
                <Button
                  key={action.key}
                  type="button"
                  variant={action.variant ?? "outline"}
                  size="sm"
                  className="h-8 shrink-0"
                  title={action.description ?? action.label}
                  disabled={typeof action.disabled === "function" ? action.disabled(toolbarActionContext) : action.disabled}
                  onClick={() => action.onClick(toolbarActionContext)}
                >
                  {action.icon}
                  {action.label}
                </Button>
              ))}
            </>
          )}
        </div>
        <div className="flex shrink-0 justify-end">
          <Button
            ref={actionSettingsButtonRef}
            type="button"
            variant="outline"
            size="sm"
            className="relative h-8 shrink-0"
            aria-label="按钮功能"
            title={hasHiddenToolbarActions ? "按钮功能显示设置；有隐藏按钮功能" : "按钮功能显示设置"}
            aria-expanded={actionSettingsOpen}
            aria-controls={actionSettingsPanelId}
            disabled={actionSettingItems.length === 0}
            onClick={() => {
              setFieldsOpen(false);
              setOpenFilterKey(null);
              setActionSettingsOpen((open) => !open);
            }}
            data-datagrid-popover
          >
            {hasHiddenToolbarActions ? (
              <span className="absolute -left-1 -top-1 size-2 rounded-full bg-destructive" aria-hidden />
            ) : null}
            <ListChecks className="size-4" aria-hidden />
            按钮
          </Button>
        </div>
      </div>
      {summaryTable ? (
        <>
          <div className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-primary/30 bg-primary/5 px-3 py-2 text-sm text-primary">
            <span>已显示汇总结果，共 {summaryTable.rows.length} 个分组</span>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8"
              onClick={() => setSummaryConfig(null)}
            >
              退出汇总
            </Button>
          </div>
          <DataTable<DataGridSummaryTableRow>
            className="overflow-visible"
            columns={summaryColumns}
            data={summaryTable.rows}
            rowKey={(row) => row.__summaryKey}
            tableClassName={cn("table-fixed", tableClassName)}
            tableStyle={summaryTableStyle}
            caption={caption}
            emptyTitle={emptyTitle}
            emptyDescription={emptyDescription}
          />
        </>
      ) : (
        <DataTable
          className="overflow-visible"
          columns={finalColumns}
          data={page.rows}
          rowKey={rowKey}
          tableClassName={cn("table-fixed", tableClassName)}
          tableStyle={tableStyle}
          selectedKey={selectedKey}
          onRowClick={onRowClick}
          caption={caption}
          emptyTitle={emptyTitle}
          emptyDescription={emptyDescription}
          footer={
            <DataGridPaginationFooter
              rangeStart={page.rangeStart}
              rangeEnd={page.rangeEnd}
              total={page.total}
              selectable={selectable}
              selectedCount={selectedKeys.length}
              pageSize={settings.pageSize}
              pageSizeOptions={safePageSizeOptions}
              pageIndex={page.pageIndex}
              pageCount={page.pageCount}
              onPageSizeChange={(pageSize) => setSettings((current) => ({ ...current, pageSize }))}
              onPageIndexChange={setPageIndex}
              onClearSelected={() => setSelectedKeys([])}
            />
          }
        />
      )}
      <DataGridFilterChips
        className="border-primary/30 bg-primary/5 text-primary"
        filters={columnFilters}
        fields={filterSummaryFields}
        onClearFilter={(key) =>
          setSettings((current) => ({ ...current, columnFilters: clearDataGridFilterKey(current.columnFilters, key) }))
        }
        onClearAll={() => setSettings((current) => ({ ...current, columnFilters: {} }))}
      />
      {querySummaryItems.length > 0 ? (
        <div
          aria-label="业务查询条件"
          className="flex flex-wrap items-center gap-2 rounded-md border border-sky-200 bg-sky-50 px-3 py-2 text-xs text-sky-800"
        >
          <span className="font-medium">业务查询</span>
          {querySummaryItems.map((item) => (
            <span
              key={item.key}
              className="inline-flex h-8 max-w-full items-center rounded-md border border-sky-100 bg-background px-2 text-foreground shadow-sm"
            >
              <span className="max-w-[18rem] truncate">{item.text}</span>
            </span>
          ))}
          {onClearQueryState ? (
            <Button type="button" variant="ghost" size="sm" className="h-8 text-sky-800" onClick={onClearQueryState}>
              <X className="size-3.5" aria-hidden />
              清除查询
            </Button>
          ) : null}
        </div>
      ) : null}
      <DataGridFieldSettingsPanel
        open={fieldsOpen}
        panelId={fieldListId}
        position={fieldsPanelPosition}
        columns={hideableColumns}
        visibleKeys={visibleKeys}
        copyableKeys={copyableKeys}
        frozenKeys={frozenKeys}
        visibleHideableCount={visibleHideableCount}
        draggingColumnKey={draggingColumnKey}
        onDraggingColumnKeyChange={setDraggingColumnKey}
        onColumnVisibleChange={updateColumnVisible}
        onColumnCopyableChange={updateColumnCopyable}
        onColumnFrozenChange={updateColumnFrozen}
        onMoveColumn={moveColumn}
        onMoveColumnByStep={moveColumnByStep}
      />
      <DataGridActionSettingsPanel
        open={actionSettingsOpen}
        panelId={actionSettingsPanelId}
        position={actionSettingsPanelPosition}
        actions={actionSettingItems}
        onActionVisibleChange={updateActionVisible}
      />
      <DataGridContextMenu
        open={Boolean(contextMenu)}
        position={contextMenu}
        areaSelectionEnabled={areaSelectionEnabled}
        hasSelectedArea={Boolean(selectedAreaBounds)}
        areaSumText={selectedAreaSumText}
        canPaste={canPaste}
        canColumnPaste={canColumnPaste}
        onClose={() => setContextMenu(null)}
        onCopyRow={() => copyContextRow(false)}
        onCopyRowWithHeader={() => copyContextRow(true)}
        onPaste={() => void pasteFromContext("cell")}
        onColumnPaste={() => void pasteFromContext("column")}
        onStartAreaSelection={startAreaSelectionFromContext}
        onCloseAreaSelection={closeAreaSelectionFromContext}
        onCopyArea={() => copySelectedArea(false)}
        onCopyAreaWithHeader={() => copySelectedArea(true)}
        onCopyAreaSum={copySelectedAreaSum}
      />
      <DataGridSummaryDialog
        open={summaryOpen}
        columns={visibleColumns}
        onOpenChange={setSummaryOpen}
        onApply={(config) => {
          setSummaryConfig(config);
          setPageIndex(0);
          setSelectedKeys([]);
        }}
      />
      <DataGridExportDialog
        open={exportOpen}
        fileName={exportFileName}
        format={exportFormat}
        rowCount={page.filteredRows.length}
        onOpenChange={setExportOpen}
        onFileNameChange={setExportFileName}
        onFormatChange={setExportFormat}
        onConfirm={confirmExport}
      />
    </div>
  );
}

const DataGridWithRef = React.forwardRef(DataGridInner) as <T>(
  props: DataGridProps<T> & React.RefAttributes<HTMLDivElement>,
) => React.ReactElement | null;

(DataGridWithRef as { displayName?: string }).displayName = "DataGrid";

export { DataGridWithRef as DataGrid };

function summaryDataTableColumns(
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

function columnLabel<T>(column: DataGridColumn<T>): string {
  return typeof column.header === "string" ? column.header : column.key;
}

function defaultCellContent<T>(row: T, column: DataGridColumn<T>): React.ReactNode {
  if (!row || typeof row !== "object" || Array.isArray(row)) return null;
  return (row as Record<string, React.ReactNode>)[column.key] ?? null;
}

function resolveDataGridExportBaseName(
  exportFileBaseName: string | undefined,
  caption: React.ReactNode,
  storageKey: string | undefined,
): string {
  if (exportFileBaseName?.trim()) return exportFileBaseName;
  if (typeof caption === "string" && caption.trim()) return caption;
  if (typeof document !== "undefined" && document.title.trim()) return document.title;
  return storageKey || "data-grid";
}

function currentColumnWidth<T>(handle: HTMLElement, column: DataGridColumn<T>, savedWidth: number | undefined): number {
  if (typeof savedWidth === "number") return savedWidth;
  if (typeof column.width === "number") return column.width;
  return handle.closest("th")?.getBoundingClientRect().width ?? 160;
}

function resolveDataGridActionDisabled(
  disabled: DataGridActionDisabled | undefined,
  context: DataGridToolbarActionContext,
  fallback = false,
): boolean {
  if (disabled === undefined) return fallback;
  return typeof disabled === "function" ? disabled(context) : disabled;
}

function resolveDataGridPasteDisabled<T>(
  disabled: DataGridPasteDisabled<T> | undefined,
  context: DataGridPasteTarget<T>,
): boolean {
  if (disabled === undefined) return false;
  return typeof disabled === "function" ? disabled(context) : disabled;
}

async function readClipboardText(): Promise<string> {
  if (!navigator.clipboard?.readText) return "";
  return navigator.clipboard.readText();
}

async function writeClipboardText(value: string) {
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

function isDataGridColumn<T>(column: DataGridColumn<T> | undefined): column is DataGridColumn<T> {
  return Boolean(column);
}

function buildDataGridActionDescriptors<T>({
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

function toolbarActionKey(key: string): string {
  return `toolbar:${key}`;
}

function contextMenuPosition(x: number, y: number): DataGridContextMenuPosition {
  if (typeof window === "undefined") return { x, y };
  const menuWidth = 192;
  const menuHeight = 240;
  return {
    x: Math.min(x, Math.max(8, window.innerWidth - menuWidth - 8)),
    y: Math.min(y, Math.max(8, window.innerHeight - menuHeight - 8)),
  };
}

function normalizedAreaBounds(selection: DataGridAreaSelection) {
  return {
    top: Math.min(selection.anchor.rowIndex, selection.focus.rowIndex),
    bottom: Math.max(selection.anchor.rowIndex, selection.focus.rowIndex),
    left: Math.min(selection.anchor.columnIndex, selection.focus.columnIndex),
    right: Math.max(selection.anchor.columnIndex, selection.focus.columnIndex),
  };
}

function selectedAreaPayload<T>(
  bounds: ReturnType<typeof normalizedAreaBounds> | null,
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

function buildDataGridClipboardText<T>(
  columns: DataGridColumn<T>[],
  rows: T[],
  includeHeader: boolean,
): string {
  const lines = rows.map((row) => columns.map((column) => getDataGridCopyText(row, column)).join("\t"));
  if (!includeHeader) return lines.join("\n");
  return [columns.map(columnLabel).join("\t"), ...lines].join("\n");
}

function buildDataGridSelectedAreaSumText<T>(
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
