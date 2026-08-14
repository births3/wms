import * as React from "react";
import {
  dataGridFilterActive,
  moveColumnBefore,
  nextSortState,
  setColumnWidth,
  toggleCopyableColumn,
  toggleFrozenColumn,
  toggleVisibleColumn,
  type DataGridColumnFilterValue,
  type DataGridLogicState,
} from "./data-grid-logic";
import { currentColumnWidth } from "./data-grid-helpers";
import type { DataGridColumn } from "./data-grid-types";

export interface UseDataGridColumnSettingsOptions<T> {
  columns: DataGridColumn<T>[];
  hideableColumns: DataGridColumn<T>[];
  settings: DataGridLogicState;
  setSettings: React.Dispatch<React.SetStateAction<DataGridLogicState>>;
}

export function useDataGridColumnSettings<T>({
  columns,
  hideableColumns,
  settings,
  setSettings,
}: UseDataGridColumnSettingsOptions<T>) {
  const columnsRef = React.useRef(columns);
  const [resizingColumn, setResizingColumn] = React.useState<{ key: string; startX: number; startWidth: number } | null>(null);

  React.useEffect(() => {
    columnsRef.current = columns;
  }, [columns]);

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
  }, [resizingColumn, setSettings]);

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

  return {
    updateSort,
    updateColumnVisible,
    updateColumnCopyable,
    updateColumnFrozen,
    moveColumn,
    moveColumnByStep,
    resetColumnWidth,
    startColumnResize,
    nudgeColumnWidth,
    updateColumnFilterValue,
  };
}
