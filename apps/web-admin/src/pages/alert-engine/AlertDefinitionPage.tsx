import * as React from "react";
import {
  Button, Card, CardContent, DataGrid, Dialog, DialogClose, DialogContent,
  DialogDescription, DialogFooter, DialogHeader, DialogTitle, PageHeader, QueryPanel,
  StatusBadge, buildQueryPanelSummaryItems, formatDateTime, type DataGridColumn, type DataGridCreateAction,
  type DataGridDeleteAction, type DataGridDetailAction, type DataGridDisableAction,
  type DataGridEditAction, type DataGridRefreshAction, type QueryPanelField, type QueryPanelValue,
} from "@wms/ui";

import {
  useAlertDefinitionsQuery, useSubmitAlertDefinitionChangeMutation, type AlertDefinition,
  type AlertDefinitionChangeRequest,
} from "@/features/alert-engine/alert-definition-queries";
import { useAlertEscalationRulesQuery } from "@/features/alert-engine/alert-runtime-queries";
import {
  AlertDefinitionFormDialog, alertDefinitionFormFor, emptyAlertDefinitionForm,
  toAlertDefinitionDraft, validateAlertDefinitionForm, type AlertDefinitionForm,
} from "./AlertDefinitionFormDialog";
import { errorText } from "@/lib/error-text";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_REFRESH, COLUMN_CREATED_AT, COLUMN_EVENT_TYPE, COLUMN_STATUS, COLUMN_UPDATED_AT,
  COLUMN_VERSION, ERROR_AUTH_API_CHECK, FIELD_KEYWORD, FILTER_ALL, LOADING_SUBMITTING,
  STATUS_DISABLED, STATUS_ENABLED,
} from "@/lib/ui-strings";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

export const alertDefinitionQueryFields: QueryPanelField[] = [
  { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "编码 / 名称 / 事件类型" },
  { key: "enabled", label: COLUMN_STATUS, type: "select", options: [{ label: FILTER_ALL, value: "" }, { label: STATUS_ENABLED, value: "true" }, { label: STATUS_DISABLED, value: "false" }] },
  { key: "severity", label: "级别", type: "select", options: [{ label: FILTER_ALL, value: "" }, { label: "提示", value: "info" }, { label: "警告", value: "warning" }, { label: "严重", value: "critical" }] },
];
export const alertDefinitionCoreQueryFieldKeys = ["keyword", "enabled"];

type Notice = { kind: "success" | "error"; text: string } | null;
type ConfirmAction = { operation: "set_enabled" | "delete"; row: AlertDefinition } | null;

const columns: DataGridColumn<AlertDefinition>[] = [
  { key: "alert_code", header: "告警编码", width: 190, minWidth: 150, mono: true, sortable: true, sortValue: (row) => row.alert_code, filterValue: (row) => row.alert_code, copyValue: (row) => row.alert_code, filter: { type: "text" } },
  { key: "name", header: "名称", width: 190, minWidth: 140, sortable: true, sortValue: (row) => row.name, filterValue: (row) => row.name, copyValue: (row) => row.name, filter: { type: "text" } },
  { key: "event_type", header: COLUMN_EVENT_TYPE, width: 190, minWidth: 140, mono: true, filterValue: (row) => row.event_type, copyValue: (row) => row.event_type, filter: { type: "text" } },
  { key: "default_severity", header: "默认级别", width: 110, minWidth: 90, render: (row) => severityLabel(row.default_severity) },
  { key: "recipient_roles", header: "接收角色", width: 220, minWidth: 160, render: (row) => row.recipient_roles.map(roleLabel).join("、"), copyValue: (row) => row.recipient_roles.join(",") },
  { key: "silence_period_seconds", header: "静默期", width: 110, minWidth: 90, render: (row) => `${Math.round(row.silence_period_seconds / 60)} 分钟`, sortValue: (row) => row.silence_period_seconds, sortable: true },
  { key: "enabled", header: COLUMN_STATUS, width: 110, minWidth: 90, render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? STATUS_ENABLED : STATUS_DISABLED} size="sm" /> },
  { key: "is_gsp_forced", header: "监管属性", width: 120, minWidth: 100, render: (row) => row.is_gsp_forced ? "GSP 强制" : "一般" },
  { key: "version", header: COLUMN_VERSION, width: 90, minWidth: 80, mono: true, sortable: true, sortValue: (row) => row.version },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 175, minWidth: 145, sortable: true, sortValue: (row) => row.created_at, filterValue: (row) => row.created_at, copyValue: (row) => row.created_at, filter: { type: "dateRange" }, render: (row) => formatDateTime(row.created_at) },
  { key: "updated_at", header: COLUMN_UPDATED_AT, width: 175, minWidth: 145, sortable: true, sortValue: (row) => row.updated_at, filterValue: (row) => row.updated_at, copyValue: (row) => row.updated_at, filter: { type: "dateRange" }, render: (row) => formatDateTime(row.updated_at) },
];

/**
 * 页面设计契约：配置型列表页；主信息载体为 QueryPanel + DataGrid；标准查询、刷新、新增、
 * 详情、编辑、启停、删除、导出和字段设置由公共组件承载；详情与确认使用瞬时弹窗，不设常驻侧栏。
 */
export function AlertDefinitionPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const query = useAlertDefinitionsQuery(toFilters(appliedQuery));
  const submitChange = useSubmitAlertDefinitionChangeMutation();
  const escalationRules = useAlertEscalationRulesQuery();
  const [selected, setSelected] = React.useState<string[]>([]);
  const {
    open: formOpen,
    target: editing,
    setOpen: setFormOpen,
    setTarget: setEditing,
  } = useDialogState<AlertDefinition>();
  const [form, setForm] = React.useState<AlertDefinitionForm>(emptyAlertDefinitionForm);
  const [formError, setFormError] = React.useState<string | null>(null);
  const [detail, setDetail] = React.useState<AlertDefinition | null>(null);
  const [confirmAction, setConfirmAction] = React.useState<ConfirmAction>(null);
  const [notice, setNotice] = React.useState<Notice>(null);
  const rows = query.data?.data ?? [];
  const selectedRow = rows.find((row) => row.id === selected[0]);
  const busy = submitChange.isPending;

  const refreshAction: DataGridRefreshAction = { label: BUTTON_REFRESH, description: "刷新告警定义列表", disabled: query.isFetching, onClick: () => void query.refetch() };
  const createAction: DataGridCreateAction = { label: "新增定义", description: "提交新增告警定义审批", disabled: busy, onClick: () => openForm(null) };
  const detailAction: DataGridDetailAction = { label: "详情", description: "查看选中告警定义", disabled: (context) => context.selectedRowKeys.length !== 1, onClick: () => setDetail(selectedRow ?? null) };
  const editAction: DataGridEditAction = { label: "编辑", description: "提交选中告警定义的编辑审批", disabled: (context) => context.selectedRowKeys.length !== 1 || busy, onClick: () => selectedRow && openForm(selectedRow) };
  const disableAction: DataGridDisableAction = { label: selectedRow?.enabled ? STATUS_DISABLED : STATUS_ENABLED, description: "提交告警定义启停审批", disabled: (context) => context.selectedRowKeys.length !== 1 || busy || cannotToggle(selectedRow), onClick: () => selectedRow && openConfirm("set_enabled", selectedRow) };
  const deleteAction: DataGridDeleteAction = { label: "删除", description: "提交删除告警定义审批", disabled: (context) => context.selectedRowKeys.length !== 1 || busy || Boolean(selectedRow?.is_gsp_forced), onClick: () => selectedRow && openConfirm("delete", selectedRow) };

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader title="H-AL 告警定义" subtitle="告警条件、级别、接收角色、升级和静默策略；所有变更经 M-QL 审批后生效" />
    {notice && <div role={notice.kind === "error" ? "alert" : "status"} className={notice.kind === "error" ? "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" : "rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success"}>{notice.text}</div>}
    <QueryPanel fields={alertDefinitionQueryFields} defaultVisibleFieldKeys={alertDefinitionCoreQueryFieldKeys} value={draftQuery} onValueChange={setDraftQuery} onQuery={() => applyQuery(draftQuery)} onReset={resetQuery} />
    <Card><CardContent className="p-5"><DataGrid storageKey="hal.alert-definitions" columns={columns} data={rows} rowKey={(row) => row.id} selectable selectedRowKeys={selected} onSelectedRowKeysChange={setSelected} caption={query.isPending ? "加载告警定义..." : undefined} emptyTitle={query.isError ? "读取告警定义失败" : "暂无告警定义"} emptyDescription={query.isError ? errorText(query.error, ERROR_AUTH_API_CHECK) : "请新增首个货主告警定义"} tableClassName="min-w-[1595px]" exportFileBaseName="H-AL 告警定义" refreshAction={refreshAction} createAction={createAction} detailAction={detailAction} editAction={editAction} disableAction={disableAction} deleteAction={deleteAction} queryState={appliedQuery} querySummaryItems={buildQueryPanelSummaryItems(alertDefinitionQueryFields, appliedQuery)} onApplyQueryState={applyGridQueryState} onClearQueryState={clearGridQueryState} /></CardContent></Card>
    <AlertDefinitionFormDialog open={formOpen} editing={editing} form={form} pending={busy} errorMessage={formError ?? submitChange.error?.message} escalationRules={escalationRules.data?.data ?? []} onOpenChange={setFormOpen} onFormChange={(next) => { setFormError(null); setForm(next); }} onSubmit={submitForm} />
    <AlertDefinitionDetailDialog row={detail} onOpenChange={(open) => !open && setDetail(null)} />
    <ConfirmDialog action={confirmAction} pending={busy} errorMessage={submitChange.error?.message} onOpenChange={(open) => !open && setConfirmAction(null)} onConfirm={() => void confirmChange()} />
  </section>;

  function applyGridQueryState(value: unknown) { applyQuery(queryValueFromUnknown(value)); }
  function clearGridQueryState() { resetQuery(); }
  function openForm(row: AlertDefinition | null) { setNotice(null); setFormError(null); submitChange.reset(); setEditing(row); setForm(row ? alertDefinitionFormFor(row) : emptyAlertDefinitionForm()); setFormOpen(true); }
  function openConfirm(operation: "set_enabled" | "delete", row: AlertDefinition) { setNotice(null); submitChange.reset(); setConfirmAction({ operation, row }); }
  async function submitForm(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validateAlertDefinitionForm(form);
    if (error) { setFormError(error); return; }
    const request: AlertDefinitionChangeRequest = { operation: "upsert", definition_id: editing?.id ?? null, expected_version: editing?.version ?? null, definition: toAlertDefinitionDraft(form), enabled: null };
    await submitRequest(request, editing ? "编辑" : "新增", () => setFormOpen(false));
  }
  async function confirmChange() {
    if (!confirmAction) return;
    const { operation, row } = confirmAction;
    const request: AlertDefinitionChangeRequest = { operation, definition_id: row.id, expected_version: row.version, definition: null, enabled: operation === "set_enabled" ? !row.enabled : null };
    await submitRequest(request, operation === "delete" ? "删除" : row.enabled ? "停用" : "启用", () => setConfirmAction(null));
  }
  async function submitRequest(request: AlertDefinitionChangeRequest, action: string, onSuccess: () => void) {
    setNotice(null);
    try { const order = await submitChange.mutateAsync(request); setSelected([]); onSuccess(); setNotice({ kind: "success", text: `${action}变更已提交质量联系单 ${order.liaison_no}，审批通过前当前定义保持不变` }); }
    catch (error) { setNotice({ kind: "error", text: errorText(error, "提交告警定义变更失败") }); }
  }
}

function ConfirmDialog({ action, pending, errorMessage, onOpenChange, onConfirm }: { action: ConfirmAction; pending: boolean; errorMessage?: string; onOpenChange: (open: boolean) => void; onConfirm: () => void }) {
  const verb = action?.operation === "delete" ? "删除" : action?.row.enabled ? "停用" : "启用";
  return <Dialog open={Boolean(action)} onOpenChange={(open) => !pending && onOpenChange(open)}><DialogContent className="sm:max-w-md"><DialogHeader><DialogTitle>{verb}告警定义</DialogTitle><DialogDescription>确认提交“{action?.row.name ?? ""}”的{verb}审批？审批通过后才会生效。</DialogDescription></DialogHeader>{errorMessage && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{errorMessage}</div>}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="button" variant={action?.operation === "delete" ? "destructive" : "default"} disabled={pending} onClick={onConfirm}>{pending ? LOADING_SUBMITTING : "提交审批"}</Button></DialogFooter></DialogContent></Dialog>;
}

function AlertDefinitionDetailDialog({ row, onOpenChange }: { row: AlertDefinition | null; onOpenChange: (open: boolean) => void }) {
  return <Dialog open={Boolean(row)} onOpenChange={onOpenChange}><DialogContent className="sm:max-w-2xl"><DialogHeader><DialogTitle>{row?.name ?? "告警定义详情"}</DialogTitle><DialogDescription>{row?.alert_code} · {row?.event_type} · 版本 {row?.version}</DialogDescription></DialogHeader>{row && <dl className="grid gap-3 text-sm sm:grid-cols-2"><Detail label={COLUMN_STATUS} value={row.enabled ? STATUS_ENABLED : STATUS_DISABLED} /><Detail label="默认级别" value={severityLabel(row.default_severity)} /><Detail label="接收角色" value={row.recipient_roles.map(roleLabel).join("、")} /><Detail label="静默期" value={`${Math.round(row.silence_period_seconds / 60)} 分钟`} /><Detail label="升级策略" value={row.escalation_ref ?? "无"} /><Detail label="监管属性" value={row.is_gsp_forced ? "GSP 强制" : "一般"} /><div className="sm:col-span-2"><dt className="text-muted-foreground">触发条件</dt><dd><pre className="mt-1 overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs">{row.condition_expression}</pre></dd></div>{Object.entries(row.message_templates).map(([locale, template]) => <div key={locale} className="sm:col-span-2"><dt className="text-muted-foreground">消息模板 · {locale}</dt><dd className="mt-1 whitespace-pre-wrap">{template}</dd></div>)}</dl>}<DialogFooter><DialogClose asChild><Button type="button" variant="outline">关闭</Button></DialogClose></DialogFooter></DialogContent></Dialog>;
}

function Detail({ label, value }: { label: string; value: string }) { return <div><dt className="text-muted-foreground">{label}</dt><dd className="mt-1">{value}</dd></div>; }
function cannotToggle(row?: AlertDefinition) { return !row || (row.enabled && (row.is_gsp_forced || !row.is_disable_allowed)); }
function defaultQuery(): QueryPanelValue { return { keyword: "", enabled: "", severity: "" }; }
function normalizeQuery(value: unknown): QueryPanelValue { const row = queryValueFromUnknown(value); return { keyword: queryString(row.keyword), enabled: queryString(row.enabled), severity: queryString(row.severity) }; }
function toFilters(value: QueryPanelValue) { const enabled = queryString(value.enabled); return { keyword: queryString(value.keyword).trim() || undefined, severity: queryString(value.severity) || undefined, enabled: enabled ? enabled === "true" : undefined }; }
function severityLabel(value: string) { return ({ info: "提示", warning: "警告", critical: "严重" } as Record<string, string>)[value] ?? value; }
function roleLabel(value: string) { return ({ warehouse_manager: "仓库经理", maintenance_operator: "养护员", system_admin: "系统管理员", owner_contact: "货主联系人" } as Record<string, string>)[value] ?? value; }
