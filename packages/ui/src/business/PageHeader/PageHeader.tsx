import * as React from "react";
import { cn } from "../../lib/utils";

/**
 * PageHeader — 管理页统一头部（标题 + 副标题 + 操作区 + 面包屑）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：所有 PC 管理页（H1/H2/H3/H4/H5/M1-M10/M-* 全模块）
 * Wave：Wave 0.5 起步，全 Wave 复用
 * 业务约束：标题可选（AppShell 提供页面级 h1，页面内不再重复显示大标题）；
 *   副标题展示业务上下文（故事 ID / 模块名 / 合规说明）
 *
 * @example
 *   <PageHeader title="审计追踪查询" subtitle="H2 / append-only · GSP 法定台账" actions={<Button>导出</Button>} />
 *   <PageHeader actions={<Button>导出</Button>} />
 */
export interface PageHeaderProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
  title?: React.ReactNode;
  subtitle?: React.ReactNode;
  actions?: React.ReactNode;
  breadcrumb?: React.ReactNode;
}

export const PageHeader = React.forwardRef<HTMLDivElement, PageHeaderProps>(
  ({ title, subtitle, actions, breadcrumb, className, ...rest }, ref) => {
    const hasTitleBlock = Boolean(breadcrumb || title || subtitle);
    if (!hasTitleBlock && !actions) {
      return null;
    }
    return (
      <div
        ref={ref}
        className={cn(
          "flex items-start gap-4 mb-6",
          hasTitleBlock ? "justify-between" : "justify-end",
          className,
        )}
        {...rest}
      >
        {hasTitleBlock && (
          <div className="flex-1 min-w-0">
            {breadcrumb && <div className="text-xs text-muted-foreground mb-1">{breadcrumb}</div>}
            {title && <h2 className="text-xl font-semibold truncate">{title}</h2>}
            {subtitle && <p className="text-sm text-muted-foreground mt-1">{subtitle}</p>}
          </div>
        )}
        {actions && <div className="flex gap-2 shrink-0">{actions}</div>}
      </div>
    );
  }
);
PageHeader.displayName = "PageHeader";
