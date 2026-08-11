import * as React from "react";
import { cn } from "../../lib/utils";
import {
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
  className,
  tableClassName,
  tableStyle,
  ...rest
}: DataTableProps<T>) {
  const [hScrollContainer, setHScrollContainer] = React.useState<HTMLDivElement | null>(null);

  return (
    <div className={cn("rounded-md border bg-background overflow-hidden font-sans", className)} {...rest}>
      {caption && (
        <div className="px-4 py-2.5 text-xs text-muted-foreground border-b bg-muted/40">{caption}</div>
      )}
      {/* overflow-y-clip：横向滚动容器不拦截 thead/页脚的垂直 sticky；原生滚动条隐藏（由页脚固定横滚轮控制） */}
      <div
        ref={setHScrollContainer}
        className="relative w-full overflow-x-auto overflow-y-clip [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
      >
        <table className={cn("w-full caption-bottom text-sm", tableClassName)} style={tableStyle}>
          <colgroup>
            {columns.map((col) => (
              <col key={col.key} style={columnStyle(col)} />
            ))}
          </colgroup>
          <TableHeader className="sticky top-0 z-20">
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
      {footer && (
        <>
          {/* 页脚吸底留白：sticky footer 滚动经过时遮住的是空位而非最后一行数据 */}
          <div aria-hidden className="h-12" />
          <div className="sticky bottom-0 z-10 -mt-12 border-t bg-background/95 shadow-[0_-4px_12px_rgba(0,0,0,0.06)] backdrop-blur">
            <DataTableHScrollBar container={hScrollContainer} />
            {footer}
          </div>
        </>
      )}
    </div>
  );
}

/**
 * DataTableHScrollBar — 横向滚动条（固定在页脚内）
 *
 * 宽表横向溢出时显示；点击轨道跳转、拖动滑块滚动。原生横向滚动条已隐藏，由本组件统一控制。
 */
function DataTableHScrollBar({ container }: { container: HTMLDivElement | null }) {
  const [view, setView] = React.useState({ left: 0, scrollable: false, ratio: 1 });
  const dragRef = React.useRef<{ startX: number; startLeft: number } | null>(null);

  React.useEffect(() => {
    if (!container) return;
    const update = () =>
      setView({
        left: container.scrollLeft,
        scrollable: container.scrollWidth > container.clientWidth,
        ratio: container.clientWidth / container.scrollWidth,
      });
    update();
    container.addEventListener("scroll", update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(container);
    return () => {
      container.removeEventListener("scroll", update);
      observer.disconnect();
    };
  }, [container]);

  if (!container || !view.scrollable) return null;

  const maxLeft = container.scrollWidth - container.clientWidth;
  const thumbWidth = Math.max(view.ratio * 100, 6);
  const thumbLeft = maxLeft > 0 ? (view.left / maxLeft) * (100 - thumbWidth) : 0;

  const jumpTo = (clientX: number) => {
    const rect = container.getBoundingClientRect();
    const pct = (clientX - rect.left) / rect.width;
    container.scrollLeft = pct * maxLeft;
  };

  return (
    <div
      role="scrollbar"
      aria-orientation="horizontal"
      className="relative h-2.5 w-full cursor-pointer touch-none select-none"
      onPointerDown={(event) => {
        const onThumb = Boolean((event.target as HTMLElement).closest("[data-thumb]"));
        if (onThumb) {
          dragRef.current = { startX: event.clientX, startLeft: container.scrollLeft };
          event.currentTarget.setPointerCapture(event.pointerId);
        } else {
          jumpTo(event.clientX);
        }
      }}
      onPointerMove={(event) => {
        if (!dragRef.current) return;
        const dx = event.clientX - dragRef.current.startX;
        container.scrollLeft = dragRef.current.startLeft + dx * view.ratio;
      }}
      onPointerUp={() => {
        dragRef.current = null;
      }}
    >
      <div
        data-thumb
        className="absolute top-0 bottom-0 rounded-full bg-muted-foreground/30 transition-colors hover:bg-muted-foreground/50"
        style={{ left: `${thumbLeft}%`, width: `${thumbWidth}%` }}
      />
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
