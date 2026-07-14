import * as React from "react";
import {
  Button, Card, CardContent, DataGrid, Dialog, DialogClose, DialogContent, DialogDescription,
  DialogFooter, DialogHeader, DialogTitle, Input, PageHeader, QueryPanel, StatusBadge,
  buildQueryPanelSummaryItems, type DataGridColumn, type DataGridCreateAction,
  type DataGridDisableAction, type DataGridEditAction, type DataGridRefreshAction,
  type QueryPanelField, type QueryPanelValue,
} from "@wms/ui";
import {
  useSetTaskTypeEnabledMutation, useTaskTypesQuery, useUpsertTaskTypeMutation,
  type TaskType, type UpsertTaskTypeRequest,
} from "@/features/task-engine/task-type-queries";

const queryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "编码 / 名称" },
  { key: "status", label: "状态", type: "select", options: [{ label: "全部", value: "" }, { label: "启用", value: "enabled" }, { label: "停用", value: "disabled" }] },
];
const mteTaskTypeCoreQueryFieldKeys = ["keyword", "status"];
const statusOptions = [{ label: "启用", value: "enabled" }, { label: "停用", value: "disabled" }];

type Form = { code: string; name: string; priority: string; minutes: string; mergeable: boolean; insertable: boolean; enabled: boolean };
type Notice = { kind: "success" | "error"; text: string } | null;

const columns: DataGridColumn<TaskType>[] = [
  { key: "task_type_code", header: "类型编码", width: 170, minWidth: 140, mono: true, sortable: true, sortValue: (r) => r.task_type_code, filterValue: (r) => r.task_type_code, copyValue: (r) => r.task_type_code, filter: { type: "text" } },
  { key: "task_type_name", header: "类型名称", width: 160, minWidth: 120, sortable: true, sortValue: (r) => r.task_type_name, filterValue: (r) => r.task_type_name, copyValue: (r) => r.task_type_name, filter: { type: "text" } },
  { key: "default_priority", header: "默认优先级", width: 120, minWidth: 100, sortable: true, sortValue: (r) => r.default_priority, filterValue: (r) => r.default_priority, copyValue: (r) => String(r.default_priority), filter: { type: "numberRange" } },
  { key: "estimated_minutes", header: "预计耗时（分）", width: 140, minWidth: 120, sortable: true, sortValue: (r) => r.estimated_minutes, filterValue: (r) => r.estimated_minutes, copyValue: (r) => String(r.estimated_minutes), filter: { type: "numberRange" } },
  { key: "mergeable", header: "可合并", width: 100, minWidth: 90, render: (r) => r.mergeable ? "是" : "否" },
  { key: "insertable", header: "可插单", width: 100, minWidth: 90, render: (r) => r.insertable ? "是" : "否" },
  { key: "enabled", header: "状态", width: 110, minWidth: 90, render: (r) => <StatusBadge status={r.enabled ? "completed" : "isolated"} label={r.enabled ? "启用" : "停用"} size="sm" /> },
  { key: "created_at", header: "创建时间", width: 175, minWidth: 145, sortable: true, sortValue: (r) => r.created_at, filterValue: (r) => r.created_at, copyValue: (r) => r.created_at, filter: { type: "dateRange" }, render: (r) => formatDateTime(r.created_at) },
];

export function TaskTypeConfigPage() {
  const query = useTaskTypesQuery();
  const save = useUpsertTaskTypeMutation();
  const toggle = useSetTaskTypeEnabledMutation();
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(defaultQuery);
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(defaultQuery);
  const [selected, setSelected] = React.useState<string[]>([]);
  const [dialogOpen, setDialogOpen] = React.useState(false);
  const [editing, setEditing] = React.useState<TaskType | null>(null);
  const [toggleTarget, setToggleTarget] = React.useState<TaskType | null>(null);
  const [toggleOpen, setToggleOpen] = React.useState(false);
  const [form, setForm] = React.useState<Form>(emptyForm);
  const [notice, setNotice] = React.useState<Notice>(null);
  const rows = (query.data?.data ?? []).filter((row) => {
    const keyword = stringValue(appliedQuery.keyword).trim().toLowerCase();
    const status = stringValue(appliedQuery.status);
    return (!keyword || `${row.task_type_code} ${row.task_type_name}`.toLowerCase().includes(keyword)) && (!status || (status === "enabled" ? row.enabled : !row.enabled));
  });
  const busy = save.isPending || toggle.isPending;
  const selectedRow = rows.find((row) => row.task_type_code === selected[0]);

  const refreshAction: DataGridRefreshAction = { label: "刷新", description: "刷新任务类型列表", disabled: query.isFetching, onClick: () => void query.refetch() };
  const createAction: DataGridCreateAction = { label: "新增类型", description: "新增自定义任务类型", disabled: busy, onClick: () => openDialog(null) };
  const editAction: DataGridEditAction = { label: "编辑", description: "编辑选中任务类型", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: (ctx) => openDialog(rows.find((r) => r.task_type_code === ctx.selectedRowKeys[0]) ?? null) };
  const disableAction: DataGridDisableAction = { label: selectedRow?.enabled ? "停用" : "启用", description: "切换任务类型状态", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: () => { if (selectedRow) { setToggleTarget(selectedRow); setToggleOpen(true); } } };

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader title="M-TE 任务类型配置" subtitle="维护预置与自定义任务类型的调度参数" />
    {notice && <div className={notice.kind === "error" ? "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" : "rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success"} role={notice.kind === "error" ? "alert" : "status"}>{notice.text}</div>}
    <QueryPanel fields={queryFields} defaultVisibleFieldKeys={mteTaskTypeCoreQueryFieldKeys} value={draftQuery} onValueChange={setDraftQuery} onQuery={() => setAppliedQuery(draftQuery)} onReset={() => { const next = defaultQuery(); setDraftQuery(next); setAppliedQuery(next); }} />
    <Card><CardContent className="p-5"><DataGrid storageKey="mte.task-types" columns={columns} data={rows} rowKey={(r) => r.task_type_code} selectable selectedRowKeys={selected} onSelectedRowKeysChange={setSelected} caption={query.isPending ? "加载任务类型..." : undefined} emptyTitle={query.isError ? "读取任务类型失败" : "暂无任务类型"} emptyDescription={query.isError ? errorText(query.error, "请检查鉴权和 API 服务") : "暂无可用任务类型"} exportFileBaseName="M-TE-task-types" refreshAction={refreshAction} createAction={createAction} editAction={editAction} disableAction={disableAction} queryState={appliedQuery} querySummaryItems={buildQueryPanelSummaryItems(queryFields, appliedQuery)} onApplyQueryState={(value) => { const next = normalizeQuery(value); setDraftQuery(next); setAppliedQuery(next); }} onClearQueryState={() => { const next = defaultQuery(); setDraftQuery(next); setAppliedQuery(next); }} /></CardContent></Card>
    {/* TaskTypeDisableDialog: 启停写操作统一经过确认。 */}<Dialog open={toggleOpen} onOpenChange={(open) => !busy && (setToggleOpen(open), !open && setToggleTarget(null))}><DialogContent className="sm:max-w-md"><DialogHeader><DialogTitle>{toggleTarget?.enabled ? "停用任务类型" : "启用任务类型"}</DialogTitle><DialogDescription>确认{toggleTarget?.enabled ? "停用" : "启用"}任务类型“{toggleTarget?.task_type_name ?? ""}”？</DialogDescription></DialogHeader><DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose><Button type="button" disabled={busy || !toggleTarget} onClick={() => void confirmToggle()}>{busy ? "处理中..." : "确认"}</Button></DialogFooter></DialogContent></Dialog>
    <TaskTypeDialog open={dialogOpen} editing={editing} form={form} pending={save.isPending} errorMessage={save.error?.message} onOpenChange={setDialogOpen} onFormChange={setForm} onSubmit={submit} />
  </section>;

  function openDialog(row: TaskType | null) { setNotice(null); setEditing(row); setForm(row ? formFor(row) : emptyForm()); setDialogOpen(true); }
  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validate(form, Boolean(editing));
    if (error) { setNotice({ kind: "error", text: error }); return; }
    const body: UpsertTaskTypeRequest = { task_type_name: form.name.trim(), default_priority: Number(form.priority), estimated_minutes: Number(form.minutes), mergeable: form.mergeable, insertable: form.insertable, enabled: form.enabled };
    try { await save.mutateAsync({ code: form.code.trim().toLowerCase(), body }); setDialogOpen(false); setSelected([]); setNotice({ kind: "success", text: `任务类型 ${form.code.trim().toLowerCase()} 已保存` }); } catch { /* 弹窗保留，允许重试。 */ }
  }
  async function setEnabled(row: TaskType) { setNotice(null); try { await toggle.mutateAsync({ code: row.task_type_code, enabled: !row.enabled }); setSelected([]); setNotice({ kind: "success", text: `${row.task_type_name} 已${row.enabled ? "停用" : "启用"}` }); } catch (error) { setNotice({ kind: "error", text: errorText(error, "更新任务类型状态失败") }); } }
  async function confirmToggle() { if (!toggleTarget) return; const row = toggleTarget; try { await setEnabled(row); setToggleOpen(false); setToggleTarget(null); } catch { /* setEnabled 已展示错误，保留确认弹窗允许重试。 */ } }
}

function TaskTypeDialog({ open, editing, form, pending, errorMessage, onOpenChange, onFormChange, onSubmit }: { open: boolean; editing: TaskType | null; form: Form; pending: boolean; errorMessage?: string; onOpenChange: (open: boolean) => void; onFormChange: (form: Form) => void; onSubmit: (event: React.FormEvent<HTMLFormElement>) => void }) {
  const set = (key: keyof Form, value: string | boolean) => onFormChange({ ...form, [key]: value });
  return <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}><DialogContent className="sm:max-w-lg"><form className="grid gap-4" onSubmit={onSubmit}><DialogHeader><DialogTitle>{editing ? "编辑任务类型" : "新增任务类型"}</DialogTitle><DialogDescription>预置类型可调整参数；新增类型编码保存后不可变更。</DialogDescription></DialogHeader>
    <label className="grid gap-1 text-sm">类型编码<Input required pattern="[A-Za-z0-9][A-Za-z0-9_.-]{0,63}" maxLength={64} readOnly={Boolean(editing)} value={form.code} onChange={(e) => set("code", e.target.value)} /></label>
    <label className="grid gap-1 text-sm">类型名称<Input required maxLength={128} value={form.name} onChange={(e) => set("name", e.target.value)} /></label>
    <div className="grid grid-cols-2 gap-3"><label className="grid gap-1 text-sm">默认优先级<Input required type="number" min="0" max="1000" step="1" value={form.priority} onChange={(e) => set("priority", e.target.value)} /></label><label className="grid gap-1 text-sm">预计耗时（分）<Input required type="number" min="1" max="10080" step="1" value={form.minutes} onChange={(e) => set("minutes", e.target.value)} /></label></div>
    <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.mergeable} onChange={(e) => set("mergeable", e.target.checked)} />可合并</label><label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.insertable} onChange={(e) => set("insertable", e.target.checked)} />可插单</label><label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.enabled} onChange={(e) => set("enabled", e.target.checked)} />启用</label>
    {errorMessage && <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">{errorMessage}</div>}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="submit" disabled={pending}>{pending ? "保存中..." : "保存"}</Button></DialogFooter></form></DialogContent></Dialog>;
}

function emptyForm(): Form { return { code: "", name: "", priority: "100", minutes: "15", mergeable: true, insertable: true, enabled: true }; }
function formFor(row: TaskType): Form { return { code: row.task_type_code, name: row.task_type_name, priority: String(row.default_priority), minutes: String(row.estimated_minutes), mergeable: row.mergeable, insertable: row.insertable, enabled: row.enabled }; }
function validate(form: Form, editing: boolean) { if (!editing && !/^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$/.test(form.code.trim())) return "类型编码必须以字母或数字开头，且只允许字母、数字、下划线、连字符或点号"; if (!form.name.trim()) return "类型名称不能为空"; const priority = Number(form.priority); if (!Number.isInteger(priority) || priority < 0 || priority > 1000) return "默认优先级必须是 0 到 1000 的整数"; const minutes = Number(form.minutes); if (!Number.isInteger(minutes) || minutes < 1 || minutes > 10080) return "预计耗时必须是 1 到 10080 分钟的整数"; return null; }
function defaultQuery(): QueryPanelValue { return { keyword: "", status: "" }; }
function stringValue(value: unknown) { return typeof value === "string" ? value : ""; }
function normalizeQuery(value: unknown): QueryPanelValue { const record = value && typeof value === "object" ? value as Record<string, unknown> : {}; return { keyword: stringValue(record.keyword), status: stringValue(record.status) }; }
function errorText(error: unknown, fallback: string) { return error instanceof Error ? error.message : fallback; }
function formatDateTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false }); }
