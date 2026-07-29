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
  usePreviewPrintTemplateMutation,
  useRecordPrintTemplateMutation,
  type PrintTemplatePreviewResponse,
} from "@/features/print-template/print-template-queries";
import {
  useReceivingOrderPrintDataQuery,
  type ReceivingOrder,
  type ReceivingOrderPrintData,
} from "@/features/inbound/inbound-queries";
import type { OwnerContext, M2InboundMode } from "./m2-inbound-page-helpers";
import { ownerLabel, statusLabel, totalExpectedQty } from "./m2-inbound-page-helpers";
import { H9TemplatePreviewDialog } from "../print-template/H9TemplatePreviewDialog";

interface M2InboundPrintDialogProps {
  open: boolean;
  mode: M2InboundMode;
  order: ReceivingOrder | null;
  currentOwner: OwnerContext;
  onOpenChange: (open: boolean) => void;
  onPrinted: (receiptNo: string) => void;
}

/** M2 只组装业务数据，模板解析、hiprint 预览和打印记录统一交给 H9。 */
export function M2InboundPrintDialog({
  open,
  mode,
  order,
  currentOwner,
  onOpenChange,
  onPrinted,
}: M2InboundPrintDialogProps) {
  const previewMutation = usePreviewPrintTemplateMutation();
  const printMutation = useRecordPrintTemplateMutation();
  const printDataQuery = useReceivingOrderPrintDataQuery(open && order ? order.id : null);
  const [preview, setPreview] = React.useState<PrintTemplatePreviewResponse | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!open || !order) {
      setPreview(null);
      setError(null);
      return;
    }

    if (printDataQuery.isPending) {
      setPreview(null);
      setError(null);
      return;
    }
    if (printDataQuery.isError) {
      setPreview(null);
      setError(printDataQuery.error.message);
      return;
    }
    if (!printDataQuery.data) return;

    let cancelled = false;
    setPreview(null);
    setError(null);
    void previewMutation
      .mutateAsync({
        template_code: null,
        template_type_code: templateTypeCode(mode),
        business_document_id: order.id,
        data: buildInboundPrintData(printDataQuery.data, currentOwner),
      })
      .then((nextPreview) => {
        if (!cancelled) setPreview(nextPreview);
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(cause instanceof Error ? cause.message : "读取入库打印模板失败");
      });

    return () => {
      cancelled = true;
    };
  }, [currentOwner.ownerCode, currentOwner.ownerId, mode, open, order?.id, printDataQuery.data, printDataQuery.error, printDataQuery.isError, printDataQuery.isPending]);

  async function recordPrint(status: "printed" | "cancelled" | "failed", failureReason: string | null) {
    if (!preview || !order) return;
    await printMutation.mutateAsync({
      template_code: preview.template_code,
      template_type_code: preview.template_type_code,
      business_module: "M2",
      business_document_type: preview.template_type_code,
      business_document_id: order.id,
      data: preview.data,
      status,
      failure_reason: failureReason,
    });
    onPrinted(order.receipt_no);
  }

  return (
    <>
      <Dialog open={open && !preview} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>准备打印</DialogTitle>
            <DialogDescription>
              {order ? `${order.receipt_no} · ${templateLabel(mode)}` : "未选择入库单"}
            </DialogDescription>
          </DialogHeader>
          {error ? (
            <div className="grid gap-3">
              <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">
                {error}
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
            {error && (
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
      <H9TemplatePreviewDialog
        open={open && Boolean(preview)}
        preview={preview}
        onOpenChange={onOpenChange}
        onPrint={recordPrint}
      />
    </>
  );

  async function retryPreview() {
    setError(null);
    setPreview(null);
    if (!order) return;
    try {
      const result = await printDataQuery.refetch();
      if (!result.data) throw new Error("读取入库打印数据失败");
      const nextPreview = await previewMutation.mutateAsync({
        template_code: null,
        template_type_code: templateTypeCode(mode),
        business_document_id: order.id,
        data: buildInboundPrintData(result.data, currentOwner),
      });
      setPreview(nextPreview);
    } catch (cause: unknown) {
      setError(cause instanceof Error ? cause.message : "读取入库打印模板失败");
    }
  }
}

function templateTypeCode(mode: M2InboundMode) {
  return mode === "receiving" ? "asn" : "acceptance_record";
}

function templateLabel(mode: M2InboundMode) {
  return mode === "receiving" ? "ASN 单" : "验收记录单";
}

function buildInboundPrintData(printData: ReceivingOrderPrintData, currentOwner: OwnerContext) {
  const order = printData.order;
  const lines = (order.lines ?? []).map((line) => ({
    line_no: line.line_no,
    product_code: line.product_code,
    batch_no: line.batch_no,
    expected_qty: line.expected_qty,
    production_date: line.production_date,
    expiry_date: line.expiry_date,
  }));
  const firstLine = lines[0] ?? null;
  const receipt = printData.receipts[printData.receipts.length - 1] ?? null;
  const signature = printData.signatures[printData.signatures.length - 1] ?? null;
  return {
    asn: {
      code: order.receipt_no,
      document_type: order.document_type,
      status: statusLabel(order.status),
      expected_arrival_at: order.expected_arrival_at,
    },
    order: {
      id: order.id,
      code: order.receipt_no,
      document_type: order.document_type,
      status: statusLabel(order.status),
      owner: ownerLabel(order.owner_id, currentOwner),
      supplier_id: order.supplier_id,
      warehouse_id: order.warehouse_id,
      total_expected_qty: totalExpectedQty(order),
    },
    owner: { id: order.owner_id, code: currentOwner.ownerCode },
    supplier: { id: order.supplier_id },
    warehouse: { id: order.warehouse_id },
    product: firstLine
      ? { code: firstLine.product_code, batch_no: firstLine.batch_no, expiry_date: firstLine.expiry_date }
      : null,
    products: lines,
    batch: firstLine
      ? { no: firstLine.batch_no, production_date: firstLine.production_date, expiry_date: firstLine.expiry_date }
      : null,
    receiving: {
      actual_qty: receipt?.actual_qty ?? null,
      shortage_qty: receipt?.shortage_qty ?? null,
      rejected_qty: receipt?.rejected_qty ?? null,
      arrival_temperature_celsius: receipt?.arrival_temperature_celsius ?? null,
      exception_note: receipt?.exception_note ?? null,
      temperature_control_method: receipt?.details?.temperature_control_method ?? null,
      vehicle_no: receipt?.details?.vehicle_no ?? null,
      origin: receipt?.details?.origin ?? null,
      departure_at: receipt?.details?.departure_at ?? null,
      arrival_at: receipt?.details?.arrival_at ?? null,
      storage_at: receipt?.details?.storage_at ?? null,
      transport_mode: receipt?.details?.transport_mode ?? null,
      carrier: receipt?.details?.carrier ?? null,
      contact_name: receipt?.details?.contact_name ?? null,
      contact_phone: receipt?.details?.contact_phone ?? null,
      contact_id_no: receipt?.details?.contact_id_no ?? null,
      seal_checked: receipt?.details?.seal_checked ?? null,
      filing_checked: receipt?.details?.filing_checked ?? null,
      transport_duration_minutes: transportDurationMinutes(
        receipt?.details?.departure_at,
        receipt?.details?.arrival_at,
      ),
    },
    inspection: {
      conclusion: printData.inspections.map((record) => record.quality_status).join("/") || null,
      first_signer_id: signature?.first_signer_id ?? null,
      second_signer_id: signature?.second_signer_id ?? null,
      records: printData.inspections,
    },
  };
}

function transportDurationMinutes(departureAt: string | null | undefined, arrivalAt: string | null | undefined) {
  if (!departureAt || !arrivalAt) return null;
  const duration = Math.round((Date.parse(arrivalAt) - Date.parse(departureAt)) / 60000);
  return Number.isFinite(duration) && duration >= 0 ? duration : null;
}
