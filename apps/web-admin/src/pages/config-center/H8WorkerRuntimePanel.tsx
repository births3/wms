import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  type DataGridColumn,
  type DataGridToolbarAction,
} from "@wms/ui";
import { FileKey, Pause, Play, RefreshCw } from "lucide-react";

import {
  useH8PayloadRetentionPoliciesQuery,
  useH8WorkerRuntimeQuery,
  useSetH8WorkerClaimControlMutation,
  useUpdateH8PayloadRetentionPolicyMutation,
  type H8PayloadRetentionPolicy,
  type H8WorkerRuntimeResponse,
} from "@/features/config-center/erp-message-queries";
import {
  useErpConnectorsQuery,
  type H8ErpConnector,
} from "@/features/config-center/erp-connector-queries";
import { errorText } from "@/lib/error-text";
import { formatDateTime } from "@/lib/format";
import { useDialogState } from "@/lib/use-dialog-state";

type WorkerRow = {
  id: string;
  workerId: string;
  workerVersion: string;
  connectorId: string;
  connectorCode: string;
  direction: string;
  currentClaims: number;
  createdAt: string | null;
  lastHeartbeatAt: string | null;
  health: string;
  paused: boolean;
  pauseReason: string;
  payloadEnabled: boolean;
  retentionDays: number;
};

const columns: DataGridColumn<WorkerRow>[] = [
  { key: "workerId", header: "Worker 实例", width: 160 },
  { key: "workerVersion", header: "版本", width: 80 },
  { key: "connectorCode", header: "连接编码", width: 130 },
  {
    key: "direction",
    header: "方向",
    width: 70,
    render: (row) => (row.direction === "inbound" ? "入站" : "出站"),
  },
  { key: "currentClaims", header: "当前认领数", width: 90 },
  {
    key: "health",
    header: "健康状态",
    width: 90,
    render: (row) => (row.health === "healthy" ? "健康" : "失联"),
  },
  {
    key: "paused",
    header: "认领控制",
    width: 90,
    render: (row) => (row.paused ? "已暂停" : "运行中"),
  },
  {
    key: "createdAt",
    header: "创建时间",
    width: 160,
    render: (row) => (row.createdAt ? formatDateTime(row.createdAt) : "—"),
  },
  {
    key: "lastHeartbeatAt",
    header: "最后心跳",
    width: 160,
    render: (row) => (row.lastHeartbeatAt ? formatDateTime(row.lastHeartbeatAt) : "未上报"),
  },
  { key: "pauseReason", header: "控制原因", width: 180 },
  {
    key: "payloadEnabled",
    header: "完整报文",
    width: 110,
    render: (row) => (row.payloadEnabled ? `${row.retentionDays} 天` : "关闭"),
  },
];

function buildRows(
  runtime?: H8WorkerRuntimeResponse,
  policies: H8PayloadRetentionPolicy[] = [],
  connectors: H8ErpConnector[] = [],
): WorkerRow[] {
  if (!runtime) return [];
  const controls = new Map(
    runtime.controls.map((control) => [`${control.connector_id}:${control.direction}`, control]),
  );
  const policyByConnector = new Map(policies.map((policy) => [policy.connector_id, policy]));
  const connectorById = new Map(connectors.map((connector) => [connector.id, connector]));
  const rows: WorkerRow[] = runtime.workers.flatMap((worker) =>
    worker.directions.map((direction) => {
      const control = controls.get(`${worker.connector_id}:${direction}`);
      const policy = policyByConnector.get(worker.connector_id);
      return {
        id: `${worker.worker_id}:${worker.connector_id}:${direction}`,
        workerId: worker.worker_id,
        workerVersion: worker.worker_version,
        connectorId: worker.connector_id,
        connectorCode: connectorById.get(worker.connector_id)?.connector_code ?? worker.connector_id,
        direction,
        currentClaims: worker.current_claims,
        createdAt: worker.created_at,
        lastHeartbeatAt: worker.last_heartbeat_at,
        health: worker.health,
        paused: control?.paused ?? false,
        pauseReason: control?.reason ?? "—",
        payloadEnabled: policy?.enabled ?? false,
        retentionDays: policy?.retention_days ?? 7,
      };
    }),
  );
  const represented = new Set(rows.map((row) => `${row.connectorId}:${row.direction}`));
  for (const connector of connectors) {
    for (const direction of connector.directions) {
      const key = `${connector.id}:${direction}`;
      if (represented.has(key)) continue;
      const control = controls.get(key);
      const policy = policyByConnector.get(connector.id);
      rows.push({
        id: `offline:${key}`,
        workerId: "未上报",
        workerVersion: "—",
        connectorId: connector.id,
        connectorCode: connector.connector_code,
        direction,
        currentClaims: 0,
        createdAt: null,
        lastHeartbeatAt: null,
        health: "stale",
        paused: control?.paused ?? false,
        pauseReason: control?.reason ?? "—",
        payloadEnabled: policy?.enabled ?? false,
        retentionDays: policy?.retention_days ?? 7,
      });
      represented.add(key);
    }
  }
  for (const control of runtime.controls) {
    const key = `${control.connector_id}:${control.direction}`;
    if (!represented.has(key)) {
      const policy = policyByConnector.get(control.connector_id);
      rows.push({
        id: `offline:${key}`,
        workerId: "未上报",
        workerVersion: "—",
        connectorId: control.connector_id,
        connectorCode:
          connectorById.get(control.connector_id)?.connector_code ?? control.connector_id,
        direction: control.direction,
        currentClaims: 0,
        createdAt: null,
        lastHeartbeatAt: null,
        health: "stale",
        paused: control.paused,
        pauseReason: control.reason,
        payloadEnabled: policy?.enabled ?? false,
        retentionDays: policy?.retention_days ?? 7,
      });
    }
  }
  return rows;
}

export function H8WorkerRuntimePanel({
  canWrite,
  onNotice,
}: {
  canWrite: boolean;
  onNotice: (message: string, isError?: boolean) => void;
}) {
  const runtimeQuery = useH8WorkerRuntimeQuery();
  const policyQuery = useH8PayloadRetentionPoliciesQuery();
  const connectorQuery = useErpConnectorsQuery();
  const controlMutation = useSetH8WorkerClaimControlMutation();
  const policyMutation = useUpdateH8PayloadRetentionPolicyMutation();
  const [selectedKeys, setSelectedKeys] = React.useState<string[]>([]);
  const controlDialog = useDialogState<WorkerRow>();
  const [controlReason, setControlReason] = React.useState("");
  const [pausedUntil, setPausedUntil] = React.useState("");
  const [controlError, setControlError] = React.useState<string | null>(null);
  const policyDialog = useDialogState<WorkerRow>();
  const [policyEnabled, setPolicyEnabled] = React.useState(false);
  const [retentionDays, setRetentionDays] = React.useState("7");
  const [policyError, setPolicyError] = React.useState<string | null>(null);

  const loadError = runtimeQuery.isError || policyQuery.isError || connectorQuery.isError;
  const loading = runtimeQuery.isLoading || policyQuery.isLoading || connectorQuery.isLoading;
  const rows =
    runtimeQuery.data && policyQuery.data && connectorQuery.data
      ? buildRows(runtimeQuery.data, policyQuery.data, connectorQuery.data)
      : [];
  const selected = rows.find((row) => row.id === selectedKeys[0]);
  const toolbarActions: DataGridToolbarAction[] = [
    {
      key: "refresh-worker",
      label: "刷新",
      icon: <RefreshCw className="size-4" aria-hidden />,
      onClick: () => {
        void runtimeQuery.refetch();
        void policyQuery.refetch();
        void connectorQuery.refetch();
      },
    },
    ...(canWrite
      ? [
          {
            key: "claim-control",
            label: selected?.paused ? "恢复认领" : "暂停认领",
            icon: selected?.paused ? (
              <Play className="size-4" aria-hidden />
            ) : (
              <Pause className="size-4" aria-hidden />
            ),
            disabled: (ctx: { selectedRowKeys: string[] }) => ctx.selectedRowKeys.length !== 1,
            onClick: () => {
              if (!selected) return;
              setControlReason("");
              setPausedUntil("");
              setControlError(null);
              controlDialog.openWith(selected);
            },
          } satisfies DataGridToolbarAction,
          {
            key: "payload-retention",
            label: "报文保留",
            icon: <FileKey className="size-4" aria-hidden />,
            disabled: (ctx: { selectedRowKeys: string[] }) =>
              ctx.selectedRowKeys.length !== 1 || policyQuery.isError || policyQuery.isLoading,
            onClick: () => {
              if (!selected) return;
              setPolicyEnabled(selected.payloadEnabled);
              setRetentionDays(String(selected.retentionDays));
              setPolicyError(null);
              policyDialog.openWith(selected);
            },
          } satisfies DataGridToolbarAction,
        ]
      : []),
  ];

  return (
    <>
      <DataGrid
        storageKey="h8.erp-worker-runtime"
        columns={columns}
        data={rows}
        rowKey={(row) => row.id}
        selectable
        selectedRowKeys={selectedKeys}
        onSelectedRowKeysChange={setSelectedKeys}
        toolbarActions={toolbarActions}
        emptyTitle={loadError ? "加载失败" : "暂无 Worker 心跳"}
        emptyDescription={loadError ? "请检查连接后刷新重试" : "Worker 启动并上报心跳后将在此显示"}
        caption={loading ? "加载 Worker 状态..." : undefined}
      />

      <Dialog
        open={controlDialog.open}
        onOpenChange={(open) => !controlMutation.isPending && controlDialog.setOpen(open)}
      >
        <DialogContent className="sm:max-w-md">
          <form
            className="grid gap-3"
            onSubmit={(event) => {
              event.preventDefault();
              // 列表刷新后选中行可能已消失：不得静默 no-op，要在弹窗内给出明确提示。
              const current = rows.find((row) => row.id === controlDialog.target?.id);
              if (!current) {
                setControlError("该 Worker 已离线或列表已刷新，请关闭弹窗刷新后重试");
                return;
              }
              void (async () => {
                try {
                  await controlMutation.mutateAsync({
                    connector_id: current.connectorId,
                    direction: current.direction,
                    paused: !current.paused,
                    reason: controlReason.trim(),
                    paused_until:
                      !current.paused && pausedUntil ? new Date(pausedUntil).toISOString() : null,
                    confirmed: true,
                  });
                  controlDialog.close();
                  onNotice(current.paused ? "已恢复认领" : "已暂停认领");
                } catch (cause: unknown) {
                  setControlError(errorText(cause, "更新认领控制失败，请重试"));
                }
              })();
            }}
          >
            <DialogHeader>
              <DialogTitle>{controlDialog.target?.paused ? "恢复认领" : "暂停认领"}</DialogTitle>
              <DialogDescription>
                按当前连接和方向生效；在途消息继续完成，不改变连接启停状态。
              </DialogDescription>
            </DialogHeader>
            {controlError ? (
              <div
                role="alert"
                className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
              >
                {controlError}
              </div>
            ) : null}
            <label className="grid gap-1 text-sm">
              原因（必填）
              <Input
                required
                value={controlReason}
                onChange={(event) => setControlReason(event.target.value)}
                placeholder="说明操作原因"
              />
            </label>
            {!controlDialog.target?.paused ? (
              <label className="grid gap-1 text-sm">
                自动恢复时间（可选）
                <Input
                  type="datetime-local"
                  value={pausedUntil}
                  onChange={(event) => setPausedUntil(event.target.value)}
                />
              </label>
            ) : null}
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  取消
                </Button>
              </DialogClose>
              <Button type="submit" disabled={controlMutation.isPending || !controlReason.trim()}>
                {controlMutation.isPending ? "提交中…" : "确认"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={policyDialog.open}
        onOpenChange={(open) => !policyMutation.isPending && policyDialog.setOpen(open)}
      >
        <DialogContent className="sm:max-w-md">
          <form
            className="grid gap-3"
            onSubmit={(event) => {
              event.preventDefault();
              // 列表刷新后选中行可能已消失：不得静默 no-op，要在弹窗内给出明确提示。
              const current = rows.find((row) => row.id === policyDialog.target?.id);
              if (!current) {
                setPolicyError("该 Worker 已离线或列表已刷新，请关闭弹窗刷新后重试");
                return;
              }
              void (async () => {
                try {
                  await policyMutation.mutateAsync({
                    connector_id: current.connectorId,
                    enabled: policyEnabled,
                    retention_days: policyEnabled ? Number(retentionDays) : undefined,
                    confirmed: true,
                  });
                  policyDialog.close();
                  onNotice(
                    policyEnabled
                      ? `已启用完整报文保留（${retentionDays} 天）`
                      : "已关闭完整报文保留",
                  );
                } catch (cause: unknown) {
                  setPolicyError(errorText(cause, "更新完整报文保留策略失败，请重试"));
                }
              })();
            }}
          >
            <DialogHeader>
              <DialogTitle>完整报文短期保留</DialogTitle>
              <DialogDescription>
                连接 {policyDialog.target?.connectorCode ?? "—"} 默认关闭；启用后加密保存，最长 30 天。
                关闭会立即清除该连接已有密文。
              </DialogDescription>
            </DialogHeader>
            {policyError ? (
              <div
                role="alert"
                className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
              >
                {policyError}
              </div>
            ) : null}
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={policyEnabled}
                onChange={(event) => setPolicyEnabled(event.target.checked)}
              />
              启用完整报文加密保留
            </label>
            <label className="grid gap-1 text-sm">
              保留天数（1–30）
              <Input
                type="number"
                min={1}
                max={30}
                required
                disabled={!policyEnabled}
                value={retentionDays}
                onChange={(event) => setRetentionDays(event.target.value)}
              />
            </label>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  取消
                </Button>
              </DialogClose>
              <Button
                type="submit"
                disabled={
                  policyMutation.isPending ||
                  (policyEnabled && !(Number(retentionDays) >= 1 && Number(retentionDays) <= 30))
                }
              >
                {policyMutation.isPending ? "保存中…" : "确认保存"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
