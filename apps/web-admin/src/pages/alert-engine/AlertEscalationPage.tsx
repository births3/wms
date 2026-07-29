import * as React from "react";
import {
  Button, Card, CardContent, DataGrid, Dialog, DialogClose, DialogContent,
  DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input, PageHeader,
  QueryPanel, StatusBadge, buildQueryPanelSummaryItems, type DataGridColumn,
  type DataGridCreateAction, type DataGridEditAction, type DataGridRefreshAction,
  type QueryPanelField, type QueryPanelValue,
} from "@wms/ui";

import {
  useAlertEscalationRulesQuery, useUpsertAlertEscalationRuleMutation,
  type AlertEscalationRule, type AlertEscalationRuleDraft,
} from "@/features/alert-engine/alert-runtime-queries";
import { errorText } from "@/lib/error-text";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

export const alertEscalationQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "规则编码 / 名称" },
  { key: "enabled", label: "状态", type: "select", options: [{ label: "全部", value: "" }, { label: "启用", value: "true" }, { label: "停用", value: "false" }] },
  { key: "handlerRole", label: "值班角色", type: "text", placeholder: "角色编码" },
];
export const alertEscalationCoreQueryFieldKeys = ["keyword", "enabled"];

interface LevelForm { thresholdMinutes: string; recipientRoles: string; }
interface RuleForm {
  ruleCode: string;
  ruleName: string;
  notifyLowerLevels: boolean;
  offHoursStart: string;
  offHoursEnd: string;
  offHoursHandlerRoles: string;
  holidayDates: string;
  enabled: boolean;
  levels: LevelForm[];
}

const columns: DataGridColumn<AlertEscalationRule>[] = [
  { key: "rule_code", header: "规则编码", width: 190, minWidth: 150, mono: true, sortable: true, sortValue: (row) => row.rule_code, filterValue: (row) => row.rule_code, copyValue: (row) => row.rule_code, filter: { type: "text" } },
  { key: "rule_name", header: "规则名称", width: 190, minWidth: 140, sortable: true, sortValue: (row) => row.rule_name, filterValue: (row) => row.rule_name, filter: { type: "text" } },
  { key: "levels", header: "升级阈值", width: 280, minWidth: 210, render: (row) => row.levels.map((level) => `L${level.level} ${duration(level.threshold_seconds)}`).join(" / ") },
  { key: "recipients", header: "升级接收角色", width: 300, minWidth: 220, render: (row) => row.levels.map((level) => `L${level.level}: ${level.recipient_roles.join("、")}`).join("；") },
  { key: "off_hours", header: "非工作时间", width: 240, minWidth: 180, render: (row) => `${row.off_hours_start}-${row.off_hours_end} · ${row.off_hours_handler_roles.join("、")}` },
  { key: "notify_lower_levels", header: "保留下级接收人", width: 135, minWidth: 115, render: (row) => row.notify_lower_levels ? "是" : "否" },
  { key: "holiday_dates", header: "节假日", width: 190, minWidth: 140, render: (row) => row.holiday_dates.join("、") || "无" },
  { key: "enabled", header: "状态", width: 100, minWidth: 90, render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? "启用" : "停用"} size="sm" /> },
  { key: "version", header: "版本", width: 80, minWidth: 70, mono: true, sortable: true, sortValue: (row) => row.version },
  { key: "created_at", header: "创建时间", width: 175, minWidth: 150, sortable: true, sortValue: (row) => row.created_at, render: (row) => formatDateTime(row.created_at) },
  { key: "updated_at", header: "更新时间", width: 175, minWidth: 150, sortable: true, sortValue: (row) => row.updated_at, filterValue: (row) => row.updated_at, filter: { type: "dateRange" }, render: (row) => formatDateTime(row.updated_at) },
];

/** 页面设计契约：配置型列表页；规则最多三级，阈值严格递增，L3 由后台每 24 小时持续提醒。 */
export function AlertEscalationPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const [selected, setSelected] = React.useState<string[]>([]);
  const {
    open: dialogOpen,
    target: editing,
    setOpen: setDialogOpen,
    setTarget: setEditing,
  } = useDialogState<AlertEscalationRule>();
  const [form, setForm] = React.useState<RuleForm>(emptyRuleForm);
  const [formError, setFormError] = React.useState<string | null>(null);
  const [notice, setNotice] = React.useState<string | null>(null);
  const query = useAlertEscalationRulesQuery();
  const upsert = useUpsertAlertEscalationRuleMutation();
  const rows = filterRules(query.data?.data ?? [], appliedQuery);
  const selectedRow = rows.find((row) => row.id === selected[0]);
  const refreshAction: DataGridRefreshAction = { label: "刷新", description: "刷新升级规则", disabled: query.isFetching, onClick: () => void query.refetch() };
  const createAction: DataGridCreateAction = { label: "新增规则", description: "新增告警升级规则", disabled: upsert.isPending, onClick: () => openForm(null) };
  const editAction: DataGridEditAction = { label: "编辑", description: "编辑选中升级规则", disabled: (context) => context.selectedRowKeys.length !== 1 || upsert.isPending, onClick: () => selectedRow && openForm(selectedRow) };

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader title="H-AL 告警升级规则" subtitle="最多三级升级；默认 30 分钟、2 小时、24 小时，支持夜间、周末与节假日值班路由" />
    {notice && <div role="status" className="rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success">{notice}</div>}
    <QueryPanel fields={alertEscalationQueryFields} defaultVisibleFieldKeys={alertEscalationCoreQueryFieldKeys} value={draftQuery} onValueChange={setDraftQuery} onQuery={() => applyQuery(draftQuery)} onReset={resetQuery} />
    <Card><CardContent className="p-5"><DataGrid storageKey="hal.alert-escalation-rules" columns={columns} data={rows} rowKey={(row) => row.id} selectable selectedRowKeys={selected} onSelectedRowKeysChange={setSelected} caption={query.isPending ? "加载升级规则..." : undefined} emptyTitle={query.isError ? "读取升级规则失败" : "暂无升级规则"} emptyDescription={query.isError ? errorText(query.error, "请检查权限和 API 服务") : "新增规则后可在告警定义中引用"} tableClassName="min-w-[1770px]" exportFileBaseName="H-AL 告警升级规则" refreshAction={refreshAction} createAction={createAction} editAction={editAction} queryState={appliedQuery} querySummaryItems={buildQueryPanelSummaryItems(alertEscalationQueryFields, appliedQuery)} onApplyQueryState={applyGridQueryState} onClearQueryState={clearGridQueryState} /></CardContent></Card>
    <EscalationRuleDialog open={dialogOpen} editing={editing} form={form} pending={upsert.isPending} errorMessage={formError ?? upsert.error?.message} onOpenChange={setDialogOpen} onFormChange={(next) => { setFormError(null); setForm(next); }} onSubmit={submitForm} />
  </section>;

  function applyGridQueryState(value: unknown) { applyQuery(queryValueFromUnknown(value)); }
  function clearGridQueryState() { resetQuery(); }
  function openForm(row: AlertEscalationRule | null) { setNotice(null); setFormError(null); upsert.reset(); setEditing(row); setForm(row ? formForRule(row) : emptyRuleForm()); setDialogOpen(true); }
  async function submitForm(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validateForm(form);
    if (error) { setFormError(error); return; }
    try { const saved = await upsert.mutateAsync(toDraft(form)); setDialogOpen(false); setSelected([saved.id]); setNotice(`升级规则 ${saved.rule_code} 已保存，版本 ${saved.version}`); }
    catch (error) { setFormError(errorText(error, "保存升级规则失败")); }
  }
}

function EscalationRuleDialog({ open, editing, form, pending, errorMessage, onOpenChange, onFormChange, onSubmit }: { open: boolean; editing: AlertEscalationRule | null; form: RuleForm; pending: boolean; errorMessage?: string; onOpenChange: (open: boolean) => void; onFormChange: (form: RuleForm) => void; onSubmit: (event: React.FormEvent<HTMLFormElement>) => void }) {
  const set = <K extends keyof RuleForm>(key: K, value: RuleForm[K]) => onFormChange({ ...form, [key]: value });
  const setLevel = (index: number, patch: Partial<LevelForm>) => set("levels", form.levels.map((level, current) => current === index ? { ...level, ...patch } : level));
  return <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}><DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl"><form className="grid gap-4" onSubmit={onSubmit}><DialogHeader><DialogTitle>{editing ? "编辑升级规则" : "新增升级规则"}</DialogTitle><DialogDescription>阈值从告警触发时间起算并严格递增；确认、关闭或忽略后立即停止升级。</DialogDescription></DialogHeader>
    <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1 text-sm">规则编码<Input required readOnly={Boolean(editing)} minLength={2} maxLength={64} pattern="[A-Za-z0-9][A-Za-z0-9_.-]{1,63}" value={form.ruleCode} onChange={(event) => set("ruleCode", event.target.value)} /></label><label className="grid gap-1 text-sm">规则名称<Input required maxLength={128} value={form.ruleName} onChange={(event) => set("ruleName", event.target.value)} /></label></div>
    <fieldset className="grid gap-3 rounded-md border p-3"><legend className="px-1 text-sm font-medium">升级级别（最多 3 级）</legend>{form.levels.map((level, index) => <div key={index} className="grid items-end gap-3 sm:grid-cols-[80px_180px_1fr_auto]"><strong className="pb-2 text-sm">L{index + 1}</strong><label className="grid gap-1 text-sm">触发阈值（分钟）<Input required type="number" min="1" max="525600" step="1" value={level.thresholdMinutes} onChange={(event) => setLevel(index, { thresholdMinutes: event.target.value })} /></label><label className="grid gap-1 text-sm">接收角色（逗号分隔）<Input required value={level.recipientRoles} onChange={(event) => setLevel(index, { recipientRoles: event.target.value })} /></label><Button type="button" variant="outline" disabled={form.levels.length === 1} onClick={() => set("levels", form.levels.filter((_, current) => current !== index))}>移除</Button></div>)}<Button type="button" variant="outline" disabled={form.levels.length >= 3} onClick={() => set("levels", [...form.levels, { thresholdMinutes: String([30, 120, 1440][form.levels.length]), recipientRoles: "system_admin" }])}>添加一级</Button></fieldset>
    <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1 text-sm">非工作时间开始<Input required type="time" value={form.offHoursStart} onChange={(event) => set("offHoursStart", event.target.value)} /></label><label className="grid gap-1 text-sm">非工作时间结束<Input required type="time" value={form.offHoursEnd} onChange={(event) => set("offHoursEnd", event.target.value)} /></label></div>
    <label className="grid gap-1 text-sm">夜间 / 周末 / 节假日值班角色（逗号分隔）<Input required value={form.offHoursHandlerRoles} onChange={(event) => set("offHoursHandlerRoles", event.target.value)} /></label>
    <label className="grid gap-1 text-sm">节假日日期（逗号分隔，YYYY-MM-DD）<Input placeholder="2026-10-01, 2026-10-02" value={form.holidayDates} onChange={(event) => set("holidayDates", event.target.value)} /></label>
    <div className="grid gap-2 sm:grid-cols-2"><label className="flex min-h-10 items-center gap-2 text-sm"><input type="checkbox" checked={form.notifyLowerLevels} onChange={(event) => set("notifyLowerLevels", event.target.checked)} />升级时保留下级接收人</label><label className="flex min-h-10 items-center gap-2 text-sm"><input type="checkbox" checked={form.enabled} onChange={(event) => set("enabled", event.target.checked)} />启用规则</label></div>
    {errorMessage && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{errorMessage}</div>}
    <DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="submit" disabled={pending}>{pending ? "保存中..." : "保存规则"}</Button></DialogFooter></form></DialogContent></Dialog>;
}

function emptyRuleForm(): RuleForm { return { ruleCode: "", ruleName: "", notifyLowerLevels: true, offHoursStart: "18:00", offHoursEnd: "08:00", offHoursHandlerRoles: "warehouse_manager,system_admin", holidayDates: "", enabled: true, levels: [{ thresholdMinutes: "30", recipientRoles: "warehouse_manager" }, { thresholdMinutes: "120", recipientRoles: "warehouse_manager,system_admin" }, { thresholdMinutes: "1440", recipientRoles: "system_admin" }] }; }
function formForRule(row: AlertEscalationRule): RuleForm { return { ruleCode: row.rule_code, ruleName: row.rule_name, notifyLowerLevels: row.notify_lower_levels, offHoursStart: row.off_hours_start.slice(0, 5), offHoursEnd: row.off_hours_end.slice(0, 5), offHoursHandlerRoles: row.off_hours_handler_roles.join(","), holidayDates: row.holiday_dates.join(","), enabled: row.enabled, levels: row.levels.map((level) => ({ thresholdMinutes: String(Math.ceil(level.threshold_seconds / 60)), recipientRoles: level.recipient_roles.join(",") })) }; }
function validateForm(form: RuleForm) { if (!/^[A-Za-z0-9][A-Za-z0-9_.-]{1,63}$/.test(form.ruleCode.trim())) return "规则编码必须为 2 到 64 位受控标识符"; if (!form.ruleName.trim()) return "规则名称不能为空"; if (!form.levels.length || form.levels.length > 3) return "升级规则必须包含 1 到 3 级"; let previous = 0; for (const level of form.levels) { const minutes = Number(level.thresholdMinutes); if (!Number.isInteger(minutes) || minutes <= previous) return "升级阈值必须是严格递增的整数分钟"; if (!splitValues(level.recipientRoles).length) return "每一级至少配置一个接收角色"; previous = minutes; } if (!splitValues(form.offHoursHandlerRoles).length) return "至少配置一个非工作时间值班角色"; if (splitValues(form.holidayDates).some((value) => !/^\d{4}-\d{2}-\d{2}$/.test(value))) return "节假日必须使用 YYYY-MM-DD 格式"; return null; }
function toDraft(form: RuleForm): AlertEscalationRuleDraft { return { rule_code: form.ruleCode.trim(), rule_name: form.ruleName.trim(), notify_lower_levels: form.notifyLowerLevels, off_hours_start: form.offHoursStart, off_hours_end: form.offHoursEnd, off_hours_handler_roles: splitValues(form.offHoursHandlerRoles), holiday_dates: splitValues(form.holidayDates), enabled: form.enabled, levels: form.levels.map((level, index) => ({ level: index + 1, threshold_seconds: Number(level.thresholdMinutes) * 60, recipient_roles: splitValues(level.recipientRoles) })) }; }
function filterRules(rows: AlertEscalationRule[], query: QueryPanelValue) { const keyword = queryString(query.keyword).trim().toLocaleLowerCase(); const enabled = queryString(query.enabled); const role = queryString(query.handlerRole).trim().toLocaleLowerCase(); return rows.filter((row) => (!keyword || `${row.rule_code} ${row.rule_name}`.toLocaleLowerCase().includes(keyword)) && (!enabled || row.enabled === (enabled === "true")) && (!role || row.off_hours_handler_roles.some((value) => value.toLocaleLowerCase().includes(role)))); }
function defaultQuery(): QueryPanelValue { return { keyword: "", enabled: "", handlerRole: "" }; }
function normalizeQuery(value: unknown): QueryPanelValue { const row = queryValueFromUnknown(value); return { keyword: queryString(row.keyword), enabled: queryString(row.enabled), handlerRole: queryString(row.handlerRole) }; }
function splitValues(value: string) { return value.split(",").map((item) => item.trim()).filter(Boolean); }
function duration(seconds: number) { const minutes = seconds / 60; return minutes < 60 ? `${minutes} 分钟` : minutes < 1440 ? `${minutes / 60} 小时` : `${minutes / 1440} 天`; }
function formatDateTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false }); }
