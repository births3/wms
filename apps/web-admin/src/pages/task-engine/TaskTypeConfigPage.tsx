import * as React from "react";
import {
  Button, Card, CardContent, DataGrid, Dialog, DialogClose, DialogContent, DialogDescription,
  DialogFooter, DialogHeader, DialogTitle, Input, PageHeader, QueryPanel, StatusBadge,
  buildQueryPanelSummaryItems, formatDateTime, type DataGridColumn, type DataGridCreateAction,
  type DataGridDisableAction, type DataGridEditAction, type DataGridRefreshAction,
  type QueryPanelField, type QueryPanelValue,
} from "@wms/ui";
import {
  useSetTaskTypeEnabledMutation, useTaskPriorityRuleQuery, useTaskTypesQuery,
  useUpsertTaskPriorityRuleMutation, useUpsertTaskTypeMutation,
  type TaskType, type UpsertTaskTypeRequest,
} from "@/features/task-engine/task-type-queries";
import { errorText } from "@/lib/error-text";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_REFRESH,
  BUTTON_SAVE,
  COLUMN_CREATED_AT,
  COLUMN_STATUS,
  ERROR_AUTH_API_CHECK,
  FIELD_KEYWORD,
  FILTER_ALL,
  LOADING_PROCESSING,
  LOADING_SAVING,
  STATUS_DISABLED,
  STATUS_ENABLED,
} from "@/lib/ui-strings";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

const queryFields: QueryPanelField[] = [
  { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "编码 / 名称" },
  { key: "status", label: COLUMN_STATUS, type: "multiSelect", options: [{ label: FILTER_ALL, value: "" }, { label: STATUS_ENABLED, value: "enabled" }, { label: STATUS_DISABLED, value: "disabled" }] },
];
const mteTaskTypeCoreQueryFieldKeys = ["keyword", "status"];
const statusOptions = [{ label: STATUS_ENABLED, value: "enabled" }, { label: STATUS_DISABLED, value: "disabled" }];

type Form = { code: string; name: string; priority: string; minutes: string; mergeable: boolean; insertable: boolean; releaseStrategy: TaskType["release_strategy"]; releaseInterval: string; releaseBatch: string; enabled: boolean };
type Notice = { kind: "success" | "error"; text: string } | null;

const columns: DataGridColumn<TaskType>[] = [
  { key: "task_type_code", header: "类型编码", width: 170, minWidth: 140, mono: true, sortable: true, sortValue: (r) => r.task_type_code, filterValue: (r) => r.task_type_code, copyValue: (r) => r.task_type_code, filter: { type: "text" } },
  { key: "task_type_name", header: "类型名称", width: 160, minWidth: 120, sortable: true, sortValue: (r) => r.task_type_name, filterValue: (r) => r.task_type_name, copyValue: (r) => r.task_type_name, filter: { type: "text" } },
  { key: "default_priority", header: "默认优先级", width: 120, minWidth: 100, sortable: true, sortValue: (r) => r.default_priority, filterValue: (r) => r.default_priority, copyValue: (r) => String(r.default_priority), filter: { type: "numberRange" } },
  { key: "estimated_minutes", header: "预计耗时（分）", width: 140, minWidth: 120, sortable: true, sortValue: (r) => r.estimated_minutes, filterValue: (r) => r.estimated_minutes, copyValue: (r) => String(r.estimated_minutes), filter: { type: "numberRange" } },
  { key: "mergeable", header: "可合并", width: 100, minWidth: 90, render: (r) => r.mergeable ? "是" : "否" },
  { key: "insertable", header: "可插单", width: 100, minWidth: 90, render: (r) => r.insertable ? "是" : "否" },
  { key: "release_strategy", header: "释放策略", width: 140, minWidth: 110, render: (r) => releaseStrategyLabels[r.release_strategy] },
  { key: "enabled", header: COLUMN_STATUS, width: 110, minWidth: 90, render: (r) => <StatusBadge status={r.enabled ? "completed" : "isolated"} label={r.enabled ? STATUS_ENABLED : STATUS_DISABLED} size="sm" /> },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 175, minWidth: 145, sortable: true, sortValue: (r) => r.created_at, filterValue: (r) => r.created_at, copyValue: (r) => r.created_at, filter: { type: "dateRange" }, render: (r) => formatDateTime(r.created_at) },
];

export function TaskTypeConfigPage() {
  const query = useTaskTypesQuery();
  const save = useUpsertTaskTypeMutation();
  const toggle = useSetTaskTypeEnabledMutation();
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const [selected, setSelected] = React.useState<string[]>([]);
  const {
    open: dialogOpen,
    target: editing,
    setOpen: setDialogOpen,
    setTarget: setEditing,
  } = useDialogState<TaskType>();
  const {
    open: toggleOpen,
    target: toggleTarget,
    openWith: openToggleWith,
    setOpen: setToggleOpen,
    setTarget: setToggleTarget,
  } = useDialogState<TaskType>();
  const [form, setForm] = React.useState<Form>(emptyForm);
  const [notice, setNotice] = React.useState<Notice>(null);
  const rows = (query.data?.data ?? []).filter((row) => {
    const keyword = queryString(appliedQuery.keyword).trim().toLowerCase();
    const status = queryString(appliedQuery.status);
    return (!keyword || `${row.task_type_code} ${row.task_type_name}`.toLowerCase().includes(keyword)) && (!status || (status === "enabled" ? row.enabled : !row.enabled));
  });
  const busy = save.isPending || toggle.isPending;
  const selectedRow = rows.find((row) => row.task_type_code === selected[0]);

  const refreshAction: DataGridRefreshAction = { label: BUTTON_REFRESH, description: "刷新任务类型列表", disabled: query.isFetching, onClick: () => void query.refetch() };
  const createAction: DataGridCreateAction = { label: "新增类型", description: "新增自定义任务类型", disabled: busy, onClick: () => openDialog(null) };
  const editAction: DataGridEditAction = { label: "编辑", description: "编辑选中任务类型", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: (ctx) => openDialog(rows.find((r) => r.task_type_code === ctx.selectedRowKeys[0]) ?? null) };
  const disableAction: DataGridDisableAction = { label: selectedRow?.enabled ? STATUS_DISABLED : STATUS_ENABLED, description: "切换任务类型状态", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: () => { if (selectedRow) openToggleWith(selectedRow); } };

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader />
    <PriorityRuleCard />
    {notice && <div className={notice.kind === "error" ? "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" : "rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success"} role={notice.kind === "error" ? "alert" : "status"}>{notice.text}</div>}
    <QueryPanel fields={queryFields} defaultVisibleFieldKeys={mteTaskTypeCoreQueryFieldKeys} value={draftQuery} onValueChange={setDraftQuery} onQuery={() => applyQuery(draftQuery)} onReset={resetQuery} />
    <Card><CardContent className="p-5"><DataGrid storageKey="mte.task-types" columns={columns} data={rows} rowKey={(r) => r.task_type_code} selectable selectedRowKeys={selected} onSelectedRowKeysChange={setSelected} caption={query.isPending ? "加载任务类型..." : undefined} emptyTitle={query.isError ? "读取任务类型失败" : "暂无任务类型"} emptyDescription={query.isError ? errorText(query.error, ERROR_AUTH_API_CHECK) : "暂无可用任务类型"} exportFileBaseName="M-TE-task-types" refreshAction={refreshAction} createAction={createAction} editAction={editAction} disableAction={disableAction} queryState={appliedQuery} querySummaryItems={buildQueryPanelSummaryItems(queryFields, appliedQuery)} onApplyQueryState={applyGridQueryState} onClearQueryState={clearGridQueryState} /></CardContent></Card>
    {/* TaskTypeDisableDialog: 启停写操作统一经过确认。 */}<Dialog open={toggleOpen} onOpenChange={(open) => !busy && (setToggleOpen(open), !open && setToggleTarget(null))}><DialogContent className="sm:max-w-md"><DialogHeader><DialogTitle>{toggleTarget?.enabled ? "停用任务类型" : "启用任务类型"}</DialogTitle><DialogDescription>确认{toggleTarget?.enabled ? "停用" : "启用"}任务类型“{toggleTarget?.task_type_name ?? ""}”？</DialogDescription></DialogHeader><DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose><Button type="button" disabled={busy || !toggleTarget} onClick={() => void confirmToggle()}>{busy ? LOADING_PROCESSING : "确认"}</Button></DialogFooter></DialogContent></Dialog>
    <TaskTypeDialog open={dialogOpen} editing={editing} form={form} pending={save.isPending} errorMessage={save.error?.message} onOpenChange={setDialogOpen} onFormChange={setForm} onSubmit={submit} />
  </section>;

  function applyGridQueryState(value: unknown) { applyQuery(queryValueFromUnknown(value)); }
  function clearGridQueryState() { resetQuery(); }
  function openDialog(row: TaskType | null) { setNotice(null); setEditing(row); setForm(row ? formFor(row) : emptyForm()); setDialogOpen(true); }
  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validate(form, Boolean(editing));
    if (error) { setNotice({ kind: "error", text: error }); return; }
    const scheduled = form.releaseStrategy === "scheduled";
    const body: UpsertTaskTypeRequest = { task_type_name: form.name.trim(), default_priority: Number(form.priority), estimated_minutes: Number(form.minutes), mergeable: form.mergeable, insertable: form.insertable, release_strategy: form.releaseStrategy, release_interval_minutes: scheduled ? Number(form.releaseInterval) : null, release_batch_size: scheduled ? Number(form.releaseBatch) : null, enabled: form.enabled };
    try { await save.mutateAsync({ code: form.code.trim().toLowerCase(), body }); setDialogOpen(false); setSelected([]); setNotice({ kind: "success", text: `任务类型 ${form.code.trim().toLowerCase()} 已保存` }); } catch { /* 弹窗保留，允许重试。 */ }
  }
  /** 返回是否成功：失败时已写入错误 Notice，由调用方决定是否保留弹窗。 */
  async function setEnabled(row: TaskType) { setNotice(null); try { await toggle.mutateAsync({ code: row.task_type_code, enabled: !row.enabled }); setSelected([]); setNotice({ kind: "success", text: `${row.task_type_name} 已${row.enabled ? "停用" : "启用"}` }); return true; } catch (error) { setNotice({ kind: "error", text: errorText(error, "更新任务类型状态失败") }); return false; } }
  async function confirmToggle() { if (!toggleTarget) return; const ok = await setEnabled(toggleTarget); if (!ok) return; /* 失败保留确认弹窗允许重试 */ setToggleOpen(false); setToggleTarget(null); }
}

function TaskTypeDialog({ open, editing, form, pending, errorMessage, onOpenChange, onFormChange, onSubmit }: { open: boolean; editing: TaskType | null; form: Form; pending: boolean; errorMessage?: string; onOpenChange: (open: boolean) => void; onFormChange: (form: Form) => void; onSubmit: (event: React.FormEvent<HTMLFormElement>) => void }) {
  const set = (key: keyof Form, value: string | boolean) => onFormChange({ ...form, [key]: value });
  return <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}><DialogContent className="sm:max-w-lg"><form className="grid gap-4" onSubmit={onSubmit}><DialogHeader><DialogTitle>{editing ? "编辑任务类型" : "新增任务类型"}</DialogTitle><DialogDescription>预置类型可调整参数；新增类型编码保存后不可变更。</DialogDescription></DialogHeader>
    <label className="grid gap-1 text-sm">类型编码<Input required pattern="[A-Za-z0-9][A-Za-z0-9_.-]{0,63}" maxLength={64} readOnly={Boolean(editing)} value={form.code} onChange={(e) => set("code", e.target.value)} /></label>
    <label className="grid gap-1 text-sm">类型名称<Input required maxLength={128} value={form.name} onChange={(e) => set("name", e.target.value)} /></label>
    <div className="grid grid-cols-2 gap-3"><label className="grid gap-1 text-sm">默认优先级<Input required type="number" min="0" max="1000" step="1" value={form.priority} onChange={(e) => set("priority", e.target.value)} /></label><label className="grid gap-1 text-sm">预计耗时（分）<Input required type="number" min="1" max="10080" step="1" value={form.minutes} onChange={(e) => set("minutes", e.target.value)} /></label></div>
    <label className="grid gap-1 text-sm">释放策略<select aria-label="释放策略" className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.releaseStrategy} onChange={(event) => set("releaseStrategy", event.target.value as TaskType["release_strategy"])}>{Object.entries(releaseStrategyLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
    {form.releaseStrategy === "scheduled" && <div className="grid grid-cols-2 gap-3"><label className="grid gap-1 text-sm">释放间隔（分）<Input required type="number" min="1" max="1440" step="1" value={form.releaseInterval} onChange={(event) => set("releaseInterval", event.target.value)} /></label><label className="grid gap-1 text-sm">每批任务数<Input required type="number" min="1" max="1000" step="1" value={form.releaseBatch} onChange={(event) => set("releaseBatch", event.target.value)} /></label></div>}
    <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.mergeable} onChange={(e) => set("mergeable", e.target.checked)} />可合并</label><label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.insertable} onChange={(e) => set("insertable", e.target.checked)} />可插单</label><label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.enabled} onChange={(e) => set("enabled", e.target.checked)} />{STATUS_ENABLED}</label>
    {errorMessage && <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">{errorMessage}</div>}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="submit" disabled={pending}>{pending ? LOADING_SAVING : BUTTON_SAVE}</Button></DialogFooter></form></DialogContent></Dialog>;
}

function emptyForm(): Form { return { code: "", name: "", priority: "100", minutes: "15", mergeable: true, insertable: true, releaseStrategy: "immediate", releaseInterval: "10", releaseBatch: "50", enabled: true }; }
function formFor(row: TaskType): Form { return { code: row.task_type_code, name: row.task_type_name, priority: String(row.default_priority), minutes: String(row.estimated_minutes), mergeable: row.mergeable, insertable: row.insertable, releaseStrategy: row.release_strategy, releaseInterval: String(row.release_interval_minutes ?? 10), releaseBatch: String(row.release_batch_size ?? 50), enabled: row.enabled }; }
function validate(form: Form, editing: boolean) { if (!editing && !/^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$/.test(form.code.trim())) return "类型编码必须以字母或数字开头，且只允许字母、数字、下划线、连字符或点号"; if (!form.name.trim()) return "类型名称不能为空"; const priority = Number(form.priority); if (!Number.isInteger(priority) || priority < 0 || priority > 1000) return "默认优先级必须是 0 到 1000 的整数"; const minutes = Number(form.minutes); if (!Number.isInteger(minutes) || minutes < 1 || minutes > 10080) return "预计耗时必须是 1 到 10080 分钟的整数"; if (form.releaseStrategy === "scheduled") { const interval = Number(form.releaseInterval); const batch = Number(form.releaseBatch); if (!Number.isInteger(interval) || interval < 1 || interval > 1440 || !Number.isInteger(batch) || batch < 1 || batch > 1000) return "定时释放间隔必须是 1 到 1440 分钟，每批任务数必须是 1 到 1000"; } return null; }
function defaultQuery(): QueryPanelValue { return { keyword: "", status: "" }; }
function normalizeQuery(value: unknown): QueryPanelValue { const record = queryValueFromUnknown(value); return { keyword: queryString(record.keyword), status: queryString(record.status) }; }

const releaseStrategyLabels: Record<TaskType["release_strategy"], string> = { immediate: "立即释放", scheduled: "定时释放", conditional: "条件释放", capacity: "容量释放" };

type PriorityRuleForm = { urgent: string; waiting: string; cold: string; expedite: string };

function PriorityRuleCard() {
  const query = useTaskPriorityRuleQuery();
  const save = useUpsertTaskPriorityRuleMutation();
  const [open, setOpen] = React.useState(false);
  const [form, setForm] = React.useState<PriorityRuleForm>({ urgent: "20", waiting: "30", cold: "20", expedite: "50" });
  const [notice, setNotice] = React.useState<string | null>(null);
  const rule = query.data;
  const set = (key: keyof PriorityRuleForm, value: string) => setForm((current) => ({ ...current, [key]: value }));
  const openDialog = () => {
    if (rule) setForm({ urgent: String(rule.urgent_order_bonus), waiting: String(rule.waiting_minutes_per_point), cold: String(rule.cold_chain_bonus), expedite: String(rule.manual_expedite_bonus) });
    setNotice(null);
    setOpen(true);
  };
  const submit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const values = [Number(form.urgent), Number(form.cold), Number(form.expedite)];
    const waiting = Number(form.waiting);
    if (values.some((value) => !Number.isInteger(value) || value < 0 || value > 1000) || !Number.isInteger(waiting) || waiting < 1 || waiting > 1440) {
      setNotice("加分必须是 0 到 1000 的整数，等待间隔必须是 1 到 1440 分钟的整数");
      return;
    }
    try {
      await save.mutateAsync({ urgent_order_bonus: values[0], waiting_minutes_per_point: waiting, cold_chain_bonus: values[1], manual_expedite_bonus: values[2] });
      setOpen(false);
    } catch { /* mutation error is rendered in the dialog */ }
  };
  return <Card><CardContent className="flex flex-wrap items-center justify-between gap-4 p-5"><div><h2 className="font-semibold">任务优先级规则</h2><p className="mt-1 text-sm text-muted-foreground">{rule ? `订单加急 +${rule.urgent_order_bonus} · 每等待 ${rule.waiting_minutes_per_point} 分钟 +1 · 冷链 +${rule.cold_chain_bonus} · 手动加急 +${rule.manual_expedite_bonus}` : query.isError ? "读取规则失败" : "加载规则中..."}</p></div><Button type="button" variant="outline" disabled={!rule || save.isPending} onClick={openDialog}>配置优先级规则</Button><Dialog open={open} onOpenChange={(next) => !save.isPending && setOpen(next)}><DialogContent className="sm:max-w-lg"><form className="grid gap-4" onSubmit={submit}><DialogHeader><DialogTitle>配置任务优先级规则</DialogTitle><DialogDescription>任务类型默认优先级在下方逐项维护；这里只配置四类附加权重。单一结构化规则不会产生表达式冲突。</DialogDescription></DialogHeader><div className="grid grid-cols-2 gap-3"><label className="grid gap-1 text-sm">订单加急加分<Input aria-label="订单加急加分" required type="number" min="0" max="1000" step="1" value={form.urgent} onChange={(event) => set("urgent", event.target.value)} /></label><label className="grid gap-1 text-sm">等待多少分钟加 1 分<Input aria-label="等待多少分钟加 1 分" required type="number" min="1" max="1440" step="1" value={form.waiting} onChange={(event) => set("waiting", event.target.value)} /></label><label className="grid gap-1 text-sm">冷链任务加分<Input aria-label="冷链任务加分" required type="number" min="0" max="1000" step="1" value={form.cold} onChange={(event) => set("cold", event.target.value)} /></label><label className="grid gap-1 text-sm">手动加急加分<Input aria-label="手动加急加分" required type="number" min="0" max="1000" step="1" value={form.expedite} onChange={(event) => set("expedite", event.target.value)} /></label></div>{(notice || save.error) && <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">{notice ?? save.error?.message}</div>}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={save.isPending}>取消</Button></DialogClose><Button type="submit" disabled={save.isPending}>{save.isPending ? LOADING_SAVING : "保存规则"}</Button></DialogFooter></form></DialogContent></Dialog></CardContent></Card>;
}
