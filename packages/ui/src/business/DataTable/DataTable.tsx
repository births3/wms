import * as React from "react";
import { cn } from "../../lib/utils";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "../../ui/table";
import { EmptyState } from "../EmptyState";

/**
 * DataTable — 通用数据表格（基于 ui/Table，含选中态/hover/空状态/翻页槽）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：所有列表型管理页（H1 角色/API Key/会话 / H2 审计/归档 / M2 ASN / M4 订单 等）
 * Wave：Wave 0.5 起步，全 Wave 复用
 * 业务约束：行点击触发 onRowClick；列定义对齐 columns[].align；空时展示 EmptyState；翻页放 footer
 *
 * @example
 *   <DataTable columns={[{key:"time",header:"时间"}]} data={[...]} rowKey="id" onRowClick={...} />
 */
export interface DataTableColumn<T> {
  key: string;
  header: React.ReactNode;
  render?: (row: T, idx: number) => React.ReactNode;
  headerProps?: React.ThHTMLAttributes<HTMLTableCellElement>;
  cellProps?: (row: T, idx: number) => React.TdHTMLAttributes<HTMLTableCellElement>;
  align?: "left" | "center" | "right";
  width?: string | number;
  /** monospace 字体（适合 ID / IP / hash） */
  mono?: boolean;
  /** 列辅助样式 */
  className?: string;
}

export interface DataTableProps<T> extends Omit<React.HTMLAttributes<HTMLDivElement>, "onSelect"> {
  columns: DataTableColumn<T>[];
  data: T[];
  /** 表格元素样式（用于最小宽度等布局控制） */
  tableClassName?: string;
  tableStyle?: React.CSSProperties;
  /** 用于 key & 选中比对（必填） */
  rowKey: (row: T) => string;
  /** 选中的行 key */
  selectedKey?: string;
  /** 行点击回调 */
  onRowClick?: (row: T, idx: number) => void;
  /** 表格上方注释（行数 / 排序说明） */
  caption?: React.ReactNode;
  /** 翻页槽（footer） */
  footer?: React.ReactNode;
  /** 空状态自定义 */
  emptyTitle?: React.ReactNode;
  emptyDescription?: React.ReactNode;
}

export function DataTable<T>({
  columns,
  data,
  rowKey,
  selectedKey,
  onRowClick,
  caption,
  footer,
  emptyTitle,
  emptyDescription,
  className,
  tableClassName,
  tableStyle,
  ...rest
}: DataTableProps<T>) {
  return (
    <div className={cn("rounded-md border bg-background overflow-hidden font-sans", className)} {...rest}>
      {caption && (
        <div className="px-4 py-2.5 text-xs text-muted-foreground border-b bg-muted/40">{caption}</div>
      )}
      <Table className={tableClassName} style={tableStyle}>
        <colgroup>
          {columns.map((col) => (
            <col key={col.key} style={columnStyle(col)} />
          ))}
        </colgroup>
        <TableHeader>
          <TableRow>
            {columns.map((col) => {
              const { className: headerClassName, style: headerStyle, ...headerProps } = col.headerProps ?? {};
              return (
                <TableHead
                  key={col.key}
                  // 动态：列宽和冻结列 left 偏移来自 DataGrid 列配置。
                  style={{ ...columnStyle(col), ...headerStyle }}
                  className={cn(col.align === "right" && "text-right", col.align === "center" && "text-center", headerClassName)}
                  {...headerProps}
                >
                  {col.header}
                </TableHead>
              );
            })}
          </TableRow>
        </TableHeader>
        <TableBody>
          {data.length === 0 ? (
            <TableRow>
              <TableCell colSpan={columns.length} className="py-10">
                {/* sticky left：宽表横滚时空态仍落在可视区左侧，避免居中到超宽表格中间不可见 */}
                <div className="sticky left-0 w-max max-w-[min(100vw-4rem,28rem)]">
                  <EmptyState title={emptyTitle ?? "暂无数据"} description={emptyDescription} />
                </div>
              </TableCell>
            </TableRow>
          ) : (
            data.map((row, idx) => {
              const key = rowKey(row);
              const selected = selectedKey === key;
              return (
                <TableRow
                  key={key}
                  data-state={selected ? "selected" : undefined}
                  onClick={onRowClick ? () => onRowClick(row, idx) : undefined}
                  className={onRowClick ? "cursor-pointer" : ""}
                >
                  {columns.map((col) => {
                    const { className: cellClassName, style: cellStyle, ...cellProps } = col.cellProps?.(row, idx) ?? {};
                    const mergedCellStyle = { ...columnStyle(col), ...cellStyle };
                    return (
                      <TableCell
                        key={col.key}
                        style={mergedCellStyle}
                        className={cn(
                          col.mono && "font-mono text-xs text-muted-foreground",
                          col.align === "right" && "text-right",
                          col.align === "center" && "text-center",
                          col.className,
                          cellClassName,
                        )}
                        {...cellProps}
                      >
                        {col.render ? col.render(row, idx) : (row as Record<string, React.ReactNode>)[col.key]}
                      </TableCell>
                    );
                  })}
                </TableRow>
              );
            })
          )}
        </TableBody>
      </Table>
      {footer && <div className="border-t">{footer}</div>}
    </div>
  );
}

function columnStyle<T>(column: DataTableColumn<T>): React.CSSProperties {
  return {
    width: column.width,
    minWidth: typeof column.width === "number" ? column.width : undefined,
    textAlign: column.align,
  };
}
