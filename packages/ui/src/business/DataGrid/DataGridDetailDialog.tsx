import * as React from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "../../ui/dialog";
import { columnLabel } from "./data-grid-helpers";
import type { DataGridColumn } from "./data-grid-types";

/**
 * DataGridDetailDialog — 通用只读详情对话框
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端 DataGrid 横向能力
 * Wave：Wave 6 管理端表格增强
 * 业务约束：由 DataGrid 内置"查看"按钮打开；按列配置渲染选中行的 key-value 只读展示。
 *
 * @example
 *   <DataGridDetailDialog row={row} columns={columns} rowKey={(r) => r.id} open onOpenChange={...} />
 */
export interface DataGridDetailDialogProps<T> extends React.HTMLAttributes<HTMLDivElement> {
  row: T | null;
  columns: DataGridColumn<T>[];
  rowKey: (row: T) => string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function DataGridDetailDialog<T>({ row, columns, open, onOpenChange }: DataGridDetailDialogProps<T>) {
  const title = row ? `记录详情 · ${rowKeyText(row)}` : "记录详情";
  const visibleColumns = columns.filter((column) => column.key !== "__rowNumber" && column.header !== undefined);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[80vh] max-w-2xl overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription className="sr-only">只读记录详情</DialogDescription>
        </DialogHeader>
        {row ? (
          <dl className="grid grid-cols-1 gap-x-6 gap-y-2 text-sm sm:grid-cols-2">
            {visibleColumns.map((column) => {
              const raw = column.filterValue ? column.filterValue(row) : (row as Record<string, unknown>)[column.key];
              const text = raw == null || raw === "" ? "-" : String(raw);
              return (
                <div key={column.key} className="flex gap-2">
                  {/* 只读标签取纯文本：header 可能是绑定 grid 交互的 DataGridHeaderCell 元素（排序/筛选/列宽），不能渲染 */}
                  <dt className="w-28 shrink-0 text-muted-foreground">
                    {typeof column.header === "string" ? column.header : columnLabel(column)}
                  </dt>
                  <dd className="min-w-0 break-words">{text}</dd>
                </div>
              );
            })}
          </dl>
        ) : (
          <p className="text-sm text-muted-foreground">未选择记录</p>
        )}
      </DialogContent>
    </Dialog>
  );
}

function rowKeyText<T>(row: T): string {
  const firstValue = Object.values(row as Record<string, unknown>).find((value) => typeof value === "string");
  return typeof firstValue === "string" ? firstValue : "-";
}
