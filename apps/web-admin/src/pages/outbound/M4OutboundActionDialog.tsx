import * as React from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  type DataGridToolbarAction,
} from "@wms/ui";
import { CheckCircle2, ClipboardCheck, Truck, XCircle } from "lucide-react";

import type { OutboundOrder, OutboundWave, PurchaseReturnOrder } from "./M4OutboundDetailDialog";
import { ActionExtraFields, TextField } from "./M4OutboundPageParts";
import type { M4OutboundMode } from "./m4-outbound-page-model";

export type ActionKind =
  | "create-order"
  | "validate"
  | "void"
  | "create-wave"
  | "release-wave"
  | "cancel-wave"
  | "review"
  | "print"
  | "ship"
  | "create-return"
  | "approve-return"
  | "reject-return"
  | "pick-return"
  | "review-return"
  | "ship-return";

export interface ActionState {
  kind: ActionKind;
  targetId?: string;
}

export interface ActionTargetContext {
  id: string;
  docNo: string;
  status: string;
  statusText: string;
  kindLabel: string;
}

export interface OutboundCreateForm {
  wmsOrderNo: string;
  erpOrderNo: string;
  documentType: string;
  customerName: string;
  productCode: string;
  batchNo: string;
  plannedQty: string;
  requiredShipDate: string;
}

export function outboundPrivateActions(
  mode: M4OutboundMode,
  selectedOrder: OutboundOrder | null,
  selectedWave: OutboundWave | null,
  selectedReturn: PurchaseReturnOrder | null,
  onAction: (kind: ActionKind, id: string) => void,
): DataGridToolbarAction[] {
  if (mode === "orders") {
    const status = selectedOrder?.status;
    return [
      toolbarAction("validate", "校验", "重新校验", <CheckCircle2 className="size-4" aria-hidden />, selectedOrder?.id, onAction, canValidateOrder(status)),
      toolbarAction("void", "作废", "作废申请", <ClipboardCheck className="size-4" aria-hidden />, selectedOrder?.id, onAction, canVoidOrder(status)),
    ];
  }

  if (mode === "waves") {
    const status = selectedWave?.status;
    return [
      toolbarAction("release-wave", "下发", "下发波次", <CheckCircle2 className="size-4" aria-hidden />, selectedWave?.id, onAction, canReleaseWave(status)),
      toolbarAction("cancel-wave", "取消", "取消波次", <ClipboardCheck className="size-4" aria-hidden />, selectedWave?.id, onAction, canCancelWave(status)),
    ];
  }

  if (mode === "review") {
    const status = selectedOrder?.status;
    return [
      toolbarAction("review", "复核", "出库复核", <ClipboardCheck className="size-4" aria-hidden />, selectedOrder?.id, onAction, canReviewOrder(status)),
      toolbarAction("ship", "交接", "发货交接", <Truck className="size-4" aria-hidden />, selectedOrder?.id, onAction, canShipOrder(status)),
    ];
  }

  const status = selectedReturn?.status;
  return [
    toolbarAction("approve-return", "审批", "采购退货审批", <CheckCircle2 className="size-4" aria-hidden />, selectedReturn?.id, onAction, canApproveReturn(status)),
    toolbarAction("reject-return", "驳回", "采购退货驳回", <XCircle className="size-4" aria-hidden />, selectedReturn?.id, onAction, canRejectReturn(status)),
    toolbarAction("pick-return", "拣货", "采购退货拣货", <Truck className="size-4" aria-hidden />, selectedReturn?.id, onAction, canPickReturn(status)),
    toolbarAction("review-return", "复核", "采购退货复核", <ClipboardCheck className="size-4" aria-hidden />, selectedReturn?.id, onAction, canReviewReturn(status)),
    toolbarAction("ship-return", "出库", "采购退货出库交接", <CheckCircle2 className="size-4" aria-hidden />, selectedReturn?.id, onAction, canShipReturn(status)),
  ];
}

function canValidateOrder(status: string | undefined) {
  return status === "pending_validation" || status === "validation_exception" || status === "confirmed";
}

function canVoidOrder(status: string | undefined) {
  return status === "pending_validation" || status === "validation_exception" || status === "confirmed";
}

function canReleaseWave(status: string | undefined) {
  return status === "draft";
}

function canCancelWave(status: string | undefined) {
  return status === "draft" || status === "released";
}

function canReviewOrder(status: string | undefined) {
  return status === "picked" || status === "picked_short";
}

function canShipOrder(status: string | undefined) {
  return status === "reviewed";
}

function canApproveReturn(status: string | undefined) {
  return status === "pending_approval";
}

function canRejectReturn(status: string | undefined) {
  return status === "pending_approval";
}

function canPickReturn(status: string | undefined) {
  return status === "approved";
}

function canReviewReturn(status: string | undefined) {
  return status === "picking";
}

function canShipReturn(status: string | undefined) {
  return status === "reviewed";
}

function toolbarAction(
  kind: ActionKind,
  label: string,
  description: string,
  icon: React.ReactNode,
  targetId: string | undefined,
  onAction: (kind: ActionKind, id: string) => void,
  enabledByStatus = true,
): DataGridToolbarAction {
  return {
    key: kind,
    label,
    description,
    icon,
    disabled: !targetId || !enabledByStatus,
    onClick: () => {
      if (targetId && enabledByStatus) onAction(kind, targetId);
    },
  };
}

export function M4OutboundActionDialog({ action, target, createForm, documentTypeOptions, reviewOrder, reviewLoading, reviewError, note, actionError, pending, setCreateForm, setNote, onClose, onSubmit }: {
  action: ActionState | null;
  target: ActionTargetContext | null;
  createForm: OutboundCreateForm;
  documentTypeOptions: Array<{ value: string; label: string }>;
  reviewOrder: OutboundOrder | null;
  reviewLoading: boolean;
  reviewError: string | null;
  note: string;
  actionError: string | null;
  pending: boolean;
  setCreateForm: React.Dispatch<React.SetStateAction<OutboundCreateForm>>;
  setNote: (value: string) => void;
  onClose: () => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
}) {
  if (!action) return null;
  const meta = actionMeta(action.kind);
  const isCreate = action.kind === "create-order" || action.kind === "create-wave" || action.kind === "create-return";
  const noteRequired = action.kind === "reject-return";
  const titleWithDocNo = !isCreate && target && (action.kind === "ship" || action.kind === "validate" || action.kind === "ship-return")
    ? `${meta.title} · ${target.docNo}`
    : meta.title;
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <form className="grid gap-3 md:grid-cols-2" onSubmit={onSubmit}>
          <DialogHeader className="md:col-span-2">
            <DialogTitle>{titleWithDocNo}</DialogTitle>
            <DialogDescription>
              {meta.description}
              {!isCreate && target ? ` 目标${target.kindLabel}：${target.docNo}（${target.statusText}）` : ""}
            </DialogDescription>
          </DialogHeader>
          {!isCreate && target && (
            <div className="md:col-span-2 rounded-md border bg-muted/30 px-3 py-2 text-sm" data-testid="action-target-context">
              <div className="font-medium">{target.kindLabel} {target.docNo}</div>
              <div className="text-xs text-muted-foreground">当前状态：{target.statusText}</div>
            </div>
          )}
          {action.kind === "review" ? (
            <ReviewDetails order={reviewOrder} loading={reviewLoading} error={reviewError} />
          ) : action.kind === "create-order" ? (
            <>
              <TextField label="WMS 单号（可选）" placeholder="留空自动生成" value={createForm.wmsOrderNo} onChange={(wmsOrderNo) => setCreateForm((value) => ({ ...value, wmsOrderNo }))} />
              <TextField label="ERP 单号" value={createForm.erpOrderNo} onChange={(erpOrderNo) => setCreateForm((value) => ({ ...value, erpOrderNo }))} />
              <label className="grid gap-1 text-sm">单据类型
                <select className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={createForm.documentType} onChange={(event) => setCreateForm((value) => ({ ...value, documentType: event.target.value }))}>
                  <option value="">请选择</option>
                  {documentTypeOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
                </select>
              </label>
              <TextField label="客户 / 门店" value={createForm.customerName} onChange={(customerName) => setCreateForm((value) => ({ ...value, customerName }))} />
              <TextField label="商品编码" value={createForm.productCode} onChange={(productCode) => setCreateForm((value) => ({ ...value, productCode }))} />
              <TextField label="批号" value={createForm.batchNo} onChange={(batchNo) => setCreateForm((value) => ({ ...value, batchNo }))} />
              <TextField label="计划数量" type="number" value={createForm.plannedQty} onChange={(plannedQty) => setCreateForm((value) => ({ ...value, plannedQty }))} />
              <TextField label="要求发货" type="date" value={createForm.requiredShipDate} onChange={(requiredShipDate) => setCreateForm((value) => ({ ...value, requiredShipDate }))} />
            </>
          ) : (
            <>
              <ActionExtraFields kind={action.kind} />
              <TextField
                className="md:col-span-2"
                label={noteRequired ? "驳回备注（必填）" : "备注"}
                value={note}
                onChange={setNote}
                placeholder={noteRequired ? "请填写驳回原因" : action.kind === "approve-return" ? "可选填写审批意见" : undefined}
              />
            </>
          )}
          {actionError && (
            <p className="md:col-span-2 text-sm text-destructive" role="alert">{actionError}</p>
          )}
          <DialogFooter className="md:col-span-2">
            <DialogClose asChild><Button type="button" variant="outline">取消</Button></DialogClose>
            <Button type="submit" disabled={pending} variant={action.kind === "reject-return" ? "destructive" : "default"}>{pending ? "提交中..." : meta.submitLabel}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function actionMeta(kind: ActionKind) {
  const map: Record<ActionKind, { title: string; description: string; submitLabel: string }> = {
    "create-order": { title: "新建出库单", description: "手工创建 PC Web 测试出库单。", submitLabel: "创建出库单" },
    validate: { title: "重新校验", description: "重新执行库存和批号校验。", submitLabel: "确认校验" },
    void: { title: "作废申请", description: "提交未进波次订单的作废申请。", submitLabel: "提交申请" },
    "create-wave": { title: "新建波次", description: "把已确认订单合并为一个波次。", submitLabel: "创建波次" },
    "release-wave": { title: "下发波次", description: "下发波次并进入库存锁定。", submitLabel: "确认下发" },
    "cancel-wave": { title: "取消波次", description: "仅未开始拣选的波次可取消。", submitLabel: "确认取消" },
    review: { title: "复核", description: "包装站复核完成后提交。", submitLabel: "提交复核" },
    print: { title: "打印", description: "提交随货同行单或快递面单打印任务。", submitLabel: "提交打印" },
    ship: { title: "发货交接", description: "记录交接对象并确认发货。", submitLabel: "确认发货" },
    "create-return": { title: "新建采购退货单", description: "创建退供应商的出库申请。", submitLabel: "创建采购退货单" },
    "approve-return": { title: "采购退货审批", description: "审批退供应商出库申请，备注可选。", submitLabel: "审批通过" },
    "reject-return": { title: "采购退货驳回", description: "驳回退供应商出库申请，备注必填。", submitLabel: "确认驳回" },
    "pick-return": { title: "采购退货拣货", description: "记录退供应商出库拣货结果。", submitLabel: "确认拣货" },
    "review-return": { title: "采购退货复核", description: "复核退供应商商品和数量。", submitLabel: "提交复核" },
    "ship-return": { title: "采购退货出库交接", description: "确认退供应商出库交接。", submitLabel: "确认出库" },
  };
  return map[kind];
}

function ReviewDetails({ order, loading, error }: { order: OutboundOrder | null; loading: boolean; error: string | null }) {
  return (
    <div className="md:col-span-2 rounded-md border bg-muted/20 p-3" data-testid="review-detail">
      <div className="mb-2 text-sm font-medium">真实复核明细</div>
      {loading && <p className="text-sm text-muted-foreground" role="status">正在读取订单复核明细...</p>}
      {error && <p className="text-sm text-destructive" role="alert">{error}</p>}
      {!loading && !error && !order && <p className="text-sm text-muted-foreground">暂无可复核明细</p>}
      {!loading && !error && order && (
        <div className="grid gap-2 text-sm">
          <div className="flex justify-between gap-3">
            <span className="text-muted-foreground">订单</span>
            <span className="font-mono">{order.wms_order_no}</span>
          </div>
          <div className="grid gap-2">
            {(order.lines ?? []).map((line) => (
              <div key={line.line_no} className="rounded border bg-background px-3 py-2">
                <div className="flex justify-between gap-3 font-medium">
                  <span>{line.product_code}</span>
                  <span>复核 {line.picked_qty} 件</span>
                </div>
                <div className="mt-1 text-xs text-muted-foreground">批号 {line.batch_no} · 拣选 {line.picked_qty} 件 · 短拣 {line.short_pick_qty} 件</div>
              </div>
            ))}
          </div>
          <p className="text-xs text-muted-foreground">PC 包装站复核将按真实拣选数量提交，复核人由当前登录用户带出。</p>
        </div>
      )}
    </div>
  );
}
