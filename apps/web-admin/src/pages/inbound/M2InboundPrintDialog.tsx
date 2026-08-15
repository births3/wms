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

import {
  useReceivingOrderPrintDataQuery,
  type ReceivingOrder,
} from "@/features/inbound/inbound-queries";
import type { M2InboundMode } from "./m2-inbound-page-helpers";
import { NO_INBOUND_ORDER_SELECTED } from "./M2InboundDialogs";
import {
  H9BusinessPrintDialog,
  type H9BusinessPrintTarget,
} from "../print-template/H9BusinessPrintDialog";

interface M2InboundPrintDialogProps {
  open: boolean;
  mode: M2InboundMode;
  order: ReceivingOrder | null;
  onOpenChange: (open: boolean) => void;
  onPrinted: (receiptNo: string) => void;
}

/** M2 只组装业务数据，模板解析、hiprint 预览和打印记录统一交给 H9。 */
export function M2InboundPrintDialog({
  open,
  mode,
  order,
  onOpenChange,
  onPrinted,
}: M2InboundPrintDialogProps) {
  const printDataQuery = useReceivingOrderPrintDataQuery(open && order ? order.id : null);
  const target = React.useMemo<H9BusinessPrintTarget | null>(
    () =>
      order && printDataQuery.data
        ? {
            templateTypeCode: templateTypeCode(mode),
            businessModule: "M2",
            businessDocumentType: templateTypeCode(mode),
            businessDocumentId: order.id,
            description: `${order.receipt_no} · ${templateLabel(mode)}`,
            data: printDataQuery.data,
          }
        : null,
    [mode, order, printDataQuery.data],
  );

  return (
    <>
      <Dialog open={open && !target} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>准备打印</DialogTitle>
            <DialogDescription>
              {order ? `${order.receipt_no} · ${templateLabel(mode)}` : NO_INBOUND_ORDER_SELECTED}
            </DialogDescription>
          </DialogHeader>
          {printDataQuery.isError ? (
            <div className="grid gap-3">
              <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">
                {printDataQuery.error.message}
              </div>
              <div className="text-sm text-muted-foreground">请确认 H9 已发布对应模板，且当前账号有打印权限。</div>
            </div>
          ) : (
            <div className="flex items-center gap-2 rounded-md border bg-muted/20 px-3 py-4 text-sm text-muted-foreground" role="status">
              <Printer className="size-4" aria-hidden />
              正在解析 H9 打印模板...
            </div>
          )}
          <DialogFooter>
            {printDataQuery.isError && (
              <Button type="button" variant="outline" onClick={() => retryPreview()}>
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
      <H9BusinessPrintDialog
        open={open && Boolean(target)}
        target={target}
        onOpenChange={onOpenChange}
        onPrinted={() => {
          if (order) onPrinted(order.receipt_no);
        }}
      />
    </>
  );

  async function retryPreview() {
    await printDataQuery.refetch();
  }
}

function templateTypeCode(mode: M2InboundMode) {
  return mode === "receiving" ? "asn" : "acceptance_record";
}

function templateLabel(mode: M2InboundMode) {
  return mode === "receiving" ? "ASN 单" : "验收记录单";
}
