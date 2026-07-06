import * as React from "react";
import {
  buildDataGridCsv,
  buildDataGridExport,
  dataGridExportFileName,
  defaultDataGridExportFileName,
  downloadDataGridCsv,
  downloadDataGridExport,
  type DataGridExportFormat,
} from "./data-grid-export";
import { resolveDataGridExportBaseName } from "./data-grid-helpers";
import type { DataGridColumn, DataGridCsvExportState } from "./data-grid-types";

export interface UseDataGridExportOptions<T> {
  columns: DataGridColumn<T>[];
  visibleColumns: DataGridColumn<T>[];
  rows: T[];
  storageKey?: string;
  caption?: React.ReactNode;
  exportFileBaseName?: string;
  csvExportPlacement: "toolbar" | "external";
  onCsvExportStateChange?: (state: DataGridCsvExportState | null) => void;
}

export function useDataGridExport<T>({
  columns,
  visibleColumns,
  rows,
  storageKey,
  caption,
  exportFileBaseName,
  csvExportPlacement,
  onCsvExportStateChange,
}: UseDataGridExportOptions<T>) {
  const [exportOpen, setExportOpen] = React.useState(false);
  const [exportFormat, setExportFormat] = React.useState<DataGridExportFormat>("xlsx");
  const [exportFileName, setExportFileName] = React.useState("");
  const snapshotRef = React.useRef<{
    columns: DataGridColumn<T>[];
    visibleColumnKeys: string[];
    rows: T[];
    storageKey: string | undefined;
  } | null>(null);

  snapshotRef.current = {
    columns,
    visibleColumnKeys: visibleColumns.map((column) => column.key),
    rows,
    storageKey,
  };

  const exportCsv = React.useCallback(() => {
    const snapshot = snapshotRef.current;
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
    onCsvExportStateChange({ disabled: rows.length === 0, exportCsv });
    return () => onCsvExportStateChange(null);
  }, [csvExportPlacement, exportCsv, onCsvExportStateChange, rows.length]);

  function openExportDialog() {
    setExportFileName(defaultDataGridExportFileName(resolveDataGridExportBaseName(exportFileBaseName, caption, storageKey)));
    setExportFormat("xlsx");
    setExportOpen(true);
  }

  function confirmExport() {
    const snapshot = snapshotRef.current;
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

  return {
    exportOpen,
    exportFormat,
    exportFileName,
    setExportOpen,
    setExportFormat,
    setExportFileName,
    openExportDialog,
    confirmExport,
  };
}
