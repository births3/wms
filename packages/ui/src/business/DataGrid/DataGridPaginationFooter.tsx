import * as React from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../../ui/select";
import type { DataGridServerPagination } from "./data-grid-types";

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
  /** 服务端分页受控模式：提供时展示值与翻页/每页条数事件完全由 serverPagination 驱动，内存分页 props 被忽略 */
  serverPagination?: DataGridServerPagination;
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
      serverPagination,
    },
    ref,
  ) => {
    const server = serverPagination;
    // 服务端受控分支：pageCount = ceil(total/pageSize)，rangeStart/rangeEnd 由 pageIndex/pageSize/total 计算
    const effectivePageSize = server ? server.pageSize : pageSize;
    const effectivePageCount = server ? Math.max(1, Math.ceil(server.total / effectivePageSize)) : pageCount;
    const effectivePageIndex = server ? Math.min(Math.max(server.pageIndex, 0), effectivePageCount - 1) : pageIndex;
    const effectiveTotal = server ? server.total : total;
    const effectiveRangeStart = server
      ? effectiveTotal === 0
        ? 0
        : effectivePageIndex * effectivePageSize + 1
      : rangeStart;
    const effectiveRangeEnd = server
      ? Math.min((effectivePageIndex + 1) * effectivePageSize, effectiveTotal)
      : rangeEnd;
    const currentPage = effectivePageIndex + 1;

    function handlePageSizeChange(value: number) {
      if (server) {
        if (server.onPageSizeChange) server.onPageSizeChange(value);
        else onPageSizeChange(value);
      } else {
        onPageSizeChange(value);
      }
    }

    function handlePageIndexChange(index: number) {
      if (server) server.onPageChange(index);
      else onPageIndexChange(index);
    }

    return (
      <div
        ref={ref}
        className={cn(
          "flex flex-col gap-2 px-4 py-3 text-xs text-muted-foreground md:flex-row md:items-center md:justify-between",
          className,
        )}
      >
        <span>
          {effectiveRangeStart}-{effectiveRangeEnd} / 共 {effectiveTotal} 条
          {selectable && selectedCount > 0 ? ` · 已选 ${selectedCount} 条` : ""}
        </span>
        <div className="flex flex-wrap items-center gap-2">
          {selectable && selectedCount > 0 && (
            <Button type="button" variant="ghost" size="sm" onClick={onClearSelected}>
              清空选择
            </Button>
          )}
          <Select
            value={String(effectivePageSize)}
            onValueChange={(value) => handlePageSizeChange(Number.parseInt(value, 10))}
          >
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
            第 {currentPage} / {effectivePageCount} 页
          </span>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={effectivePageIndex === 0}
            onClick={() => handlePageIndexChange(Math.max(0, effectivePageIndex - 1))}
          >
            <ChevronLeft className="size-4" aria-hidden />
            上一页
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={effectivePageIndex >= effectivePageCount - 1}
            onClick={() => handlePageIndexChange(Math.min(effectivePageCount - 1, effectivePageIndex + 1))}
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
