import {
  Button,
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
} from "@wms/ui";

import type { ExpressTrackingResponse, ExpressWaybill } from "@/features/express/express-queries";
import { formatDateTime } from "@/lib/format";
import { providerOptions } from "./h5-express-model";

export interface CarrierForm {
  carrierCode: string;
  carrierName: string;
  apiUrl: string;
  apiKeyAlias: string;
  apiSecretAlias: string;
  accountNo: string;
  enabled: boolean;
  priority: string;
  conditionsText: string;
}

export interface RuleForm {
  ruleCode: string;
  ruleName: string;
  deliveryProviderType: "own_fleet" | "third_party_express";
  carrierCode: string;
  priority: string;
  enabled: boolean;
  fallbackStrategy: string;
  conditionsText: string;
}

export interface WaybillForm {
  packageNo: string;
  carrierCode: string;
  senderName: string;
  senderMobile: string;
  senderAddress: string;
  receiverName: string;
  receiverMobile: string;
  receiverAddress: string;
  weightGrams: string;
  volumeCm3: string;
  packageCount: string;
}

export function TrackingDialog({ open, waybill, tracking, error, loading, onOpenChange, onRefresh }: {
  open: boolean;
  waybill: ExpressWaybill | null;
  tracking: ExpressTrackingResponse | null;
  error?: string | null;
  loading: boolean;
  onOpenChange: (open: boolean) => void;
  onRefresh: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader><DialogTitle>轨迹详情</DialogTitle></DialogHeader>
        <DialogErrorNotice error={error} />
        {waybill ? (
          <div className="space-y-4">
            <WaybillInfo waybill={waybill} />
            <div className="space-y-2">
              {(tracking?.events ?? []).map((event) => (
                <div key={event.id} className="rounded-md border bg-background px-3 py-2 text-sm">
                  <div className="flex items-center justify-between gap-3">
                    <span className="font-medium">{event.description}</span>
                    <span className="text-xs text-muted-foreground">{formatDateTime(event.event_time)}</span>
                  </div>
                  <div className="mt-1 text-xs text-muted-foreground">{event.location || "-"} · {event.source}</div>
                </div>
              ))}
              {tracking && tracking.events.length === 0 && <div className="text-sm text-muted-foreground">暂无轨迹事件</div>}
              {!tracking && <div className="text-sm text-muted-foreground">点击刷新获取轨迹。</div>}
            </div>
          </div>
        ) : (
          <div className="rounded-md border border-dashed p-5 text-sm text-muted-foreground">暂无运单，请先下单。</div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>关闭</Button>
          <Button onClick={onRefresh} disabled={!waybill || loading}>{loading ? "刷新中" : "刷新轨迹"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function CancelWaybillDialog({ open, waybill, error, saving, onOpenChange, onConfirm }: {
  open: boolean;
  waybill: ExpressWaybill | null;
  error?: string | null;
  saving: boolean;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader><DialogTitle>取消运单</DialogTitle></DialogHeader>
        <DialogErrorNotice error={error} />
        <div className="text-sm text-muted-foreground">
          确认取消运单 {waybill?.waybill_no ?? "-"}？取消后将写入管理端取消原因。
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>关闭</Button>
          <Button variant="destructive" disabled={!waybill || saving} onClick={onConfirm}>{saving ? "取消中" : "确认取消"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

const waybillPrintAreaId = "h5-waybill-print-area";

export function WaybillPrintDialog({ open, waybill, onOpenChange, onPrinted }: {
  open: boolean;
  waybill: ExpressWaybill | null;
  onOpenChange: (open: boolean) => void;
  onPrinted: () => void;
}) {
  function printWaybill() {
    window.print();
    onPrinted();
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        {/* 打印隔离：@media print 只显示面单容器，避免 window.print 把整页（菜单/表格）一并打印。 */}
        <style>{`@media print {
          body * { visibility: hidden !important; }
          #${waybillPrintAreaId}, #${waybillPrintAreaId} * { visibility: visible !important; }
          #${waybillPrintAreaId} { position: absolute; left: 0; top: 0; width: 100%; }
        }`}</style>
        <DialogHeader><DialogTitle>打印面单</DialogTitle></DialogHeader>
        {waybill ? (
          <div id={waybillPrintAreaId}>
            <WaybillInfo waybill={waybill} />
          </div>
        ) : (
          <div className="text-sm text-muted-foreground">暂无可打印运单。</div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>关闭</Button>
          <Button disabled={!waybill} onClick={printWaybill}>打印</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function CarrierDialog({ open, form, error, saving, onFormChange, onOpenChange, onSave }: {
  open: boolean;
  form: CarrierForm;
  error?: string | null;
  saving: boolean;
  onFormChange: (form: CarrierForm) => void;
  onOpenChange: (open: boolean) => void;
  onSave: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader><DialogTitle>快递商配置</DialogTitle></DialogHeader>
        <DialogErrorNotice error={error} />
        <div className="grid gap-3 md:grid-cols-2">
          <FormInput label="快递商编码" value={form.carrierCode} onChange={(carrierCode) => onFormChange({ ...form, carrierCode })} />
          <FormInput label="快递商名称" value={form.carrierName} onChange={(carrierName) => onFormChange({ ...form, carrierName })} />
          <FormInput label="接口地址" value={form.apiUrl} onChange={(apiUrl) => onFormChange({ ...form, apiUrl })} className="md:col-span-2" />
          <FormInput label="Key 别名" value={form.apiKeyAlias} onChange={(apiKeyAlias) => onFormChange({ ...form, apiKeyAlias })} />
          <FormInput label="Secret 别名" value={form.apiSecretAlias} onChange={(apiSecretAlias) => onFormChange({ ...form, apiSecretAlias })} />
          <FormInput label="账号" value={form.accountNo} onChange={(accountNo) => onFormChange({ ...form, accountNo })} />
          <FormInput label="优先级" type="number" value={form.priority} onChange={(priority) => onFormChange({ ...form, priority })} />
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={form.enabled} onChange={(event) => onFormChange({ ...form, enabled: event.currentTarget.checked })} />
            启用
          </label>
          <TextAreaInput label="条件参数 JSON" value={form.conditionsText} onChange={(conditionsText) => onFormChange({ ...form, conditionsText })} className="md:col-span-2" />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={onSave} disabled={saving}>{saving ? "保存中" : "保存"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function RuleDialog({ open, form, error, saving, onFormChange, onOpenChange, onSave }: {
  open: boolean;
  form: RuleForm;
  error?: string | null;
  saving: boolean;
  onFormChange: (form: RuleForm) => void;
  onOpenChange: (open: boolean) => void;
  onSave: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader><DialogTitle>快递选择规则</DialogTitle></DialogHeader>
        <DialogErrorNotice error={error} />
        <div className="grid gap-3 md:grid-cols-2">
          <FormInput label="规则编码" value={form.ruleCode} onChange={(ruleCode) => onFormChange({ ...form, ruleCode })} />
          <FormInput label="规则名称" value={form.ruleName} onChange={(ruleName) => onFormChange({ ...form, ruleName })} />
          <label className="text-sm">
            <span className="mb-1 block font-medium">配送方式</span>
            <select
              className="h-9 w-full rounded-md border bg-background px-3 text-sm"
              value={form.deliveryProviderType}
              onChange={(event) => onFormChange({ ...form, deliveryProviderType: event.currentTarget.value as RuleForm["deliveryProviderType"] })}
            >
              {providerOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
            </select>
          </label>
          <FormInput label="快递商编码" value={form.carrierCode} onChange={(carrierCode) => onFormChange({ ...form, carrierCode })} />
          <FormInput label="优先级" type="number" value={form.priority} onChange={(priority) => onFormChange({ ...form, priority })} />
          <FormInput label="兜底策略" value={form.fallbackStrategy} onChange={(fallbackStrategy) => onFormChange({ ...form, fallbackStrategy })} />
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={form.enabled} onChange={(event) => onFormChange({ ...form, enabled: event.currentTarget.checked })} />
            启用
          </label>
          <TextAreaInput label="匹配条件 JSON" value={form.conditionsText} onChange={(conditionsText) => onFormChange({ ...form, conditionsText })} className="md:col-span-2" />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={onSave} disabled={saving}>{saving ? "保存中" : "保存"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export function WaybillDialog({ open, form, error, saving, onFormChange, onOpenChange, onSave }: {
  open: boolean;
  form: WaybillForm;
  error?: string | null;
  saving: boolean;
  onFormChange: (form: WaybillForm) => void;
  onOpenChange: (open: boolean) => void;
  onSave: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader><DialogTitle>快递下单</DialogTitle></DialogHeader>
        <DialogErrorNotice error={error} />
        <div className="grid gap-3 md:grid-cols-3">
          <FormInput label="包裹号" value={form.packageNo} onChange={(packageNo) => onFormChange({ ...form, packageNo })} />
          <FormInput label="快递商编码" value={form.carrierCode} onChange={(carrierCode) => onFormChange({ ...form, carrierCode })} />
          <FormInput label="件数" type="number" value={form.packageCount} onChange={(packageCount) => onFormChange({ ...form, packageCount })} />
          <FormInput label="重量 g" type="number" value={form.weightGrams} onChange={(weightGrams) => onFormChange({ ...form, weightGrams })} />
          <FormInput label="体积 cm3" type="number" value={form.volumeCm3} onChange={(volumeCm3) => onFormChange({ ...form, volumeCm3 })} />
          <FormInput label="寄件人" value={form.senderName} onChange={(senderName) => onFormChange({ ...form, senderName })} />
          <FormInput label="寄件电话" value={form.senderMobile} onChange={(senderMobile) => onFormChange({ ...form, senderMobile })} />
          <FormInput label="收件人" value={form.receiverName} onChange={(receiverName) => onFormChange({ ...form, receiverName })} />
          <FormInput label="收件电话" value={form.receiverMobile} onChange={(receiverMobile) => onFormChange({ ...form, receiverMobile })} />
          <FormInput label="寄件地址" value={form.senderAddress} onChange={(senderAddress) => onFormChange({ ...form, senderAddress })} className="md:col-span-3" />
          <FormInput label="收件地址" value={form.receiverAddress} onChange={(receiverAddress) => onFormChange({ ...form, receiverAddress })} className="md:col-span-3" />
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>取消</Button>
          <Button onClick={onSave} disabled={saving}>{saving ? "下单中" : "下单"}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** 弹窗提交失败提示：渲染在弹窗内部，避免页面层 notice 被模态遮挡。 */
function DialogErrorNotice({ error }: { error?: string | null }) {
  if (!error) return null;
  return (
    <div className="rounded-md border border-red-200 bg-red-50 px-4 py-2 text-sm text-red-700" role="alert">
      {error}
    </div>
  );
}

function FormInput({ label, value, onChange, type = "text", className = "" }: {
  label: string;
  value: string;
  type?: string;
  className?: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className={`text-sm ${className}`}>
      <span className="mb-1 block font-medium">{label}</span>
      <Input type={type} value={value} onChange={(event) => onChange(event.currentTarget.value)} />
    </label>
  );
}

function TextAreaInput({ label, value, onChange, className = "" }: {
  label: string;
  value: string;
  className?: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className={`text-sm ${className}`}>
      <span className="mb-1 block font-medium">{label}</span>
      <textarea className="min-h-24 w-full rounded-md border bg-background px-3 py-2 font-mono text-sm" value={value} onChange={(event) => onChange(event.currentTarget.value)} />
    </label>
  );
}

function WaybillInfo({ waybill }: { waybill: ExpressWaybill }) {
  return (
    <div className="grid gap-3 text-sm md:grid-cols-2">
      <Info label="运单号" value={waybill.waybill_no} mono />
      <Info label="状态" value={waybill.status} />
      <Info label="快递商" value={waybill.carrier_code} mono />
      <Info label="包裹号" value={waybill.package_no} mono />
      <Info label="收件人" value={`${waybill.receiver_name} / ${waybill.receiver_mobile}`} />
      <Info label="预计到达" value={waybill.eta_at ? formatDateTime(waybill.eta_at) : "-"} />
    </div>
  );
}

function Info({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="rounded-md border bg-background px-3 py-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className={mono ? "mt-1 font-mono" : "mt-1"}>{value}</div>
    </div>
  );
}
