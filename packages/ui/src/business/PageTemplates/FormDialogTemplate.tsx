/**
 * FormDialogTemplate — 标准表单弹窗模板
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端弹窗表单统一能力
 * Wave：Wave 6 管理端增强
 * 业务约束：统一新建、编辑、审核等业务弹窗的网格分栏、错误反馈与底部按钮交互
 *
 * @example
 *   <FormDialogTemplate open={open} onOpenChange={setOpen} title="新建货主" onSubmit={submit}>...</FormDialogTemplate>
 */

import * as React from "react";
import { Loader2 } from "lucide-react";
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

export interface FormDialogTemplateProps extends React.HTMLAttributes<HTMLDivElement> {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: React.ReactNode;
  description?: React.ReactNode;
  errorMessage?: React.ReactNode;
  loading?: boolean;
  submitLabel?: string;
  cancelLabel?: string;
  submitDisabled?: boolean;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void | Promise<void>;
  columns?: 1 | 2 | 3;
  maxWidthClassName?: string;
  children: React.ReactNode;
  extraFooterActions?: React.ReactNode;
}

export const FormDialogTemplate = React.forwardRef<HTMLDivElement, FormDialogTemplateProps>(
  (
    {
      open,
      onOpenChange,
      title,
      description,
      errorMessage,
      loading = false,
      submitLabel = "确认保存",
      cancelLabel = "取消",
      submitDisabled = false,
      onSubmit,
      columns = 1,
      maxWidthClassName = "sm:max-w-xl",
      className,
      children,
      extraFooterActions,
      ...rest
    },
    ref
  ) => {
    const gridClass =
      columns === 3
        ? "grid grid-cols-1 md:grid-cols-3 gap-4"
        : columns === 2
          ? "grid grid-cols-1 md:grid-cols-2 gap-4"
          : "grid grid-cols-1 gap-4";

    return (
      <Dialog open={open} onOpenChange={(next) => !loading && onOpenChange(next)}>
        <DialogContent ref={ref} className={cn("max-h-[90vh] overflow-y-auto font-sans", maxWidthClassName, className)} {...rest}>
          <form onSubmit={onSubmit} className="space-y-5">
            <DialogHeader>
              <DialogTitle className="text-base font-semibold">{title}</DialogTitle>
              {description && (
                <DialogDescription className="text-xs text-muted-foreground leading-relaxed">
                  {description}
                </DialogDescription>
              )}
            </DialogHeader>

            {errorMessage && (
              <div
                className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive leading-relaxed"
                role="alert"
              >
                {errorMessage}
              </div>
            )}

            <div className={gridClass}>{children}</div>

            <DialogFooter className="flex items-center justify-end gap-2 pt-2 border-t">
              {extraFooterActions}
              <DialogClose asChild>
                <Button type="button" variant="outline" size="sm" disabled={loading}>
                  {cancelLabel}
                </Button>
              </DialogClose>
              <Button type="submit" size="sm" disabled={loading || submitDisabled}>
                {loading && <Loader2 className="mr-1.5 size-3.5 animate-spin" aria-hidden />}
                {submitLabel}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    );
  }
);
FormDialogTemplate.displayName = "FormDialogTemplate";

