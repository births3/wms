import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { Check, X, Clock, Lock, AlertTriangle, ArrowRight, Cloud } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * StatusBadge — 业务状态标签
 *
 * 层级：Layer 2 业务复合
 * 关联故事：全部状态机故事（M2/M3/M4/M-BA/M-VR/...）
 * Wave：Wave 0.5 起步，覆盖至 Wave 5
 * 业务约束：颜色映射严格对齐 docs/prototypes/component-registry.md §4.3
 *
 * @example
 *   <StatusBadge status="qualified" size="default" />
 *   <StatusBadge status="unqualified" label="外观破损" size="sm" />
 */

const badgeVariants = cva(
  "inline-flex items-center gap-1 rounded-md font-medium whitespace-nowrap",
  {
    variants: {
      status: {
        qualified: "bg-wms-success text-white",
        unqualified: "bg-destructive text-destructive-foreground",
        pending: "bg-wms-warning text-white",
        isolated: "bg-muted-foreground text-white",
        expired: "bg-destructive text-destructive-foreground",
        near_expiry: "bg-wms-warning text-white",
        in_progress: "bg-primary text-primary-foreground",
        completed: "bg-wms-success text-white",
        offline_cached: "bg-muted-foreground/70 text-white",
      },
      size: {
        sm: "text-[11px] px-1.5 py-0.5 [&>svg]:size-3",
        default: "text-sm px-2.5 py-1 [&>svg]:size-3.5",
        lg: "text-base px-3.5 py-1.5 font-semibold [&>svg]:size-4",
      },
    },
    defaultVariants: { status: "qualified", size: "default" },
  }
);

export type StatusKey = NonNullable<VariantProps<typeof badgeVariants>["status"]>;

const STATUS_META: Record<StatusKey, { icon: React.ComponentType<{ className?: string }>; text: string }> = {
  qualified: { icon: Check, text: "合格" },
  unqualified: { icon: X, text: "不合格" },
  pending: { icon: Clock, text: "待处理" },
  isolated: { icon: Lock, text: "隔离" },
  expired: { icon: AlertTriangle, text: "已过期" },
  near_expiry: { icon: AlertTriangle, text: "近效期" },
  in_progress: { icon: ArrowRight, text: "进行中" },
  completed: { icon: Check, text: "已完成" },
  offline_cached: { icon: Cloud, text: "离线暂存" },
};

export interface StatusBadgeProps
  extends Omit<React.HTMLAttributes<HTMLSpanElement>, "children">,
    VariantProps<typeof badgeVariants> {
  status: StatusKey;
  /** 自定义文字（覆盖默认） */
  label?: string;
}

export const StatusBadge = React.forwardRef<HTMLSpanElement, StatusBadgeProps>(
  ({ status, size, label, className, ...rest }, ref) => {
    const meta = STATUS_META[status];
    const Icon = meta.icon;
    return (
      <span
        ref={ref}
        data-status={status}
        className={cn(badgeVariants({ status, size }), className)}
        {...rest}
      >
        <Icon aria-hidden />
        <span>{label ?? meta.text}</span>
      </span>
    );
  }
);
StatusBadge.displayName = "StatusBadge";
