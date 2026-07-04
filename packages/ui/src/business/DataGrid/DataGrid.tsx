import * as React from "react";
import { Ban, Download, Eye, Pencil, Plus, Printer, RefreshCw, Search, Settings2, Trash2 } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Checkbox } from "../../ui/checkbox";
import { DataTable, type DataTableColumn, type DataTableProps } from "../DataTable";
import { DataGridCellContent } from "./DataGridCellContent";
import { DataGridFieldSettingsPanel } from "./DataGridFieldSettingsPanel";
import { DataGridFilterChips } from "./DataGridFilterChips";
import { DataGridHeaderCell } from "./DataGridHeaderCell";
import { DataGridNamedViewsToolbar } from "./DataGridNamedViewsToolbar";
import { DataGridPaginationFooter } from "./DataGridPaginationFooter";
import { buildDataGridCsv, downloadDataGridCsv } from "./data-grid-export";
import { clearDataGridFilterKey } from "./data-grid-filter-summary";
import { useDataGridPopoverDismiss } from "./data-grid-popover-dismiss";
import {
  getDataGridCopyText,
  dataGridFloatingPanelPosition,
  dataGridTableWidth,
  dataGridFilterConfigForData,
  getDataGridPage,
  moveColumnBefore,
  nextSortState,
  reconcileDataGridSelectedRowKeys,
  sanitizeGridState,
  sanitizeDataGridColumnFiltersForData,
  setColumnWidth,
  toggleCopyableColumn,
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
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridQueryAction {
  label?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridCreateAction {
  label?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridDetailAction {
  label?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridEditAction {
  label?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridDeleteAction {
  label?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridDisableAction {
  label?: string;
  disabled?: DataGridActionDisabled;
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridPrintAction {
  label?: string;
  disabled?: DataGridActionDisabled;
  onClick?: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridExportAction {
  label?: string;
  disabled?: DataGridActionDisabled;
  onClick?: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridToolbarAction {
  key: string;
  label: string;
  icon?: React.ReactNode;
  disabled?: DataGridActionDisabled;
  variant?: "default" | "destructive" | "outline" | "secondary" | "ghost" | "link";
  onClick: (context: DataGridToolbarActionContext) => void;
}

export interface DataGridToolbarActionContext {
  selectedRowKeys: string[];
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
  refreshAction?: DataGridRefreshAction;
  queryAction?: DataGridQueryAction;
  createAction?: DataGridCreateAction;
  detailAction?: DataGridDetailAction;
  editAction?: DataGridEditAction;
  deleteAction?: DataGridDeleteAction;
  disableAction?: DataGridDisableAction;
  printAction?: DataGridPrintAction | false;
  exportAction?: DataGridExportAction | false;
  toolbarActions?: DataGridToolbarAction[];
  showPrintAction?: boolean;
  showExportAction?: boolean;
}

const defaultPageSizeOptions = [10, 20, 50, 100];

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
    refreshAction,
    queryAction,
    createAction,
    detailAction,
    editAction,
    deleteAction,
    disableAction,
    printAction,
    exportAction,
    toolbarActions = [],
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
  const columnsRef = React.useRef(columns);
  const fieldListId = React.useId();
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
  const [settings, setSettings] = React.useState<DataGridLogicState>(() =>
    loadGridSettings(storageKey, columns, safePageSizeOptions, defaultPageSize),
  );
  const [pageIndex, setPageIndex] = React.useState(0);
  const [columnFilters, setColumnFilters] = React.useState<DataGridColumnFilters>({});
  const [fieldsOpen, setFieldsOpen] = React.useState(false);
  const [fieldsPanelPosition, setFieldsPanelPosition] = React.useState<DataGridFloatingPanelPosition | null>(null);
  const [openFilterKey, setOpenFilterKey] = React.useState<string | null>(null);
  const [internalSelectedRowKeys, setInternalSelectedRowKeys] = React.useState<string[]>([]);
  const [draggingColumnKey, setDraggingColumnKey] = React.useState<string | null>(null);
  const [resizingColumn, setResizingColumn] = React.useState<{ key: string; startX: number; startWidth: number } | null>(null);
  const [copyNotice, setCopyNotice] = React.useState<{ cellKey: string; text: string } | null>(null);

  React.useImperativeHandle(ref, () => rootRef.current as HTMLDivElement);

  React.useEffect(() => {
    columnsRef.current = columns;
  }, [columns]);

  React.useEffect(() => {
    setSettings((current) => sanitizeGridState(current, columns, safePageSizeOptions, defaultPageSize));
  }, [columnSignature, defaultPageSize, safePageSizeOptions]);

  React.useEffect(() => {
    saveGridSettings(storageKey, settings);
  }, [settings, storageKey]);

  React.useEffect(() => {
    setPageIndex(0);
  }, [columnFilters, data, settings.pageSize, settings.sort?.key, settings.sort?.direction]);

  React.useEffect(() => {
    setColumnFilters((current) => sanitizeDataGridColumnFiltersForData(current, columns, data));
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
    open: fieldsOpen || openFilterKey !== null,
    onDismiss: () => {
      setFieldsOpen(false);
      setOpenFilterKey(null);
    },
  });

  React.useEffect(() => {
    if (!fieldsOpen) return;

    function updatePosition() {
      const rect = fieldButtonRef.current?.getBoundingClientRect();
      if (!rect) return;
      setFieldsPanelPosition(
        dataGridFloatingPanelPosition(rect, { width: window.innerWidth, height: window.innerHeight }, 256),
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

  const page = getDataGridPage({
    data,
    columns,
    visibleColumns: settings.visibleColumns,
    columnFilters,
    sort: settings.sort,
    pageIndex,
    pageSize: settings.pageSize,
  });
  const visibleKeys = new Set(settings.visibleColumns);
  const copyableKeys = new Set(settings.copyableColumns);
  const columnsByKey = new Map(columns.map((column) => [column.key, column]));
  const orderedColumns = settings.columnOrder.map((key) => columnsByKey.get(key)).filter(isDataGridColumn);
  const orderedHideableColumns = orderedColumns.filter((column) => column.hideable !== false);
  const fixedColumns = orderedColumns.filter((column) => column.hideable === false);
  const visibleColumns = [...orderedHideableColumns.filter((column) => visibleKeys.has(column.key)), ...fixedColumns];
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
    setColumnFilters((current) => {
      if (!dataGridFilterActive(value)) {
        const next = { ...current };
        delete next[key];
        return next;
      }
      return { ...current, [key]: value };
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
      fileName: snapshot.storageKey ? `${snapshot.storageKey}.xls` : "data-grid.xls",
      document: typeof document === "undefined" ? undefined : document,
    });
  }, []);

  React.useEffect(() => {
    if (csvExportPlacement !== "external" || !onCsvExportStateChange) return;
    onCsvExportStateChange({ disabled: page.filteredRows.length === 0, exportCsv });
    return () => onCsvExportStateChange(null);
  }, [csvExportPlacement, exportCsv, onCsvExportStateChange, page.filteredRows.length]);

  function applyNamedViewState(state: DataGridLogicState) {
    setSettings(state);
    setPageIndex(0);
  }

  const tableColumns: DataTableColumn<T>[] = visibleColumns.map((column) => {
    const sourceRender = column.render;
    const columnCanCopy = column.copyable !== false && copyableKeys.has(column.key);
    const columnWidth = settings.columnWidths[column.key] ?? column.width;

    return {
      ...column,
      width: columnWidth,
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
  // 动态：表格宽度由当前显示列、用户拖动列宽和字符串列宽共同计算。
  const tableStyle = { width: finalTableWidth, minWidth: finalTableWidth };

  return (
    <div ref={rootRef} className={cn("space-y-3", className)} {...rest}>
      <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div className="flex min-w-0 flex-1 flex-wrap items-start gap-3">
          <div className="flex flex-wrap items-center gap-2 rounded-md border bg-muted/20 px-2 py-1.5 [&_svg]:size-4">
            <span className="text-xs font-medium text-muted-foreground">功能能力</span>
          {refreshAction && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              disabled={resolveDataGridActionDisabled(refreshAction.disabled, toolbarActionContext)}
              onClick={() => refreshAction.onClick(toolbarActionContext)}
            >
              <RefreshCw className="size-4" aria-hidden />
              {refreshAction.label ?? "刷新"}
            </Button>
          )}
          {queryAction && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              disabled={resolveDataGridActionDisabled(queryAction.disabled, toolbarActionContext)}
              onClick={() => queryAction.onClick(toolbarActionContext)}
            >
              <Search className="size-4" aria-hidden />
              {queryAction.label ?? "查询"}
            </Button>
          )}
          {createAction && (
            <Button
              type="button"
              variant="default"
              size="sm"
              className="h-8 shrink-0"
              disabled={resolveDataGridActionDisabled(createAction.disabled, toolbarActionContext)}
              onClick={() => createAction.onClick(toolbarActionContext)}
            >
              <Plus className="size-4" aria-hidden />
              {createAction.label ?? "新增"}
            </Button>
          )}
          {detailAction && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              disabled={resolveDataGridActionDisabled(detailAction.disabled, toolbarActionContext, selectedKeys.length !== 1)}
              onClick={() => detailAction.onClick(toolbarActionContext)}
            >
              <Eye className="size-4" aria-hidden />
              {detailAction.label ?? "详情"}
            </Button>
          )}
          {editAction && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              disabled={resolveDataGridActionDisabled(editAction.disabled, toolbarActionContext, selectedKeys.length !== 1)}
              onClick={() => editAction.onClick(toolbarActionContext)}
            >
              <Pencil className="size-4" aria-hidden />
              {editAction.label ?? "修改"}
            </Button>
          )}
          {deleteAction && (
            <Button
              type="button"
              variant="destructive"
              size="sm"
              className="h-8 shrink-0"
              disabled={resolveDataGridActionDisabled(deleteAction.disabled, toolbarActionContext, selectedKeys.length === 0)}
              onClick={() => deleteAction.onClick(toolbarActionContext)}
            >
              <Trash2 className="size-4" aria-hidden />
              {deleteAction.label ?? "删除"}
            </Button>
          )}
          {disableAction && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              disabled={resolveDataGridActionDisabled(disableAction.disabled, toolbarActionContext, selectedKeys.length === 0)}
              onClick={() => disableAction.onClick(toolbarActionContext)}
            >
              <Ban className="size-4" aria-hidden />
              {disableAction.label ?? "停用"}
            </Button>
          )}
          <DataGridNamedViewsToolbar
            storageKey={storageKey}
            columns={columns}
            pageSizeOptions={safePageSizeOptions}
            defaultPageSize={defaultPageSize}
            settings={settings}
            onApplyView={applyNamedViewState}
          />
          <Button
            ref={fieldButtonRef}
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            aria-label="字段显示"
            aria-expanded={fieldsOpen}
            aria-controls={fieldListId}
            disabled={hideableColumns.length === 0}
            onClick={() => {
              setOpenFilterKey(null);
              setFieldsOpen((open) => !open);
            }}
            data-datagrid-popover
          >
            <Settings2 className="size-4" aria-hidden />
            字段显示
          </Button>
          {showPrintAction && printAction !== false && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
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
          {showExportAction && csvExportPlacement === "toolbar" && exportAction !== false && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 shrink-0"
              disabled={resolveDataGridActionDisabled(exportAction?.disabled, toolbarActionContext, page.filteredRows.length === 0)}
              onClick={() => {
                if (exportAction?.onClick) {
                  exportAction.onClick(toolbarActionContext);
                  return;
                }
                exportCsv();
              }}
            >
              <Download className="size-4" aria-hidden />
              {exportAction?.label ?? "导出 Excel"}
            </Button>
          )}
          </div>
          {toolbarActions.length > 0 && (
            <div className="flex flex-wrap items-center gap-2 rounded-md border bg-background px-2 py-1.5 [&_svg]:size-4">
              <span className="text-xs font-medium text-muted-foreground">私有能力</span>
              {toolbarActions.map((action) => (
                <Button
                  key={action.key}
                  type="button"
                  variant={action.variant ?? "outline"}
                  size="sm"
                  className="h-8 shrink-0"
                  disabled={typeof action.disabled === "function" ? action.disabled(toolbarActionContext) : action.disabled}
                  onClick={() => action.onClick(toolbarActionContext)}
                >
                  {action.icon}
                  {action.label}
                </Button>
              ))}
            </div>
          )}
        </div>
        <DataGridFilterChips
          className="min-w-0 flex-1 md:justify-end"
          filters={columnFilters}
          fields={filterSummaryFields}
          onClearFilter={(key) =>
            setColumnFilters((current) => clearDataGridFilterKey(current, key))
          }
          onClearAll={() => setColumnFilters({})}
        />
      </div>
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
      <DataGridFieldSettingsPanel
        open={fieldsOpen}
        panelId={fieldListId}
        position={fieldsPanelPosition}
        columns={hideableColumns}
        visibleKeys={visibleKeys}
        copyableKeys={copyableKeys}
        visibleHideableCount={visibleHideableCount}
        draggingColumnKey={draggingColumnKey}
        onDraggingColumnKeyChange={setDraggingColumnKey}
        onColumnVisibleChange={updateColumnVisible}
        onColumnCopyableChange={updateColumnCopyable}
        onMoveColumn={moveColumn}
        onMoveColumnByStep={moveColumnByStep}
      />
    </div>
  );
}

const DataGridWithRef = React.forwardRef(DataGridInner) as <T>(
  props: DataGridProps<T> & React.RefAttributes<HTMLDivElement>,
) => React.ReactElement | null;

(DataGridWithRef as { displayName?: string }).displayName = "DataGrid";

export { DataGridWithRef as DataGrid };

function columnLabel<T>(column: DataGridColumn<T>): string {
  return typeof column.header === "string" ? column.header : column.key;
}

function defaultCellContent<T>(row: T, column: DataGridColumn<T>): React.ReactNode {
  if (!row || typeof row !== "object" || Array.isArray(row)) return null;
  return (row as Record<string, React.ReactNode>)[column.key] ?? null;
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
