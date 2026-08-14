import * as React from "react";
import { X } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import type { DataGridAdvancedFilterState } from "./data-grid-operators";
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
 *     advancedFilters={advancedFilters}
 *     fields={fields}
 *     onClearFilter={clearFilter}
 *     onClearAll={clearAll}
 *   />
 */
import type { DataGridQuerySummaryItem } from "./data-grid-types";

export interface DataGridFilterChipsProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  filters: DataGridColumnFilters;
  advancedFilters?: DataGridAdvancedFilterState;
  fields: DataGridFilterSummaryField[];
  querySummaryItems?: DataGridQuerySummaryItem[];
  onClearFilter: (key: string) => void;
  onClearAll: () => void;
}

export const DataGridFilterChips = React.forwardRef<HTMLDivElement, DataGridFilterChipsProps>(
  (
    {
      filters,
      advancedFilters,
      fields,
      querySummaryItems = [],
      onClearFilter,
      onClearAll,
      className,
      ...rest
    },
    ref,
  ) => {
    const localItems = buildDataGridFilterSummaryItems(filters, fields, advancedFilters);
    const queryItems = querySummaryItems.map((item) => ({
      key: `query-${item.key}`,
      rawKey: item.key,
      label: item.label,
      text: item.text,
      isQuery: true,
    }));
    const allItems = [...queryItems, ...localItems];

    if (allItems.length === 0) return null;

    return (
      <div
        ref={ref}
        aria-label="已启用筛选"
        className={cn(
          "flex flex-wrap items-center gap-2 rounded-md border bg-muted/20 px-3 py-1.5 text-xs",
          className,
        )}
        {...rest}
      >
        <span className="font-medium text-muted-foreground">已应用条件</span>
        {allItems.map((item) => (
          <span
            key={item.key}
            className={cn(
              "inline-flex h-7 max-w-full items-center gap-1 rounded-md border",
              "border-border/60 bg-background px-2 text-foreground shadow-sm",
            )}
          >
            <span className="max-w-[18rem] truncate font-sans text-xs">{item.text}</span>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="size-5 shrink-0 hover:bg-muted"
              aria-label={`清除${item.label}筛选`}
              onClick={() => onClearFilter("rawKey" in item ? item.rawKey : item.key)}
            >
              <X className="size-3 text-muted-foreground hover:text-foreground" aria-hidden />
            </Button>
          </span>
        ))}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 text-xs text-muted-foreground hover:text-foreground"
          onClick={onClearAll}
        >
          清除全部
        </Button>
      </div>
    );
  },
);
DataGridFilterChips.displayName = "DataGridFilterChips";
