import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  QueryPanel,
  type DataGridColumn,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { AlertCircle, RefreshCw } from "lucide-react";

import { useDialogState } from "@/lib/use-dialog-state";
import {
  useBindDeviceMutation,
  useDevicesQuery,
  useRegisterDeviceMutation,
  useToggleDeviceEnabledMutation,
  type Device,
} from "@/features/device/device-queries";

type Notice = { kind: "success" | "error"; text: string } | null;

export const queryFields: QueryPanelField[] = [
  { key: "status", label: "在线状态", type: "multiSelect", options: [{ label: "全部", value: "" }, { label: "在线", value: "online" }, { label: "离线", value: "offline" }, { label: "停用", value: "disabled" }] },
  { key: "device_type", label: "设备类型", type: "multiSelect", options: [{ label: "全部", value: "" }, { label: "AGV", value: "agv" }, { label: "PTL", value: "ptl_light" }, { label: "DWS", value: "dws" }, { label: "RFID", value: "rfid_antenna" }, { label: "堆垛机", value: "stacker" }] },
  { key: "online_status", label: "在线状态筛选", type: "text", placeholder: "online/offline/disabled" },
];

export const defaultVisibleFieldKeys = ["status"];

const DEVICE_TYPES: Record<string, string> = {
  agv: "AGV 搬运车",
  ptl_light: "PTL 电子标签",
  dws: "DWS 称重复核",
  rfid_antenna: "RFID 天线",
  stacker: "立库堆垛机",
};

const STATUS_LABEL: Record<string, string> = { online: "在线", offline: "离线", disabled: "停用" };

type RegisterForm = {
  device_code: string;
  device_type: string;
  vendor: string;
  model: string;
  protocol: string;
  ip_address: string;
};

function emptyForm(): RegisterForm {
  return { device_code: "", device_type: "ptl_light", vendor: "", model: "", protocol: "http", ip_address: "" };
}

export function M1DevicePage() {
  const [queryValue, setQueryValue] = React.useState<QueryPanelValue>({});
  const listQuery = useDevicesQuery();
  const registerMutation = useRegisterDeviceMutation();
  const toggleMutation = useToggleDeviceEnabledMutation();
  const bindMutation = useBindDeviceMutation();
  const [notice, setNotice] = React.useState<Notice>(null);
  const [selected, setSelected] = React.useState<string[]>([]);
  const registerDialog = useDialogState<null>();
  const bindDialog = useDialogState<Device>();
  const [form, setForm] = React.useState<RegisterForm>(emptyForm());
  const [bindForm, setBindForm] = React.useState({
    location_id: "",
    binding_role: "ptl_light",
    point_address: "",
  });

  const rows = listQuery.data ?? [];
  const selectedRow = rows.find((row) => row.id === selected[0]);
  const busy =
    registerMutation.isPending || toggleMutation.isPending || bindMutation.isPending;

  const columns: DataGridColumn<Device>[] = [
    { key: "device_code", header: "设备编码", width: 140, sortable: true, filterValue: (row) => row.device_code, copyValue: (row) => row.device_code },
    { key: "device_type", header: "设备类型", width: 130, render: (row) => DEVICE_TYPES[row.device_type] ?? row.device_type },
    { key: "vendor", header: "厂商", width: 120 },
    { key: "model", header: "型号", width: 120 },
    { key: "protocol", header: "协议", width: 90 },
    { key: "ip_address", header: "IP 地址", width: 130, mono: true },
    { key: "online_status", header: "在线状态", width: 100, render: (row) => STATUS_LABEL[row.online_status] ?? row.online_status },
    { key: "last_heartbeat_at", header: "最近心跳", width: 160, render: (row) => (row.last_heartbeat_at ? new Date(row.last_heartbeat_at).toLocaleString() : "—") },
    { key: "enabled", header: "启停", width: 80, render: (row) => (
      <Button type="button" variant="outline" size="sm" onClick={() => void onSubmitToggle(row)} disabled={busy}>
        {row.enabled ? "停用" : "启用"}
      </Button>
    ) },
    { key: "created_at", header: "创建时间", width: 160 },
  ];

  const toolbarActions: DataGridToolbarAction[] = [
    {
      key: "register",
      label: "注册设备",
      description: "登记新设备档案",
      disabled: busy,
      onClick: () => {
        setForm(emptyForm());
        registerDialog.openWith(null);
      },
    },
    {
      key: "bind",
      label: "库位绑定",
      description: "绑定设备到库位点位",
      disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
      onClick: () => {
        if (selectedRow) {
          setBindForm({ location_id: "", binding_role: "ptl_light", point_address: "" });
          bindDialog.openWith(selectedRow);
        }
      },
    },
  ];

  async function onSubmitToggle(row: Device) {
    try {
      await toggleMutation.mutateAsync({ id: row.id, enabled: !row.enabled });
      setNotice({ kind: "success", text: row.enabled ? "设备已停用" : "设备已启用" });
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "启停失败" });
    }
  }

  async function onSubmitRegister() {
    try {
      await registerMutation.mutateAsync({
        device_code: form.device_code,
        device_type: form.device_type,
        vendor: form.vendor || undefined,
        model: form.model || undefined,
        protocol: form.protocol,
        ip_address: form.ip_address || undefined,
        port: undefined,
        extra_config: {},
      });
      setNotice({ kind: "success", text: "设备注册成功" });
      registerDialog.close();
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "注册失败" });
    }
  }

  async function onSubmitBind() {
    const target = bindDialog.target;
    if (!target) return;
    try {
      await bindMutation.mutateAsync({
        device_id: target.id,
        location_id: bindForm.location_id,
        binding_role: bindForm.binding_role,
        point_address: bindForm.point_address || undefined,
      });
      setNotice({ kind: "success", text: "库位绑定成功" });
      bindDialog.close();
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "绑定失败" });
    }
  }

  return (
    <div data-testid="m1-device-page" className="flex flex-col gap-4 p-4">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold">M1 设备档案</h1>
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <AlertCircle className="h-4 w-4" />
          <span>模拟器先行：设备心跳与事件经模拟网关通道</span>
          <Button type="button" variant="outline" size="sm" onClick={() => void listQuery.refetch()} disabled={listQuery.isFetching}>
            <RefreshCw className="h-3 w-3" />
            刷新
          </Button>
        </div>
      </div>

      <QueryPanel
        fields={queryFields}
        defaultVisibleFieldKeys={defaultVisibleFieldKeys}
        value={queryValue}
        onValueChange={setQueryValue}
        onQuery={() => void listQuery.refetch()}
      />

      <DataGrid
        storageKey="m1.devices"
        columns={columns}
        data={rows}
        rowKey={(row) => row.id}
        selectable
        selectedRowKeys={selected}
        onSelectedRowKeysChange={setSelected}
        toolbarActions={toolbarActions}
        caption={listQuery.isPending ? "加载设备..." : undefined}
        emptyTitle={listQuery.isError ? "读取设备失败" : "暂无设备"}
        emptyDescription={listQuery.isError ? "请检查后端服务" : "可注册新设备档案"}
      />

      <Dialog open={registerDialog.open} onOpenChange={(open) => !busy && registerDialog.setOpen(open)}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>注册设备</DialogTitle>
            <DialogDescription>登记设备档案（模拟网关通道）。</DialogDescription>
          </DialogHeader>
          <div className="grid grid-cols-2 gap-3">
            <label className="text-sm">
              设备编码
              <Input required value={form.device_code} onChange={(event) => setForm({ ...form, device_code: event.target.value })} className="mt-1" />
            </label>
            <label className="text-sm">
              设备类型
              <select className="mt-1 w-full rounded border px-2 py-1" value={form.device_type} onChange={(event) => setForm({ ...form, device_type: event.target.value })}>
                {Object.entries(DEVICE_TYPES).map(([value, label]) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </label>
            <label className="text-sm">
              厂商
              <Input value={form.vendor} onChange={(event) => setForm({ ...form, vendor: event.target.value })} className="mt-1" />
            </label>
            <label className="text-sm">
              型号
              <Input value={form.model} onChange={(event) => setForm({ ...form, model: event.target.value })} className="mt-1" />
            </label>
            <label className="text-sm">
              协议
              <select className="mt-1 w-full rounded border px-2 py-1" value={form.protocol} onChange={(event) => setForm({ ...form, protocol: event.target.value })}>
                <option value="http">http</option>
                <option value="tcp">tcp</option>
                <option value="modbus_tcp">modbus_tcp</option>
                <option value="mqtt">mqtt</option>
              </select>
            </label>
            <label className="text-sm">
              IP 地址
              <Input value={form.ip_address} onChange={(event) => setForm({ ...form, ip_address: event.target.value })} className="mt-1" />
            </label>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => registerDialog.close()} disabled={busy}>取消</Button>
            <Button type="button" onClick={() => void onSubmitRegister()} disabled={busy || !form.device_code}>注册</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={bindDialog.open} onOpenChange={(open) => !busy && bindDialog.setOpen(open)}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>库位绑定（{bindDialog.target?.device_code ?? ""}）</DialogTitle>
            <DialogDescription>绑定角色与设备类型须匹配；同一库位同一角色仅一条生效绑定。</DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <label className="text-sm">
              库位 ID
              <Input value={bindForm.location_id} onChange={(event) => setBindForm({ ...bindForm, location_id: event.target.value })} className="mt-1" />
            </label>
            <label className="text-sm">
              绑定角色
              <select className="mt-1 w-full rounded border px-2 py-1" value={bindForm.binding_role} onChange={(event) => setBindForm({ ...bindForm, binding_role: event.target.value })}>
                <option value="ptl_light">ptl_light</option>
                <option value="rfid_antenna">rfid_antenna</option>
              </select>
            </label>
            <label className="text-sm">
              点位地址（PTL 灯位地址码）
              <Input value={bindForm.point_address} onChange={(event) => setBindForm({ ...bindForm, point_address: event.target.value })} className="mt-1" />
            </label>
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => bindDialog.close()} disabled={busy}>取消</Button>
            <Button type="button" onClick={() => void onSubmitBind()} disabled={busy || !bindForm.location_id}>绑定</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {notice && (
        <div className={`rounded border p-2 text-sm ${notice.kind === "success" ? "border-green-300 text-green-700" : "border-red-300 text-red-700"}`}>
          {notice.text}
        </div>
      )}
    </div>
  );
}
