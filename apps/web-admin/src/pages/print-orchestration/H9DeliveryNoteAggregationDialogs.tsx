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

import { useCustomerAddressesQuery } from "@/features/master-data/master-data-queries/queries";
import type {
  CreateCutoffPlanRequest,
  DeliveryNoteCandidate,
  PublishRouteBindingRequest,
} from "@/features/print-orchestration/print-orchestration-queries";

export interface H9SelectOption {
  label: string;
  value: string;
}

interface CommonDialogProps {
  open: boolean;
  pending: boolean;
  errorMessage?: string;
  onOpenChange: (open: boolean) => void;
}

interface ManualCutoffDialogProps extends CommonDialogProps {
  rows: DeliveryNoteCandidate[];
  onSubmit: (reason: string) => Promise<void>;
}

export function ManualCutoffDialog({
  open,
  pending,
  errorMessage,
  rows,
  onOpenChange,
  onSubmit,
}: ManualCutoffDialogProps) {
  const [reason, setReason] = React.useState("");

  React.useEffect(() => {
    if (open) setReason("");
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>授权人工截单</DialogTitle>
          <DialogDescription>
            将同一仓库、客户地址和冻结线路下的 {rows.length} 张订单归入一个随货同行单号。
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <p className="rounded-md bg-muted p-3 text-sm">
            {rows[0]
              ? `${rows[0].warehouse_code} · ${rows[0].customer_name} · ${rows[0].delivery_address} · ${rows[0].route_code}`
              : "未选择订单"}
          </p>
          <Field label="截单原因">
            <Input
              value={reason}
              maxLength={500}
              placeholder="请输入授权人工截单原因"
              onChange={(event) => setReason(event.target.value)}
            />
          </Field>
          <ErrorText message={errorMessage} />
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline" disabled={pending}>取消</Button>
          </DialogClose>
          <Button
            type="button"
            disabled={pending || rows.length === 0 || !reason.trim()}
            onClick={() => void onSubmit(reason.trim()).catch(() => undefined)}
          >
            {pending ? "截单中..." : "确认截单"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface RouteBindingDialogProps extends CommonDialogProps {
  warehouses: H9SelectOption[];
  customers: H9SelectOption[];
  onSubmit: (request: PublishRouteBindingRequest) => Promise<void>;
}

export function RouteBindingDialog({
  open,
  pending,
  errorMessage,
  warehouses,
  customers,
  onOpenChange,
  onSubmit,
}: RouteBindingDialogProps) {
  const [warehouseId, setWarehouseId] = React.useState("");
  const [customerId, setCustomerId] = React.useState("");
  const [addressId, setAddressId] = React.useState("");
  const [routeCode, setRouteCode] = React.useState("");
  const [effectiveFrom, setEffectiveFrom] = React.useState("");
  const [effectiveTo, setEffectiveTo] = React.useState("");
  const addressesQuery = useCustomerAddressesQuery(customerId || null);
  const addresses = addressesQuery.data ?? [];

  React.useEffect(() => {
    if (!open) return;
    setWarehouseId(warehouses[0]?.value ?? "");
    setCustomerId(customers[0]?.value ?? "");
    setAddressId("");
    setRouteCode("");
    setEffectiveFrom(localDateTime(new Date()));
    setEffectiveTo("");
  }, [customers, open, warehouses]);

  React.useEffect(() => {
    if (addresses.length > 0 && !addresses.some((item) => item.id === addressId)) {
      setAddressId(addresses[0].id);
    }
  }, [addressId, addresses]);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!warehouseId || !customerId || !addressId || !routeCode.trim() || !effectiveFrom) return;
    await onSubmit({
      warehouse_id: warehouseId,
      customer_id: customerId,
      delivery_address_id: addressId,
      route_code: routeCode.trim(),
      effective_from: new Date(effectiveFrom).toISOString(),
      effective_to: effectiveTo ? new Date(effectiveTo).toISOString() : null,
    });
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>发布送货地址线路</DialogTitle>
          <DialogDescription>订单进入时按生效线路冻结；同一地址在同一时间只能有一条有效线路。</DialogDescription>
        </DialogHeader>
        <form className="grid gap-4 sm:grid-cols-2" onSubmit={(event) => void submit(event).catch(() => undefined)}>
          <Field label="仓库"><NativeSelect value={warehouseId} options={warehouses} onChange={setWarehouseId} /></Field>
          <Field label="客户"><NativeSelect value={customerId} options={customers} onChange={setCustomerId} /></Field>
          <Field label="送货地址" wide>
            <NativeSelect
              value={addressId}
              options={addresses.map((item) => ({
                value: item.id,
                label: `${item.province}${item.city}${item.district}${item.detail_address}`,
              }))}
              onChange={setAddressId}
            />
          </Field>
          <Field label="线路编码"><Input value={routeCode} maxLength={64} onChange={(event) => setRouteCode(event.target.value)} /></Field>
          <Field label="生效时间"><Input type="datetime-local" value={effectiveFrom} onChange={(event) => setEffectiveFrom(event.target.value)} /></Field>
          <Field label="失效时间（可选）"><Input type="datetime-local" value={effectiveTo} onChange={(event) => setEffectiveTo(event.target.value)} /></Field>
          <div className="sm:col-span-2"><ErrorText message={errorMessage ?? addressesQuery.error?.message} /></div>
          <DialogFooter className="sm:col-span-2">
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !warehouseId || !customerId || !addressId || !routeCode.trim()}>
              {pending ? "发布中..." : "发布线路"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface CutoffPlanDialogProps extends CommonDialogProps {
  warehouses: H9SelectOption[];
  customers: H9SelectOption[];
  onSubmit: (request: CreateCutoffPlanRequest) => Promise<void>;
}

const weekdays = [
  [1, "周一"],
  [2, "周二"],
  [3, "周三"],
  [4, "周四"],
  [5, "周五"],
  [6, "周六"],
  [7, "周日"],
] as const;

export function CutoffPlanDialog({
  open,
  pending,
  errorMessage,
  warehouses,
  customers,
  onOpenChange,
  onSubmit,
}: CutoffPlanDialogProps) {
  const [name, setName] = React.useState("");
  const [warehouseId, setWarehouseId] = React.useState("");
  const [scope, setScope] = React.useState<CreateCutoffPlanRequest["scope"]>("owner_warehouse");
  const [customerId, setCustomerId] = React.useState("");
  const [routeCode, setRouteCode] = React.useState("");
  const [effectiveFrom, setEffectiveFrom] = React.useState("");
  const [effectiveTo, setEffectiveTo] = React.useState("");
  const [weekly, setWeekly] = React.useState(() => defaultWeekly());
  const [exceptions, setExceptions] = React.useState<Array<{ date: string; cutoffTime: string }>>([]);

  React.useEffect(() => {
    if (!open) return;
    setName("");
    setWarehouseId(warehouses[0]?.value ?? "");
    setScope("owner_warehouse");
    setCustomerId(customers[0]?.value ?? "");
    setRouteCode("");
    setEffectiveFrom(localDateTime(new Date()));
    setEffectiveTo("");
    setWeekly(defaultWeekly());
    setExceptions([]);
  }, [customers, open, warehouses]);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const weeklySchedule = weekly
      .filter((item) => item.enabled)
      .map((item) => ({ weekday: item.weekday, cutoff_time: item.cutoffTime }));
    if (!name.trim() || !warehouseId || weeklySchedule.length === 0 || !effectiveFrom) return;
    if (scope === "customer" && !customerId) return;
    if (scope === "route" && !routeCode.trim()) return;
    await onSubmit({
      name: name.trim(),
      warehouse_id: warehouseId,
      scope,
      customer_id: scope === "customer" ? customerId : null,
      route_code: scope === "route" ? routeCode.trim() : null,
      utc_offset_minutes: 480,
      weekly_schedule: weeklySchedule,
      exceptions: exceptions
        .filter((item) => item.date)
        .map((item) => ({ date: item.date, cutoff_time: item.cutoffTime || null })),
      effective_from: new Date(effectiveFrom).toISOString(),
      effective_to: effectiveTo ? new Date(effectiveTo).toISOString() : null,
    });
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>新建截单计划</DialogTitle>
          <DialogDescription>匹配优先级固定为客户、线路、货主加仓库；草稿发布后才参与自动截单。</DialogDescription>
        </DialogHeader>
        <form className="space-y-5" onSubmit={(event) => void submit(event).catch(() => undefined)}>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="计划名称"><Input value={name} maxLength={100} onChange={(event) => setName(event.target.value)} /></Field>
            <Field label="仓库"><NativeSelect value={warehouseId} options={warehouses} onChange={setWarehouseId} /></Field>
            <Field label="适用层级">
              <NativeSelect
                value={scope}
                options={[
                  { value: "customer", label: "客户" },
                  { value: "route", label: "线路" },
                  { value: "owner_warehouse", label: "货主加仓库" },
                ]}
                onChange={(value) => setScope(value as CreateCutoffPlanRequest["scope"])}
              />
            </Field>
            {scope === "customer" && <Field label="客户"><NativeSelect value={customerId} options={customers} onChange={setCustomerId} /></Field>}
            {scope === "route" && <Field label="线路编码"><Input value={routeCode} maxLength={64} onChange={(event) => setRouteCode(event.target.value)} /></Field>}
            <Field label="生效时间"><Input type="datetime-local" value={effectiveFrom} onChange={(event) => setEffectiveFrom(event.target.value)} /></Field>
            <Field label="失效时间（可选）"><Input type="datetime-local" value={effectiveTo} onChange={(event) => setEffectiveTo(event.target.value)} /></Field>
          </div>
          <fieldset className="space-y-2 rounded-md border p-4">
            <legend className="px-1 text-sm font-medium">每周截单时间（北京时间）</legend>
            {weekly.map((item, index) => (
              <div key={item.weekday} className="grid grid-cols-[80px_1fr] items-center gap-3">
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={item.enabled}
                    onChange={(event) => setWeekly((current) => current.map((row, rowIndex) => rowIndex === index ? { ...row, enabled: event.target.checked } : row))}
                  />
                  {weekdays[index][1]}
                </label>
                <Input
                  type="time"
                  value={item.cutoffTime}
                  disabled={!item.enabled}
                  onChange={(event) => setWeekly((current) => current.map((row, rowIndex) => rowIndex === index ? { ...row, cutoffTime: event.target.value } : row))}
                />
              </div>
            ))}
          </fieldset>
          <fieldset className="space-y-3 rounded-md border p-4">
            <div className="flex items-center justify-between">
              <legend className="text-sm font-medium">例外日期</legend>
              <Button type="button" size="sm" variant="outline" onClick={() => setExceptions((current) => [...current, { date: "", cutoffTime: "" }])}>添加例外</Button>
            </div>
            {exceptions.length === 0 && <p className="text-sm text-muted-foreground">暂无例外；时间留空表示当天不截单。</p>}
            {exceptions.map((item, index) => (
              <div key={index} className="grid grid-cols-[1fr_1fr_auto] gap-2">
                <Input type="date" aria-label={`例外日期 ${index + 1}`} value={item.date} onChange={(event) => updateException(setExceptions, index, "date", event.target.value)} />
                <Input type="time" aria-label={`例外截单时间 ${index + 1}`} value={item.cutoffTime} onChange={(event) => updateException(setExceptions, index, "cutoffTime", event.target.value)} />
                <Button type="button" variant="ghost" onClick={() => setExceptions((current) => current.filter((_, rowIndex) => rowIndex !== index))}>移除</Button>
              </div>
            ))}
          </fieldset>
          <ErrorText message={errorMessage} />
          <DialogFooter>
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !name.trim() || !warehouseId || weekly.every((item) => !item.enabled)}>
              {pending ? "保存中..." : "保存草稿"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Field({ label, wide, children }: { label: string; wide?: boolean; children: React.ReactNode }) {
  return <label className={`space-y-2 text-sm font-medium${wide ? " sm:col-span-2" : ""}`}><span>{label}</span>{children}</label>;
}

function NativeSelect({ value, options, onChange }: { value: string; options: H9SelectOption[]; onChange: (value: string) => void }) {
  return (
    <select
      className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
      value={value}
      onChange={(event) => onChange(event.target.value)}
    >
      <option value="">请选择</option>
      {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
    </select>
  );
}

function ErrorText({ message }: { message?: string }) {
  return message ? <p className="text-sm text-destructive" role="alert">{message}</p> : null;
}

function localDateTime(date: Date) {
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

function defaultWeekly() {
  return weekdays.map(([weekday]) => ({ weekday, cutoffTime: "17:00", enabled: weekday <= 5 }));
}

function updateException(
  setExceptions: React.Dispatch<React.SetStateAction<Array<{ date: string; cutoffTime: string }>>>,
  index: number,
  key: "date" | "cutoffTime",
  value: string,
) {
  setExceptions((current) => current.map((item, rowIndex) => rowIndex === index ? { ...item, [key]: value } : item));
}
