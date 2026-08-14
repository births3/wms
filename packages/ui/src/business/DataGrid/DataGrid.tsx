import * as React from "react";
import { cn } from "../../lib/utils";
import type { DataGridSummaryConfig } from "./DataGridSummaryDialog";
import { DataGridAdvancedFilterPanel } from "./DataGridAdvancedFilterPanel";
import { DataGridContent } from "./DataGridContent";
import { DataGridDetailDialog } from "./DataGridDetailDialog";
import type { DataGridActionSettingItem } from "./DataGridActionSettingsPanel";
import { DataGridFilterChips } from "./DataGridFilterChips";
import { DataGridFilterHistory } from "./DataGridFilterHistory";
import { DataGridToolbar } from "./DataGridToolbar";
import {
  getDataGridFilterHistoryStorage,
  loadDataGridFilterHistoryFromStorage,
  recordDataGridFilterHistory,
  saveDataGridFilterHistoryToStorage,
  type DataGridFilterHistoryEntry,
} from "./data-grid-filter-history";
import { clearDataGridFilterKey } from "./data-grid-filter-summary";
import {
  buildDataGridActionDescriptors,
  columnLabel,
  isDataGridColumn,
  summaryDataTableColumns,
  writeClipboardText,
} from "./data-grid-helpers";
import { useDataGridPopoverDismiss } from "./data-grid-popover-dismiss";
import { buildDataGridSummaryTable } from "./data-grid-summary";
import {
  dataGridFloatingPanelPosition,
  dataGridFrozenColumnOffsets,
  dataGridTableWidth,
  dataGridFilterActive,
  dataGridFilterConfigForData,
  getDataGridCopyText,
  getDataGridPage,
  orderedColumnsWithFrozen,
  reconcileDataGridSelectedRowKeys,
  sanitizeGridState,
  sanitizeDataGridColumnFiltersForData,
  toggleHiddenAction,
  type DataGridColumnFilters,
  type DataGridFloatingPanelPosition,
  type DataGridLogicState,
  type DataGridPageResult,
} from "./data-grid-logic";
import { getDataGridPrefetchPageIndexes } from "./data-grid-pagination-prefetch";
import { loadGridSettings, saveGridSettings } from "./data-grid-storage";
import {
  defaultColumnWidth,
  defaultPageSizeOptions,
  type DataGridActionDisabled,
  type DataGridColumn,
  type DataGridDetailAction,
  type DataGridProps,
} from "./data-grid-types";
import { buildDataGridColumns } from "./DataGridColumns";
import { useDataGridColumnSettings } from "./use-data-grid-column-settings";
import { useDataGridContextMenu } from "./use-data-grid-context-menu";
import { useDataGridExport } from "./use-data-grid-export";

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

function DataGridInner<T>(
  {
    columns,
    data,
    rowKey,
    selectedKey,
    onRowClick,
    onRowDoubleClick,
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
    serverPagination,
    showRowNumber = true,
    className,
    tableClassName,
    maxHeight,
    ...rest
  }: DataGridProps<T>,
  ref: React.ForwardedRef<HTMLDivElement>,
) {
  const rootRef = React.useRef<HTMLDivElement | null>(null);
  const fieldButtonRef = React.useRef<HTMLButtonElement | null>(null);
  const actionSettingsButtonRef = React.useRef<HTMLButtonElement | null>(null);
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
  // 内置"查看"：未传 detailAction 时默认提供（通用详情对话框）；双击行同样打开
  const [builtinDetailRow, setBuiltinDetailRow] = React.useState<T | null>(null);
  const effectiveDetailAction: DataGridDetailAction | undefined = detailAction ?? (selectable ? {
    label: "查看",
    description: "查看选中记录详情",
    disabled: (({ selectedRowKeys: keys }: { selectedRowKeys: string[] }) => keys.length !== 1) as DataGridActionDisabled,
    onClick: ({ selectedRowKeys: keys }) => {
      const row = keys.length === 1 ? data.find((item) => rowKey(item) === keys[0]) : undefined;
      if (row) setBuiltinDetailRow(row);
    },
  } : undefined);
  // 行双击：页面未传 onRowDoubleClick 时复用当前“查看”动作
  const handleRowDoubleClick: (row: T, idx: number) => void = onRowDoubleClick
    ? onRowDoubleClick
    : (row: T) => {
        // 双击与"查看"按钮走同一 detailAction（自定义或内置），展示内容一致
        if (effectiveDetailAction) {
          effectiveDetailAction.onClick({ selectedRowKeys: [rowKey(row)] });
        } else {
          setBuiltinDetailRow(row);
        }
      };
  const actionDescriptors = React.useMemo(
    () =>
      buildDataGridActionDescriptors({
        refreshAction,
        queryAction,
        createAction,
        detailAction: effectiveDetailAction,
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
      effectiveDetailAction,
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
  // 最近筛选历史：自动记录激活筛选组合，最多 5 条，随 settings 保存/恢复
  const [filterHistory, setFilterHistory] = React.useState<DataGridFilterHistoryEntry[]>(() =>
    loadDataGridFilterHistoryFromStorage(getDataGridFilterHistoryStorage(), storageKey),
  );
  const [internalSelectedRowKeys, setInternalSelectedRowKeys] = React.useState<string[]>([]);
  const [draggingColumnKey, setDraggingColumnKey] = React.useState<string | null>(null);
  const [copyNotice, setCopyNotice] = React.useState<{ cellKey: string; text: string } | null>(null);

  const serverPageIndex = serverPagination?.pageIndex;
  const serverPageSize = serverPagination?.pageSize;
  const serverTotal = serverPagination?.total;
  const serverPrefetchPageCount = serverPagination?.prefetchPageCount;
  const onPrefetchPage = serverPagination?.onPrefetchPage;

  React.useEffect(() => {
    if (!onPrefetchPage || serverPageIndex === undefined || serverPageSize === undefined || serverTotal === undefined) {
      return;
    }

    const pageIndexes = getDataGridPrefetchPageIndexes({
      pageIndex: serverPageIndex,
      pageSize: serverPageSize,
      total: serverTotal,
      prefetchCount: serverPrefetchPageCount ?? 0,
    });
    for (const pageIndex of pageIndexes) {
      try {
        void Promise.resolve(onPrefetchPage(pageIndex, serverPageSize)).catch(() => undefined);
      } catch {
        // 预加载是最佳努力，不阻断当前页展示。
      }
    }
  }, [data, onPrefetchPage, serverPageIndex, serverPageSize, serverTotal, serverPrefetchPageCount]);

  React.useImperativeHandle(ref, () => rootRef.current as HTMLDivElement);

  React.useEffect(() => {
    setSettings((current) => sanitizeGridState(current, columns, safePageSizeOptions, defaultPageSize, actionKeys));
  }, [actionSignature, columnSignature, defaultPageSize, safePageSizeOptions]);

  React.useEffect(() => {
    saveGridSettings(storageKey, settings);
  }, [settings, storageKey]);

  // 最近筛选自动记录：筛选组合变化且存在激活筛选时防抖 500ms 记录；
  // 初始加载/清空筛选（空组合）不记录，记录是筛选流程的旁路。
  React.useEffect(() => {
    const hasActiveFilters = Object.keys(settings.columnFilters).some((key) =>
      dataGridFilterActive(settings.columnFilters[key]),
    );
    if (!hasActiveFilters) return;

    const timer = window.setTimeout(() => {
      setFilterHistory((current) =>
        recordDataGridFilterHistory(current, settings.columnFilters, new Date().toISOString()),
      );
    }, 500);
    return () => window.clearTimeout(timer);
  }, [settings.columnFilters]);

  React.useEffect(() => {
    saveDataGridFilterHistoryToStorage(getDataGridFilterHistoryStorage(), storageKey, filterHistory);
  }, [filterHistory, storageKey]);

  const [advancedFilterOpen, setAdvancedFilterOpen] = React.useState(false);

  React.useEffect(() => {
    setPageIndex(0);
  }, [settings.columnFilters, settings.advancedFilters, data, settings.pageSize, settings.sort?.key, settings.sort?.direction]);

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

  const internalPage = getDataGridPage({
    data,
    columns,
    visibleColumns: settings.visibleColumns,
    columnFilters: settings.columnFilters,
    advancedFilters: settings.advancedFilters,
    sort: settings.sort,
    pageIndex,
    pageSize: settings.pageSize,
  });
  // 服务端分页受控模式：data 已是服务端返回的当前页数据，DataGrid 不自行切页（rows 取完整过滤/排序结果），分页展示与翻页事件由页脚 serverPagination 驱动
  const page: DataGridPageResult<T> = serverPagination
    ? {
        ...internalPage,
        rows: internalPage.filteredRows,
        pageIndex: serverPagination.pageIndex,
        total: serverPagination.total,
      }
    : internalPage;
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
  // 默认序号列（行号，跨页连续）；hideable false 不参与字段面板/筛选/排序
  const rowNumberColumn: DataGridColumn<T> = {
    key: "__rowNumber",
    header: "#",
    width: 56,
    minWidth: 48,
    hideable: false,
    copyable: false,
    sortable: false,
    filter: false,
    align: "center",
    className: "text-xs text-muted-foreground",
    render: (_row, idx) => (serverPagination?.pageIndex ?? pageIndex) * (serverPagination?.pageSize ?? settings.pageSize) + idx + 1,
  };
  const effectiveVisibleColumns = showRowNumber ? [rowNumberColumn, ...visibleColumns] : visibleColumns;
  const frozenColumnOffsets = dataGridFrozenColumnOffsets(visibleColumns, frozenKeys, defaultColumnWidth);
  const summaryTable = summaryConfig
    ? buildDataGridSummaryTable(visibleColumns, page.filteredRows, summaryConfig.groupColumnKeys, summaryConfig.selections)
    : null;
  const summaryColumns = summaryTable ? summaryDataTableColumns(summaryTable.columns) : [];
  const {
    exportOpen,
    exportFormat,
    exportFileName,
    setExportOpen,
    setExportFormat,
    setExportFileName,
    openExportDialog,
    confirmExport,
  } = useDataGridExport({
    columns,
    visibleColumns,
    rows: page.filteredRows,
    storageKey,
    caption,
    exportFileBaseName,
    csvExportPlacement,
    onCsvExportStateChange,
  });
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
  // 最近筛选摘要覆盖全部可筛选列（含当前隐藏列），保证历史条目始终可读
  const filterHistoryFields = React.useMemo(
    () =>
      columns
        .filter((column) => column.filter !== false)
        .map((column) => ({
          key: column.key,
          label: columnLabel(column),
          filter: dataGridFilterConfigForData(column, data),
        })),
    [columns, data],
  );
  const visibleAction = React.useCallback((key: string) => !hiddenActionKeys.has(key), [settings.hiddenActions]);
  const hasHiddenToolbarActions = actionDescriptors.some((action) => hiddenActionKeys.has(action.key));
  const actionSettingItems: DataGridActionSettingItem[] = actionDescriptors.map((action) => ({
    ...action,
    visible: visibleAction(action.key),
  }));
  const columnSettings = useDataGridColumnSettings({
    columns,
    hideableColumns,
    settings,
    setSettings,
  });
  const contextMenuState = useDataGridContextMenu({
    rows: page.rows,
    visibleColumns,
    selectedKeys,
    pasteAction,
    columnPasteAction,
  });

  React.useEffect(() => {
    if (!selectable || selectedKeys.length === 0) return;
    const nextKeys = reconcileDataGridSelectedRowKeys(selectedKeys, filteredRowKeys);
    const changed = nextKeys.length !== selectedKeys.length || nextKeys.some((key, index) => key !== selectedKeys[index]);
    if (changed) setSelectedKeys(nextKeys);
  }, [selectable, selectedKeys, filteredRowKeys]);

  function updateActionVisible(key: string, visible: boolean) {
    setSettings((current) => ({
      ...current,
      hiddenActions: toggleHiddenAction(current.hiddenActions, actionKeys, key, visible),
    }));
  }

  function clearColumnFilter(key: string) {
    setSettings((current) => ({
      ...current,
      columnFilters: clearDataGridFilterKey(current.columnFilters, key),
    }));
  }

  function clearColumnFilters() {
    setSettings((current) => ({ ...current, columnFilters: {} }));
  }

  // 一键复用最近筛选：恢复筛选组合并回到第一页
  function applyFilterHistory(filters: DataGridColumnFilters) {
    setSettings((current) => ({ ...current, columnFilters: filters }));
    setPageIndex(0);
  }

  function clearFilterHistory() {
    setFilterHistory([]);
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

  function applyNamedViewState(state: DataGridLogicState, nextQueryState?: unknown) {
    setSettings(state);
    if (nextQueryState !== undefined) onApplyQueryState?.(nextQueryState);
    setPageIndex(0);
  }

  const finalColumns = buildDataGridColumns({
    visibleColumns: effectiveVisibleColumns,
    data,
    rowKey,
    copyableKeys,
    columnWidths: settings.columnWidths,
    defaultColumnWidth,
    frozenColumnOffsets,
    areaSelectionEnabled: contextMenuState.areaSelectionEnabled,
    selectedAreaBounds: contextMenuState.selectedAreaBounds,
    sort: settings.sort,
    columnFilters,
    openFilterKey,
    copyNotice,
    selectable,
    allPageSelected,
    selectedPageCount,
    pageRowKeys,
    selectedKeySet,
    onOpenContextMenu: contextMenuState.openContextMenu,
    onStartCellAreaSelection: contextMenuState.startCellAreaSelection,
    onUpdateCellAreaSelection: contextMenuState.updateCellAreaSelection,
    onCopyCellValue: (row, column, cellKey) => void copyCellValue(row, column, cellKey),
    onSort: columnSettings.updateSort,
    onToggleFilter: (key) => {
      setFieldsOpen(false);
      setOpenFilterKey((current) => (current === key ? null : key));
    },
    onFilterChange: columnSettings.updateColumnFilterValue,
    onCloseFilter: () => setOpenFilterKey(null),
    onResetColumnWidth: columnSettings.resetColumnWidth,
    onStartResize: columnSettings.startColumnResize,
    onNudgeColumnWidth: columnSettings.nudgeColumnWidth,
    onPageSelected: updatePageSelected,
    onRowSelected: updateRowSelected,
  });
  const finalTableWidth = dataGridTableWidth(finalColumns);
  const summaryTableWidth = summaryColumns.length > 0 ? dataGridTableWidth(summaryColumns) : finalTableWidth;
  // 动态：表格宽度由当前显示列、用户拖动列宽和字符串列宽共同计算。
  const tableStyle = { width: finalTableWidth, minWidth: finalTableWidth };
  const summaryTableStyle = { width: summaryTableWidth, minWidth: summaryTableWidth };

  const hasActiveAdvancedFilters = Boolean(
    settings.advancedFilters?.items && settings.advancedFilters.items.length > 0,
  );
  const hasActiveFilters =
    Object.keys(columnFilters).some((key) => dataGridFilterActive(columnFilters[key])) ||
    hasActiveAdvancedFilters;

  return (
    // flex 撑满父容器：工具栏固定、表格区占剩余空间，数据少时保持最小撑开高度
    <div ref={rootRef} className={cn("flex h-full min-h-[380px] flex-col gap-3", className)} {...rest}>
      {hasActiveFilters || filterHistory.length > 0 ? (
        <div className="flex flex-wrap items-start gap-2">
          <DataGridFilterChips
            className="min-w-0 flex-1 border border-border bg-card text-foreground shadow-sm"
            filters={columnFilters}
            advancedFilters={settings.advancedFilters}
            fields={filterSummaryFields}
            onClearFilter={(key) => {
              if (key.startsWith("adv-")) {
                const advId = key.replace("adv-", "");
                setSettings((current) => ({
                  ...current,
                  advancedFilters: {
                    joinOperator: current.advancedFilters?.joinOperator ?? "and",
                    items: (current.advancedFilters?.items ?? []).filter((item) => item.id !== advId),
                  },
                }));
                setPageIndex(0);
                return;
              }
              clearColumnFilter(key);
            }}
            onClearAll={() => {
              clearColumnFilters();
              setSettings((current) => ({
                ...current,
                advancedFilters: { joinOperator: "and", items: [] },
              }));
              setPageIndex(0);
            }}
          />
          {filterHistory.length > 0 ? (
            <DataGridFilterHistory
              entries={filterHistory}
              fields={filterHistoryFields}
              onApply={applyFilterHistory}
              onClear={clearFilterHistory}
            />
          ) : null}
        </div>
      ) : null}
      <DataGridToolbar
        refreshAction={refreshAction}
        queryAction={queryAction}
        createAction={createAction}
        detailAction={effectiveDetailAction}
        editAction={editAction}
        deleteAction={deleteAction}
        disableAction={disableAction}
        printAction={printAction}
        exportAction={exportAction}
        toolbarActions={toolbarActions}
        showPrintAction={showPrintAction}
        showExportAction={showExportAction}
        csvExportPlacement={csvExportPlacement}
        pageFilteredRowCount={page.filteredRows.length}
        toolbarActionContext={toolbarActionContext}
        visibleAction={visibleAction}
        storageKey={storageKey}
        columns={columns}
        actionKeys={actionKeys}
        pageSizeOptions={safePageSizeOptions}
        defaultPageSize={defaultPageSize}
        settings={settings}
        queryState={queryState}
        fieldButtonRef={fieldButtonRef}
        actionSettingsButtonRef={actionSettingsButtonRef}
        fieldsOpen={fieldsOpen}
        fieldListId={fieldListId}
        actionSettingsOpen={actionSettingsOpen}
        actionSettingsPanelId={actionSettingsPanelId}
        hideableColumnCount={hideableColumns.length}
        actionSettingCount={actionSettingItems.length}
        hasHiddenToolbarActions={hasHiddenToolbarActions}
        advancedFilterOpen={advancedFilterOpen}
        activeAdvancedFilterCount={settings.advancedFilters?.items?.length ?? 0}
        onApplyView={applyNamedViewState}
        onToggleFields={() => {
          setActionSettingsOpen(false);
          setOpenFilterKey(null);
          setFieldsOpen((open) => !open);
        }}
        onToggleActionSettings={() => {
          setFieldsOpen(false);
          setOpenFilterKey(null);
          setActionSettingsOpen((open) => !open);
        }}
        onToggleAdvancedFilter={() => {
          setFieldsOpen(false);
          setActionSettingsOpen(false);
          setAdvancedFilterOpen((open) => !open);
        }}
        onOpenSummary={() => setSummaryOpen(true)}
        onOpenExportDialog={openExportDialog}
      />
      {advancedFilterOpen && (
        <DataGridAdvancedFilterPanel
          columns={columns}
          state={settings.advancedFilters}
          onStateChange={(nextAdvancedFilters) => {
            setSettings((current) => ({ ...current, advancedFilters: nextAdvancedFilters }));
            setPageIndex(0);
          }}
          onClose={() => setAdvancedFilterOpen(false)}
          onReset={() => {
            setSettings((current) => ({
              ...current,
              advancedFilters: { joinOperator: "and", items: [] },
            }));
            setPageIndex(0);
          }}
        />
      )}
      <DataGridContent
        tableClassName={tableClassName}
        tableStyle={tableStyle}
        summaryTableStyle={summaryTableStyle}
        maxHeight={maxHeight}
        caption={caption}
        emptyTitle={emptyTitle}
        emptyDescription={emptyDescription}
        summaryTable={summaryTable}
        summaryColumns={summaryColumns}
        finalColumns={finalColumns}
        page={page}
        rowKey={rowKey}
        selectedKey={selectedKey}
        onRowClick={onRowClick}
        onRowDoubleClick={handleRowDoubleClick}
        selectable={selectable}
        selectedCount={selectedKeys.length}
        pageSize={settings.pageSize}
        pageSizeOptions={safePageSizeOptions}
        serverPagination={serverPagination}
        onExitSummary={() => setSummaryConfig(null)}
        onPageSizeChange={(pageSize) => setSettings((current) => ({ ...current, pageSize }))}
        onPageIndexChange={setPageIndex}
        onClearSelected={() => setSelectedKeys([])}
        querySummaryItems={querySummaryItems}
        onClearQueryState={onClearQueryState}
        fieldsOpen={fieldsOpen}
        fieldListId={fieldListId}
        fieldsPanelPosition={fieldsPanelPosition}
        hideableColumns={hideableColumns}
        visibleKeys={visibleKeys}
        copyableKeys={copyableKeys}
        frozenKeys={frozenKeys}
        visibleHideableCount={visibleHideableCount}
        draggingColumnKey={draggingColumnKey}
        onDraggingColumnKeyChange={setDraggingColumnKey}
        onColumnVisibleChange={columnSettings.updateColumnVisible}
        onColumnCopyableChange={columnSettings.updateColumnCopyable}
        onColumnFrozenChange={columnSettings.updateColumnFrozen}
        onMoveColumn={columnSettings.moveColumn}
        onMoveColumnByStep={columnSettings.moveColumnByStep}
        actionSettingsOpen={actionSettingsOpen}
        actionSettingsPanelId={actionSettingsPanelId}
        actionSettingsPanelPosition={actionSettingsPanelPosition}
        actionSettingItems={actionSettingItems}
        onActionVisibleChange={updateActionVisible}
        contextMenu={contextMenuState.contextMenu}
        areaSelectionEnabled={contextMenuState.areaSelectionEnabled}
        hasSelectedArea={Boolean(contextMenuState.selectedAreaBounds)}
        selectedAreaSumText={contextMenuState.selectedAreaSumText}
        canPaste={contextMenuState.canPaste}
        canColumnPaste={contextMenuState.canColumnPaste}
        onCloseContextMenu={contextMenuState.closeContextMenu}
        onCopyRow={() => contextMenuState.copyContextRow(false)}
        onCopyRowWithHeader={() => contextMenuState.copyContextRow(true)}
        onPaste={() => void contextMenuState.pasteFromContext("cell")}
        onColumnPaste={() => void contextMenuState.pasteFromContext("column")}
        onStartAreaSelection={contextMenuState.startAreaSelectionFromContext}
        onCloseAreaSelection={contextMenuState.closeAreaSelectionFromContext}
        onCopyArea={() => contextMenuState.copySelectedArea(false)}
        onCopyAreaWithHeader={() => contextMenuState.copySelectedArea(true)}
        onCopyAreaSum={contextMenuState.copySelectedAreaSum}
        summaryOpen={summaryOpen}
        summaryColumnsSource={visibleColumns}
        onSummaryOpenChange={setSummaryOpen}
        onApplySummary={(config) => {
          setSummaryConfig(config);
          setPageIndex(0);
          setSelectedKeys([]);
        }}
        exportOpen={exportOpen}
        exportFileName={exportFileName}
        exportFormat={exportFormat}
        onExportOpenChange={setExportOpen}
        onExportFileNameChange={setExportFileName}
        onExportFormatChange={setExportFormat}
        onConfirmExport={confirmExport}
      />
      <DataGridDetailDialog
        row={builtinDetailRow}
        columns={visibleColumns}
        rowKey={rowKey}
        open={builtinDetailRow !== null}
        onOpenChange={(open) => { if (!open) setBuiltinDetailRow(null); }}
      />
    </div>
  );
}

const DataGridWithRef = React.forwardRef(DataGridInner) as <T>(
  props: DataGridProps<T> & React.RefAttributes<HTMLDivElement>,
) => React.ReactElement | null;

(DataGridWithRef as { displayName?: string }).displayName = "DataGrid";

export { DataGridWithRef as DataGrid };
