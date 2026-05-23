import * as React from "react";
import { cn } from "../../lib/utils";
import { Inbox } from "lucide-react";

/**
 * EmptyState — 空状态展示（图标 + 标题 + 描述 + CTA）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：所有列表 / 看板 / 详情场景（无数据时统一展示）
 * Wave：Wave 0.5 起步
 * 业务约束：默认图标 Inbox；可自定义图标 / 描述 / 操作按钮
 *
 * @example
 *   <EmptyState title="暂无审计事件" description="尝试调整筛选条件" />
 */
export interface EmptyStateProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
  icon?: React.ReactNode;
  title: React.ReactNode;
  description?: React.ReactNode;
  action?: React.ReactNode;
}

export const EmptyState = React.forwardRef<HTMLDivElement, EmptyStateProps>(
  ({ icon, title, description, action, className, ...rest }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("flex flex-col items-center justify-center py-12 px-6 text-center font-sans", className)}
        {...rest}
      >
        <div className="mb-3 text-muted-foreground/70">
          {icon ?? <Inbox className="size-10" aria-hidden />}
        </div>
        <div className="text-sm font-medium">{title}</div>
        {description && <div className="text-xs text-muted-foreground mt-1 max-w-sm">{description}</div>}
        {action && <div className="mt-4">{action}</div>}
      </div>
    );
  }
);
EmptyState.displayName = "EmptyState";
