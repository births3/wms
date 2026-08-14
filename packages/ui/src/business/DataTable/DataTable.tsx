import * as React from "react";
import { cn } from "../../lib/utils";
import {
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "../../ui/table";
import { ScrollBar } from "../../ui/scroll-bar";
import { EmptyState } from "../EmptyState";
import { useScrollAreaMaxHeight } from "./use-scroll-area-max-height";

/**
 * DataTable — 通用数据表格（基于 ui/Table，含选中态/hover/空状态/翻页槽）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：所有列表型管理页（H1 角色/API Key/会话 / H2 审计/归档 / M2 ASN / M4 订单 等）
 * Wave：Wave 0.5 起步，全 Wave 复用
 * 业务约束：行点击触发 onRowClick；列定义对齐 columns[].align；空时展示 EmptyState；翻页放 footer；
 *   列表自管纵向滚动（视口测量高度），悬停表格滚轮只滚列表数据；横向由 ScrollBar 自绘滚动条控制。
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
  /** 滚动区最大高度：默认按视口测量（悬停表格滚轮滚动列表数据、页面不滚动）；传值覆盖测量 */
  maxHeight?: string | number;
  /** 用于 key & 选中比对（必填） */
  rowKey: (row: T) => string;
  /** 选中的行 key */
  selectedKey?: string;
  /** 行点击回调 */
  onRowClick?: (row: T, idx: number) => void;
  /** 行双击回调（查看详情等） */
  onRowDoubleClick?: (row: T, idx: number) => void;
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
  onRowDoubleClick,
  caption,
  footer,
  emptyTitle,
  emptyDescription,
  maxHeight,
  className,
  tableClassName,
  tableStyle,
  ...rest
}: DataTableProps<T>) {
  const rootRef = React.useRef<HTMLDivElement | null>(null);
  const scrollAreaRef = React.useRef<HTMLDivElement | null>(null);
  const bottomBarRef = React.useRef<HTMLDivElement | null>(null);
  const tableRef = React.useRef<HTMLTableElement | null>(null);
  // state 承载滚动容器节点：ref 突变不触发重渲染，ScrollBar 的 container prop 依赖节点就绪
  const [scrollAreaNode, setScrollAreaNode] = React.useState<HTMLDivElement | null>(null);
  const [hScrollable, setHScrollable] = React.useState(false);
  const measuredMaxHeight = useScrollAreaMaxHeight(maxHeight, scrollAreaRef, bottomBarRef, rootRef);
  // 动态：显式 maxHeight 优先，否则用视口测量兜底
  const effectiveMaxHeight = maxHeight !== undefined ? maxHeight : measuredMaxHeight;

  return (
    <div
      ref={rootRef}
      data-datatable-root="true"
      className={cn("flex flex-1 min-h-0 min-h-[380px] flex-col rounded-md border bg-background overflow-hidden font-sans", className)}
      {...rest}
    >
      {caption && (
        <div className="px-4 py-2.5 text-xs text-muted-foreground border-b bg-muted/40">{caption}</div>
      )}
      {/*
        单滚动容器双轴同滚：thead sticky top 直接粘附本容器（表头吸顶）；
        WebKit 用 ::-webkit-scrollbar:horizontal 只隐藏横向原生条、保留纵向条；
        Firefox 无逐轴控制，scrollbar-width:none 整体隐藏（滚轮/自绘横滚条不受影响）。
      */}
      <div
        ref={(node) => {
          scrollAreaRef.current = node;
          setScrollAreaNode(node);
        }}
        className="min-h-0 flex-1 overflow-auto overscroll-contain [&::-webkit-scrollbar:horizontal]:hidden [scrollbar-width:none]"
        style={effectiveMaxHeight !== undefined ? { maxHeight: effectiveMaxHeight } : undefined}
      >
        <table ref={tableRef} className={cn("w-full caption-bottom text-sm", tableClassName)} style={tableStyle}>
          <colgroup>
            {columns.map((col) => (
              <col key={col.key} style={columnStyle(col)} />
            ))}
          </colgroup>
          <TableHeader className="sticky top-0 z-20 bg-muted shadow-[0_1px_0_hsl(var(--border))]">
            <TableRow className="bg-muted hover:bg-muted">
              {columns.map((col) => {
                const { className: headerClassName, style: headerStyle, ...headerProps } = col.headerProps ?? {};
                return (
                  <TableHead
                    key={col.key}
                    // 动态：列宽和冻结列 left 偏移来自 DataGrid 列配置。
                    style={{ ...columnStyle(col), ...headerStyle }}
                    className={cn("bg-muted", col.align === "right" && "text-right", col.align === "center" && "text-center", headerClassName)}
                    {...headerProps}
                  >
                    {col.header}
                  </TableHead>
                );
              })}
            </TableRow>
          </TableHeader>
          <TableBody className="[&_tr]:bg-background">
            {data.length === 0 ? (
              <TableRow className="hover:bg-transparent">
                <TableCell colSpan={columns.length} className="h-64 py-12 text-center align-middle">
                  {/* sticky left：宽表横滚时空态仍落在可视区左侧，避免居中到超宽表格中间不可见 */}
                  <div className="sticky left-0 flex w-full justify-center">
                    <div className="max-w-[min(100vw-4rem,28rem)]">
                      <EmptyState title={emptyTitle ?? "暂无数据"} description={emptyDescription} />
                    </div>
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
                    onDoubleClick={onRowDoubleClick ? () => onRowDoubleClick(row, idx) : undefined}
                    className={onRowClick || onRowDoubleClick ? "cursor-pointer" : ""}
                  >
                    {columns.map((col) => {
                      const { className: cellClassName, style: cellStyle, ...cellProps } = col.cellProps?.(row, idx) ?? {};
                      const mergedCellStyle = { ...columnStyle(col), ...cellStyle };
                      return (
                        <TableCell
                          key={col.key}
                          style={mergedCellStyle}
                          className={cn(
                            "px-4 py-2.5",
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
        </table>
      </div>
      {/* 底部栏常驻：横滚轮在上、翻页在下；无 footer 且无横向溢出时整体隐藏（汇总表分支） */}
      <div
        ref={bottomBarRef}
        className={cn(
          "mt-auto shrink-0 border-t bg-background shadow-[0_-4px_12px_rgba(0,0,0,0.06)]",
          !footer && !hScrollable && "hidden",
        )}
      >
        <ScrollBar container={scrollAreaNode} contentRef={tableRef} onScrollableChange={setHScrollable} />
        {footer}
      </div>
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
