/**
 * TwoPanePageTemplate — 双栏目录型页面黄金模板
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端双栏目录布局能力
 * Wave：Wave 6 管理端增强
 * 业务约束：为系统字典、规则中心、配置分类等页面提供统一的“左侧目录/树 + 右侧明细/表单”自适应排版
 *
 * @example
 *   <TwoPanePageTemplate leftPane={<LeftTree />} rightPane={<RightForm />} />
 */

import * as React from "react";
import { cn } from "../../lib/utils";
import { Card, CardContent } from "../../ui/card";
import { PageHeader, type PageHeaderProps } from "../PageHeader";

export interface TwoPanePageTemplateProps extends React.HTMLAttributes<HTMLDivElement> {
  header?: PageHeaderProps;
  banner?: React.ReactNode;
  leftPane: React.ReactNode;
  rightPane: React.ReactNode;
  leftWidthClassName?: string;
  className?: string;
}

export const TwoPanePageTemplate = React.forwardRef<HTMLDivElement, TwoPanePageTemplateProps>(
  (
    {
      header,
      banner,
      leftPane,
      rightPane,
      leftWidthClassName = "w-full lg:w-72 xl:w-80",
      className,
      ...rest
    },
    ref
  ) => {
    return (
      <div ref={ref} className={cn("flex flex-1 flex-col min-h-0 space-y-4 font-sans", className)} {...rest}>
        {header && <PageHeader {...header} />}
        {banner}

        <div className="flex flex-1 flex-col lg:flex-row min-h-0 gap-4">
          {/* 左栏目录卡片 */}
          <Card className={cn("shrink-0 flex flex-col min-h-0 overflow-hidden shadow-sm", leftWidthClassName)}>
            <CardContent className="flex flex-1 flex-col min-h-0 p-4">{leftPane}</CardContent>
          </Card>

          {/* 右栏内容区卡片 */}
          <Card className="flex flex-1 flex-col min-h-0 overflow-hidden shadow-sm">
            <CardContent className="flex flex-1 flex-col min-h-0 p-5">{rightPane}</CardContent>
          </Card>
        </div>
      </div>
    );
  }
);
TwoPanePageTemplate.displayName = "TwoPanePageTemplate";

