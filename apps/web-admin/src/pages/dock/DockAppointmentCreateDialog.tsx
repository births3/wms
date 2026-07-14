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

import type { Dock } from "@/features/dock/dock-queries";

export type DockAppointmentForm = {
  dockId?: string;
  appointmentNo: string;
  documentType: string;
  documentNo: string;
  windowStartAt: string;
  windowEndAt: string;
  vehiclePlateNo: string;
  vehicleType: string;
  driverName: string;
  driverPhone: string;
  reason?: string;
};

export type DockAppointmentCreateDialogProps = {
  open: boolean;
  dock: Dock | null;
  warehouseLabel: string;
  form: DockAppointmentForm;
  pending: boolean;
  errorMessage?: string;
  onOpenChange: (open: boolean) => void;
  onFormChange: (form: DockAppointmentForm) => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
};

const documentTypeOptions = [
  { label: "采购入库", value: "purchase_inbound" },
  { label: "销售出库", value: "sales_outbound" },
  { label: "销售退货", value: "sales_return" },
];

const vehicleTypeOptions = [
  { label: "常温", value: "normal" },
  { label: "冷藏车", value: "refrigerated" },
];

export function DockAppointmentCreateDialog({
  open,
  dock,
  warehouseLabel,
  form,
  pending,
  errorMessage,
  onOpenChange,
  onFormChange,
  onSubmit,
}: DockAppointmentCreateDialogProps) {
  const update = <K extends keyof DockAppointmentForm>(key: K, value: DockAppointmentForm[K]) => {
    onFormChange({ ...form, [key]: value });
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <form className="grid gap-4" onSubmit={onSubmit}>
          <DialogHeader>
            <DialogTitle>创建月台预约</DialogTitle>
            <DialogDescription>预约只允许关联当前选中的启用月台。</DialogDescription>
          </DialogHeader>
          <div className="grid gap-3 rounded-md border bg-muted/30 p-3 text-sm sm:grid-cols-2">
            <ContextValue label="仓库" value={warehouseLabel || "当前查询仓库"} />
            <ContextValue label="月台" value={dock?.dock_code ?? "未选择月台"} />
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="预约编号">
              <Input required value={form.appointmentNo} onChange={(event) => update("appointmentNo", event.target.value)} />
            </Field>
            <Field label="关联单据类型">
              <Select value={form.documentType} options={documentTypeOptions} onChange={(value) => update("documentType", value)} />
            </Field>
            <Field label="关联单据号">
              <Input required value={form.documentNo} onChange={(event) => update("documentNo", event.target.value)} placeholder="请输入单据号" />
            </Field>
            <Field label="车辆类型">
              <Select value={form.vehicleType} options={vehicleTypeOptions} onChange={(value) => update("vehicleType", value)} />
            </Field>
            <Field label="预约开始">
              <Input required type="datetime-local" value={form.windowStartAt} onChange={(event) => update("windowStartAt", event.target.value)} />
            </Field>
            <Field label="预约结束">
              <Input required min={form.windowStartAt || undefined} type="datetime-local" value={form.windowEndAt} onChange={(event) => update("windowEndAt", event.target.value)} />
            </Field>
            <Field label="车牌号">
              <Input value={form.vehiclePlateNo} onChange={(event) => update("vehiclePlateNo", event.target.value)} placeholder="选填" />
            </Field>
            <Field label="司机姓名">
              <Input required value={form.driverName} onChange={(event) => update("driverName", event.target.value)} />
            </Field>
            <Field label="司机电话">
              <Input required inputMode="tel" value={form.driverPhone} onChange={(event) => update("driverPhone", event.target.value)} />
            </Field>
          </div>
          {errorMessage && <ErrorNotice message={errorMessage} />}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={pending}>取消</Button>
            </DialogClose>
            <Button type="submit" disabled={pending || !dock}>{pending ? "创建中..." : "创建预约"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function ContextValue({ label, value }: { label: string; value: string }) {
  return <div><div className="text-muted-foreground">{label}</div><div className="font-medium">{value}</div></div>;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="grid gap-1 text-sm"><span>{label}</span>{children}</label>;
}

function Select({ value, options, onChange }: { value: string; options: Array<{ label: string; value: string }>; onChange: (value: string) => void }) {
  return <select className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={value} onChange={(event) => onChange(event.target.value)}>{options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select>;
}

function ErrorNotice({ message }: { message: string }) {
  return <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">{message}</div>;
}
