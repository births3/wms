import * as React from "react";
import { cn } from "../../lib/utils";
import { Checkbox } from "../../ui/checkbox";
import type { DataTableColumn } from "../DataTable";
import { DataGridCellContent } from "./DataGridCellContent";
import { DataGridHeaderCell } from "./DataGridHeaderCell";
import { columnLabel, defaultCellContent } from "./data-grid-helpers";
import {
  dataGridFilterConfigForData,
  getDataGridCopyText,
  type DataGridColumnFilterValue,
  type DataGridColumnFilters,
  type DataGridLogicState,
} from "./data-grid-logic";
import type { DataGridAreaBounds, DataGridColumn } from "./data-grid-types";

/**
 * DataGridColumns — 构造 DataGrid 的 DataTable 列定义
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端 DataGrid 横向能力
 * Wave：Wave 6 管理端表格增强
 * 业务约束：只封装列渲染、冻结列、复制、字段筛选和行选择接线。
 *
 * @example
 *   buildDataGridColumns({ visibleColumns, data, rowKey, selectable })
 */
export interface BuildDataGridColumnsOptions<T> {
  visibleColumns: DataGridColumn<T>[];
  data: T[];
  rowKey: (row: T) => string;
  copyableKeys: Set<string>;
  columnWidths: Record<string, number>;
  defaultColumnWidth: number;
  frozenColumnOffsets: Record<string, number | string>;
  areaSelectionEnabled: boolean;
  selectedAreaBounds: DataGridAreaBounds | null;
  sort: DataGridLogicState["sort"];
  columnFilters: DataGridColumnFilters;
  openFilterKey: string | null;
  copyNotice: { cellKey: string; text: string } | null;
  selectable: boolean;
  allPageSelected: boolean;
  selectedPageCount: number;
  pageRowKeys: string[];
  selectedKeySet: Set<string>;
  onOpenContextMenu: (event: React.MouseEvent, rowIndex: number, columnIndex: number) => void;
  onStartCellAreaSelection: (event: React.MouseEvent, rowIndex: number, columnIndex: number) => void;
  onUpdateCellAreaSelection: (rowIndex: number, columnIndex: number) => void;
  onCopyCellValue: (row: T, column: DataGridColumn<T>, cellKey: string) => void;
  onSort: (column: DataGridColumn<T>) => void;
  onToggleFilter: (key: string) => void;
  onFilterChange: (key: string, value: DataGridColumnFilterValue) => void;
  onCloseFilter: () => void;
  onResetColumnWidth: (key: string) => void;
  onStartResize: (handle: HTMLElement, column: DataGridColumn<T>, clientX: number) => void;
  onNudgeColumnWidth: (handle: HTMLElement, column: DataGridColumn<T>, delta: number) => void;
  onPageSelected: (selected: boolean) => void;
  onRowSelected: (key: string, selected: boolean) => void;
}

export function buildDataGridColumns<T>({
  visibleColumns,
  data,
  rowKey,
  copyableKeys,
  columnWidths,
  defaultColumnWidth,
  frozenColumnOffsets,
  areaSelectionEnabled,
  selectedAreaBounds,
  sort,
  columnFilters,
  openFilterKey,
  copyNotice,
  selectable,
  allPageSelected,
  selectedPageCount,
  pageRowKeys,
  selectedKeySet,
  onOpenContextMenu,
  onStartCellAreaSelection,
  onUpdateCellAreaSelection,
  onCopyCellValue,
  onSort,
  onToggleFilter,
  onFilterChange,
  onCloseFilter,
  onResetColumnWidth,
  onStartResize,
  onNudgeColumnWidth,
  onPageSelected,
  onRowSelected,
}: BuildDataGridColumnsOptions<T>): DataTableColumn<T>[] {
  const tableColumns: DataTableColumn<T>[] = visibleColumns.map((column, columnIndex) => {
    const sourceRender = column.render;
    const columnCanCopy = column.copyable !== false && copyableKeys.has(column.key);
    const columnWidth = columnWidths[column.key] ?? column.width ?? defaultColumnWidth;
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
        onContextMenu: (event) => onOpenContextMenu(event, rowIndex, columnIndex),
        onMouseDown: (event) => onStartCellAreaSelection(event, rowIndex, columnIndex),
        onMouseEnter: () => onUpdateCellAreaSelection(rowIndex, columnIndex),
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
            onCopy={() => onCopyCellValue(row, column, cellKey)}
            onDoubleClick={column.onDoubleClick}
          />
        );
      },
      header: (
        <DataGridHeaderCell
          column={column}
          sort={sort}
          filter={dataGridFilterConfigForData(column, data)}
          filterValue={columnFilters[column.key]}
          filterOpen={openFilterKey === column.key}
          onSort={onSort}
          onToggleFilter={onToggleFilter}
          onFilterChange={onFilterChange}
          onCloseFilter={onCloseFilter}
          onResetColumnWidth={onResetColumnWidth}
          onStartResize={onStartResize}
          onNudgeColumnWidth={onNudgeColumnWidth}
        />
      ),
    };
  });

  if (!selectable) return tableColumns;

  return [
    {
      key: "__select",
      header: (
        <Checkbox
          checked={allPageSelected || (selectedPageCount > 0 ? "indeterminate" : false)}
          disabled={pageRowKeys.length === 0}
          aria-label="选择当前页"
          onCheckedChange={(value) => onPageSelected(value === true)}
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
            onCheckedChange={(value) => onRowSelected(key, value === true)}
          />
        );
      },
    },
    ...tableColumns,
  ];
}
