import * as React from "react";
import { cn } from "../../lib/utils";
import { PageHeader, type PageHeaderProps } from "../PageHeader/PageHeader";

/**
 * DashboardPageTemplate — WMS 管理端仪表盘与实时监控型页面标准模板组件
 *
 * 层级：Layer 2 业务复合
 * 关联故事：工作台看板、活动告警监控、月台预约监控等页面
 * Wave：Wave 6 管理端页面标准化
 * 业务约束：统一封装顶部 KPI 栅格 + 实时监控 DataGrid，保证垂直视口撑满。
 *
 * @example
 *   <DashboardPageTemplate kpiSlot={<KpiGrid />} mainSlot={<AlertStream />} />
 */
export interface DashboardPageTemplateProps
  extends Omit<React.HTMLAttributes<HTMLElement>, "title"> {
  header?: PageHeaderProps;
  notice?: { kind: "error" | "success" | "info" | "warning"; text: string } | null;
  /** 顶部 KPI 统计卡片栅格 */
  kpiSlot?: React.ReactNode;
  /** 核心监控或报表内容卡片（通常包含 DataGrid） */
  mainSlot: React.ReactNode;
  dialogs?: React.ReactNode;
  children?: React.ReactNode;
}

export const DashboardPageTemplate = React.forwardRef<
  HTMLElement,
  DashboardPageTemplateProps
>(function DashboardPageTemplate(
  {
    header,
    notice,
    kpiSlot,
    mainSlot,
    dialogs,
    children,
    className,
    ...rest
  },
  ref,
) {
  return (
    <section
      ref={ref}
      className={cn(
        "flex w-full flex-1 min-h-0 flex-col gap-5 px-4 py-8 lg:px-8",
        className,
      )}
      {...rest}
    >
      <PageHeader {...header} />

      {notice && (
        <div
          role={notice.kind === "error" ? "alert" : "status"}
          className={cn(
            "rounded-md border px-3 py-2 text-sm",
            notice.kind === "error" && "border-destructive/30 bg-destructive/10 text-destructive",
            notice.kind === "success" && "border-wms-success/30 bg-wms-success/10 text-wms-success",
            notice.kind === "warning" && "border-wms-warning/30 bg-wms-warning/10 text-wms-warning",
            notice.kind === "info" && "border-primary/30 bg-primary/10 text-primary",
          )}
        >
          {notice.text}
        </div>
      )}

      {kpiSlot}

      <div className="flex flex-1 flex-col min-h-0 min-w-0">{mainSlot}</div>

      {children}
      {dialogs}
    </section>
  );
});
DashboardPageTemplate.displayName = "DashboardPageTemplate";

