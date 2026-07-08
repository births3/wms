/**
 * M2InboundDialogs — 入库动作弹窗表单
 *
 * 层级：Layer 3 页面局部组件
 * 关联故事：US-M2-001, US-M2-002, US-M2-003, US-M2-004, US-M2-005
 * Wave：Wave 6
 * 业务约束：管理端写操作通过按钮打开 Dialog 后提交。
 *
 * @example
 *   <M2InboundDialogs activeDialog="receive" ... />
 */

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
  Input,
} from "@wms/ui";
import { Ban, CheckCircle2, ClipboardCheck, PackageCheck, Plus } from "lucide-react";

import { useMasterDataRowsQuery, type MasterDataRow } from "@/features/master-data/master-data-queries";
import type { InboundDocumentType } from "./m2-inbound-document-type";
import { ProductLookupDialog, ProductLookupField } from "./M2InboundProductLookup";

export type InboundDialog = "create" | "receive" | "reject" | "inspect" | "putaway";

export interface CreateFormState {
  receiptNo: string;
  documentType: InboundDocumentType | "";
  supplierId: string;
  warehouseId: string;
  expectedArrivalDate: string;
  productCode: string;
  batchNo: string;
  expectedQty: string;
  productionDate: string;
  expiryDate: string;
}

export interface ReceiveFormState {
  actualQty: string;
  shortageQty: string;
  rejectedQty: string;
  temperature: string;
  temperatureControl: string;
  vehicleNo: string;
  origin: string;
  departureTime: string;
  arrivalTime: string;
  storageTime: string;
  transportMode: string;
  carrier: string;
  contactName: string;
  contactPhone: string;
  contactIdNo: string;
  sealChecked: string;
  filingChecked: string;
  deliveryQty: string;
  batchQty: string;
  secondReceiverId: string;
  note: string;
}

export interface RejectFormState {
  reason: string;
}

export interface InspectFormState {
  batchNo: string;
  acceptedQty: string;
  rejectedQty: string;
  productionDate: string;
  expiryDate: string;
  qualityStatus: string;
  traceCodes: string;
  appearanceCheck: string;
  packageCheck: string;
  instructionCheck: string;
  labelCheck: string;
  note: string;
}

export interface SignFormState {
  firstSignerId: string;
  secondSignerId: string;
  dualRequired: boolean;
  strategyNote: string;
  note: string;
}

export interface InspectFormExamples {
  batchNo: string;
  acceptedQty: string;
  rejectedQty: string;
  productionDate: string;
  expiryDate: string;
  traceCodes: string;
  appearanceCheck: string;
  packageCheck: string;
  instructionCheck: string;
  labelCheck: string;
  firstSignerId: string;
  secondSignerId: string;
  strategyNote: string;
}

export interface PutawayFormState {
  lpn: string;
  productCode: string;
  batchNo: string;
  qty: string;
  recommendedLocation: string;
  locationId: string;
  locationCode: string;
  qualityStatus: string;
  validationResult: string;
  note: string;
}

interface M2InboundDialogsProps {
  activeDialog: InboundDialog | null;
  orderReceiptNo: string | null;
  hasOrder: boolean;
  pending: boolean;
  errorMessage?: string;
  productTemperatureAttribute: string;
  derivedTemperatureControl: string;
  createForm: CreateFormState;
  receiveForm: ReceiveFormState;
  rejectForm: RejectFormState;
  inspectForm: InspectFormState;
  inspectExamples: InspectFormExamples;
  signForm: SignFormState;
  putawayForm: PutawayFormState;
  setActiveDialog: (dialog: InboundDialog | null) => void;
  setCreateForm: React.Dispatch<React.SetStateAction<CreateFormState>>;
  setReceiveForm: React.Dispatch<React.SetStateAction<ReceiveFormState>>;
  setRejectForm: React.Dispatch<React.SetStateAction<RejectFormState>>;
  setInspectForm: React.Dispatch<React.SetStateAction<InspectFormState>>;
  setSignForm: React.Dispatch<React.SetStateAction<SignFormState>>;
  setPutawayForm: React.Dispatch<React.SetStateAction<PutawayFormState>>;
  submitCreate: (event: React.FormEvent<HTMLFormElement>) => void;
  submitReceive: (event: React.FormEvent<HTMLFormElement>) => void;
  submitReject: (event?: React.FormEvent<HTMLFormElement>) => void;
  submitInspect: (event: React.FormEvent<HTMLFormElement>) => void;
  submitPutaway: (event: React.FormEvent<HTMLFormElement>) => void;
}

export function M2InboundDialogs({
  activeDialog,
  orderReceiptNo,
  hasOrder,
  pending,
  errorMessage,
  productTemperatureAttribute,
  derivedTemperatureControl,
  createForm,
  receiveForm,
  rejectForm,
  inspectForm,
  inspectExamples,
  signForm,
  putawayForm,
  setActiveDialog,
  setCreateForm,
  setReceiveForm,
  setRejectForm,
  setInspectForm,
  setSignForm,
  setPutawayForm,
  submitCreate,
  submitReceive,
  submitReject,
  submitInspect,
  submitPutaway,
}: M2InboundDialogsProps) {
  const productsQuery = useMasterDataRowsQuery("m1-products", activeDialog === "create");
  const [productLookupOpen, setProductLookupOpen] = React.useState(false);

  React.useEffect(() => {
    if (activeDialog !== "create") setProductLookupOpen(false);
  }, [activeDialog]);

  if (!activeDialog) return null;
  const coldChainReceiving = isColdChainTemperatureControl(derivedTemperatureControl);
  const productRows = productsQuery.data ?? [];

  function selectCreateProduct(product: MasterDataRow) {
    setCreateForm((value) => ({ ...value, productCode: product.code }));
    setProductLookupOpen(false);
  }

  return (
    <Dialog open onOpenChange={(open) => !open && setActiveDialog(null)}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        {errorMessage && (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">
            {errorMessage}
          </div>
        )}

        {activeDialog === "create" && (
          <form className="grid gap-3 md:grid-cols-2" onSubmit={submitCreate}>
            <DialogHeader className="md:col-span-2">
              <DialogTitle>新建 ASN</DialogTitle>
              <DialogDescription>手工创建入库通知单。</DialogDescription>
            </DialogHeader>
            <TextField label="ASN 号" required placeholder="例如 ASN-M2-PC-0002" value={createForm.receiptNo} onChange={(receiptNo) => setCreateForm((value) => ({ ...value, receiptNo }))} />
            <div>
              <label className="mb-1 block text-xs text-muted-foreground">单据类型</label>
              <select
                aria-label="单据类型"
                required
                className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
                value={createForm.documentType}
                onChange={(event) =>
                  setCreateForm((value) => ({
                    ...value,
                    documentType: event.target.value as CreateFormState["documentType"],
                  }))
                }
              >
                <option value="" disabled>请选择单据类型</option>
                <option value="purchase_inbound">采购入库</option>
                <option value="sales_return">销售退货</option>
              </select>
            </div>
            <TextField label="供应商 ID" placeholder="例如 00000000-0000-0000-0000-000000005001" value={createForm.supplierId} onChange={(supplierId) => setCreateForm((value) => ({ ...value, supplierId }))} />
            <TextField label="仓库 ID" required placeholder="例如 00000000-0000-0000-0000-000000003001" value={createForm.warehouseId} onChange={(warehouseId) => setCreateForm((value) => ({ ...value, warehouseId }))} />
            <TextField label="预计到货" type="date" placeholder="例如 2026-06-27" value={createForm.expectedArrivalDate} onChange={(expectedArrivalDate) => setCreateForm((value) => ({ ...value, expectedArrivalDate }))} />
            <ProductLookupField
              batchNo={createForm.batchNo}
              errorMessage={productsQuery.error?.message}
              loading={productsQuery.isFetching}
              placeholder="例如 P-M2-002"
              products={productRows}
              required
              value={createForm.productCode}
              onChange={(productCode) => setCreateForm((value) => ({ ...value, productCode }))}
              onOpenLookup={() => setProductLookupOpen(true)}
              onSelect={selectCreateProduct}
            />
            {createForm.documentType === "sales_return" && (
              <TextField label="ASN 批号" placeholder="例如 BATCH-202606" value={createForm.batchNo} onChange={(batchNo) => setCreateForm((value) => ({ ...value, batchNo }))} />
            )}
            <TextField label="预报数量" type="number" required placeholder="例如 60" value={createForm.expectedQty} onChange={(expectedQty) => setCreateForm((value) => ({ ...value, expectedQty }))} />
            <TextField label="生产日期" type="date" placeholder="例如 2026-02-01" value={createForm.productionDate} onChange={(productionDate) => setCreateForm((value) => ({ ...value, productionDate }))} />
            <TextField label="有效期至" type="date" placeholder="例如 2028-02-01" value={createForm.expiryDate} onChange={(expiryDate) => setCreateForm((value) => ({ ...value, expiryDate }))} />
            <DialogFooter className="md:col-span-2">
              <CancelButton />
              <Button type="submit" disabled={pending}>
                <Plus className="size-4" aria-hidden />
                创建 ASN
              </Button>
            </DialogFooter>
          </form>
        )}

        {activeDialog === "receive" && (
          <form className="grid gap-3 md:grid-cols-3" onSubmit={submitReceive}>
            <DialogHeader className="md:col-span-3">
              <DialogTitle>收货</DialogTitle>
              <DialogDescription>{orderReceiptNo ?? "未选择入库单"}</DialogDescription>
            </DialogHeader>
            <TextField label="送货数量" type="number" value={receiveForm.deliveryQty} onChange={(deliveryQty) => setReceiveForm((value) => ({ ...value, deliveryQty }))} />
            <TextField label="实际到货数量" type="number" value={receiveForm.actualQty} onChange={(actualQty) => setReceiveForm((value) => ({ ...value, actualQty }))} />
            <TextField label="缺货数量" type="number" value={receiveForm.shortageQty} onChange={(shortageQty) => setReceiveForm((value) => ({ ...value, shortageQty }))} />
            <TextField label="拒收数量" type="number" value={receiveForm.rejectedQty} onChange={(rejectedQty) => setReceiveForm((value) => ({ ...value, rejectedQty }))} />
            <TextField label="批号 + 数量" value={receiveForm.batchQty} onChange={(batchQty) => setReceiveForm((value) => ({ ...value, batchQty }))} />
            <ReadOnlyField label="商品温度属性" value={productTemperatureAttribute} />
            <ReadOnlyField label="温控方式" value={derivedTemperatureControl} />
            {coldChainReceiving && (
              <section className="grid gap-3 rounded-md border bg-muted/20 p-3 md:col-span-3 md:grid-cols-4">
                <div className="text-xs font-medium text-muted-foreground md:col-span-4">冷链字段</div>
                <TextField label="到货温度" type="number" required value={receiveForm.temperature} onChange={(temperature) => setReceiveForm((value) => ({ ...value, temperature }))} />
                <TextField label="启运时间" type="datetime-local" required value={receiveForm.departureTime} onChange={(departureTime) => setReceiveForm((value) => ({ ...value, departureTime }))} />
                <TextField label="到货时间" type="datetime-local" required value={receiveForm.arrivalTime} onChange={(arrivalTime) => setReceiveForm((value) => ({ ...value, arrivalTime }))} />
                <TextField label="冷链运输方式" required value={receiveForm.transportMode} onChange={(transportMode) => setReceiveForm((value) => ({ ...value, transportMode }))} />
              </section>
            )}
            <TextField label="车牌号" value={receiveForm.vehicleNo} onChange={(vehicleNo) => setReceiveForm((value) => ({ ...value, vehicleNo }))} />
            <TextField label="发运地点" value={receiveForm.origin} onChange={(origin) => setReceiveForm((value) => ({ ...value, origin }))} />
            <TextField label="收货入库时间" type="datetime-local" value={receiveForm.storageTime} onChange={(storageTime) => setReceiveForm((value) => ({ ...value, storageTime }))} />
            <TextField label="承运商" value={receiveForm.carrier} onChange={(carrier) => setReceiveForm((value) => ({ ...value, carrier }))} />
            <TextField label="联系人" value={receiveForm.contactName} onChange={(contactName) => setReceiveForm((value) => ({ ...value, contactName }))} />
            <TextField label="电话" value={receiveForm.contactPhone} onChange={(contactPhone) => setReceiveForm((value) => ({ ...value, contactPhone }))} />
            <TextField label="身份证" value={receiveForm.contactIdNo} onChange={(contactIdNo) => setReceiveForm((value) => ({ ...value, contactIdNo }))} />
            <TextField label="印章样式核对" value={receiveForm.sealChecked} onChange={(sealChecked) => setReceiveForm((value) => ({ ...value, sealChecked }))} />
            <TextField label="备案件样式核对" value={receiveForm.filingChecked} onChange={(filingChecked) => setReceiveForm((value) => ({ ...value, filingChecked }))} />
            <TextField label="第二收货员验证" value={receiveForm.secondReceiverId} onChange={(secondReceiverId) => setReceiveForm((value) => ({ ...value, secondReceiverId }))} />
            <TextField className="md:col-span-3" label="异常备注" value={receiveForm.note} onChange={(note) => setReceiveForm((value) => ({ ...value, note }))} />
            <section className="grid gap-3 rounded-md border border-destructive/30 bg-destructive/5 p-3 md:col-span-3">
              <div>
                <div className="text-sm font-medium text-destructive">整单拒收</div>
                <div className="mt-1 text-xs text-muted-foreground">整单拒收会关闭当前入库单。</div>
              </div>
              <TextField
                label="拒收原因"
                value={rejectForm.reason}
                onChange={(reason) => setRejectForm((value) => ({ ...value, reason }))}
              />
              <div className="flex justify-end">
                <Button type="button" variant="destructive" disabled={!hasOrder || pending || !rejectForm.reason.trim()} onClick={() => submitReject()}>
                  <Ban className="size-4" aria-hidden />
                  整单拒收
                </Button>
              </div>
            </section>
            <DialogFooter className="md:col-span-3">
              <CancelButton />
              <SubmitButton icon={<CheckCircle2 className="size-4" />} label="确认收货" disabled={!hasOrder || pending} />
            </DialogFooter>
          </form>
        )}

        {activeDialog === "reject" && (
          <form className="grid gap-3" onSubmit={submitReject}>
            <DialogHeader>
              <DialogTitle>整单拒收</DialogTitle>
              <DialogDescription>{orderReceiptNo ?? "未选择入库单"}</DialogDescription>
            </DialogHeader>
            <TextField
              label="拒收原因"
              required
              value={rejectForm.reason}
              onChange={(reason) => setRejectForm((value) => ({ ...value, reason }))}
            />
            <DialogFooter>
              <CancelButton />
              <SubmitButton icon={<Ban className="size-4" />} label="确认拒收" disabled={!hasOrder || pending} />
            </DialogFooter>
          </form>
        )}

        {activeDialog === "inspect" && (
          <form className="grid gap-3 md:grid-cols-2" onSubmit={submitInspect}>
            <DialogHeader className="md:col-span-2">
              <DialogTitle>验收</DialogTitle>
              <DialogDescription>{orderReceiptNo ?? "未选择入库单"}</DialogDescription>
            </DialogHeader>
            <TextField label="验收批号" required placeholder={inspectExamples.batchNo} value={inspectForm.batchNo} onChange={(batchNo) => setInspectForm((value) => ({ ...value, batchNo }))} />
            <TextField label="通过数量" type="number" required placeholder={inspectExamples.acceptedQty} value={inspectForm.acceptedQty} onChange={(acceptedQty) => setInspectForm((value) => ({ ...value, acceptedQty }))} />
            <TextField label="拒收数量" type="number" required placeholder={inspectExamples.rejectedQty} value={inspectForm.rejectedQty} onChange={(rejectedQty) => setInspectForm((value) => ({ ...value, rejectedQty }))} />
            <TextField label="生产日期" type="date" required placeholder={inspectExamples.productionDate} value={inspectForm.productionDate} onChange={(productionDate) => setInspectForm((value) => ({ ...value, productionDate }))} />
            <TextField label="有效期至" type="date" required placeholder={inspectExamples.expiryDate} value={inspectForm.expiryDate} onChange={(expiryDate) => setInspectForm((value) => ({ ...value, expiryDate }))} />
            <TextField label="追溯码" required placeholder={inspectExamples.traceCodes} value={inspectForm.traceCodes} onChange={(traceCodes) => setInspectForm((value) => ({ ...value, traceCodes }))} />
            <SelectField label="质量状态" required placeholder="请选择质量状态" value={inspectForm.qualityStatus} onChange={(qualityStatus) => setInspectForm((value) => ({ ...value, qualityStatus }))} options={[["qualified", "合格"], ["unqualified", "不合格"], ["quarantine", "待复验 / 隔离"]]} />
            <TextField label="外观核对" required placeholder={inspectExamples.appearanceCheck} value={inspectForm.appearanceCheck} onChange={(appearanceCheck) => setInspectForm((value) => ({ ...value, appearanceCheck }))} />
            <TextField label="包装核对" required placeholder={inspectExamples.packageCheck} value={inspectForm.packageCheck} onChange={(packageCheck) => setInspectForm((value) => ({ ...value, packageCheck }))} />
            <TextField label="说明书核对" required placeholder={inspectExamples.instructionCheck} value={inspectForm.instructionCheck} onChange={(instructionCheck) => setInspectForm((value) => ({ ...value, instructionCheck }))} />
            <TextField label="标签核对" required placeholder={inspectExamples.labelCheck} value={inspectForm.labelCheck} onChange={(labelCheck) => setInspectForm((value) => ({ ...value, labelCheck }))} />
            <section className="grid gap-3 rounded-md border bg-muted/20 p-3 md:col-span-2 md:grid-cols-2">
              <div className="text-xs font-medium text-muted-foreground md:col-span-2">验收复核</div>
              <TextField label="第一签字人" required placeholder={inspectExamples.firstSignerId} value={signForm.firstSignerId} onChange={(firstSignerId) => setSignForm((value) => ({ ...value, firstSignerId }))} />
              <TextField label="第二签字人" required={signForm.dualRequired} placeholder={inspectExamples.secondSignerId} value={signForm.secondSignerId} onChange={(secondSignerId) => setSignForm((value) => ({ ...value, secondSignerId }))} />
              <TextField label="策略命中说明" placeholder={inspectExamples.strategyNote} value={signForm.strategyNote} onChange={(strategyNote) => setSignForm((value) => ({ ...value, strategyNote }))} />
              <TextField label="签字备注" value={signForm.note} onChange={(note) => setSignForm((value) => ({ ...value, note }))} />
              <label className="flex items-center gap-2 text-sm text-muted-foreground md:col-span-2">
                <input type="checkbox" checked={signForm.dualRequired} onChange={(event) => setSignForm((value) => ({ ...value, dualRequired: event.target.checked }))} />
                需要双人签字
              </label>
            </section>
            <TextField className="md:col-span-2" label="验收备注" value={inspectForm.note} onChange={(note) => setInspectForm((value) => ({ ...value, note }))} />
            <DialogFooter className="md:col-span-2">
              <CancelButton />
              <SubmitButton icon={<ClipboardCheck className="size-4" />} label="提交验收" disabled={!hasOrder || pending} />
            </DialogFooter>
          </form>
        )}

        {activeDialog === "putaway" && (
          <form className="grid gap-3" onSubmit={submitPutaway}>
            <DialogHeader>
              <DialogTitle>上架</DialogTitle>
              <DialogDescription>{orderReceiptNo ?? "未选择入库单"}</DialogDescription>
            </DialogHeader>
            <TextField label="容器 LPN" value={putawayForm.lpn} onChange={(lpn) => setPutawayForm((value) => ({ ...value, lpn }))} />
            <TextField label="上架商品编码" value={putawayForm.productCode} onChange={(productCode) => setPutawayForm((value) => ({ ...value, productCode }))} />
            <TextField label="上架批号" value={putawayForm.batchNo} onChange={(batchNo) => setPutawayForm((value) => ({ ...value, batchNo }))} />
            <TextField label="数量" type="number" value={putawayForm.qty} onChange={(qty) => setPutawayForm((value) => ({ ...value, qty }))} />
            <TextField label="推荐库位 Top N" value={putawayForm.recommendedLocation} onChange={(recommendedLocation) => setPutawayForm((value) => ({ ...value, recommendedLocation }))} />
            <TextField label="实际库位" value={putawayForm.locationCode} onChange={(locationCode) => setPutawayForm((value) => ({ ...value, locationCode }))} />
            <TextField label="校验结果" value={putawayForm.validationResult} onChange={(validationResult) => setPutawayForm((value) => ({ ...value, validationResult }))} />
            <TextField label="上架备注" value={putawayForm.note} onChange={(note) => setPutawayForm((value) => ({ ...value, note }))} />
            <DialogFooter>
              <CancelButton />
              <SubmitButton icon={<PackageCheck className="size-4" />} label="确认上架" disabled={!hasOrder || pending} />
            </DialogFooter>
          </form>
        )}
        {activeDialog === "create" && (
          <ProductLookupDialog
            batchNo={createForm.batchNo}
            open={productLookupOpen}
            products={productRows}
            query={createForm.productCode}
            onOpenChange={setProductLookupOpen}
            onSelect={selectCreateProduct}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

function CancelButton() {
  return (
    <DialogClose asChild>
      <Button type="button" variant="outline">取消</Button>
    </DialogClose>
  );
}

function SubmitButton({
  icon,
  label,
  disabled,
}: {
  icon: React.ReactNode;
  label: string;
  disabled: boolean;
}) {
  return (
    <Button type="submit" variant="outline" className="justify-start" disabled={disabled}>
      {icon}
      {label}
    </Button>
  );
}

function TextField({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
  className,
  required = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: React.HTMLInputTypeAttribute;
  placeholder?: string;
  className?: string;
  required?: boolean;
}) {
  return (
    <label className={`grid gap-1 text-xs text-muted-foreground ${className ?? ""}`}>
      {label}
      <Input type={type} required={required} placeholder={placeholder} value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

function ReadOnlyField({ label, value }: { label: string; value: string }) {
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      {label}
      <Input aria-label={label} value={value} readOnly disabled />
    </label>
  );
}

function SelectField({
  label,
  value,
  onChange,
  options,
  placeholder,
  required = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: Array<[string, string]>;
  placeholder?: string;
  required?: boolean;
}) {
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      {label}
      <select required={required} className="h-10 rounded-md border border-input bg-background px-3 text-sm text-foreground" value={value} onChange={(event) => onChange(event.target.value)}>
        {placeholder && <option value="" disabled>{placeholder}</option>}
        {options.map(([optionValue, optionLabel]) => <option key={optionValue} value={optionValue}>{optionLabel}</option>)}
      </select>
    </label>
  );
}

export function isColdChainTemperatureControl(value: string) {
  return /冷链|冷藏|冷冻/.test(value);
}
