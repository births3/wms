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
  ListPageTemplate,
  type DataGridColumn,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { usePageQueryState } from "@/lib/use-page-query-state";
import { useDialogState } from "@/lib/use-dialog-state";
import {
  useConfirmSkipWcsTaskMutation,
  useDeviceDashboardQuery,
  useResendWcsTaskMutation,
  useVoidWcsTaskMutation,
  useWcsTasksQuery,
  type WcsTask,
} from "@/features/device/device-queries";

type Notice = { kind: "success" | "error"; text: string } | null;

export const queryFields: QueryPanelField[] = [
  { key: "status", label: "任务状态", type: "multiSelect", options: [{ label: "全部", value: "" }, { label: "待派发", value: "pending" }, { label: "已下发", value: "sent" }, { label: "执行中", value: "executing" }, { label: "成功", value: "succeeded" }, { label: "失败", value: "failed" }, { label: "超时", value: "timeout" }] },
  { key: "task_type", label: "指令类型", type: "multiSelect", options: [{ label: "全部", value: "" }, { label: "PTL 亮灯", value: "ptl_light_on" }, { label: "PTL 灭灯", value: "ptl_light_off" }, { label: "搬运", value: "pod_move" }, { label: "称重", value: "dws_weigh" }, { label: "RFID 扫描", value: "rfid_scan" }] },
];

export const defaultVisibleFieldKeys = ["status", "task_type"];

type QueryValue = {
  status?: string;
  task_type?: string;
};

const defaultQuery: QueryValue = {};

function normalizeQuery(value: QueryPanelValue): QueryValue {
  return {
    status: typeof value.status === "string" ? value.status : undefined,
    task_type: typeof value.task_type === "string" ? value.task_type : undefined,
  };
}

const STATUS_LABEL: Record<string, string> = {
  pending: "待派发",
  sent: "已下发",
  executing: "执行中",
  succeeded: "成功",
  failed: "失败",
  timeout: "超时",
};

export function M1DeviceDashboardPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryValue>(() => defaultQuery, normalizeQuery);
  const listQuery = useWcsTasksQuery(appliedQuery);
  const dashboardQuery = useDeviceDashboardQuery();
  const resendMutation = useResendWcsTaskMutation();
  const voidMutation = useVoidWcsTaskMutation();
  const confirmSkipMutation = useConfirmSkipWcsTaskMutation();
  const [selected, setSelected] = React.useState<string[]>([]);
  const resendDialog = useDialogState<WcsTask>();
  const voidDialog = useDialogState<WcsTask>();
  const skipDialog = useDialogState<WcsTask>();
  const [reason, setReason] = React.useState("");
  const [notice, setNotice] = React.useState<Notice>(null);

  const rows = listQuery.data ?? [];
  const selectedRow = rows.find((row) => row.id === selected[0]);
  const busy = resendMutation.isPending || voidMutation.isPending || confirmSkipMutation.isPending;
  const summary = dashboardQuery.data;

  const columns: DataGridColumn<WcsTask>[] = [
    { key: "task_no", header: "任务号", width: 180, sortable: true, filterValue: (row) => row.task_no, copyValue: (row) => row.task_no },
    { key: "task_type", header: "指令类型", width: 130 },
    { key: "device_id", header: "设备", width: 150 },
    { key: "status", header: "状态", width: 100, render: (row) => STATUS_LABEL[row.status] ?? row.status },
    { key: "retry_count", header: "重试次数", width: 90, render: (row) => `${row.retry_count}/${row.max_retries}` },
    { key: "error_code", header: "错误码", width: 180 },
    { key: "created_by", header: "触发来源", width: 130 },
  ];

  const toolbarActions: DataGridToolbarAction[] = [
    {
      key: "resend",
      label: "重发",
      description: "人工重发失败/超时指令",
      disabled: (ctx) =>
        ctx.selectedRowKeys.length !== 1 ||
        busy ||
        !["failed", "timeout"].includes(selectedRow?.status ?? ""),
      onClick: () => {
        if (selectedRow) {
          setReason("");
          resendDialog.openWith(selectedRow);
        }
      },
    },
    {
      key: "void",
      label: "作废",
      description: "作废未落账指令",
      disabled: (ctx) =>
        ctx.selectedRowKeys.length !== 1 || busy || selectedRow?.status === "succeeded",
      onClick: () => {
        if (selectedRow) {
          setReason("");
          voidDialog.openWith(selectedRow);
        }
      },
    },
    {
      key: "confirm-skip",
      label: "跳过确认",
      description: "现场已人工完成，凭证据补录账务",
      disabled: (ctx) =>
        ctx.selectedRowKeys.length !== 1 || busy || selectedRow?.status === "succeeded",
      onClick: () => {
        if (selectedRow) {
          setReason("");
          skipDialog.openWith(selectedRow);
        }
      },
    },
  ];

  async function onSubmitResend() {
    const target = resendDialog.target;
    if (!target) return;
    try {
      await resendMutation.mutateAsync({ id: target.id, reason });
      setNotice({ kind: "success", text: "指令已重新派发" });
      resendDialog.close();
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "重发失败" });
    }
  }

  async function onSubmitVoid() {
    const target = voidDialog.target;
    if (!target) return;
    try {
      await voidMutation.mutateAsync({ id: target.id, reason });
      setNotice({ kind: "success", text: "指令已作废" });
      voidDialog.close();
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "作废失败" });
    }
  }

  async function onSubmitSkip() {
    const target = skipDialog.target;
    if (!target) return;
    try {
      await confirmSkipMutation.mutateAsync({ id: target.id, reason });
      setNotice({ kind: "success", text: "已跳过确认并补录账务" });
      skipDialog.close();
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "跳过确认失败" });
    }
  }

  return (
    <ListPageTemplate
      data-testid="m1-device-dashboard-page"
      notice={
        notice ??
        (summary
          ? {
              kind: "success" as const,
              text: `在线 ${summary.online_devices}/${summary.total_devices} · 待作业 ${summary.pending_tasks} · 失败 ${summary.failed_tasks} · 超时 ${summary.timeout_tasks} · 受影响库位 ${summary.affected_location_ids.length}`,
            }
          : null)
      }
      queryFields={queryFields}
      coreQueryFieldKeys={defaultVisibleFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={setDraftQuery}
      onQuery={() => applyQuery(draftQuery)}
      onReset={resetQuery}
      gridProps={{
        storageKey: ["m1", "device-dashboard"].join("."),
        columns,
        data: rows,
        rowKey: (row) => row.id,
        selectable: true,
        selectedRowKeys: selected,
        onSelectedRowKeysChange: setSelected,
        caption: listQuery.isPending ? "加载指令任务..." : undefined,
        emptyTitle: listQuery.isError ? "读取指令任务失败" : "暂无指令任务",
        emptyDescription: listQuery.isError ? "请检查后端服务" : "可等待业务动作生成指令",
        toolbarActions,
      }}
      dialogs={
        <>
          <Dialog open={resendDialog.open} onOpenChange={(open) => !busy && resendDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-md">
              <DialogHeader>
                <DialogTitle>重发指令</DialogTitle>
                <DialogDescription>仅失败/超时任务可重发，重试次数将重置。</DialogDescription>
              </DialogHeader>
              <div className="grid gap-3">
                <label className="text-sm">
                  重发原因
                  <Input value={reason} onChange={(event) => setReason(event.target.value)} className="mt-1" />
                </label>
              </div>
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => resendDialog.close()} disabled={busy}>取消</Button>
                <Button type="button" onClick={() => void onSubmitResend()} disabled={busy || !reason}>重发</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>

          <Dialog open={voidDialog.open} onOpenChange={(open) => !busy && voidDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-md">
              <DialogHeader>
                <DialogTitle>作废指令</DialogTitle>
                <DialogDescription>已落账（succeeded）任务不可作废。</DialogDescription>
              </DialogHeader>
              <div className="grid gap-3">
                <label className="text-sm">
                  作废原因
                  <Input value={reason} onChange={(event) => setReason(event.target.value)} className="mt-1" />
                </label>
              </div>
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => voidDialog.close()} disabled={busy}>取消</Button>
                <Button type="button" variant="destructive" onClick={() => void onSubmitVoid()} disabled={busy || !reason}>作废</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>

          <Dialog open={skipDialog.open} onOpenChange={(open) => !busy && skipDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-md">
              <DialogHeader>
                <DialogTitle>跳过确认</DialogTitle>
                <DialogDescription>现场已人工完成时凭证据补录账务，已成功任务不可再确认。</DialogDescription>
              </DialogHeader>
              <div className="grid gap-3">
                <label className="text-sm">
                  确认原因
                  <Input value={reason} onChange={(event) => setReason(event.target.value)} className="mt-1" />
                </label>
              </div>
              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => skipDialog.close()} disabled={busy}>取消</Button>
                <Button type="button" onClick={() => void onSubmitSkip()} disabled={busy || !reason}>跳过确认</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </>
      }
    />
  );
}
