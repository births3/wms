/**
 * DetailDrawerTemplate — 标准业务详情抽屉模板
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端详情抽屉统一能力
 * Wave：Wave 6 管理端增强
 * 业务约束：统一单据详情、批次档案等右侧滑出抽屉的头部信息区、状态徽章、标签页切换与操作轨迹
 *
 * @example
 *   <DetailDrawerTemplate open={open} onOpenChange={setOpen} title="入库单详情">...</DetailDrawerTemplate>
 */

import * as React from "react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../../ui/dialog";

export interface DetailDrawerTemplateProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "title"> {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  statusBadge?: React.ReactNode;
  headerActions?: React.ReactNode;
  widthClassName?: string;
  children: React.ReactNode;
  footerActions?: React.ReactNode;
  closeLabel?: string;
}

export const DetailDrawerTemplate = React.forwardRef<HTMLDivElement, DetailDrawerTemplateProps>(
  (
    {
      open,
      onOpenChange,
      title,
      subtitle,
      statusBadge,
      headerActions,
      widthClassName = "sm:max-w-3xl",
      className,
      children,
      footerActions,
      closeLabel = "关闭",
      ...rest
    },
    ref
  ) => {
    return (
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent
          ref={ref}
          className={cn("max-h-[92vh] overflow-y-auto font-sans flex flex-col p-6", widthClassName, className)}
          {...rest}
        >
          <DialogHeader className="flex flex-row items-start justify-between gap-4 border-b pb-4">
            <div className="space-y-1 text-left">
              <div className="flex items-center gap-2.5">
                <DialogTitle className="text-lg font-semibold tracking-normal text-foreground">
                  {title}
                </DialogTitle>
                {statusBadge}
              </div>
              {subtitle && (
                <DialogDescription className="text-xs text-muted-foreground leading-relaxed">
                  {subtitle}
                </DialogDescription>
              )}
            </div>
            {headerActions && <div className="flex items-center gap-2 shrink-0">{headerActions}</div>}
          </DialogHeader>

          <div className="flex-1 overflow-y-auto py-4 space-y-5">{children}</div>

          <DialogFooter className="flex items-center justify-between border-t pt-4 mt-auto">
            <div className="flex items-center gap-2">{footerActions}</div>
            <DialogClose asChild>
              <Button type="button" variant="outline" size="sm">
                {closeLabel}
              </Button>
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);
DetailDrawerTemplate.displayName = "DetailDrawerTemplate";

