import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { Wifi, AlertTriangle, Loader2 } from "lucide-react";
import { cn } from "../../lib/utils";

/**
 * OfflineIndicator — PDA 顶部离线/同步状态 banner
 *
 * 层级：Layer 2 业务复合
 * 关联故事：全部 PDA 故事（M2/M3/M4/M-SA/M-PK/...）；H1 §7 离线策略
 * Wave：Wave 0.5 起步
 * 业务约束：online 状态默认隐藏（避免占屏）；syncing 100% 完成自动隐藏
 *
 * @example
 *   <OfflineIndicator state="offline" pendingCount={12} />
 */

const indicatorVariants = cva(
  "flex items-center justify-between w-full px-4 py-2 text-base font-medium font-sans box-border",
  {
    variants: {
      state: {
        online: "bg-wms-success text-white",
        offline: "bg-wms-warning text-white",
        syncing: "bg-primary text-primary-foreground",
      },
    },
    defaultVariants: { state: "online" },
  }
);

export type OfflineState = NonNullable<VariantProps<typeof indicatorVariants>["state"]>;

const STATE_TEXT: Record<OfflineState, string> = {
  online: "在线",
  offline: "离线模式",
  syncing: "同步中",
};

export interface OfflineIndicatorProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children">,
    VariantProps<typeof indicatorVariants> {
  state: OfflineState;
  pendingCount?: number;
  /** 同步进度 0-100（state=syncing 时使用） */
  syncProgress?: number;
}

export const OfflineIndicator = React.forwardRef<HTMLDivElement, OfflineIndicatorProps>(
  ({ state, pendingCount, syncProgress, className, ...rest }, ref) => {
    // online 状态默认不展示
    if (state === "online" && !pendingCount) return null;
    // syncing 100% 完成且无暂存 → 同步刚结束自动隐藏
    if (state === "syncing" && syncProgress === 100 && !pendingCount) return null;

    const Icon = state === "syncing" ? Loader2 : state === "offline" ? AlertTriangle : Wifi;

    return (
      <div
        ref={ref}
        role="status"
        aria-live="polite"
        data-state={state}
        className={cn(indicatorVariants({ state }), className)}
        {...rest}
      >
        <span className="flex items-center gap-2">
          <Icon aria-hidden className={cn("size-4", state === "syncing" && "animate-spin")} />
          <span>{STATE_TEXT[state]}</span>
          {pendingCount !== undefined && pendingCount > 0 && (
            <span className="rounded-full bg-white/25 px-2 py-0.5 text-sm">
              待同步 {pendingCount}
            </span>
          )}
        </span>
        {state === "syncing" && syncProgress !== undefined && (
          <span className="text-sm">{Math.round(syncProgress)}%</span>
        )}
      </div>
    );
  }
);
OfflineIndicator.displayName = "OfflineIndicator";
