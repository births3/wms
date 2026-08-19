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
  ListPageTemplate,
  StatusBadge,
  type DataGridColumn,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import {
  useCancelReplenishmentTaskMutation,
  useCreateReplenishmentTaskMutation,
  useReassignReplenishmentTaskMutation,
  useReplenishmentTasksQuery,
  type ReplenishmentTask,
} from "@/features/replenishment/replenishment-task-queries";
import { errorText } from "@/lib/error-text";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_REFRESH,
  BUTTON_SAVE,
  COLUMN_STATUS,
  ERROR_AUTH_API_CHECK,
  FILTER_ALL,
  LOADING_PROCESSING,
  LOADING_SAVING,
} from "@/lib/ui-strings";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";
import { isUuid } from "@/lib/uuid";

export const queryFields: QueryPanelField[] = [
  { key: "status", label: COLUMN_STATUS, type: "multiSelect", options: [{ label: FILTER_ALL, value: "" }, { label: "待领取", value: "pending" }, { label: "作业中", value: "in_progress" }, { label: "挂起", value: "suspended" }, { label: "完成", value: "done" }, { label: "取消", value: "cancelled" }] },
  { key: "trigger_mode", label: "触发模式", type: "multiSelect", options: [{ label: FILTER_ALL, value: "" }, { label: "Min-Max", value: "min_max" }, { label: "波次缺口", value: "wave_gap" }, { label: "手工", value: "manual" }] },
  { key: "priority", label: "优先级", type: "multiSelect", options: [{ label: FILTER_ALL, value: "" }, { label: "紧急", value: "urgent" }, { label: "普通", value: "normal" }] },
  { key: "location", label: "库位", type: "text", placeholder: "来源/目标库位 ID" },
  { key: "operator", label: "作业员", type: "text", placeholder: "作业员 ID" },
  { key: "owner", label: "货主", type: "text", placeholder: "货主 ID" },
];
export const defaultVisibleFieldKeys = ["status", "trigger_mode"];

type CreateForm = { sourceLocationId: string; sourceBatchId: string; targetLocationId: string; qty: string };
type Notice = { kind: "success" | "error"; text: string } | null;

const columns: DataGridColumn<ReplenishmentTask>[] = [
  { key: "task_no", header: "任务号", width: 160, mono: true, sortable: true, filterValue: (row) => row.task_no, copyValue: (row) => row.task_no, filter: { type: "text" } },
  { key: "trigger_mode", header: "触发模式", width: 110, filterValue: (row) => row.trigger_mode, copyValue: (row) => row.trigger_mode },
  { key: "priority", header: "优先级", width: 120, render: (row) => isTimeoutRow(row) ? "紧急 · 超时" : row.priority === "urgent" ? "紧急" : "普通", filterValue: (row) => row.priority },
  { key: "source_location_id", header: "来源位", width: 160, mono: true, filterValue: (row) => row.source_location_id, copyValue: (row) => row.source_location_id },
  { key: "target_location_id", header: "目标位", width: 160, mono: true, filterValue: (row) => row.target_location_id, copyValue: (row) => row.target_location_id },
  { key: "qty", header: "数量/已送达", width: 130, mono: true, render: (row) => `${row.done_qty} / ${row.qty}`, copyValue: (row) => `${row.done_qty}/${row.qty}` },
  { key: "status", header: COLUMN_STATUS, width: 100, render: (row) => <StatusBadge status={statusTone(row)} label={statusLabel(row.status)} size="sm" />, filterValue: (row) => row.status },
  { key: "operator_id", header: "作业员", width: 140, mono: true, render: (row) => row.operator_id ?? "—", filterValue: (row) => row.operator_id ?? "" },
  { key: "created_by", header: "创建/更新", width: 160, filterValue: (row) => row.created_by, copyValue: (row) => row.created_by },
];

export function M3ReplenishmentTaskPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const listQuery = useReplenishmentTasksQuery(toTaskFilters(appliedQuery));
  const createMutation = useCreateReplenishmentTaskMutation();
  const cancelMutation = useCancelReplenishmentTaskMutation();
  const reassignMutation = useReassignReplenishmentTaskMutation();
  const [selected, setSelected] = React.useState<string[]>([]);
  const [form, setForm] = React.useState<CreateForm>(emptyForm());
  const [cancelReason, setCancelReason] = React.useState("supervisor_cancel");
  const createDialog = useDialogState<null>();
  const cancelDialog = useDialogState<ReplenishmentTask>();
  const reassignDialog = useDialogState<ReplenishmentTask>();
  const [notice, setNotice] = React.useState<Notice>(null);
  const rows = listQuery.data?.data ?? [];
  const selectedRow = rows.find((row) => row.id === selected[0]);
  const busy = createMutation.isPending || cancelMutation.isPending || reassignMutation.isPending;
  const refreshAction: DataGridRefreshAction = { label: BUTTON_REFRESH, description: "刷新补货任务", disabled: listQuery.isFetching, onClick: () => void listQuery.refetch() };
  const toolbarActions: DataGridToolbarAction[] = [
    { key: "create", label: "手动发起", description: "手工发起补货任务", disabled: busy, onClick: () => { setForm(emptyForm()); createDialog.openWith(null); } },
    { key: "reassign", label: "重派", description: "将任务改派回池", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: () => selectedRow && reassignDialog.openWith(selectedRow) },
    { key: "cancel", label: "取消", description: "取消未下架任务", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: () => selectedRow && cancelDialog.openWith(selectedRow) },
  ];

  return (
    <ListPageTemplate
      data-testid="m3-replenishment-task-page"
      notice={notice}
      queryFields={queryFields}
      coreQueryFieldKeys={defaultVisibleFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={setDraftQuery}
      onQuery={() => applyQuery(draftQuery)}
      onReset={resetQuery}
      gridProps={{
        storageKey: ["m3", "replenishment-tasks"].join("."),
        columns,
        data: rows,
        rowKey: (row) => row.id,
        selectable: true,
        selectedRowKeys: selected,
        onSelectedRowKeysChange: setSelected,
        caption: listQuery.isPending ? "加载补货任务..." : undefined,
        emptyTitle: listQuery.isError ? "读取补货任务失败" : "暂无补货任务",
        emptyDescription: listQuery.isError ? errorText(listQuery.error, ERROR_AUTH_API_CHECK) : "可手工发起或等待巡检/波次生成",
        exportFileBaseName: "M3-replenishment-tasks",
        refreshAction,
        toolbarActions,
        queryState: appliedQuery,
        onApplyQueryState: (value) => applyQuery(queryValueFromUnknown(value)),
        onClearQueryState: resetQuery,
      }}
      dialogs={
        <>
          <Dialog open={createDialog.open} onOpenChange={(open) => !busy && createDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-lg">
              <form className="grid gap-4" onSubmit={submitCreate}>
                <DialogHeader>
                  <DialogTitle>手动发起</DialogTitle>
                  <DialogDescription>手工发起一条补货任务，走同一生成与预占。</DialogDescription>
                </DialogHeader>
                <Field label="来源库位 ID"><Input required value={form.sourceLocationId} onChange={(event) => setForm((current) => ({ ...current, sourceLocationId: event.target.value }))} /></Field>
                <Field label="来源批次 ID"><Input required value={form.sourceBatchId} onChange={(event) => setForm((current) => ({ ...current, sourceBatchId: event.target.value }))} /></Field>
                <Field label="目标库位 ID"><Input required value={form.targetLocationId} onChange={(event) => setForm((current) => ({ ...current, targetLocationId: event.target.value }))} /></Field>
                <Field label="数量"><Input required value={form.qty} onChange={(event) => setForm((current) => ({ ...current, qty: event.target.value }))} /></Field>
                <DialogFooter>
                  <DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose>
                  <Button type="submit" disabled={busy}>{busy ? LOADING_SAVING : BUTTON_SAVE}</Button>
                </DialogFooter>
              </form>
            </DialogContent>
          </Dialog>
          <Dialog open={reassignDialog.open} onOpenChange={(open) => !busy && reassignDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-md">
              <DialogHeader>
                <DialogTitle>重派补货任务</DialogTitle>
                <DialogDescription>确认将任务 {reassignDialog.target?.task_no} 改派回池？</DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose>
                <Button type="button" disabled={busy || !reassignDialog.target} onClick={() => void confirmReassign()}>{busy ? LOADING_PROCESSING : "确认重派"}</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
          <Dialog open={cancelDialog.open} onOpenChange={(open) => !busy && cancelDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-md">
              <DialogHeader>
                <DialogTitle>取消补货任务</DialogTitle>
                <DialogDescription>确认取消任务 {cancelDialog.target?.task_no}？已下架任务不可取消。</DialogDescription>
              </DialogHeader>
              <Field label="原因"><Input value={cancelReason} onChange={(event) => setCancelReason(event.target.value)} /></Field>
              <DialogFooter>
                <DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose>
                <Button type="button" disabled={busy || !cancelDialog.target} onClick={() => void confirmCancel()}>{busy ? LOADING_PROCESSING : "确认取消"}</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </>
      }
    />
  );

  async function submitCreate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (![form.sourceLocationId, form.sourceBatchId, form.targetLocationId].every((value) => isUuid(value.trim()))) {
      setNotice({ kind: "error", text: "来源库位、批次和目标库位必须是合法 UUID" });
      return;
    }
    try {
      const saved = await createMutation.mutateAsync({
        source_location_id: form.sourceLocationId.trim(),
        source_batch_id: form.sourceBatchId.trim(),
        target_location_id: form.targetLocationId.trim(),
        qty: form.qty.trim(),
      });
      createDialog.setOpen(false);
      setSelected([]);
      setNotice({ kind: "success", text: `任务 ${saved.task_no} 已发起` });
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "手工发起补货任务失败") });
    }
  }
  async function confirmReassign() {
    const row = reassignDialog.target;
    if (!row) return;
    try {
      await reassignMutation.mutateAsync({ id: row.id, version: row.version });
      reassignDialog.setOpen(false);
      setSelected([]);
      setNotice({ kind: "success", text: `任务 ${row.task_no} 已改派` });
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "改派补货任务失败") });
    }
  }
  async function confirmCancel() {
    const row = cancelDialog.target;
    if (!row) return;
    try {
      await cancelMutation.mutateAsync({ id: row.id, version: row.version, reason: cancelReason.trim() || "supervisor_cancel" });
      cancelDialog.setOpen(false);
      setSelected([]);
      setNotice({ kind: "success", text: `任务 ${row.task_no} 已取消` });
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "取消补货任务失败") });
    }
  }
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="grid gap-1 text-sm">{label}{children}</label>;
}
function defaultQuery(): QueryPanelValue { return { status: "", trigger_mode: "", priority: "", location: "", operator: "", owner: "" }; }
function normalizeQuery(value: QueryPanelValue): QueryPanelValue {
  return { status: queryString(value.status), trigger_mode: queryString(value.trigger_mode), priority: queryString(value.priority), location: queryString(value.location), operator: queryString(value.operator), owner: queryString(value.owner) };
}
function emptyForm(): CreateForm { return { sourceLocationId: "", sourceBatchId: "", targetLocationId: "", qty: "1" }; }
function statusLabel(status: string) {
  if (status === "pending") return "待领取";
  if (status === "in_progress") return "作业中";
  if (status === "suspended") return "挂起";
  if (status === "done") return "完成";
  if (status === "cancelled") return "取消";
  return status;
}
function statusTone(row: ReplenishmentTask): "pending" | "completed" | "isolated" | "near_expiry" {
  if (row.status === "done") return "completed";
  if (row.status === "cancelled" || row.status === "suspended") return "isolated";
  if (isTimeoutRow(row)) return "near_expiry";
  return "pending";
}
function isTimeoutRow(row: ReplenishmentTask) {
  const now = Date.now();
  const createdAt = Date.parse(row.created_at ?? "");
  const lastProgressAt = Date.parse(row.last_progress_at ?? "");
  if (row.priority === "urgent" && row.status === "pending" && Number.isFinite(createdAt)) {
    return now - createdAt >= 10 * 60 * 1000;
  }
  if (row.status === "in_progress" && Number.isFinite(lastProgressAt)) {
    return now - lastProgressAt >= 60 * 60 * 1000;
  }
  return false;
}
function toTaskFilters(query: QueryPanelValue) {
  return {
    status: queryString(query.status),
    trigger_mode: queryString(query.trigger_mode),
    priority: queryString(query.priority),
    location_id: queryString(query.location),
    operator_id: queryString(query.operator),
    keyword: queryString(query.keyword),
  };
}
