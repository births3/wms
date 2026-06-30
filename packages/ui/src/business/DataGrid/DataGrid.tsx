import * as React from "react";
import { createPortal } from "react-dom";
import { ArrowDown, ArrowUp, ArrowUpDown, ChevronLeft, ChevronRight, Filter, GripVertical, Settings2, X } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Checkbox } from "../../ui/checkbox";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../../ui/select";
import { DataTable, type DataTableColumn, type DataTableProps } from "../DataTable";
import { DataGridColumnFilter } from "./DataGridColumnFilter";
import {
  getDataGridCopyText,
  dataGridFloatingPanelPosition,
  dataGridTableWidth,
  dataGridFilterConfigForData,
  getDataGridPage,
  moveColumnBefore,
  nextSortState,
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
  type DataGridSortState,
} from "./data-grid-logic";

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
  const currentPage = page.pageIndex + 1;
  const selectedKeys = selectedRowKeys ?? internalSelectedRowKeys;
  const selectedKeySet = new Set(selectedKeys);
  const pageRowKeys = page.rows.map(rowKey);
  const selectedPageCount = pageRowKeys.filter((key) => selectedKeySet.has(key)).length;
  const allPageSelected = pageRowKeys.length > 0 && selectedPageCount === pageRowKeys.length;

  const fieldSettingsPanel =
    fieldsOpen && fieldsPanelPosition && typeof document !== "undefined"
      ? createPortal(
          <div
            id={fieldListId}
            className="fixed z-50 w-64 overflow-auto rounded-md border border-primary/30 bg-background p-2 text-left text-sm shadow-lg"
            style={{
              top: fieldsPanelPosition.top,
              left: fieldsPanelPosition.left,
              maxHeight: fieldsPanelPosition.maxHeight,
            }}
            data-datagrid-popover
          >
            {hideableColumns.map((item) => {
              const checked = visibleKeys.has(item.key);
              const copyable = copyableKeys.has(item.key);
              const disabled = checked && visibleHideableCount <= 1;
              const checkboxId = `${fieldListId}-${item.key}`;
              const copyCheckboxId = `${fieldListId}-${item.key}-copy`;
              return (
                <div
                  key={item.key}
                  draggable
                  onDragStart={() => setDraggingColumnKey(item.key)}
                  onDragOver={(event) => event.preventDefault()}
                  onDrop={() => {
                    if (draggingColumnKey) moveColumn(draggingColumnKey, item.key);
                    setDraggingColumnKey(null);
                  }}
                  onDragEnd={() => setDraggingColumnKey(null)}
                  className={cn(
                    "flex items-center gap-2 rounded-sm px-2 py-1.5",
                    draggingColumnKey === item.key ? "bg-muted" : "hover:bg-muted/60",
                  )}
                >
                  <GripVertical className="size-4 shrink-0 cursor-grab text-muted-foreground" aria-hidden />
                  <Checkbox
                    id={checkboxId}
                    checked={checked}
                    disabled={disabled}
                    onCheckedChange={(value) => updateColumnVisible(item.key, value === true)}
                  />
                  <label htmlFor={checkboxId} className="min-w-0 flex-1 truncate text-muted-foreground">
                    {columnLabel(item)}
                  </label>
                  <label htmlFor={copyCheckboxId} className="flex shrink-0 items-center gap-1 text-xs text-muted-foreground">
                    <Checkbox
                      id={copyCheckboxId}
                      checked={copyable}
                      disabled={item.copyable === false}
                      onCheckedChange={(value) => updateColumnCopyable(item.key, value === true)}
                    />
                    复制
                  </label>
                </div>
              );
            })}
          </div>,
          document.body,
        )
      : null;

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

  function resetColumnWidth(key: string) {
    setSettings((current) => ({ ...current, columnWidths: setColumnWidth(current.columnWidths, columns, key, null) }));
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
        const canDoubleClick = Boolean(column.onDoubleClick);
        if (!copyText && !canDoubleClick) return content;
        const cellKey = `${rowKey(row)}:${column.key}`;
        const cellNotice = copyNotice?.cellKey === cellKey ? copyNotice.text : null;

        return (
          <div
            role="button"
            tabIndex={0}
            title={copyText ? "点击复制" : "双击打开"}
            aria-label={copyText ? `复制${columnLabel(column)}` : `打开${columnLabel(column)}`}
            className={cn(
              "relative rounded-sm px-1 py-0.5 text-left outline-none transition hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring",
              column.align === "right" && "text-right",
            )}
            onClick={(event) => {
              if (!copyText) return;
              event.stopPropagation();
              void copyCellValue(row, column, cellKey);
            }}
            onDoubleClick={(event) => {
              if (!column.onDoubleClick) return;
              event.stopPropagation();
              column.onDoubleClick(row);
            }}
            onKeyDown={(event) => {
              if (!copyText || (event.key !== "Enter" && event.key !== " ")) return;
              event.preventDefault();
              event.stopPropagation();
              void copyCellValue(row, column, cellKey);
            }}
          >
            <div className="relative inline-block max-w-full align-middle">
              <div className="min-w-0 truncate">{content}</div>
              {cellNotice && (
                <span className="pointer-events-none absolute left-full top-1/2 z-20 ml-2 -translate-y-1/2 whitespace-nowrap rounded-sm bg-foreground px-1.5 py-0.5 text-[11px] font-normal text-background shadow-sm">
                  {cellNotice}
                </span>
              )}
            </div>
          </div>
        );
      },
      header: (
        <div className={cn("relative flex min-w-0 items-center gap-1 pr-2", column.align === "right" && "justify-end")}>
          <div className="min-w-0">
          {column.sortable ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="-ml-3 h-8 px-2"
              onClick={() => updateSort(column)}
            >
              {column.header}
              {sortIcon(settings.sort, column.key)}
            </Button>
          ) : (
            <span>{column.header}</span>
          )}
          </div>
          {columnFilterable(column) && (
            <>
              <Button
                type="button"
                variant={dataGridFilterActive(columnFilters[column.key]) ? "secondary" : "ghost"}
                size="icon"
                className="size-7 shrink-0"
                aria-label={`筛选${columnLabel(column)}`}
                aria-expanded={openFilterKey === column.key}
                onClick={() => {
                  setFieldsOpen(false);
                  setOpenFilterKey((key) => (key === column.key ? null : column.key));
                }}
                data-datagrid-popover
              >
                <Filter className="size-3.5" aria-hidden />
              </Button>
              {openFilterKey === column.key && (
                <div
                  className={cn(
                    "absolute top-full z-30 mt-2 w-56 rounded-md border bg-background p-3 text-left shadow-lg",
                    column.align === "right" ? "right-0" : "left-0",
                  )}
                  data-datagrid-popover
                >
                  <DataGridColumnFilter
                    columnKey={column.key}
                    label={columnLabel(column)}
                    filter={dataGridFilterConfigForData(column, data)}
                    value={columnFilters[column.key]}
                    onChange={(value) => updateColumnFilterValue(column.key, value)}
                  />
                  <div className="mt-2 flex justify-end gap-2">
                    <Button type="button" variant="ghost" size="sm" onClick={() => updateColumnFilterValue(column.key, "")}>
                      <X className="size-3.5" aria-hidden />
                      清除
                    </Button>
                    <Button type="button" variant="outline" size="sm" onClick={() => setOpenFilterKey(null)}>
                      关闭
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
          {column.key === lastVisibleColumnKey && (
            <div className="relative ml-1 border-l pl-2">
              <Button
                ref={fieldButtonRef}
                type="button"
                variant="outline"
                size="icon"
                className="size-7 border-primary/40 bg-primary/5 text-primary hover:bg-primary/10"
                aria-label="字段设置"
                aria-expanded={fieldsOpen}
                aria-controls={fieldListId}
                disabled={hideableColumns.length === 0}
                onClick={() => {
                  setOpenFilterKey(null);
                  setFieldsOpen((open) => !open);
                }}
                data-datagrid-popover
              >
                <Settings2 className="size-3.5" aria-hidden />
              </Button>
            </div>
          )}
          {column.resizable !== false && (
            <button
              type="button"
              className="absolute -right-1 top-0 h-full w-2 cursor-col-resize rounded-sm hover:bg-primary/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              aria-label={`调整${columnLabel(column)}列宽`}
              title="拖动调整列宽，双击恢复默认"
              onClick={(event) => event.stopPropagation()}
              onDoubleClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                resetColumnWidth(column.key);
              }}
              onPointerDown={(event) => {
                event.preventDefault();
                event.stopPropagation();
                setResizingColumn({
                  key: column.key,
                  startX: event.clientX,
                  startWidth: currentColumnWidth(event.currentTarget, column, settings.columnWidths[column.key]),
                });
              }}
            />
          )}
        </div>
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

  return (
    <div ref={rootRef} className={cn("space-y-3", className)} {...rest}>
      <DataTable
        className="overflow-visible"
        columns={finalColumns}
        data={page.rows}
        rowKey={rowKey}
        tableClassName={cn("table-fixed", tableClassName)}
        tableStyle={{ width: finalTableWidth, minWidth: finalTableWidth }}
        selectedKey={selectedKey}
        onRowClick={onRowClick}
        caption={caption}
        emptyTitle={emptyTitle}
        emptyDescription={emptyDescription}
        footer={
          <div className="flex flex-col gap-2 px-4 py-3 text-xs text-muted-foreground md:flex-row md:items-center md:justify-between">
            <span>
              {page.rangeStart}-{page.rangeEnd} / 共 {page.total} 条
              {selectable && selectedKeys.length > 0 ? ` · 已选 ${selectedKeys.length} 条` : ""}
            </span>
            <div className="flex flex-wrap items-center gap-2">
              {selectable && selectedKeys.length > 0 && (
                <Button type="button" variant="ghost" size="sm" onClick={() => setSelectedKeys([])}>
                  清空选择
                </Button>
              )}
              <Select
                value={String(settings.pageSize)}
                onValueChange={(value) =>
                  setSettings((current) => ({ ...current, pageSize: Number.parseInt(value, 10) }))
                }
              >
                <SelectTrigger className="h-8 w-[116px]" aria-label="每页条数">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {safePageSizeOptions.map((option) => (
                    <SelectItem key={option} value={String(option)}>
                      {option} 条/页
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <span>
                第 {currentPage} / {page.pageCount} 页
              </span>
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={page.pageIndex === 0}
                onClick={() => setPageIndex((value) => Math.max(0, value - 1))}
              >
                <ChevronLeft className="size-4" aria-hidden />
                上一页
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={page.pageIndex >= page.pageCount - 1}
                onClick={() => setPageIndex((value) => Math.min(page.pageCount - 1, value + 1))}
              >
                下一页
                <ChevronRight className="size-4" aria-hidden />
              </Button>
            </div>
          </div>
        }
      />
      {fieldSettingsPanel}
    </div>
  );
}

const DataGridWithRef = React.forwardRef(DataGridInner) as <T>(
  props: DataGridProps<T> & React.RefAttributes<HTMLDivElement>,
) => React.ReactElement | null;

(DataGridWithRef as { displayName?: string }).displayName = "DataGrid";

export { DataGridWithRef as DataGrid };

function sortIcon(sort: DataGridSortState | null, key: string) {
  if (sort?.key !== key) return <ArrowUpDown className="size-3.5 text-muted-foreground" aria-hidden />;
  if (sort.direction === "asc") return <ArrowUp className="size-3.5" aria-hidden />;
  return <ArrowDown className="size-3.5" aria-hidden />;
}

function columnLabel<T>(column: DataGridColumn<T>): string {
  return typeof column.header === "string" ? column.header : column.key;
}

function columnFilterable<T>(column: DataGridColumn<T>): boolean {
  return column.hideable !== false && column.filter !== false;
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

function loadGridSettings<T>(
  storageKey: string | undefined,
  columns: DataGridColumn<T>[],
  pageSizeOptions: number[],
  defaultPageSize: number,
): DataGridLogicState {
  if (!storageKey || typeof window === "undefined") {
    return sanitizeGridState(null, columns, pageSizeOptions, defaultPageSize);
  }

  try {
    const raw = window.localStorage.getItem(storageKey);
    return sanitizeGridState(parseStoredGridSettings(raw ? JSON.parse(raw) : null), columns, pageSizeOptions, defaultPageSize);
  } catch {
    return sanitizeGridState(null, columns, pageSizeOptions, defaultPageSize);
  }
}

function saveGridSettings(storageKey: string | undefined, settings: DataGridLogicState) {
  if (!storageKey || typeof window === "undefined") return;
  try {
    window.localStorage.setItem(storageKey, JSON.stringify(settings));
  } catch {
    // localStorage 可能被禁用；表格仍使用当前内存状态。
  }
}

function parseStoredGridSettings(value: unknown): Partial<DataGridLogicState> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  return {
    visibleColumns: Array.isArray(record.visibleColumns) ? record.visibleColumns.filter(isString) : undefined,
    copyableColumns: Array.isArray(record.copyableColumns) ? record.copyableColumns.filter(isString) : undefined,
    columnWidths: parseStoredColumnWidths(record.columnWidths),
    columnOrder: Array.isArray(record.columnOrder) ? record.columnOrder.filter(isString) : undefined,
    pageSize: typeof record.pageSize === "number" ? record.pageSize : undefined,
    sort: parseStoredSort(record.sort),
  };
}

function parseStoredColumnWidths(value: unknown): Record<string, number> | undefined {
  if (!value || typeof value !== "object" || Array.isArray(value)) return undefined;
  const widths: Record<string, number> = {};
  for (const [key, width] of Object.entries(value)) {
    if (typeof width === "number" && Number.isFinite(width)) widths[key] = width;
  }
  return widths;
}

function parseStoredSort(value: unknown): DataGridSortState | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.key !== "string") return null;
  if (record.direction !== "asc" && record.direction !== "desc") return null;
  return { key: record.key, direction: record.direction };
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}
