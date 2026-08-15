import * as React from "react";
import { AlertCircle, Inbox, RefreshCw, SearchX } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";

/**
 * EmptyState — 空状态与异常恢复复合组件
 *
 * 层级：Layer 2 业务复合
 * 关联故事：全部列表/看板/详情场景（无数据时统一展示）
 * Wave：Wave 0.5 起步
 * 业务约束：默认图标 Inbox；支持 search/error/empty 预设恢复动作
 *
 * @example
 *   <EmptyState variant="search" onClearFilter={() => reset()} />
 */
export type EmptyStateVariant = "empty" | "search" | "error" | "custom";

export interface EmptyStateProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
  variant?: EmptyStateVariant;
  icon?: React.ReactNode;
  title?: React.ReactNode;
  description?: React.ReactNode;
  action?: React.ReactNode;
  onClearFilter?: () => void;
  onCreate?: () => void;
  onRetry?: () => void;
  clearFilterLabel?: string;
  createLabel?: string;
  retryLabel?: string;
}

export const EmptyState = React.forwardRef<HTMLDivElement, EmptyStateProps>(
  (
    {
      variant = "empty",
      icon,
      title,
      description,
      action,
      onClearFilter,
      onCreate,
      onRetry,
      clearFilterLabel = "清空筛选条件",
      createLabel = "立即新建",
      retryLabel = "重新加载",
      className,
      ...rest
    },
    ref
  ) => {
    // 根据 variant 推导默认图标、标题、描述与动作
    let defaultIcon: React.ReactNode = <Inbox className="size-10 text-muted-foreground/60" aria-hidden />;
    let defaultTitle: React.ReactNode = "暂无数据";
    let defaultDescription: React.ReactNode = "当前列表没有任何记录";
    let defaultAction: React.ReactNode = null;

    if (variant === "search") {
      defaultIcon = <SearchX className="size-10 text-muted-foreground/60" aria-hidden />;
      defaultTitle = "未找到匹配结果";
      defaultDescription = "没有找到符合当前筛选条件的数据，请尝试调整或重置筛选。";
      if (onClearFilter) {
        defaultAction = (
          <Button type="button" variant="outline" size="sm" onClick={onClearFilter}>
            {clearFilterLabel}
          </Button>
        );
      }
    } else if (variant === "error") {
      defaultIcon = <AlertCircle className="size-10 text-destructive/80" aria-hidden />;
      defaultTitle = "数据加载失败";
      defaultDescription = "网络连接异常或服务暂不可用，请稍后重试。";
      if (onRetry) {
        defaultAction = (
          <Button type="button" variant="outline" size="sm" onClick={onRetry}>
            <RefreshCw className="mr-1.5 size-3.5" aria-hidden />
            {retryLabel}
          </Button>
        );
      }
    } else if (variant === "empty" && onCreate) {
      defaultAction = (
        <Button type="button" size="sm" onClick={onCreate}>
          {createLabel}
        </Button>
      );
    }

    const finalIcon = icon ?? defaultIcon;
    const finalTitle = title ?? defaultTitle;
    const finalDescription = description !== undefined ? description : defaultDescription;
    const finalAction = action ?? defaultAction;

    return (
      <div
        ref={ref}
        className={cn("flex flex-col items-center justify-center py-12 px-6 text-center font-sans", className)}
        {...rest}
      >
        <div className="mb-3 text-muted-foreground/70">{finalIcon}</div>
        <div className="text-sm font-semibold text-foreground tracking-normal">{finalTitle}</div>
        {finalDescription && (
          <div className="text-xs text-muted-foreground mt-1 max-w-sm leading-relaxed">{finalDescription}</div>
        )}
        {finalAction && <div className="mt-4">{finalAction}</div>}
      </div>
    );
  }
);
EmptyState.displayName = "EmptyState";

