import * as React from "react";
import { createPortal } from "react-dom";
import { ArrowDown, ArrowUp, ArrowUpDown, Filter, X } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { DataGridColumnFilter } from "./DataGridColumnFilter";
import type { DataGridColumn } from "./data-grid-types";
import {
  dataGridFilterActive,
  dataGridFloatingPanelPosition,
  type DataGridColumnFilterValue,
  type DataGridFilterConfig,
  type DataGridFloatingPanelPosition,
  type DataGridSortState,
} from "./data-grid-logic";

/**
 * DataGridHeaderCell — DataGrid 表头交互单元
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2 收货管理列表
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：排序、字段筛选和列宽调整集中在表头
 *
 * @example
 *   <DataGridHeaderCell column={column} sort={sort} />
 */
export interface DataGridHeaderCellProps<T> {
  column: DataGridColumn<T>;
  sort: DataGridSortState | null;
  filter: DataGridFilterConfig | false | undefined;
  filterValue: DataGridColumnFilterValue | undefined;
  filterOpen: boolean;
  className?: string;
  onSort: (column: DataGridColumn<T>) => void;
  onToggleFilter: (key: string) => void;
  onFilterChange: (key: string, value: DataGridColumnFilterValue) => void;
  onCloseFilter: () => void;
  onResetColumnWidth: (key: string) => void;
  onStartResize: (handle: HTMLElement, column: DataGridColumn<T>, clientX: number) => void;
  onNudgeColumnWidth: (handle: HTMLElement, column: DataGridColumn<T>, delta: number) => void;
}

export function DataGridHeaderCell<T>({
  column,
  sort,
  filter,
  filterValue,
  filterOpen,
  className,
  onSort,
  onToggleFilter,
  onFilterChange,
  onCloseFilter,
  onResetColumnWidth,
  onStartResize,
  onNudgeColumnWidth,
}: DataGridHeaderCellProps<T>) {
  const label = columnLabel(column);
  const filterButtonRef = React.useRef<HTMLButtonElement | null>(null);
  const [filterPanelPosition, setFilterPanelPosition] = React.useState<DataGridFloatingPanelPosition | null>(null);

  React.useEffect(() => {
    if (!filterOpen) return;

    function updatePosition() {
      const rect = filterButtonRef.current?.getBoundingClientRect();
      if (!rect) return;
      setFilterPanelPosition(
        dataGridFloatingPanelPosition(rect, { width: window.innerWidth, height: window.innerHeight }, 224),
      );
    }

    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [filterOpen]);

  return (
    <div className={cn("relative flex min-w-0 items-center gap-1 pr-2", column.align === "right" && "justify-end", className)}>
      <div className="min-w-0">
        {column.sortable ? (
          <Button type="button" variant="ghost" size="sm" className="-ml-3 h-8 px-2" onClick={() => onSort(column)}>
            {column.header}
            {sortIcon(sort, column.key)}
          </Button>
        ) : (
          <span>{column.header}</span>
        )}
      </div>
      {columnFilterable(column) && (
        <>
          <Button
            ref={filterButtonRef}
            type="button"
            variant={dataGridFilterActive(filterValue) ? "secondary" : "ghost"}
            size="icon"
            className="size-7 shrink-0"
            aria-label={`筛选${label}`}
            aria-expanded={filterOpen}
            onClick={() => onToggleFilter(column.key)}
            data-datagrid-popover
          >
            <Filter className="size-3.5" aria-hidden />
          </Button>
          {filterOpen && filterPanelPosition && typeof document !== "undefined"
            ? createPortal(
                <div
                  className="fixed z-50 w-56 overflow-auto rounded-md border bg-background p-3 text-left shadow-lg"
                  // 动态：字段筛选浮层跟随筛选按钮位置和视口高度。
                  style={{
                    top: filterPanelPosition.top,
                    left: filterPanelPosition.left,
                    maxHeight: filterPanelPosition.maxHeight,
                  }}
                  data-datagrid-popover
                >
                  <DataGridColumnFilter
                    columnKey={column.key}
                    label={label}
                    filter={filter}
                    value={filterValue}
                    onChange={(value) => onFilterChange(column.key, value)}
                  />
                  <div className="mt-2 flex justify-end gap-2">
                    <Button type="button" variant="ghost" size="sm" onClick={() => onFilterChange(column.key, "")}>
                      <X className="size-3.5" aria-hidden />
                      清除
                    </Button>
                    <Button type="button" variant="outline" size="sm" onClick={onCloseFilter}>
                      关闭
                    </Button>
                  </div>
                </div>,
                document.body,
              )
            : null}
        </>
      )}
      {column.resizable !== false && (
        <button
          type="button"
          className="absolute -right-1 top-0 h-full w-2 cursor-col-resize rounded-sm hover:bg-primary/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label={`调整${label}列宽`}
          title="拖动调整列宽，双击恢复默认"
          onClick={(event) => event.stopPropagation()}
          onDoubleClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onResetColumnWidth(column.key);
          }}
          onPointerDown={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onStartResize(event.currentTarget, column, event.clientX);
          }}
          onKeyDown={(event) => {
            if (event.key === "Home") {
              event.preventDefault();
              onResetColumnWidth(column.key);
              return;
            }

            if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
            event.preventDefault();
            onNudgeColumnWidth(event.currentTarget, column, event.key === "ArrowLeft" ? -16 : 16);
          }}
        />
      )}
    </div>
  );
}

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
