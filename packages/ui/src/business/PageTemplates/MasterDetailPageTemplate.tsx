import * as React from "react";
import { cn } from "../../lib/utils";
import { PageHeader, type PageHeaderProps } from "../PageHeader/PageHeader";
import {
  QueryPanel,
  type QueryPanelField,
  type QueryPanelOption,
  type QueryPanelProps,
  type QueryPanelValue,
} from "../QueryPanel/QueryPanel";

/**
 * MasterDetailPageTemplate — WMS 管理端双栏目录/联动型页面标准模板组件
 *
 * 层级：Layer 2 业务复合
 * 关联故事：角色权限矩阵、打印模板、菜单管理等双栏联动页面
 * Wave：Wave 6 管理端页面标准化
 * 业务约束：左右两栏响应式自适应撑满到底部，支持左右独立滚动与数据联动。
 *
 * @example
 *   <MasterDetailPageTemplate leftSlot={<LeftPanel />} rightSlot={<RightPanel />} />
 */
export interface MasterDetailPageTemplateProps
  extends Omit<React.HTMLAttributes<HTMLElement>, "title"> {
  header?: PageHeaderProps;
  notice?: { kind: "error" | "success" | "info" | "warning"; text: string } | null;
  /** 自定义横幅插槽 */
  banner?: React.ReactNode;
  queryFields?: QueryPanelField[];
  fieldOptions?: Record<string, QueryPanelOption[]>;
  coreQueryFieldKeys?: string[];
  queryValue?: QueryPanelValue;
  onQueryValueChange?: (value: QueryPanelValue) => void;
  onQuery?: () => void;
  onReset?: () => void;
  queryPanelProps?: Partial<QueryPanelProps>;
  /** 左侧主栏内容（通常为 DataGrid 或 TreeCatalog） */
  leftSlot: React.ReactNode;
  /** 右侧从属面板内容（通常为详情卡片、权限矩阵或属性表单） */
  rightSlot: React.ReactNode;
  /** 左右分栏栅格配置（默认 1.1fr : 0.9fr） */
  gridClassName?: string;
  dialogs?: React.ReactNode;
  children?: React.ReactNode;
}

export const MasterDetailPageTemplate = React.forwardRef<
  HTMLElement,
  MasterDetailPageTemplateProps
>(function MasterDetailPageTemplate(
  {
    header,
    notice,
    banner,
    queryFields,
    fieldOptions,
    coreQueryFieldKeys,
    queryValue,
    onQueryValueChange,
    onQuery,
    onReset,
    queryPanelProps,
    leftSlot,
    rightSlot,
    gridClassName,
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

      {banner}

      {!banner && notice && (
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

      {queryFields && queryFields.length > 0 && (
        <QueryPanel
          fields={queryFields}
          fieldOptions={fieldOptions}
          defaultVisibleFieldKeys={coreQueryFieldKeys}
          value={queryValue ?? {}}
          onValueChange={onQueryValueChange}
          onQuery={onQuery}
          onReset={onReset}
          {...queryPanelProps}
        />
      )}

      <div
        className={cn(
          "grid flex-1 min-h-0 gap-4 xl:grid-cols-[minmax(30rem,1.1fr)_minmax(26rem,0.9fr)]",
          gridClassName,
        )}
      >
        <div className="flex flex-1 flex-col min-h-0 min-w-0">{leftSlot}</div>
        <div className="flex flex-1 flex-col min-h-0 min-w-0">{rightSlot}</div>
      </div>

      {children}
      {dialogs}
    </section>
  );
});
MasterDetailPageTemplate.displayName = "MasterDetailPageTemplate";

