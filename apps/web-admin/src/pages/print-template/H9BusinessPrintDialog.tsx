import * as React from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@wms/ui";
import { Printer, RotateCcw } from "lucide-react";

import { ApiError } from "@/features/auth/auth-queries";
import {
  usePreviewPrintTemplateMutation,
  useRecordPrintTemplateMutation,
  type PrintTemplatePreviewRequest,
  type PrintTemplatePreviewResponse,
} from "@/features/print-template/print-template-queries";
import { H9TemplatePreviewDialog } from "./H9TemplatePreviewDialog";

export interface H9BusinessPrintTarget {
  templateTypeCode: string;
  businessModule: string;
  businessDocumentType: string;
  businessDocumentId: string;
  description: string;
  data: PrintTemplatePreviewRequest["data"];
}

export function H9BusinessPrintDialog({
  open,
  target,
  onOpenChange,
  onPrinted,
}: {
  open: boolean;
  target: H9BusinessPrintTarget | null;
  onOpenChange: (open: boolean) => void;
  onPrinted?: (target: H9BusinessPrintTarget) => void;
}) {
  const previewMutation = usePreviewPrintTemplateMutation();
  const printMutation = useRecordPrintTemplateMutation();
  const [preview, setPreview] = React.useState<PrintTemplatePreviewResponse | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!open || !target) {
      setPreview(null);
      setError(null);
      return;
    }
    let cancelled = false;
    setPreview(null);
    setError(null);
    void loadPreview(target)
      .then((value) => {
        if (!cancelled) setPreview(value);
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(printTemplateErrorMessage(cause));
      });
    return () => {
      cancelled = true;
    };
  }, [open, target]);

  async function loadPreview(current: H9BusinessPrintTarget) {
    return previewMutation.mutateAsync({
      template_code: null,
      template_type_code: current.templateTypeCode,
      business_document_id: current.businessDocumentId,
      data: current.data,
    });
  }

  async function retryPreview() {
    if (!target) return;
    setPreview(null);
    setError(null);
    try {
      setPreview(await loadPreview(target));
    } catch (cause: unknown) {
      setError(printTemplateErrorMessage(cause));
    }
  }

  async function recordPrint(
    status: "printed" | "cancelled" | "failed",
    failureReason: string | null,
  ) {
    if (!preview || !target) return;
    await printMutation.mutateAsync({
      template_code: preview.template_code,
      template_type_code: preview.template_type_code,
      business_module: target.businessModule,
      business_document_type: target.businessDocumentType,
      business_document_id: target.businessDocumentId,
      data: preview.data,
      status,
      failure_reason: failureReason,
    });
    // 仅在实际完成打印时回调；取消/失败也会登记结果，但不触发调用方的“已打印”提示。
    if (status === "printed") onPrinted?.(target);
  }

  return (
    <>
      <Dialog open={open && Boolean(target) && !preview} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>准备打印</DialogTitle>
            <DialogDescription>{target?.description ?? "未选择业务单据"}</DialogDescription>
          </DialogHeader>
          {error ? (
            <div className="grid gap-3">
              <div
                className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
                role="alert"
              >
                {error}
              </div>
              <div className="text-sm text-muted-foreground">
                请在 H9 打印模板中修复对应模板或字段库后重试。
              </div>
            </div>
          ) : (
            <div
              className="flex items-center gap-2 rounded-md border bg-muted/20 px-3 py-4 text-sm text-muted-foreground"
              role="status"
            >
              <Printer className="size-4" aria-hidden />
              正在解析 H9 打印模板...
            </div>
          )}
          <DialogFooter>
            {error && (
              <Button type="button" variant="outline" onClick={() => void retryPreview()}>
                <RotateCcw className="size-4" aria-hidden />
                重试
              </Button>
            )}
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              关闭
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <H9TemplatePreviewDialog
        open={open && Boolean(preview)}
        preview={preview}
        onOpenChange={onOpenChange}
        onPrint={recordPrint}
      />
    </>
  );
}

export function printTemplateErrorMessage(cause: unknown) {
  if (!(cause instanceof ApiError)) {
    return cause instanceof Error ? cause.message : "读取打印模板失败";
  }
  const messages: Record<string, string> = {
    H9_TEMPLATE_NOT_FOUND: "未找到可用打印模板，请先发布货主默认或全局默认模板。",
    H9_TEMPLATE_DISABLED: "打印模板已停用，请启用模板或选择其他模板。",
    H9_FIELD_LIBRARY_NOT_PUBLISHED: "模板绑定的字段库尚未发布，请先发布字段库。",
    H9_TEMPLATE_FIELD_MISMATCH: "模板与当前字段库不匹配，请重新绑定字段并发布新版本。",
  };
  return messages[cause.code] ?? cause.message;
}
