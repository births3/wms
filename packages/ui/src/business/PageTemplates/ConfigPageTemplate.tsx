import * as React from "react";
import { cn } from "../../lib/utils";
import { PageHeader, type PageHeaderProps } from "../PageHeader/PageHeader";

/**
 * ConfigPageTemplate — WMS 管理端配置中心/表单向导型页面标准模板组件
 *
 * 层级：Layer 2 业务复合
 * 关联故事：FeatureFlag 配置中心、ERP 连接器、计费规则等配置类页面
 * Wave：Wave 6 管理端页面标准化
 * 业务约束：统一封装配置区与结果反馈区，保证视口弹性撑满到底部。
 *
 * @example
 *   <ConfigPageTemplate configSlot={<Form />} feedbackSlot={<DataGrid />} />
 */
export interface ConfigPageTemplateProps
  extends Omit<React.HTMLAttributes<HTMLElement>, "title"> {
  header?: PageHeaderProps;
  notice?: { kind: "error" | "success" | "info" | "warning"; text: string } | null;
  /** 配置表单或操作卡片 */
  configSlot: React.ReactNode;
  /** 实时反馈表格或列表卡片 */
  feedbackSlot?: React.ReactNode;
  dialogs?: React.ReactNode;
  children?: React.ReactNode;
}

export const ConfigPageTemplate = React.forwardRef<
  HTMLElement,
  ConfigPageTemplateProps
>(function ConfigPageTemplate(
  {
    header,
    notice,
    configSlot,
    feedbackSlot,
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

      {configSlot}

      {feedbackSlot && (
        <div className="flex flex-1 flex-col min-h-0 min-w-0">{feedbackSlot}</div>
      )}

      {children}
      {dialogs}
    </section>
  );
});
ConfigPageTemplate.displayName = "ConfigPageTemplate";

