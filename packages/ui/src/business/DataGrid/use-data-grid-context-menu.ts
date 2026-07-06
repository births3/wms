import * as React from "react";
import {
  buildDataGridClipboardText,
  buildDataGridSelectedAreaSumText,
  contextMenuPosition,
  normalizedAreaBounds,
  readClipboardText,
  resolveDataGridPasteDisabled,
  selectedAreaPayload,
  writeClipboardText,
} from "./data-grid-helpers";
import type {
  DataGridAreaSelection,
  DataGridColumn,
  DataGridContextMenuState,
  DataGridPasteAction,
  DataGridPasteTarget,
} from "./data-grid-types";

export interface UseDataGridContextMenuOptions<T> {
  rows: T[];
  visibleColumns: DataGridColumn<T>[];
  selectedKeys: string[];
  pasteAction?: DataGridPasteAction<T>;
  columnPasteAction?: DataGridPasteAction<T>;
}

export function useDataGridContextMenu<T>({
  rows,
  visibleColumns,
  selectedKeys,
  pasteAction,
  columnPasteAction,
}: UseDataGridContextMenuOptions<T>) {
  const [areaSelectionEnabled, setAreaSelectionEnabled] = React.useState(false);
  const [areaSelection, setAreaSelection] = React.useState<DataGridAreaSelection | null>(null);
  const [areaSelecting, setAreaSelecting] = React.useState(false);
  const [contextMenu, setContextMenu] = React.useState<DataGridContextMenuState | null>(null);
  const selectedAreaBounds = areaSelection ? normalizedAreaBounds(areaSelection) : null;
  const selectedAreaSumText = selectedAreaBounds
    ? buildDataGridSelectedAreaSumText(
        visibleColumns.slice(selectedAreaBounds.left, selectedAreaBounds.right + 1),
        rows.slice(selectedAreaBounds.top, selectedAreaBounds.bottom + 1),
      )
    : null;

  React.useEffect(() => {
    if (!areaSelecting) return;
    const stopSelecting = () => setAreaSelecting(false);
    window.addEventListener("mouseup", stopSelecting, { once: true });
    return () => window.removeEventListener("mouseup", stopSelecting);
  }, [areaSelecting]);

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

  function closeContextMenu() {
    setContextMenu(null);
  }

  function copyContextRow(includeHeader: boolean) {
    if (!contextMenu) return;
    const row = rows[contextMenu.rowIndex];
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
    const selectedRows = rows.slice(selectedAreaBounds.top, selectedAreaBounds.bottom + 1);
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
    const row = rows[contextMenu.rowIndex];
    const column = visibleColumns[contextMenu.columnIndex];
    if (!row || !column) return null;
    return {
      row,
      rowIndex: contextMenu.rowIndex,
      column,
      columnIndex: contextMenu.columnIndex,
      selectedRowKeys: selectedKeys,
      selectedArea: selectedAreaPayload(selectedAreaBounds, rows, visibleColumns),
    };
  }

  const contextPasteTarget = buildPasteTarget();
  const canPaste = Boolean(pasteAction && contextPasteTarget && !resolveDataGridPasteDisabled(pasteAction.disabled, contextPasteTarget));
  const canColumnPaste = Boolean(
    columnPasteAction &&
      contextPasteTarget &&
      !resolveDataGridPasteDisabled(columnPasteAction.disabled, contextPasteTarget),
  );

  return {
    areaSelectionEnabled,
    selectedAreaBounds,
    selectedAreaSumText,
    contextMenu,
    canPaste,
    canColumnPaste,
    openContextMenu,
    startCellAreaSelection,
    updateCellAreaSelection,
    closeContextMenu,
    copyContextRow,
    startAreaSelectionFromContext,
    closeAreaSelectionFromContext,
    copySelectedArea,
    copySelectedAreaSum,
    pasteFromContext,
  };
}
