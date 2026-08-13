import * as React from "react";
import { History, Trash2 } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import type { DataGridFilterHistoryEntry } from "./data-grid-filter-history";
import {
  buildDataGridFilterSummaryItems,
  type DataGridFilterSummaryField,
} from "./data-grid-filter-summary";
import type { DataGridColumnFilters } from "./data-grid-logic";

/**
 * DataGridFilterHistory — DataGrid 最近筛选条（自动记录 + 一键复用）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2 收货管理列表字段筛选
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：直接展示最近 5 个激活筛选组合为可点击 chip；
 * 空历史时不渲染；应用筛选与清空历史由调用方执行。
 *
 * @example
 *   <DataGridFilterHistory
 *     entries={entries}
 *     fields={fields}
 *     onApply={applyFilters}
 *     onClear={clearHistory}
 *   />
 */
export interface DataGridFilterHistoryProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  entries: DataGridFilterHistoryEntry[];
  fields: DataGridFilterSummaryField[];
  onApply: (filters: DataGridColumnFilters) => void;
  onClear: () => void;
}

export const DataGridFilterHistory = React.forwardRef<HTMLDivElement, DataGridFilterHistoryProps>(
  ({ entries, fields, onApply, onClear, className, ...rest }, ref) => {
    if (entries.length === 0) return null;

    return (
      <div
        ref={ref}
        aria-label="最近筛选"
        data-datagrid-filter-history
        className={cn(
          "flex flex-wrap items-center gap-2 rounded-md border bg-muted/30 px-3 py-2 text-xs",
          className,
        )}
        {...rest}
      >
        <History className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
        <span className="font-medium text-muted-foreground">最近筛选</span>
        {entries.map((entry, index) => {
          const summary = buildFilterHistorySummary(entry.filters, fields);
          return (
            <button
              key={`${entry.savedAt}-${index}`}
              type="button"
              aria-label={summary ? `恢复最近筛选：${summary}` : "恢复最近筛选"}
              title={summary ? `${summary}（${formatFilterHistoryTime(entry.savedAt)}）` : undefined}
              className="inline-flex h-8 max-w-full items-center gap-1 rounded-md border bg-background px-2 text-foreground shadow-sm transition-colors hover:bg-accent hover:text-accent-foreground"
              onClick={() => onApply(entry.filters)}
            >
              <span className="max-w-[18rem] truncate">{summary || "已设置筛选"}</span>
            </button>
          );
        })}
        <Button type="button" variant="ghost" size="sm" className="h-8" aria-label="清空最近筛选" onClick={onClear}>
          <Trash2 className="size-3.5" aria-hidden />
          清空历史
        </Button>
      </div>
    );
  },
);
DataGridFilterHistory.displayName = "DataGridFilterHistory";

function buildFilterHistorySummary(filters: DataGridColumnFilters, fields: DataGridFilterSummaryField[]): string {
  const items = buildDataGridFilterSummaryItems(filters, fields);
  if (items.length === 0) return "";

  const shown = items
    .slice(0, 3)
    .map((item) => item.text)
    .join("、");
  return items.length > 3 ? `${shown} 等 ${items.length} 项` : shown;
}

function formatFilterHistoryTime(savedAt: string): string {
  const date = new Date(savedAt);
  if (Number.isNaN(date.getTime())) return "";

  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
