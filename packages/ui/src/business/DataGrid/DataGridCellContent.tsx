import * as React from "react";
import { cn } from "../../lib/utils";
import type { DataGridColumn } from "./DataGrid";

/**
 * DataGridCellContent — DataGrid 可复制单元格内容
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2 收货管理列表
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：单击复制、双击打开详情的交互在单元格内闭环
 *
 * @example
 *   <DataGridCellContent content="ASN-001" copyText="ASN-001" column={column} row={row} />
 */
export interface DataGridCellContentProps<T> {
  row: T;
  column: DataGridColumn<T>;
  content: React.ReactNode;
  copyText: string;
  cellNotice: string | null;
  label: string;
  className?: string;
  onCopy: () => void;
  onDoubleClick?: (row: T) => void;
}

export function DataGridCellContent<T>({
  row,
  column,
  content,
  copyText,
  cellNotice,
  label,
  className,
  onCopy,
  onDoubleClick,
}: DataGridCellContentProps<T>) {
  const canDoubleClick = Boolean(onDoubleClick);
  if (!copyText && !canDoubleClick) {
    return className ? <span className={className}>{content}</span> : <>{content}</>;
  }

  return (
    <div
      role="button"
      tabIndex={0}
      title={copyText ? "点击复制" : "双击打开"}
      aria-label={copyText ? `复制${label}` : `打开${label}`}
      className={cn(
        "relative rounded-sm px-1 py-0.5 text-left outline-none transition hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring",
        column.align === "right" && "text-right",
        className,
      )}
      onClick={(event) => {
        if (!copyText) return;
        event.stopPropagation();
        onCopy();
      }}
      onDoubleClick={(event) => {
        if (!onDoubleClick) return;
        event.stopPropagation();
        onDoubleClick(row);
      }}
      onKeyDown={(event) => {
        if (!copyText || (event.key !== "Enter" && event.key !== " ")) return;
        event.preventDefault();
        event.stopPropagation();
        onCopy();
      }}
    >
      <div className="relative inline-block max-w-full align-middle">
        <div className="min-w-0 truncate">{content}</div>
        {cellNotice && (
          <span className="pointer-events-none absolute left-full top-1/2 z-20 ml-2 -translate-y-1/2 whitespace-nowrap rounded-sm bg-foreground px-1.5 py-0.5 text-[11px] font-normal text-background shadow-sm">
            {cellNotice}
          </span>
        )}
      </div>
    </div>
  );
}
