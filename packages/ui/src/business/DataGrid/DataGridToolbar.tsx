import * as React from "react";
import {
  Ban,
  Calculator,
  Download,
  Eye,
  Filter,
  ListChecks,
  Pencil,
  Plus,
  Printer,
  RefreshCw,
  Search,
  Settings2,
  Trash2,
} from "lucide-react";
import { Button } from "../../ui/button";
import { DataGridNamedViewsToolbar } from "./DataGridNamedViewsToolbar";
import { resolveDataGridActionDisabled, toolbarActionKey } from "./data-grid-helpers";
import type { DataGridLogicState } from "./data-grid-logic";
import type {
  DataGridColumn,
  DataGridCreateAction,
  DataGridDeleteAction,
  DataGridDetailAction,
  DataGridDisableAction,
  DataGridEditAction,
  DataGridExportAction,
  DataGridPrintAction,
  DataGridQueryAction,
  DataGridRefreshAction,
  DataGridToolbarAction,
  DataGridToolbarActionContext,
} from "./data-grid-types";

/**
 * DataGridToolbar — 渲染 DataGrid 标准工具栏和按钮显示设置入口
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端 DataGrid 横向能力
 * Wave：Wave 6 管理端表格增强
 * 业务约束：只负责工具栏按钮展示和触发，不保存业务数据。
 *
 * @example
 *   <DataGridToolbar toolbarActions={actions} visibleAction={(key) => true} />
 */
export interface DataGridToolbarProps<T> {
  className?: string;
  refreshAction?: DataGridRefreshAction;
  queryAction?: DataGridQueryAction;
  createAction?: DataGridCreateAction;
  detailAction?: DataGridDetailAction;
  editAction?: DataGridEditAction;
  deleteAction?: DataGridDeleteAction;
  disableAction?: DataGridDisableAction;
  printAction?: DataGridPrintAction | false;
  exportAction?: DataGridExportAction | false;
  toolbarActions: DataGridToolbarAction[];
  showPrintAction: boolean;
  showExportAction: boolean;
  csvExportPlacement: "toolbar" | "external";
  pageFilteredRowCount: number;
  toolbarActionContext: DataGridToolbarActionContext;
  visibleAction: (key: string) => boolean;
  storageKey?: string;
  columns: DataGridColumn<T>[];
  actionKeys: string[];
  pageSizeOptions: number[];
  defaultPageSize: number;
  settings: DataGridLogicState;
  queryState?: unknown;
  fieldButtonRef: React.Ref<HTMLButtonElement>;
  actionSettingsButtonRef: React.Ref<HTMLButtonElement>;
  fieldsOpen: boolean;
  fieldListId: string;
  actionSettingsOpen: boolean;
  actionSettingsPanelId: string;
  hideableColumnCount: number;
  actionSettingCount: number;
  hasHiddenToolbarActions: boolean;
  advancedFilterOpen?: boolean;
  activeAdvancedFilterCount?: number;
  onApplyView: (state: DataGridLogicState, queryState?: unknown) => void;
  onToggleFields: () => void;
  onToggleActionSettings: () => void;
  onToggleAdvancedFilter?: () => void;
  onOpenSummary: () => void;
  onOpenExportDialog: () => void;
}

export function DataGridToolbar<T>({
  refreshAction,
  queryAction,
  createAction,
  detailAction,
  editAction,
  deleteAction,
  disableAction,
  printAction,
  exportAction,
  toolbarActions,
  showPrintAction,
  showExportAction,
  csvExportPlacement,
  pageFilteredRowCount,
  toolbarActionContext,
  visibleAction,
  storageKey,
  columns,
  actionKeys,
  pageSizeOptions,
  defaultPageSize,
  settings,
  queryState,
  fieldButtonRef,
  actionSettingsButtonRef,
  fieldsOpen,
  fieldListId,
  actionSettingsOpen,
  actionSettingsPanelId,
  hideableColumnCount,
  actionSettingCount,
  hasHiddenToolbarActions,
  advancedFilterOpen = false,
  activeAdvancedFilterCount = 0,
  onApplyView,
  onToggleFields,
  onToggleActionSettings,
  onToggleAdvancedFilter,
  onOpenSummary,
  onOpenExportDialog,
}: DataGridToolbarProps<T>) {
  const selectedCount = toolbarActionContext.selectedRowKeys.length;

  return (
    <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2 [&_svg]:size-4">
        {refreshAction && visibleAction("refresh") && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            title={refreshAction.description ?? refreshAction.label ?? "刷新列表"}
            disabled={resolveDataGridActionDisabled(refreshAction.disabled, toolbarActionContext)}
            onClick={() => refreshAction.onClick(toolbarActionContext)}
          >
            <RefreshCw className="size-4" aria-hidden />
            {refreshAction.label ?? "刷新"}
          </Button>
        )}
        {queryAction && visibleAction("query") && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            title={queryAction.description ?? queryAction.label ?? "查询列表"}
            disabled={resolveDataGridActionDisabled(queryAction.disabled, toolbarActionContext)}
            onClick={() => queryAction.onClick(toolbarActionContext)}
          >
            <Search className="size-4" aria-hidden />
            {queryAction.label ?? "查询"}
          </Button>
        )}
        {createAction && visibleAction("create") && (
          <Button
            type="button"
            variant="default"
            size="sm"
            className="h-8 shrink-0"
            title={createAction.description ?? createAction.label ?? "新增记录"}
            disabled={resolveDataGridActionDisabled(createAction.disabled, toolbarActionContext)}
            onClick={() => createAction.onClick(toolbarActionContext)}
          >
            <Plus className="size-4" aria-hidden />
            {createAction.label ?? "新增"}
          </Button>
        )}
        {detailAction && visibleAction("detail") && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            title={detailAction.description ?? detailAction.label ?? "查看详情"}
            disabled={resolveDataGridActionDisabled(detailAction.disabled, toolbarActionContext, selectedCount !== 1)}
            onClick={() => detailAction.onClick(toolbarActionContext)}
          >
            <Eye className="size-4" aria-hidden />
            {detailAction.label ?? "详情"}
          </Button>
        )}
        {editAction && visibleAction("edit") && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            title={editAction.description ?? editAction.label ?? "修改记录"}
            disabled={resolveDataGridActionDisabled(editAction.disabled, toolbarActionContext, selectedCount !== 1)}
            onClick={() => editAction.onClick(toolbarActionContext)}
          >
            <Pencil className="size-4" aria-hidden />
            {editAction.label ?? "修改"}
          </Button>
        )}
        {deleteAction && visibleAction("delete") && (
          <Button
            type="button"
            variant="destructive"
            size="sm"
            className="h-8 shrink-0"
            title={deleteAction.description ?? deleteAction.label ?? "删除记录"}
            disabled={resolveDataGridActionDisabled(deleteAction.disabled, toolbarActionContext, selectedCount === 0)}
            onClick={() => deleteAction.onClick(toolbarActionContext)}
          >
            <Trash2 className="size-4" aria-hidden />
            {deleteAction.label ?? "删除"}
          </Button>
        )}
        {disableAction && visibleAction("disable") && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            title={disableAction.description ?? disableAction.label ?? "停用记录"}
            disabled={resolveDataGridActionDisabled(disableAction.disabled, toolbarActionContext, selectedCount === 0)}
            onClick={() => disableAction.onClick(toolbarActionContext)}
          >
            <Ban className="size-4" aria-hidden />
            {disableAction.label ?? "停用"}
          </Button>
        )}
        {visibleAction("view") && (
          <DataGridNamedViewsToolbar
            storageKey={storageKey}
            columns={columns}
            actionKeys={actionKeys}
            pageSizeOptions={pageSizeOptions}
            defaultPageSize={defaultPageSize}
            settings={settings}
            queryState={queryState}
            onApplyView={onApplyView}
          />
        )}
        {onToggleAdvancedFilter && (
          <Button
            type="button"
            variant={activeAdvancedFilterCount > 0 || advancedFilterOpen ? "secondary" : "outline"}
            size="sm"
            className="h-8 shrink-0 gap-1.5"
            aria-label="高级筛选"
            title="高级条件筛选器"
            aria-expanded={advancedFilterOpen}
            onClick={onToggleAdvancedFilter}
          >
            <Filter className="size-4" aria-hidden />
            <span>筛选器</span>
            {activeAdvancedFilterCount > 0 && (
              <span className="rounded bg-primary/15 px-1.5 py-0.2 text-[10px] font-semibold text-primary">
                {activeAdvancedFilterCount}
              </span>
            )}
          </Button>
        )}
        {visibleAction("field") && (
          <Button
            ref={fieldButtonRef}
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            aria-label="字段显示"
            title="字段显示"
            aria-expanded={fieldsOpen}
            aria-controls={fieldListId}
            disabled={hideableColumnCount === 0}
            onClick={onToggleFields}
            data-datagrid-popover
          >
            <Settings2 className="size-4" aria-hidden />
            字段
          </Button>
        )}
        {visibleAction("summary") && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            title="汇总统计"
            disabled={pageFilteredRowCount === 0}
            onClick={onOpenSummary}
          >
            <Calculator className="size-4" aria-hidden />
            汇总
          </Button>
        )}
        {showPrintAction && printAction !== false && visibleAction("print") && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            title={printAction?.description ?? printAction?.label ?? "打印列表"}
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
        {showExportAction && csvExportPlacement === "toolbar" && exportAction !== false && visibleAction("export") && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 shrink-0"
            title={exportAction?.description ?? exportAction?.label ?? "导出 Excel"}
            disabled={resolveDataGridActionDisabled(exportAction?.disabled, toolbarActionContext, pageFilteredRowCount === 0)}
            onClick={() => {
              if (exportAction?.onClick) {
                exportAction.onClick(toolbarActionContext);
                return;
              }
              onOpenExportDialog();
            }}
          >
            <Download className="size-4" aria-hidden />
            {exportAction?.label ?? "导出"}
          </Button>
        )}
        {toolbarActions.some((action) => visibleAction(toolbarActionKey(action.key))) && (
          <>
            <span className="mx-1 h-5 w-px bg-border" aria-hidden />
            {toolbarActions.filter((action) => visibleAction(toolbarActionKey(action.key))).map((action) => (
              <Button
                key={action.key}
                type="button"
                variant={action.variant ?? "outline"}
                size="sm"
                className="h-8 shrink-0"
                title={action.description ?? action.label}
                disabled={typeof action.disabled === "function" ? action.disabled(toolbarActionContext) : action.disabled}
                onClick={() => action.onClick(toolbarActionContext)}
              >
                {action.icon}
                {action.label}
              </Button>
            ))}
          </>
        )}
      </div>
      <div className="flex shrink-0 justify-end">
        <Button
          ref={actionSettingsButtonRef}
          type="button"
          variant="outline"
          size="sm"
          className="relative h-8 shrink-0"
          aria-label="按钮功能"
          title={hasHiddenToolbarActions ? "按钮功能显示设置；有隐藏按钮功能" : "按钮功能显示设置"}
          aria-expanded={actionSettingsOpen}
          aria-controls={actionSettingsPanelId}
          disabled={actionSettingCount === 0}
          onClick={onToggleActionSettings}
          data-datagrid-popover
        >
          {hasHiddenToolbarActions ? (
            <span className="absolute -left-1 -top-1 size-2 rounded-full bg-destructive" aria-hidden />
          ) : null}
          <ListChecks className="size-4" aria-hidden />
          按钮
        </Button>
      </div>
    </div>
  );
}
