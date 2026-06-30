import * as React from "react";
import { cn } from "../../lib/utils";
import { Checkbox } from "../../ui/checkbox";
import { DataTable, type DataTableColumn, type DataTableProps } from "../DataTable";
import { DataGridCellContent } from "./DataGridCellContent";
import { DataGridFieldSettingsPanel } from "./DataGridFieldSettingsPanel";
import { DataGridFilterChips } from "./DataGridFilterChips";
import { DataGridHeaderCell } from "./DataGridHeaderCell";
import { DataGridPaginationFooter } from "./DataGridPaginationFooter";
import { clearDataGridFilterKey } from "./data-grid-filter-summary";
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

  React.useEffect(() => {
    if (!fieldsOpen && !openFilterKey) return;

    function closePanels(event: PointerEvent) {
      const target = event.target instanceof Element ? event.target : null;
      if (target?.closest("[data-datagrid-popover]")) return;
      setFieldsOpen(false);
      setOpenFilterKey(null);
    }

    document.addEventListener("pointerdown", closePanels);
    return () => document.removeEventListener("pointerdown", closePanels);
  }, [fieldsOpen, openFilterKey]);

  React.useEffect(() => {
    if (!fieldsOpen && !openFilterKey) return;

    function closePanelsByKeyboard(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      setFieldsOpen(false);
      setOpenFilterKey(null);
    }

    document.addEventListener("keydown", closePanelsByKeyboard);
    return () => document.removeEventListener("keydown", closePanelsByKeyboard);
  }, [fieldsOpen, openFilterKey]);

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
  const lastVisibleColumnKey = visibleColumns.at(-1)?.key;
  const hideableColumns = orderedHideableColumns;
  const visibleHideableCount = hideableColumns.filter((column) => visibleKeys.has(column.key)).length;
  const selectedKeys = selectedRowKeys ?? internalSelectedRowKeys;
  const selectedKeySet = new Set(selectedKeys);
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
          isLastVisibleColumn={column.key === lastVisibleColumnKey}
          fieldListId={fieldListId}
          fieldButtonRef={fieldButtonRef}
          fieldsOpen={fieldsOpen}
          hideableColumnsLength={hideableColumns.length}
          onSort={updateSort}
          onToggleFilter={(key) => {
            setFieldsOpen(false);
            setOpenFilterKey((current) => (current === key ? null : key));
          }}
          onFilterChange={updateColumnFilterValue}
          onCloseFilter={() => setOpenFilterKey(null)}
          onToggleFields={() => {
            setOpenFilterKey(null);
            setFieldsOpen((open) => !open);
          }}
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
      <DataGridFilterChips
        filters={columnFilters}
        fields={filterSummaryFields}
        onClearFilter={(key) => setColumnFilters((current) => clearDataGridFilterKey(current, key))}
        onClearAll={() => setColumnFilters({})}
      />
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
