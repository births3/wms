import * as React from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../../ui/select";

/**
 * DataGridPaginationFooter — DataGrid 分页页脚
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2 收货管理列表
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：页大小、翻页和清空选择集中在表格底部
 *
 * @example
 *   <DataGridPaginationFooter total={100} pageIndex={0} pageCount={10} />
 */
export interface DataGridPaginationFooterProps {
  rangeStart: number;
  rangeEnd: number;
  total: number;
  selectable: boolean;
  selectedCount: number;
  pageSize: number;
  pageSizeOptions: number[];
  pageIndex: number;
  pageCount: number;
  className?: string;
  onPageSizeChange: (pageSize: number) => void;
  onPageIndexChange: (pageIndex: number) => void;
  onClearSelected: () => void;
}

export const DataGridPaginationFooter = React.forwardRef<HTMLDivElement, DataGridPaginationFooterProps>(
  (
    {
      rangeStart,
      rangeEnd,
      total,
      selectable,
      selectedCount,
      pageSize,
      pageSizeOptions,
      pageIndex,
      pageCount,
      className,
      onPageSizeChange,
      onPageIndexChange,
      onClearSelected,
    },
    ref,
  ) => {
    const currentPage = pageIndex + 1;

    return (
      <div
        ref={ref}
        className={cn(
          "flex flex-col gap-2 px-4 py-3 text-xs text-muted-foreground md:flex-row md:items-center md:justify-between",
          className,
        )}
      >
        <span>
          {rangeStart}-{rangeEnd} / 共 {total} 条
          {selectable && selectedCount > 0 ? ` · 已选 ${selectedCount} 条` : ""}
        </span>
        <div className="flex flex-wrap items-center gap-2">
          {selectable && selectedCount > 0 && (
            <Button type="button" variant="ghost" size="sm" onClick={onClearSelected}>
              清空选择
            </Button>
          )}
          <Select value={String(pageSize)} onValueChange={(value) => onPageSizeChange(Number.parseInt(value, 10))}>
            <SelectTrigger className="h-8 w-[116px]" aria-label="每页条数">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {pageSizeOptions.map((option) => (
                <SelectItem key={option} value={String(option)}>
                  {option} 条/页
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <span>
            第 {currentPage} / {pageCount} 页
          </span>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={pageIndex === 0}
            onClick={() => onPageIndexChange(Math.max(0, pageIndex - 1))}
          >
            <ChevronLeft className="size-4" aria-hidden />
            上一页
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={pageIndex >= pageCount - 1}
            onClick={() => onPageIndexChange(Math.min(pageCount - 1, pageIndex + 1))}
          >
            下一页
            <ChevronRight className="size-4" aria-hidden />
          </Button>
        </div>
      </div>
    );
  },
);
DataGridPaginationFooter.displayName = "DataGridPaginationFooter";
