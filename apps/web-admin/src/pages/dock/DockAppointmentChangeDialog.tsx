import * as React from "react";
import { Button, Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input } from "@wms/ui";

import type { Dock, DockAppointment } from "@/features/dock/dock-queries";
import type { DockAppointmentForm } from "@/pages/dock/DockAppointmentCreateDialog";

type Props = {
  open: boolean;
  appointment: DockAppointment | null;
  docks: Dock[];
  form: DockAppointmentForm;
  pending: boolean;
  errorMessage?: string;
  onOpenChange: (open: boolean) => void;
  onFormChange: (form: DockAppointmentForm) => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
};

export function DockAppointmentChangeDialog({ open, appointment, docks, form, pending, errorMessage, onOpenChange, onFormChange, onSubmit }: Props) {
  const update = <K extends keyof DockAppointmentForm>(key: K, value: DockAppointmentForm[K]) => onFormChange({ ...form, [key]: value });
  return <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}><DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl"><form className="grid gap-4" onSubmit={onSubmit}>
    <DialogHeader><DialogTitle>变更月台预约</DialogTitle><DialogDescription>变更会保留原预约，并创建递增版本；关联单据不可修改。</DialogDescription></DialogHeader>
    <div className="grid gap-3 rounded-md border bg-muted/30 p-3 text-sm sm:grid-cols-3"><ContextValue label="预约编号" value={appointment?.appointment_no ?? "-"} /><ContextValue label="关联单据" value={appointment ? `${appointment.document_type} / ${appointment.document_no}` : "-"} /><ContextValue label="当前月台" value={docks.find((dock) => dock.id === appointment?.dock_id)?.dock_code ?? "-"} /></div>
    <div className="grid gap-4 sm:grid-cols-2"><Field label="月台"><Select value={form.dockId ?? appointment?.dock_id ?? ""} options={docks.map((dock) => ({ label: dock.dock_code, value: dock.id }))} onChange={(value) => update("dockId", value)} /></Field><Field label="预约开始"><Input required type="datetime-local" value={form.windowStartAt} onChange={(event) => update("windowStartAt", event.target.value)} /></Field><Field label="预约结束"><Input required min={form.windowStartAt || undefined} type="datetime-local" value={form.windowEndAt} onChange={(event) => update("windowEndAt", event.target.value)} /></Field><Field label="车牌号"><Input value={form.vehiclePlateNo} onChange={(event) => update("vehiclePlateNo", event.target.value)} /></Field><Field label="车辆类型"><Select value={form.vehicleType} options={[{ label: "常温", value: "normal" }, { label: "冷藏车", value: "refrigerated" }]} onChange={(value) => update("vehicleType", value)} /></Field><Field label="司机姓名"><Input required value={form.driverName} onChange={(event) => update("driverName", event.target.value)} /></Field><Field label="司机电话"><Input required inputMode="tel" value={form.driverPhone} onChange={(event) => update("driverPhone", event.target.value)} /></Field><Field label="变更原因"><Input required value={form.reason ?? ""} onChange={(event) => update("reason", event.target.value)} placeholder="请输入变更原因" /></Field></div>
    {errorMessage && <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">{errorMessage}</div>}
    <DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="submit" disabled={pending || !appointment}>{pending ? "保存中..." : "保存变更"}</Button></DialogFooter>
  </form></DialogContent></Dialog>;
}

function ContextValue({ label, value }: { label: string; value: string }) { return <div><div className="text-muted-foreground">{label}</div><div className="font-medium">{value}</div></div>; }
function Field({ label, children }: { label: string; children: React.ReactNode }) { return <label className="grid gap-1 text-sm"><span>{label}</span>{children}</label>; }
function Select({ value, options, onChange }: { value: string; options: Array<{ label: string; value: string }>; onChange: (value: string) => void }) { return <select className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={value} onChange={(event) => onChange(event.target.value)}>{options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select>; }
