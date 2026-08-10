import * as React from "react";
import { X } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { DataTable, type DataTableColumn, type DataTableProps } from "../DataTable";
import { DataGridActionSettingsPanel, type DataGridActionSettingItem } from "./DataGridActionSettingsPanel";
import { DataGridContextMenu } from "./DataGridContextMenu";
import { DataGridExportDialog } from "./DataGridExportDialog";
import { DataGridFieldSettingsPanel } from "./DataGridFieldSettingsPanel";
import { DataGridFilterChips } from "./DataGridFilterChips";
import { DataGridPaginationFooter } from "./DataGridPaginationFooter";
import { DataGridSummaryDialog, type DataGridSummaryConfig } from "./DataGridSummaryDialog";
import type { DataGridExportFormat } from "./data-grid-export";
import type { DataGridFilterSummaryField } from "./data-grid-filter-summary";
import type {
  DataGridColumnFilters,
  DataGridFloatingPanelPosition,
  DataGridPageResult,
} from "./data-grid-logic";
import type { DataGridSummaryTableResult, DataGridSummaryTableRow } from "./data-grid-summary";
import type {
  DataGridColumn,
  DataGridContextMenuState,
  DataGridQuerySummaryItem,
  DataGridServerPagination,
} from "./data-grid-types";

/**
 * DataGridContent — 渲染 DataGrid 主体表格、筛选状态和浮层
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端 DataGrid 横向能力
 * Wave：Wave 6 管理端表格增强
 * 业务约束：只接收上层计算后的数据和回调，不持有业务状态。
 *
 * @example
 *   <DataGridContent page={page} finalColumns={columns} rowKey={(row) => row.id} />
 */
export interface DataGridContentProps<T> {
  className?: string;
  tableClassName?: string;
  tableStyle: React.CSSProperties;
  summaryTableStyle: React.CSSProperties;
  /** 表格区域最大高度（自建垂直滚动容器）；string 直接作 max-height，number 按 px */
  maxHeight?: string | number;
  caption?: React.ReactNode;
  emptyTitle?: React.ReactNode;
  emptyDescription?: React.ReactNode;
  summaryTable: DataGridSummaryTableResult | null;
  summaryColumns: DataTableColumn<DataGridSummaryTableRow>[];
  finalColumns: DataTableColumn<T>[];
  page: DataGridPageResult<T>;
  rowKey: (row: T) => string;
  selectedKey?: string;
  onRowClick?: DataTableProps<T>["onRowClick"];
  selectable: boolean;
  selectedCount: number;
  pageSize: number;
  pageSizeOptions: number[];
  onExitSummary: () => void;
  onPageSizeChange: (pageSize: number) => void;
  onPageIndexChange: (pageIndex: number) => void;
  onClearSelected: () => void;
  /** 服务端分页受控模式透传给页脚；未提供时页脚走现有内存分页分支 */
  serverPagination?: DataGridServerPagination;
  columnFilters: DataGridColumnFilters;
  filterSummaryFields: DataGridFilterSummaryField[];
  onClearColumnFilter: (key: string) => void;
  onClearColumnFilters: () => void;
  querySummaryItems: DataGridQuerySummaryItem[];
  onClearQueryState?: () => void;
  fieldsOpen: boolean;
  fieldListId: string;
  fieldsPanelPosition: DataGridFloatingPanelPosition | null;
  hideableColumns: DataGridColumn<T>[];
  visibleKeys: Set<string>;
  copyableKeys: Set<string>;
  frozenKeys: Set<string>;
  visibleHideableCount: number;
  draggingColumnKey: string | null;
  onDraggingColumnKeyChange: (key: string | null) => void;
  onColumnVisibleChange: (key: string, visible: boolean) => void;
  onColumnCopyableChange: (key: string, copyable: boolean) => void;
  onColumnFrozenChange: (key: string, frozen: boolean) => void;
  onMoveColumn: (key: string, beforeKey: string) => void;
  onMoveColumnByStep: (key: string, step: -1 | 1) => void;
  actionSettingsOpen: boolean;
  actionSettingsPanelId: string;
  actionSettingsPanelPosition: DataGridFloatingPanelPosition | null;
  actionSettingItems: DataGridActionSettingItem[];
  onActionVisibleChange: (key: string, visible: boolean) => void;
  contextMenu: DataGridContextMenuState | null;
  areaSelectionEnabled: boolean;
  hasSelectedArea: boolean;
  selectedAreaSumText: string | null;
  canPaste: boolean;
  canColumnPaste: boolean;
  onCloseContextMenu: () => void;
  onCopyRow: () => void;
  onCopyRowWithHeader: () => void;
  onPaste: () => void;
  onColumnPaste: () => void;
  onStartAreaSelection: () => void;
  onCloseAreaSelection: () => void;
  onCopyArea: () => void;
  onCopyAreaWithHeader: () => void;
  onCopyAreaSum: () => void;
  summaryOpen: boolean;
  summaryColumnsSource: DataGridColumn<T>[];
  onSummaryOpenChange: (open: boolean) => void;
  onApplySummary: (config: DataGridSummaryConfig) => void;
  exportOpen: boolean;
  exportFileName: string;
  exportFormat: DataGridExportFormat;
  onExportOpenChange: (open: boolean) => void;
  onExportFileNameChange: (fileName: string) => void;
  onExportFormatChange: (format: DataGridExportFormat) => void;
  onConfirmExport: () => void;
}

export function DataGridContent<T>({
  tableClassName,
  tableStyle,
  summaryTableStyle,
  maxHeight,
  caption,
  emptyTitle,
  emptyDescription,
  summaryTable,
  summaryColumns,
  finalColumns,
  page,
  rowKey,
  selectedKey,
  onRowClick,
  selectable,
  selectedCount,
  pageSize,
  pageSizeOptions,
  onExitSummary,
  onPageSizeChange,
  onPageIndexChange,
  onClearSelected,
  serverPagination,
  columnFilters,
  filterSummaryFields,
  onClearColumnFilter,
  onClearColumnFilters,
  querySummaryItems,
  onClearQueryState,
  fieldsOpen,
  fieldListId,
  fieldsPanelPosition,
  hideableColumns,
  visibleKeys,
  copyableKeys,
  frozenKeys,
  visibleHideableCount,
  draggingColumnKey,
  onDraggingColumnKeyChange,
  onColumnVisibleChange,
  onColumnCopyableChange,
  onColumnFrozenChange,
  onMoveColumn,
  onMoveColumnByStep,
  actionSettingsOpen,
  actionSettingsPanelId,
  actionSettingsPanelPosition,
  actionSettingItems,
  onActionVisibleChange,
  contextMenu,
  areaSelectionEnabled,
  hasSelectedArea,
  selectedAreaSumText,
  canPaste,
  canColumnPaste,
  onCloseContextMenu,
  onCopyRow,
  onCopyRowWithHeader,
  onPaste,
  onColumnPaste,
  onStartAreaSelection,
  onCloseAreaSelection,
  onCopyArea,
  onCopyAreaWithHeader,
  onCopyAreaSum,
  summaryOpen,
  summaryColumnsSource,
  onSummaryOpenChange,
  onApplySummary,
  exportOpen,
  exportFileName,
  exportFormat,
  onExportOpenChange,
  onExportFileNameChange,
  onExportFormatChange,
  onConfirmExport,
}: DataGridContentProps<T>) {
  return (
    <>
      {/* 自建垂直滚动容器：flex 撑满父容器时精确占剩余；父无高度约束时 max-h 兜底限制（页面级不滚动） */}
      <div
        className="min-h-0 flex-1 overflow-auto max-h-[calc(100vh-23rem)]"
        style={maxHeight !== undefined ? { maxHeight } : undefined}
      >
        {summaryTable ? (
          <>
            <div className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-primary/30 bg-primary/5 px-3 py-2 text-sm text-primary">
              <span>已显示汇总结果，共 {summaryTable.rows.length} 个分组</span>
              <Button type="button" variant="outline" size="sm" className="h-8" onClick={onExitSummary}>
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
                selectedCount={selectedCount}
                pageSize={pageSize}
                pageSizeOptions={pageSizeOptions}
                pageIndex={page.pageIndex}
                pageCount={page.pageCount}
                onPageSizeChange={onPageSizeChange}
                onPageIndexChange={onPageIndexChange}
                onClearSelected={onClearSelected}
                serverPagination={serverPagination}
              />
            }
          />
        )}
      </div>
      <DataGridFilterChips
        className="border-primary/30 bg-primary/5 text-primary"
        filters={columnFilters}
        fields={filterSummaryFields}
        onClearFilter={onClearColumnFilter}
        onClearAll={onClearColumnFilters}
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
        onDraggingColumnKeyChange={onDraggingColumnKeyChange}
        onColumnVisibleChange={onColumnVisibleChange}
        onColumnCopyableChange={onColumnCopyableChange}
        onColumnFrozenChange={onColumnFrozenChange}
        onMoveColumn={onMoveColumn}
        onMoveColumnByStep={onMoveColumnByStep}
      />
      <DataGridActionSettingsPanel
        open={actionSettingsOpen}
        panelId={actionSettingsPanelId}
        position={actionSettingsPanelPosition}
        actions={actionSettingItems}
        onActionVisibleChange={onActionVisibleChange}
      />
      <DataGridContextMenu
        open={Boolean(contextMenu)}
        position={contextMenu}
        areaSelectionEnabled={areaSelectionEnabled}
        hasSelectedArea={hasSelectedArea}
        areaSumText={selectedAreaSumText}
        canPaste={canPaste}
        canColumnPaste={canColumnPaste}
        onClose={onCloseContextMenu}
        onCopyRow={onCopyRow}
        onCopyRowWithHeader={onCopyRowWithHeader}
        onPaste={onPaste}
        onColumnPaste={onColumnPaste}
        onStartAreaSelection={onStartAreaSelection}
        onCloseAreaSelection={onCloseAreaSelection}
        onCopyArea={onCopyArea}
        onCopyAreaWithHeader={onCopyAreaWithHeader}
        onCopyAreaSum={onCopyAreaSum}
      />
      <DataGridSummaryDialog
        open={summaryOpen}
        columns={summaryColumnsSource}
        onOpenChange={onSummaryOpenChange}
        onApply={onApplySummary}
      />
      <DataGridExportDialog
        open={exportOpen}
        fileName={exportFileName}
        format={exportFormat}
        rowCount={page.filteredRows.length}
        onOpenChange={onExportOpenChange}
        onFileNameChange={onExportFileNameChange}
        onFormatChange={onExportFormatChange}
        onConfirm={onConfirmExport}
      />
    </>
  );
}
