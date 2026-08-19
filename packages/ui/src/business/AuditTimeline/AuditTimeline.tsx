import * as React from "react";
import { cn } from "../../lib/utils";
import { StatusBadge, type StatusKey } from "../StatusBadge";

/**
 * AuditTimeline — 审计时间线（纵向时间轴 + 展开详情）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-H2-002 / US-M6-002 / US-M6-004（审计追踪查询）
 * Wave：Wave 4（M6 报表页）
 * 业务约束：append-only（不可改不可删，仅展示）；按时间倒序
 *
 * @example
 *   <AuditTimeline events={[{time:"...",actor:"张三",action:"验收",status:"completed"}]} />
 */

export interface AuditTimelineEvent {
  id: string;
  time: string;
  actor: string;
  action: string;
  module?: string;
  resource?: string;
  status: StatusKey;
  detail?: React.ReactNode;
}

export interface AuditTimelineProps extends React.HTMLAttributes<HTMLDivElement> {
  events: AuditTimelineEvent[];
  /** 当前展开的事件 id */
  expandedId?: string;
  onExpand?: (id: string) => void;
}

export const AuditTimeline = React.forwardRef<HTMLDivElement, AuditTimelineProps>(
  ({ events, expandedId, onExpand, className, ...rest }, ref) => {
    return (
      <div ref={ref} className={cn("relative font-sans", className)} {...rest}>
        {/* 竖线 */}
        <div className="absolute left-[15px] top-2 bottom-2 w-px bg-border" aria-hidden />
        <ol className="flex flex-col gap-4">
          {events.map((e) => {
            const expanded = expandedId === e.id;
            return (
              <li key={e.id} className="relative pl-10">
                {/* 圆点 */}
                <div className="absolute left-2 top-1.5 size-4 rounded-full bg-background border-2 border-primary z-10" />
                {/* 内容 */}
                <button
                  type="button"
                  onClick={() => onExpand?.(e.id)}
                  className="block w-full text-left hover:bg-accent/40 rounded-md p-2 -ml-2 transition-colors"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="text-xs text-muted-foreground font-mono">{e.time}</div>
                    <StatusBadge status={e.status} size="sm" />
                  </div>
                  <div className="mt-1 flex items-baseline gap-2 flex-wrap">
                    <span className="font-medium text-sm">{e.actor}</span>
                    <span className="text-sm">{e.action}</span>
                    {e.module && (
                      <span className="text-xs text-muted-foreground bg-muted/60 px-1.5 py-0.5 rounded">{e.module}</span>
                    )}
                    {e.resource && <code className="text-xs text-muted-foreground font-mono">{e.resource}</code>}
                  </div>
                </button>
                {expanded && e.detail && (
                  <div className="ml-0 mt-2 p-3 bg-muted/40 rounded-md border-l-2 border-primary text-xs">
                    {e.detail}
                  </div>
                )}
              </li>
            );
          })}
        </ol>
      </div>
    );
  }
);
AuditTimeline.displayName = "AuditTimeline";
