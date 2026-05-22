import * as React from "react";
import { cn } from "@/lib/utils";
import { StatusBadge, type StatusKey } from "../StatusBadge";
import { ChevronRight } from "lucide-react";

/**
 * ApprovalFlow — 审批流程面板（节点 + 当前态 + 意见）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-QL-003（质量联系单审批）/ US-BA-002（批号调整双签）/ US-DOCK-004（月台预约）
 * Wave：Wave 4（M-QL/BA 业务页）
 * 业务约束：节点状态 4 档（pending/approved/rejected/skipped）；驳回必须有意见
 *
 * @example
 *   <ApprovalFlow nodes={[{role:"主管",approver:"张三",status:"approved"},...]} />
 */

export type ApprovalNodeStatus = "pending" | "approved" | "rejected" | "current" | "skipped";

export interface ApprovalNode {
  role: string;
  approver?: string;
  time?: string;
  comment?: string;
  status: ApprovalNodeStatus;
}

export interface ApprovalFlowProps extends React.HTMLAttributes<HTMLDivElement> {
  nodes: ApprovalNode[];
}

const STATUS_TO_BADGE: Record<ApprovalNodeStatus, { status: StatusKey; label: string }> = {
  pending: { status: "pending", label: "待审" },
  current: { status: "in_progress", label: "审批中" },
  approved: { status: "completed", label: "已通过" },
  rejected: { status: "unqualified", label: "已驳回" },
  skipped: { status: "isolated", label: "已跳过" },
};

export const ApprovalFlow = React.forwardRef<HTMLDivElement, ApprovalFlowProps>(
  ({ nodes, className, ...rest }, ref) => {
    return (
      <div ref={ref} className={cn("space-y-3 font-sans", className)} {...rest}>
        {nodes.map((n, i) => {
          const meta = STATUS_TO_BADGE[n.status];
          const isLast = i === nodes.length - 1;
          const isRejected = n.status === "rejected";
          return (
            <div key={i} className="relative">
              <div
                className={cn(
                  "rounded-md border p-3 transition-colors",
                  n.status === "current" && "border-primary bg-primary/5",
                  isRejected && "border-destructive bg-destructive/5",
                  n.status === "approved" && "border-wms-success/40"
                )}
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={cn(
                        "size-7 rounded-full flex items-center justify-center text-xs font-semibold shrink-0",
                        n.status === "approved" && "bg-wms-success text-white",
                        n.status === "current" && "bg-primary text-primary-foreground",
                        n.status === "rejected" && "bg-destructive text-destructive-foreground",
                        n.status === "pending" && "bg-muted text-muted-foreground border",
                        n.status === "skipped" && "bg-muted-foreground text-white"
                      )}
                    >
                      {i + 1}
                    </div>
                    <div className="min-w-0">
                      <div className="text-sm font-medium truncate">
                        {n.role}
                        {n.approver && <span className="text-muted-foreground ml-1.5 font-normal">· {n.approver}</span>}
                      </div>
                      {n.time && <div className="text-xs text-muted-foreground">{n.time}</div>}
                    </div>
                  </div>
                  <StatusBadge status={meta.status} size="sm" label={meta.label} />
                </div>
                {n.comment && (
                  <div className={cn("mt-2 ml-10 text-xs px-3 py-2 rounded", isRejected ? "bg-destructive/10 text-destructive" : "bg-muted/60 text-foreground/70")}>
                    "{n.comment}"
                  </div>
                )}
              </div>
              {!isLast && (
                <div className="flex justify-start ml-3.5 my-1 text-muted-foreground/60">
                  <ChevronRight className="size-3 rotate-90" />
                </div>
              )}
            </div>
          );
        })}
      </div>
    );
  }
);
ApprovalFlow.displayName = "ApprovalFlow";
