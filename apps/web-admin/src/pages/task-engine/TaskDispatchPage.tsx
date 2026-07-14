import * as React from "react";
import {
  Button, Card, CardContent, DataGrid, Dialog, DialogClose, DialogContent, DialogDescription,
  DialogFooter, DialogHeader, DialogTitle, PageHeader, QueryPanel, Select, SelectContent,
  SelectItem, SelectTrigger, SelectValue, StatusBadge, buildQueryPanelSummaryItems,
  type DataGridColumn, type DataGridRefreshAction, type DataGridToolbarAction,
  type QueryPanelField, type QueryPanelValue,
} from "@wms/ui";
import { CheckCircle2, RotateCcw, Send, UserPlus, XCircle, Zap } from "lucide-react";

import {
  useTaskGroupsQuery, useTaskWorkersQuery, useTransitionWarehouseTaskMutation, useWarehouseTasksQuery,
  type TaskTransitionAction, type WarehouseTask,
} from "@/features/task-engine/task-engine-queries";

const statusLabels: Record<string, string> = { pending_release: "待释放", pending_assignment: "待分配", assigned: "已分配", dispatched: "已下发", in_progress: "执行中", completed: "已完成", exception: "异常", cancelled: "已取消" };

export const mteTaskDispatchQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "任务号 / 来源单号 / 商品" },
  { key: "status", label: "状态", type: "select", options: [{ label: "全部", value: "" }, ...Object.entries(statusLabels).map(([value, label]) => ({ label, value }))] },
  { key: "taskTypeCode", label: "任务类型", type: "text", placeholder: "pick / putaway / loading" },
  { key: "warehouseId", label: "仓库 ID", type: "text", placeholder: "warehouse_id" },
];
export const mteTaskDispatchCoreQueryFieldKeys = ["keyword", "status"];

type Notice = { kind: "success" | "error"; text: string } | null;

/**
 * 页面设计契约：列表型调度页；主信息载体为 QueryPanel + DataGrid；刷新与分派/下发/召回/处置动作放在 DataGrid；
 * 人员选择通过 Dialog 完成；不常驻任务详情、轨迹、执行表单或审计明细。
 */
export function TaskDispatchPage() {
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(defaultQuery);
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(defaultQuery);
  const filters = { status: text(appliedQuery.status), taskTypeCode: text(appliedQuery.taskTypeCode), warehouseId: text(appliedQuery.warehouseId) };
  const tasksQuery = useWarehouseTasksQuery(filters);
  const groupsQuery = useTaskGroupsQuery();
  const usersQuery = useTaskWorkersQuery();
  const transition = useTransitionWarehouseTaskMutation();
  const [selected, setSelected] = React.useState<string[]>([]);
  const [assignOpen, setAssignOpen] = React.useState(false);
  const [confirmOpen, setConfirmOpen] = React.useState(false);
  const [confirmAction, setConfirmAction] = React.useState<{ task: WarehouseTask; action: TaskTransitionAction } | null>(null);
  const [assigneeId, setAssigneeId] = React.useState("");
  const [assignAction, setAssignAction] = React.useState<TaskTransitionAction>("assign");
  const [notice, setNotice] = React.useState<Notice>(null);
  const keyword = text(appliedQuery.keyword).trim().toLowerCase();
  const rows = (tasksQuery.data?.data ?? []).filter((row) => !keyword || `${row.task_no} ${row.source_doc_no} ${row.product_code}`.toLowerCase().includes(keyword));
  const selectedTask = rows.find((row) => row.id === selected[0]);
  const selectedGroup = (groupsQuery.data?.data ?? []).find((group) => group.task_group_code === selectedTask?.task_group_code);
  const qualifiedUsers = (usersQuery.data?.data ?? []).filter((user) => selectedGroup?.member_user_ids.includes(user.user_id));
  const workerNames = new Map((usersQuery.data?.data ?? []).map((user) => [user.user_id, user.display_name]));
  const busy = transition.isPending;
  const refreshAction: DataGridRefreshAction = { label: "刷新", description: "刷新统一任务队列", disabled: tasksQuery.isFetching, onClick: () => void tasksQuery.refetch() };
  const toolbarActions: DataGridToolbarAction[] = [
    { key: "auto-assign", label: "自动分派", description: "按任务组资格和当前负荷选择执行人", icon: <UserPlus className="size-4" aria-hidden />, disabled: () => !selectedTask || selectedTask.status !== "pending_assignment" || busy, onClick: () => { if (selectedTask) { setConfirmAction({ task: selectedTask, action: "assign" }); setConfirmOpen(true); } } },
    { key: "assign", label: "分派", description: "把待分配任务指派给合格成员", icon: <UserPlus className="size-4" aria-hidden />, disabled: () => !selectedTask || selectedTask.status !== "pending_assignment" || busy, onClick: () => openAssign("assign") },
    { key: "reassign", label: "改派", description: "重新指派已分配或已下发任务", icon: <RotateCcw className="size-4" aria-hidden />, disabled: () => !selectedTask || !["assigned", "dispatched"].includes(selectedTask.status) || busy, onClick: () => openAssign("reassign") },
    { key: "dispatch", label: "下发", description: "下发到保管员 PDA 待办", icon: <Send className="size-4" aria-hidden />, disabled: () => !selectedTask || selectedTask.status !== "assigned" || busy, onClick: () => { if (selectedTask) { setConfirmAction({ task: selectedTask, action: "dispatch" }); setConfirmOpen(true); } } },
    { key: "recall", label: "召回", description: "从已分配或已下发状态召回待分配池", icon: <RotateCcw className="size-4" aria-hidden />, disabled: () => !selectedTask || !["assigned", "dispatched"].includes(selectedTask.status) || busy, onClick: () => { if (selectedTask) { setConfirmAction({ task: selectedTask, action: "recall" }); setConfirmOpen(true); } } },
    { key: "expedite", label: "手动加急", description: "按优先级规则一次性提升任务优先级", icon: <Zap className="size-4" aria-hidden />, disabled: () => !selectedTask || selectedTask.manually_expedited || ["completed", "cancelled"].includes(selectedTask.status) || busy, onClick: () => { if (selectedTask) { setConfirmAction({ task: selectedTask, action: "expedite" }); setConfirmOpen(true); } } },
    { key: "resolve", label: "处置完成", description: "主管确认异常任务完成", icon: <CheckCircle2 className="size-4" aria-hidden />, disabled: () => !selectedTask || selectedTask.status !== "exception" || busy, onClick: () => { if (selectedTask) { setConfirmAction({ task: selectedTask, action: "resolve_complete" }); setConfirmOpen(true); } } },
    { key: "cancel", label: "取消", description: "取消待分配或异常任务", icon: <XCircle className="size-4" aria-hidden />, variant: "destructive", disabled: () => !selectedTask || !["pending_assignment", "exception"].includes(selectedTask.status) || busy, onClick: () => { if (selectedTask) { setConfirmAction({ task: selectedTask, action: "cancel" }); setConfirmOpen(true); } } },
  ];

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader title="M-TE 任务调度" subtitle="统一查看、分派、下发和处置仓内物理任务" />
    {notice && <div className={notice.kind === "error" ? "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" : "rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success"} role={notice.kind === "error" ? "alert" : "status"}>{notice.text}</div>}
    <QueryPanel fields={mteTaskDispatchQueryFields} defaultVisibleFieldKeys={mteTaskDispatchCoreQueryFieldKeys} value={draftQuery} onValueChange={setDraftQuery} onQuery={() => setAppliedQuery(draftQuery)} onReset={() => { const next = defaultQuery(); setDraftQuery(next); setAppliedQuery(next); }} />
    <Card><CardContent className="p-5"><DataGrid storageKey="mte.task-dispatch" columns={taskColumns(workerNames)} data={rows} rowKey={(row) => row.id} selectable selectedRowKeys={selected} onSelectedRowKeysChange={setSelected} caption={tasksQuery.isPending ? "加载任务队列..." : undefined} emptyTitle={tasksQuery.isError ? "读取任务队列失败" : "暂无任务"} emptyDescription={tasksQuery.isError ? errorText(tasksQuery.error, "请检查鉴权和 API 服务") : "业务事件触发后任务会进入待分配池"} refreshAction={refreshAction} toolbarActions={toolbarActions} exportFileBaseName="M-TE-tasks" queryState={appliedQuery} querySummaryItems={buildQueryPanelSummaryItems(mteTaskDispatchQueryFields, appliedQuery)} onApplyQueryState={(value) => { const next = normalizeQuery(value); setDraftQuery(next); setAppliedQuery(next); }} onClearQueryState={() => { const next = defaultQuery(); setDraftQuery(next); setAppliedQuery(next); }} /></CardContent></Card>
    <Dialog open={assignOpen} onOpenChange={(open) => !busy && setAssignOpen(open)}><DialogContent className="sm:max-w-md"><DialogHeader><DialogTitle>{assignAction === "assign" ? "分派任务" : "改派任务"}</DialogTitle><DialogDescription>仅展示任务组“{selectedTask?.task_group_code ?? ""}”内的有效成员。</DialogDescription></DialogHeader><label className="grid gap-1 text-sm">执行人<Select value={assigneeId} onValueChange={setAssigneeId}><SelectTrigger><SelectValue placeholder="请选择合格成员" /></SelectTrigger><SelectContent>{qualifiedUsers.map((user) => <SelectItem key={user.user_id} value={user.user_id}>{user.display_name}（{user.username}）</SelectItem>)}</SelectContent></Select></label>{qualifiedUsers.length === 0 && <div className="rounded-md border border-wms-warning/30 bg-wms-warning/10 px-3 py-2 text-sm text-wms-warning" role="status">该任务组尚未配置成员，请先在“任务组与人员资格”页面维护。</div>}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose><Button type="button" disabled={busy || !selectedTask || !assigneeId} onClick={() => selectedTask && void mutate(selectedTask, assignAction, assigneeId)}>{busy ? "处理中..." : "确认分派"}</Button></DialogFooter></DialogContent></Dialog>
    <Dialog open={confirmOpen} onOpenChange={(open) => !busy && setConfirmOpen(open)}><DialogContent className="sm:max-w-md"><DialogHeader><DialogTitle>确认任务操作</DialogTitle><DialogDescription>{confirmAction ? `确认对任务 ${confirmAction.task.task_no} 执行“${actionLabels[confirmAction.action]}”？此操作会立即更新任务并写入审计。` : "请确认任务操作。"}</DialogDescription></DialogHeader><DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose><Button type="button" variant={confirmAction?.action === "cancel" ? "destructive" : "default"} disabled={busy || !confirmAction} onClick={() => confirmAction && void mutate(confirmAction.task, confirmAction.action)}>{busy ? "处理中..." : "确认执行"}</Button></DialogFooter></DialogContent></Dialog>
  </section>;

  function openAssign(action: TaskTransitionAction) { setAssignAction(action); setAssigneeId(""); setAssignOpen(true); setNotice(null); }
  async function mutate(task: WarehouseTask, action: TaskTransitionAction, assigneeUserId?: string) {
    setNotice(null);
    try {
      const result = await transition.mutateAsync({ taskId: task.id, body: { action, assignee_user_id: assigneeUserId } });
      setAssignOpen(false); setConfirmOpen(false); setConfirmAction(null); setSelected([result.id]); setNotice({ kind: "success", text: action === "expedite" ? `任务 ${result.task_no} 已手动加急` : `任务 ${result.task_no} 已更新为${statusLabels[result.status] ?? result.status}` });
    } catch (error) { setNotice({ kind: "error", text: errorText(error, "任务状态更新失败") }); }
  }
}

const actionLabels: Record<TaskTransitionAction, string> = { assign: "自动分派", dispatch: "下发", reassign: "改派", recall: "召回", start: "开始", complete: "完成", report_exception: "上报异常", resolve_complete: "处置完成", cancel: "取消", expedite: "手动加急" };

function taskColumns(workerNames: ReadonlyMap<string, string>): DataGridColumn<WarehouseTask>[] { return [
  { key: "task_no", header: "任务号", width: 190, minWidth: 160, mono: true, sortable: true, sortValue: (row) => row.task_no, filterValue: (row) => row.task_no, copyValue: (row) => row.task_no, filter: { type: "text" } },
  { key: "status", header: "状态", width: 110, minWidth: 95, render: (row) => <StatusBadge status={statusTone(row.status)} label={statusLabels[row.status] ?? row.status} size="sm" /> },
  { key: "task_type_code", header: "任务类型", width: 120, minWidth: 100, mono: true, sortable: true, sortValue: (row) => row.task_type_code, filterValue: (row) => row.task_type_code, copyValue: (row) => row.task_type_code, filter: { type: "text" } },
  { key: "priority", header: "优先级", width: 95, minWidth: 80, sortable: true, sortValue: (row) => row.priority, render: (row) => String(row.priority) },
  { key: "priority_factors", header: "优先因素", width: 150, minWidth: 120, render: (row) => [row.urgent_order && "订单加急", row.cold_chain && "冷链", row.manually_expedited && "手动加急"].filter(Boolean).join(" / ") || "默认" },
  { key: "source_doc_no", header: "来源单号", width: 180, minWidth: 140, mono: true, filterValue: (row) => row.source_doc_no, copyValue: (row) => row.source_doc_no, filter: { type: "text" } },
  { key: "product_code", header: "商品编码", width: 150, minWidth: 120, mono: true, filterValue: (row) => row.product_code, copyValue: (row) => row.product_code, filter: { type: "text" } },
  { key: "planned_qty", header: "计划数量", width: 105, minWidth: 90, sortable: true, sortValue: (row) => row.planned_qty, render: (row) => String(row.planned_qty) },
  { key: "route", header: "源 → 目标", width: 210, minWidth: 160, render: (row) => `${row.source_location_code ?? "—"} → ${row.target_location_code ?? "—"}` },
  { key: "task_group_code", header: "任务组", width: 170, minWidth: 130, mono: true, copyValue: (row) => row.task_group_code },
  { key: "assignee_user_id", header: "执行人", width: 190, minWidth: 150, render: (row) => row.assignee_user_id ? workerNames.get(row.assignee_user_id) ?? row.assignee_user_id : "待分配", copyValue: (row) => row.assignee_user_id ?? "" },
  { key: "created_at", header: "创建时间", width: 180, minWidth: 150, sortable: true, sortValue: (row) => row.created_at, render: (row) => formatDateTime(row.created_at) },
]; }

function statusTone(status: string): "completed" | "isolated" | "pending" { if (status === "completed") return "completed"; if (status === "exception" || status === "cancelled") return "isolated"; return "pending"; }
function defaultQuery(): QueryPanelValue { return { keyword: "", status: "", taskTypeCode: "", warehouseId: "" }; }
function text(value: unknown) { return typeof value === "string" ? value : ""; }
function normalizeQuery(value: unknown): QueryPanelValue { const row = value && typeof value === "object" ? value as Record<string, unknown> : {}; return { keyword: text(row.keyword), status: text(row.status), taskTypeCode: text(row.taskTypeCode), warehouseId: text(row.warehouseId) }; }
function errorText(error: unknown, fallback: string) { return error instanceof Error ? error.message : fallback; }
function formatDateTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false }); }
