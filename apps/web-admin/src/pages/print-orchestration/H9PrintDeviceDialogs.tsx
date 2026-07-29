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

import type {
  CreatePrintSiteRequest,
  CreatePrinterRequest,
  CreatePrinterTrayRequest,
  DeviceLease,
  Printer,
  PrinterTray,
  ReleaseDeviceLeaseRequest,
} from "@/features/print-orchestration/print-device-queries";

export interface H9DeviceSelectOption {
  label: string;
  value: string;
}

interface CommonDialogProps {
  open: boolean;
  pending: boolean;
  errorMessage?: string;
  onOpenChange: (open: boolean) => void;
}

export interface DeviceWriteConfirmation {
  title: string;
  description: string;
  confirmLabel: string;
  destructive?: boolean;
}

interface DeviceWriteConfirmDialogProps {
  action: DeviceWriteConfirmation | null;
  pending: boolean;
  errorMessage?: string;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}

export function DeviceWriteConfirmDialog({
  action,
  pending,
  errorMessage,
  onOpenChange,
  onConfirm,
}: DeviceWriteConfirmDialogProps) {
  return (
    <Dialog open={Boolean(action)} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{action?.title ?? "确认设备操作"}</DialogTitle>
          <DialogDescription>{action?.description ?? "请确认本次操作。"}</DialogDescription>
        </DialogHeader>
        <ErrorText message={errorMessage} />
        <DialogFooter>
          <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
          <Button
            type="button"
            variant={action?.destructive ? "destructive" : "default"}
            disabled={pending || !action}
            onClick={onConfirm}
          >
            {pending ? "处理中..." : action?.confirmLabel ?? "确认"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface PrintSiteDialogProps extends CommonDialogProps {
  onSubmit: (request: CreatePrintSiteRequest) => Promise<void>;
}

export function PrintSiteDialog({ open, pending, errorMessage, onOpenChange, onSubmit }: PrintSiteDialogProps) {
  const [siteCode, setSiteCode] = React.useState("");
  const [siteName, setSiteName] = React.useState("");

  React.useEffect(() => {
    if (!open) return;
    setSiteCode("");
    setSiteName("");
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>新建物理打印站点</DialogTitle>
          <DialogDescription>
            站点是打印机、纸盒、设备租约和 Print Agent 的资源边界；设备禁止跨站点引用。
          </DialogDescription>
        </DialogHeader>
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (!siteCode.trim() || !siteName.trim()) return;
            void onSubmit({ site_code: siteCode.trim(), site_name: siteName.trim() }).catch(() => undefined);
          }}
        >
          <Field label="站点编码">
            <Input value={siteCode} maxLength={64} placeholder="如 SITE-EAST-1" onChange={(event) => setSiteCode(event.target.value)} />
          </Field>
          <Field label="站点名称">
            <Input value={siteName} maxLength={100} placeholder="如 东区一号打印站" onChange={(event) => setSiteName(event.target.value)} />
          </Field>
          <ErrorText message={errorMessage} />
          <DialogFooter>
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !siteCode.trim() || !siteName.trim()}>
              {pending ? "创建中..." : "创建站点"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface SiteOwnerMappingDialogProps extends CommonDialogProps {
  siteLabel: string;
  ownerLabel: string;
  warehouses: H9DeviceSelectOption[];
  onSubmit: (warehouseId: string) => Promise<void>;
}

export function SiteOwnerMappingDialog({
  open,
  pending,
  errorMessage,
  siteLabel,
  ownerLabel,
  warehouses,
  onOpenChange,
  onSubmit,
}: SiteOwnerMappingDialogProps) {
  const [warehouseId, setWarehouseId] = React.useState("");

  React.useEffect(() => {
    if (!open) return;
    setWarehouseId(warehouses[0]?.value ?? "");
  }, [open, warehouses]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>映射货主仓</DialogTitle>
          <DialogDescription>
            只有显式映射的货主 + 仓库才能使用本站点设备；停用映射为软删并保留历史。
          </DialogDescription>
        </DialogHeader>
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (!warehouseId) return;
            void onSubmit(warehouseId).catch(() => undefined);
          }}
        >
          <p className="rounded-md bg-muted p-3 text-sm">站点：{siteLabel}</p>
          <div className="space-y-2 text-sm font-medium">
            <span>货主</span>
            <p className="rounded-md border border-input bg-muted px-3 py-2 font-normal">{ownerLabel}</p>
          </div>
          <Field label="仓库">
            <NativeSelect value={warehouseId} options={warehouses} onChange={setWarehouseId} />
          </Field>
          <ErrorText message={errorMessage} />
          <DialogFooter>
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !warehouseId}>
              {pending ? "映射中..." : "确认映射"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface PrinterDialogProps extends CommonDialogProps {
  sites: H9DeviceSelectOption[];
  onSubmit: (request: CreatePrinterRequest) => Promise<void>;
}

export function PrinterDialog({ open, pending, errorMessage, sites, onOpenChange, onSubmit }: PrinterDialogProps) {
  const [siteId, setSiteId] = React.useState("");
  const [printerName, setPrinterName] = React.useState("");
  const [printerModel, setPrinterModel] = React.useState("");
  const [connectionType, setConnectionType] = React.useState("network");
  const [releaseMode, setReleaseMode] = React.useState("");

  React.useEffect(() => {
    if (!open) return;
    setSiteId(sites[0]?.value ?? "");
    setPrinterName("");
    setPrinterModel("");
    setConnectionType("network");
    setReleaseMode("");
  }, [open, sites]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>新建打印机</DialogTitle>
          <DialogDescription>
            打印机归属唯一物理打印站点；USB 打印机租约语义单机，只归实际连接它的本机。
          </DialogDescription>
        </DialogHeader>
        <form
          className="grid gap-4 sm:grid-cols-2"
          onSubmit={(event) => {
            event.preventDefault();
            if (!siteId || !printerName.trim()) return;
            void onSubmit({
              site_id: siteId,
              printer_name: printerName.trim(),
              printer_model: printerModel.trim() ? printerModel.trim() : null,
              connection_type: connectionType,
              release_mode_override: releaseMode ? releaseMode : null,
            }).catch(() => undefined);
          }}
        >
          <Field label="所属站点" wide>
            <NativeSelect value={siteId} options={sites} onChange={setSiteId} />
          </Field>
          <Field label="打印机名称">
            <Input value={printerName} maxLength={100} onChange={(event) => setPrinterName(event.target.value)} />
          </Field>
          <Field label="型号（可选）">
            <Input value={printerModel} maxLength={100} onChange={(event) => setPrinterModel(event.target.value)} />
          </Field>
          <Field label="连接类型">
            <NativeSelect
              value={connectionType}
              options={[
                { value: "network", label: "网络打印机" },
                { value: "usb", label: "USB 打印机（单机）" },
              ]}
              onChange={setConnectionType}
            />
          </Field>
          <Field label="释放模式覆盖（可选）">
            <NativeSelect
              value={releaseMode}
              options={[
                { value: "manual_only", label: "仅人工释放" },
                { value: "safe_auto", label: "安全自动释放" },
              ]}
              onChange={setReleaseMode}
            />
          </Field>
          <p className="text-sm text-muted-foreground sm:col-span-2">
            不选择覆盖时继承全局默认（仅人工释放）；运行中的租约使用创建时的配置快照。
          </p>
          <div className="sm:col-span-2"><ErrorText message={errorMessage} /></div>
          <DialogFooter className="sm:col-span-2">
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !siteId || !printerName.trim()}>
              {pending ? "创建中..." : "创建打印机"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface PrinterTrayDialogProps extends CommonDialogProps {
  printer: Printer | null;
  onSubmit: (request: CreatePrinterTrayRequest) => Promise<void>;
}

export function PrinterTrayDialog({ open, pending, errorMessage, printer, onOpenChange, onSubmit }: PrinterTrayDialogProps) {
  const [trayCode, setTrayCode] = React.useState("");
  const [paperSize, setPaperSize] = React.useState("A4");
  const [paperType, setPaperType] = React.useState("普通纸");

  React.useEffect(() => {
    if (!open) return;
    setTrayCode("");
    setPaperSize("A4");
    setPaperType("普通纸");
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>新建纸盒</DialogTitle>
          <DialogDescription>纸盒声明可承载纸张能力；模板/打印项声明纸张要求后按能力匹配。</DialogDescription>
        </DialogHeader>
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (!trayCode.trim() || !paperSize.trim() || !paperType.trim()) return;
            void onSubmit({
              tray_code: trayCode.trim(),
              paper_size: paperSize.trim(),
              paper_type: paperType.trim(),
            }).catch(() => undefined);
          }}
        >
          <p className="rounded-md bg-muted p-3 text-sm">打印机：{printer ? `${printer.printer_name}（${printer.site_name}）` : "未选择"}</p>
          <Field label="纸盒设备标识">
            <Input value={trayCode} maxLength={64} placeholder="如 TRAY-1" onChange={(event) => setTrayCode(event.target.value)} />
          </Field>
          <Field label="纸张尺寸">
            <Input value={paperSize} maxLength={32} placeholder="如 A4 / A5 / 241x93" onChange={(event) => setPaperSize(event.target.value)} />
          </Field>
          <Field label="纸张类型">
            <Input value={paperType} maxLength={64} placeholder="如 普通纸 / 不干胶标签纸" onChange={(event) => setPaperType(event.target.value)} />
          </Field>
          <ErrorText message={errorMessage} />
          <DialogFooter>
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !printer || !trayCode.trim() || !paperSize.trim() || !paperType.trim()}>
              {pending ? "创建中..." : "创建纸盒"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface TestPrintDialogProps extends CommonDialogProps {
  printer: Printer | null;
  trays: PrinterTray[];
  onSubmit: (trayId: string) => Promise<void>;
}

export function TestPrintDialog({ open, pending, errorMessage, printer, trays, onOpenChange, onSubmit }: TestPrintDialogProps) {
  const [trayId, setTrayId] = React.useState("");
  const enabledTrays = trays.filter((tray) => tray.enabled);

  React.useEffect(() => {
    if (!open) return;
    setTrayId("");
  }, [open]);

  React.useEffect(() => {
    if (enabledTrays.length > 0 && !enabledTrays.some((tray) => tray.id === trayId)) {
      setTrayId(enabledTrays[0].id);
    }
  }, [enabledTrays, trayId]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>测试打印</DialogTitle>
          <DialogDescription>
            对指定打印机和纸盒下发受控测试指令并保存结果记录；真实硬件回执待 Print Agent 接入后登记。
          </DialogDescription>
        </DialogHeader>
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (!trayId) return;
            void onSubmit(trayId).catch(() => undefined);
          }}
        >
          <p className="rounded-md bg-muted p-3 text-sm">打印机：{printer ? `${printer.printer_name}（${printer.site_name}）` : "未选择"}</p>
          <Field label="目标纸盒">
            <NativeSelect
              value={trayId}
              options={enabledTrays.map((tray) => ({
                value: tray.id,
                label: `${tray.tray_code} · ${tray.paper_size} · ${tray.paper_type}`,
              }))}
              onChange={setTrayId}
            />
          </Field>
          {enabledTrays.length === 0 && (
            <p className="text-sm text-destructive" role="alert">该打印机没有启用中的纸盒，请先维护纸盒。</p>
          )}
          <ErrorText message={errorMessage} />
          <DialogFooter>
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !printer || !trayId}>
              {pending ? "下发中..." : "下发测试打印"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface ReleaseLeaseDialogProps extends CommonDialogProps {
  lease: DeviceLease | null;
  canRelease: boolean;
  onSubmit: (request: ReleaseDeviceLeaseRequest) => Promise<void>;
}

export function ReleaseLeaseDialog({
  open,
  pending,
  errorMessage,
  lease,
  canRelease,
  onOpenChange,
  onSubmit,
}: ReleaseLeaseDialogProps) {
  const [reason, setReason] = React.useState("");
  const [confirmed, setConfirmed] = React.useState(false);
  const busy = Boolean(lease && lease.busy_state !== "idle");

  React.useEffect(() => {
    if (!open) return;
    setReason("");
    setConfirmed(false);
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>人工释放设备租约</DialogTitle>
          <DialogDescription>
            人工释放需要专用权限、原因和二次确认；只可覆盖“仅人工释放”模式，不能覆盖打印中、
            结果不明或待对账的硬安全条件。
          </DialogDescription>
        </DialogHeader>
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (!reason.trim() || !confirmed) return;
            void onSubmit({ reason: reason.trim(), confirm: true }).catch(() => undefined);
          }}
        >
          <p className="rounded-md bg-muted p-3 text-sm">
            {lease
              ? `${lease.printer_name} · 租约 ${lease.lease_token} · 模式 ${releaseModeLabel(lease.release_mode)} · 状态 ${busyStateLabel(lease.busy_state)}`
              : "未选择租约"}
          </p>
          {busy && (
            <p className="text-sm text-destructive" role="alert">
              该租约处于 {busyStateLabel(lease?.busy_state ?? "")} 状态，必须先完成打印结果确认或对账，任何人都不得释放。
            </p>
          )}
          {!canRelease && (
            <p className="text-sm text-destructive" role="alert">当前账号缺少人工释放租约专用权限。</p>
          )}
          <Field label="释放原因">
            <Input value={reason} maxLength={500} placeholder="请输入人工释放原因" onChange={(event) => setReason(event.target.value)} />
          </Field>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(event) => setConfirmed(event.target.checked)}
            />
            <span>我已二次确认释放该设备租约</span>
          </label>
          <ErrorText message={errorMessage} />
          <DialogFooter>
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" variant="destructive" disabled={pending || !lease || busy || !canRelease || !reason.trim() || !confirmed}>
              {pending ? "释放中..." : "确认释放"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface PrinterReleaseModeDialogProps extends CommonDialogProps {
  printer: Printer | null;
  onSubmit: (releaseModeOverride: string) => Promise<void>;
}

export function PrinterReleaseModeDialog({
  open,
  pending,
  errorMessage,
  printer,
  onOpenChange,
  onSubmit,
}: PrinterReleaseModeDialogProps) {
  const [mode, setMode] = React.useState("inherit");

  React.useEffect(() => {
    if (!open) return;
    setMode(printer?.release_mode_override ?? "inherit");
  }, [open, printer]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>释放模式覆盖</DialogTitle>
          <DialogDescription>
            全局默认在参数页维护（默认仅人工释放）；此处为打印机单机覆盖，运行中的租约保持快照不变。
          </DialogDescription>
        </DialogHeader>
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            void onSubmit(mode).catch(() => undefined);
          }}
        >
          <p className="rounded-md bg-muted p-3 text-sm">
            {printer ? `${printer.printer_name} · 当前生效：${releaseModeLabel(printer.effective_release_mode)}` : "未选择打印机"}
          </p>
          <Field label="覆盖模式">
            <NativeSelect
              value={mode}
              options={[
                { value: "inherit", label: "继承全局默认" },
                { value: "manual_only", label: "仅人工释放" },
                { value: "safe_auto", label: "安全自动释放" },
              ]}
              onChange={setMode}
            />
          </Field>
          <ErrorText message={errorMessage} />
          <DialogFooter>
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !printer || !mode}>
              {pending ? "保存中..." : "保存覆盖"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export function releaseModeLabel(mode: string) {
  return mode === "safe_auto" ? "安全自动释放" : mode === "manual_only" ? "仅人工释放" : mode;
}

export function busyStateLabel(state: string) {
  const labels: Record<string, string> = {
    idle: "空闲",
    printing: "打印中",
    result_unknown: "结果不明",
    reconciling: "待对账",
  };
  return labels[state] ?? state;
}

function Field({ label, wide, children }: { label: string; wide?: boolean; children: React.ReactNode }) {
  return <label className={`space-y-2 text-sm font-medium${wide ? " sm:col-span-2" : ""}`}><span>{label}</span>{children}</label>;
}

function NativeSelect({ value, options, onChange }: { value: string; options: H9DeviceSelectOption[]; onChange: (value: string) => void }) {
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
