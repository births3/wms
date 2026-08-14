import * as React from "react";
import { Plus, RotateCcw, X, Filter } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../../ui/select";
import { DataGridAdvancedFilterRow } from "./DataGridAdvancedFilterRow";
import {
  getDefaultOperatorForFilterType,
  type DataGridAdvancedFilterState,
  type DataGridFilterItem,
} from "./data-grid-operators";
import type { DataGridColumn } from "./data-grid-types";

/**
 * DataGridAdvancedFilterPanel — DataGrid 高级条件筛选器面板
 *
 * 层级：Layer 2 业务复合内部组件
 * 关联故事：H7 / M2 管理端表格增强
 * Wave：Wave 6 管理端表格增强
 * 业务约束：支持多行字段算子组合与 AND/OR 逻辑切换，支持添加、修改、删除和全部重置。
 *
 * @example
 *   <DataGridAdvancedFilterPanel columns={columns} state={state} onStateChange={setState} onClose={close} />
 */
export interface DataGridAdvancedFilterPanelProps<T> extends Omit<React.HTMLAttributes<HTMLDivElement>, "onChange"> {
  columns: DataGridColumn<T>[];
  state: DataGridAdvancedFilterState | undefined;
  onStateChange: (nextState: DataGridAdvancedFilterState) => void;
  onClose?: () => void;
  onReset?: () => void;
}

export function DataGridAdvancedFilterPanel<T>({
  columns,
  state,
  onStateChange,
  onClose,
  onReset,
  className,
  ...rest
}: DataGridAdvancedFilterPanelProps<T>) {
  const filterableColumns = React.useMemo(
    () => columns.filter((col) => col.hideable !== false && col.filter !== false),
    [columns],
  );

  const items = state?.items ?? [];
  const joinOperator = state?.joinOperator ?? "and";

  const handleAddFilter = () => {
    const firstCol = filterableColumns[0];
    if (!firstCol) return;

    const filterType = firstCol.filter === false ? "text" : firstCol.filter?.type ?? "text";
    const defaultOp = getDefaultOperatorForFilterType(filterType);

    const newItem: DataGridFilterItem = {
      id: `filter-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      columnKey: firstCol.key,
      operator: defaultOp,
      value: undefined,
    };

    onStateChange({
      joinOperator,
      items: [...items, newItem],
    });
  };

  const handleUpdateItem = (index: number, nextItem: DataGridFilterItem) => {
    const nextItems = [...items];
    nextItems[index] = nextItem;
    onStateChange({
      joinOperator,
      items: nextItems,
    });
  };

  const handleRemoveItem = (index: number) => {
    const nextItems = items.filter((_, i) => i !== index);
    onStateChange({
      joinOperator,
      items: nextItems,
    });
  };

  const handleJoinOperatorChange = (nextJoin: string) => {
    onStateChange({
      joinOperator: nextJoin === "or" ? "or" : "and",
      items,
    });
  };

  const handleReset = () => {
    onStateChange({
      joinOperator: "and",
      items: [],
    });
    onReset?.();
  };

  return (
    <div
      className={cn(
        "flex flex-col gap-3 rounded-lg border bg-card p-3.5 shadow-sm transition-all animate-in fade-in-50",
        className,
      )}
      {...rest}
    >
      {/* 头部控制栏 */}
      <div className="flex items-center justify-between gap-2 border-b pb-2.5">
        <div className="flex items-center gap-2">
          <div className="flex size-7 items-center justify-center rounded-md bg-primary/10 text-primary">
            <Filter className="size-4" />
          </div>
          <span className="text-xs font-semibold">高级条件筛选器</span>
          {items.length > 0 && (
            <span className="rounded-full bg-muted px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
              {items.length} 个条件
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <span>规则逻辑:</span>
            <Select value={joinOperator} onValueChange={handleJoinOperatorChange}>
              <SelectTrigger className="h-7 w-24 text-xs font-medium" aria-label="规则逻辑关系">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="and">且 (AND)</SelectItem>
                <SelectItem value="or">或 (OR)</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {onClose && (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-7"
              aria-label="收起筛选面板"
              onClick={onClose}
            >
              <X className="size-4" />
            </Button>
          )}
        </div>
      </div>

      {/* 筛选条件行列表 */}
      <div className="flex flex-col gap-2">
        {items.length === 0 ? (
          <div className="flex flex-col items-center justify-center rounded-md border border-dashed py-6 text-center text-xs text-muted-foreground">
            <p>暂无启用的高级筛选条件</p>
            <p className="mt-1 text-[11px]">点击下方按钮添加第一个字段过滤规则</p>
          </div>
        ) : (
          items.map((item, index) => (
            <DataGridAdvancedFilterRow
              key={item.id}
              item={item}
              columns={columns}
              onChange={(nextItem) => handleUpdateItem(index, nextItem)}
              onRemove={() => handleRemoveItem(index)}
            />
          ))
        )}
      </div>

      {/* 底部操作区 */}
      <div className="flex items-center justify-between pt-1">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-8 gap-1.5 text-xs font-medium"
          onClick={handleAddFilter}
          disabled={filterableColumns.length === 0}
        >
          <Plus className="size-3.5" />
          添加条件
        </Button>

        {items.length > 0 && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-8 gap-1 text-xs text-muted-foreground hover:text-foreground"
            onClick={handleReset}
          >
            <RotateCcw className="size-3.5" />
            清空所有条件
          </Button>
        )}
      </div>
    </div>
  );
}
