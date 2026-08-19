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

import {
  COLUMN_BATCH_NO,
  COLUMN_PRODUCT_CODE,
  COLUMN_WAREHOUSE,
  FIELD_PLATE_NO,
  LOADING_SUBMITTING,
} from "@/lib/ui-strings";
import type { OutboundOrder, OutboundWave, PurchaseReturnOrder } from "./m4-outbound-page-model";
import { ActionExtraFields, TextField } from "./M4OutboundPageParts";
import {
  outboundCarrierTypeOptions,
  type OutboundShipForm,
} from "./m4-outbound-page-helpers";
import type { M4OutboundMode } from "./m4-outbound-page-model";

export type ActionKind =
  | "create-order"
  | "validate"
  | "void"
  | "create-wave"
  | "release-wave"
  | "cancel-wave"
  | "review"
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
  warehouseId: string;
  customerId: string;
  deliveryAddressId: string;
  productCode: string;
  batchNo: string;
  plannedQty: string;
  requiredShipDate: string;
}

export interface PurchaseReturnCreateForm {
  returnNo: string;
  sourcePurchaseOrderNo: string;
  supplierName: string;
  reason: string;
  warehouseId: string;
  productCode: string;
  qty: string;
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
  // 与后端发货前置一致：reviewed | reviewed_short。
  return status === "reviewed" || status === "reviewed_short";
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

export function M4OutboundActionDialog({ action, target, createForm, purchaseReturnForm, shipForm, documentTypeOptions, warehouseOptions, customerOptions, addressOptions, reviewOrder, reviewLoading, reviewError, reviewPolicy, reviewPolicyLoading, secondReviewerId, note, actionError, pending, setCreateForm, setPurchaseReturnForm, setShipForm, setSecondReviewerId, setNote, onClose, onSubmit }: {
  action: ActionState | null;
  target: ActionTargetContext | null;
  createForm: OutboundCreateForm;
  purchaseReturnForm: PurchaseReturnCreateForm;
  shipForm: OutboundShipForm;
  documentTypeOptions: Array<{ value: string; label: string }>;
  warehouseOptions: Array<{ value: string; label: string }>;
  customerOptions: Array<{ value: string; label: string }>;
  addressOptions: Array<{ value: string; label: string }>;
  reviewOrder: OutboundOrder | null;
  reviewLoading: boolean;
  reviewError: string | null;
  reviewPolicy: "single" | "dual_scan" | "dual_scan_with_approval" | null;
  reviewPolicyLoading: boolean;
  secondReviewerId: string;
  note: string;
  actionError: string | null;
  pending: boolean;
  setCreateForm: React.Dispatch<React.SetStateAction<OutboundCreateForm>>;
  setPurchaseReturnForm: React.Dispatch<React.SetStateAction<PurchaseReturnCreateForm>>;
  setShipForm: React.Dispatch<React.SetStateAction<OutboundShipForm>>;
  setSecondReviewerId: (value: string) => void;
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
          {action.kind === "ship" && (
            <div className="md:col-span-2 rounded-md border border-primary/30 bg-primary/5 px-3 py-2 text-sm" role="note">
              客户药检副本暂缺、处理中或生成失败时不阻塞发货；副本可用后由客户平台异步补齐。
            </div>
          )}
          {action.kind === "review" ? (
            <>
              <ReviewDetails order={reviewOrder} loading={reviewLoading} error={reviewError} />
              <div className="md:col-span-2 rounded-md border bg-muted/20 px-3 py-2 text-sm" role="status">
                {reviewPolicyLoading
                  ? "正在读取 M-VR 出库/复核策略..."
                  : reviewPolicy === "dual_scan_with_approval"
                    ? "M-VR：双人扫码 + 主管审批。请先完成业务单号对应的 H4 审批。"
                    : reviewPolicy === "dual_scan"
                      ? "M-VR：双人扫码。第二复核员必须是本货主有效保管员。"
                      : reviewPolicy === "single"
                        ? "M-VR：单人复核。仍强制复核员与拣选员分离。"
                        : "未能在页面解析策略，提交时由服务端按整单最严格策略校验。"}
              </div>
              <TextField
                className="md:col-span-2"
                label="第二复核员用户 ID"
                required={reviewPolicy === "dual_scan" || reviewPolicy === "dual_scan_with_approval"}
                value={secondReviewerId}
                onChange={setSecondReviewerId}
                placeholder="策略要求双人时必填"
              />
            </>
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
              <SelectField label={COLUMN_WAREHOUSE} value={createForm.warehouseId} options={warehouseOptions} onChange={(warehouseId) => setCreateForm((value) => ({ ...value, warehouseId }))} />
              <SelectField label="客户 / 门店" value={createForm.customerId} options={customerOptions} onChange={(customerId) => setCreateForm((value) => ({ ...value, customerId, deliveryAddressId: "" }))} />
              <SelectField label="送货地址" value={createForm.deliveryAddressId} options={addressOptions} onChange={(deliveryAddressId) => setCreateForm((value) => ({ ...value, deliveryAddressId }))} />
              <TextField label={COLUMN_PRODUCT_CODE} value={createForm.productCode} onChange={(productCode) => setCreateForm((value) => ({ ...value, productCode }))} />
              <TextField label={COLUMN_BATCH_NO} value={createForm.batchNo} onChange={(batchNo) => setCreateForm((value) => ({ ...value, batchNo }))} />
              <TextField label="计划数量" type="number" value={createForm.plannedQty} onChange={(plannedQty) => setCreateForm((value) => ({ ...value, plannedQty }))} />
              <TextField label="要求发货" type="date" value={createForm.requiredShipDate} onChange={(requiredShipDate) => setCreateForm((value) => ({ ...value, requiredShipDate }))} />
            </>
          ) : action.kind === "create-return" ? (
            <>
              <TextField required label="采购退货单号" value={purchaseReturnForm.returnNo} onChange={(returnNo) => setPurchaseReturnForm((value) => ({ ...value, returnNo }))} />
              <TextField required label="原采购入库单" value={purchaseReturnForm.sourcePurchaseOrderNo} onChange={(sourcePurchaseOrderNo) => setPurchaseReturnForm((value) => ({ ...value, sourcePurchaseOrderNo }))} />
              <TextField required label="供应商" value={purchaseReturnForm.supplierName} onChange={(supplierName) => setPurchaseReturnForm((value) => ({ ...value, supplierName }))} />
              <SelectField required label={COLUMN_WAREHOUSE} value={purchaseReturnForm.warehouseId} options={warehouseOptions} onChange={(warehouseId) => setPurchaseReturnForm((value) => ({ ...value, warehouseId }))} />
              <TextField required className="md:col-span-2" label="退货原因" value={purchaseReturnForm.reason} onChange={(reason) => setPurchaseReturnForm((value) => ({ ...value, reason }))} />
              <TextField required label={COLUMN_PRODUCT_CODE} value={purchaseReturnForm.productCode} onChange={(productCode) => setPurchaseReturnForm((value) => ({ ...value, productCode }))} />
              <TextField required label="数量" type="number" value={purchaseReturnForm.qty} onChange={(qty) => setPurchaseReturnForm((value) => ({ ...value, qty }))} />
            </>
          ) : action.kind === "ship" ? (
            <>
              <SelectField
                required
                label="配送方类型"
                value={shipForm.deliveryProviderType}
                options={outboundCarrierTypeOptions}
                onChange={(deliveryProviderType) => setShipForm((value) => ({ ...value, deliveryProviderType }))}
              />
              <TextField required label={FIELD_PLATE_NO} value={shipForm.plateNo} onChange={(plateNo) => setShipForm((value) => ({ ...value, plateNo }))} />
              {shipForm.deliveryProviderType === "own_fleet" ? (
                <>
                  <TextField required label="车辆编号" value={shipForm.vehicleNo} onChange={(vehicleNo) => setShipForm((value) => ({ ...value, vehicleNo }))} />
                  <TextField required label="司机用户 ID" value={shipForm.driverUserId} onChange={(driverUserId) => setShipForm((value) => ({ ...value, driverUserId }))} />
                </>
              ) : shipForm.deliveryProviderType === "third_party_express" ? (
                <>
                  <TextField required label="快递员姓名" value={shipForm.courierName} onChange={(courierName) => setShipForm((value) => ({ ...value, courierName }))} />
                  <TextField required label="快递员电话" value={shipForm.courierPhone} onChange={(courierPhone) => setShipForm((value) => ({ ...value, courierPhone }))} />
                </>
              ) : null}
              <TextField
                required={shipForm.deliveryProviderType === "third_party_express"}
                label="签字附件 ID"
                value={shipForm.signatureAttachmentId}
                onChange={(signatureAttachmentId) => setShipForm((value) => ({ ...value, signatureAttachmentId }))}
              />
              <TextField required label="包裹数量" type="number" value={shipForm.packageCount} onChange={(packageCount) => setShipForm((value) => ({ ...value, packageCount }))} />
              <TextField label="装车温度（冷链必填）" type="number" value={shipForm.loadingTemperatureCelsius} onChange={(loadingTemperatureCelsius) => setShipForm((value) => ({ ...value, loadingTemperatureCelsius }))} />
              <TextField label="保温箱编号（冷链必填）" value={shipForm.insulatedContainerNo} onChange={(insulatedContainerNo) => setShipForm((value) => ({ ...value, insulatedContainerNo }))} />
              <TextField label="冰袋数量（冷链必填）" type="number" value={shipForm.icePackCount} onChange={(icePackCount) => setShipForm((value) => ({ ...value, icePackCount }))} />
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
            <Button type="submit" disabled={pending} variant={action.kind === "reject-return" ? "destructive" : "default"}>{pending ? LOADING_SUBMITTING : meta.submitLabel}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function SelectField({ label, value, options, onChange, required = false }: {
  label: string;
  value: string;
  options: Array<{ value: string; label: string }>;
  onChange: (value: string) => void;
  required?: boolean;
}) {
  return (
    <label className="grid gap-1 text-sm">{label}
      <select required={required} className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={value} onChange={(event) => onChange(event.target.value)}>
        <option value="">请选择</option>
        {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
      </select>
    </label>
  );
}

function actionMeta(kind: ActionKind) {
  const map: Record<ActionKind, { title: string; description: string; submitLabel: string }> = {
    "create-order": { title: "新建出库单", description: "创建出库订单并提交业务校验。", submitLabel: "创建出库单" },
    validate: { title: "重新校验", description: "重新执行库存和批号校验。", submitLabel: "确认校验" },
    void: { title: "作废申请", description: "提交未进波次订单的作废申请。", submitLabel: "提交申请" },
    "create-wave": { title: "新建波次", description: "把已确认订单合并为一个波次。", submitLabel: "创建波次" },
    "release-wave": { title: "下发波次", description: "下发波次并进入库存锁定。", submitLabel: "确认下发" },
    "cancel-wave": { title: "取消波次", description: "仅未开始拣选的波次可取消。", submitLabel: "确认取消" },
    review: { title: "复核", description: "包装站复核完成后提交。", submitLabel: "提交复核" },
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
