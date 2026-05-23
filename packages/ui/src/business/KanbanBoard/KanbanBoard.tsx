import * as React from "react";
import { cn } from "../../lib/utils";
import { StatusBadge, type StatusKey } from "../StatusBadge";

/**
 * KanbanBoard — 多列看板（卡片 + 实时刷新）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-M2-008（收货进度看板）/ US-M4-007（出库看板）/ US-TE-009（任务跟踪）/ US-DOCK-006（月台占用）
 * Wave：Wave 3（M2/M4 看板）
 * 业务约束：实时刷新 ≤ 3s；列数 2-6；超 50 张卡片虚拟滚动
 *
 * @example
 *   <KanbanBoard columns={[{title:"待验收",items:[...]}, ...]} />
 */

export interface KanbanCard {
  id: string;
  title: string;
  subtitle?: string;
  status?: StatusKey;
  meta?: { label: string; value: string }[];
  /** 警告 / 优先级颜色（左边条） */
  priority?: "low" | "normal" | "high" | "urgent";
}

export interface KanbanColumn {
  title: string;
  count?: number;
  /** 列头颜色 */
  variant?: "default" | "warning" | "success" | "danger";
  items: KanbanCard[];
}

export interface KanbanBoardProps extends React.HTMLAttributes<HTMLDivElement> {
  columns: KanbanColumn[];
  /** 卡片点击 */
  onCardClick?: (card: KanbanCard, columnIdx: number) => void;
}

const COL_VARIANT = {
  default: "bg-muted/40",
  warning: "bg-wms-warning/10",
  success: "bg-wms-success/10",
  danger: "bg-destructive/10",
};

const PRIORITY_BORDER = {
  low: "border-l-muted-foreground/30",
  normal: "border-l-primary",
  high: "border-l-wms-warning",
  urgent: "border-l-destructive",
};

export const KanbanBoard = React.forwardRef<HTMLDivElement, KanbanBoardProps>(
  ({ columns, onCardClick, className, ...rest }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("grid gap-3 font-sans", className)}
        style={{ gridTemplateColumns: `repeat(${columns.length}, minmax(0, 1fr))` }}
        {...rest}
      >
        {columns.map((col, ci) => (
          <div key={ci} className="flex flex-col rounded-md border bg-background overflow-hidden min-h-[400px]">
            <div className={cn("px-3 py-2 border-b flex items-center justify-between", COL_VARIANT[col.variant ?? "default"])}>
              <span className="text-sm font-semibold">{col.title}</span>
              <span className="text-xs text-muted-foreground bg-background/80 px-2 py-0.5 rounded">
                {col.count ?? col.items.length}
              </span>
            </div>
            <div className="flex-1 p-2 space-y-2 overflow-auto">
              {col.items.map((card) => (
                <button
                  key={card.id}
                  type="button"
                  onClick={() => onCardClick?.(card, ci)}
                  className={cn(
                    "w-full text-left bg-background rounded border p-2 hover:shadow-sm transition-shadow border-l-4",
                    PRIORITY_BORDER[card.priority ?? "normal"]
                  )}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <div className="text-sm font-medium truncate">{card.title}</div>
                      {card.subtitle && <div className="text-xs text-muted-foreground mt-0.5 truncate">{card.subtitle}</div>}
                    </div>
                    {card.status && <StatusBadge status={card.status} size="sm" />}
                  </div>
                  {card.meta && card.meta.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
                      {card.meta.map((m, i) => (
                        <span key={i}>
                          {m.label}: <span className="text-foreground/80">{m.value}</span>
                        </span>
                      ))}
                    </div>
                  )}
                </button>
              ))}
              {col.items.length === 0 && (
                <div className="text-center text-xs text-muted-foreground py-8">空</div>
              )}
            </div>
          </div>
        ))}
      </div>
    );
  }
);
KanbanBoard.displayName = "KanbanBoard";
