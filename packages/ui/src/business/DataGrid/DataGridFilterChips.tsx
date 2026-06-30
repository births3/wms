import * as React from "react";
import { X } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import {
  buildDataGridFilterSummaryItems,
  type DataGridFilterSummaryField,
} from "./data-grid-filter-summary";
import type { DataGridColumnFilters } from "./data-grid-logic";

/**
 * DataGridFilterChips — DataGrid 已启用筛选标签栏
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2 收货管理列表字段筛选
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：只展示已启用筛选、单个清除和全部清除；
 * 筛选状态由调用方持有。
 *
 * @example
 *   <DataGridFilterChips
 *     filters={filters}
 *     fields={fields}
 *     onClearFilter={clearFilter}
 *     onClearAll={clearAll}
 *   />
 */
export interface DataGridFilterChipsProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  filters: DataGridColumnFilters;
  fields: DataGridFilterSummaryField[];
  onClearFilter: (key: string) => void;
  onClearAll: () => void;
}

export const DataGridFilterChips = React.forwardRef<HTMLDivElement, DataGridFilterChipsProps>(
  ({ filters, fields, onClearFilter, onClearAll, className, ...rest }, ref) => {
    const items = buildDataGridFilterSummaryItems(filters, fields);
    if (items.length === 0) return null;

    return (
      <div
        ref={ref}
        aria-label="已启用筛选"
        className={cn(
          "flex flex-wrap items-center gap-2 rounded-md border bg-muted/30 px-3 py-2 text-xs",
          className,
        )}
        {...rest}
      >
        <span className="font-medium text-muted-foreground">已启用筛选</span>
        {items.map((item) => (
          <span
            key={item.key}
            className={cn(
              "inline-flex h-8 max-w-full items-center gap-1 rounded-md border",
              "bg-background px-2 text-foreground shadow-sm",
            )}
          >
            <span className="max-w-[18rem] truncate">{item.text}</span>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-6 shrink-0"
              aria-label={`清除${item.label}筛选`}
              onClick={() => onClearFilter(item.key)}
            >
              <X className="size-3.5" aria-hidden />
            </Button>
          </span>
        ))}
        <Button type="button" variant="ghost" size="sm" className="h-8" onClick={onClearAll}>
          清除全部
        </Button>
      </div>
    );
  },
);
DataGridFilterChips.displayName = "DataGridFilterChips";
