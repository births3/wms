import * as React from "react";
import {
  Button, Card, CardContent, DataGrid, Dialog, DialogClose, DialogContent, DialogDescription,
  DialogFooter, DialogHeader, DialogTitle, Input, PageHeader, QueryPanel, StatusBadge,
  buildQueryPanelSummaryItems, cn, type DataGridColumn, type DataGridCreateAction,
  type DataGridEditAction, type DataGridRefreshAction, type DataGridToolbarAction,
  type QueryPanelField, type QueryPanelValue,
} from "@wms/ui";
import { Pencil, PlugZap } from "lucide-react";

import type { CurrentUser } from "@/features/auth/auth-queries";
import {
  useChangeDrugInspectionPlatformStatusMutation, useDrugInspectionPlatformsQuery,
  useUpsertDrugInspectionPlatformMutation, type DrugInspectionPlatform,
  type UpsertDrugInspectionPlatformRequest,
} from "@/features/drug-inspection/drug-inspection-queries";
import { BUTTON_REFRESH, COLUMN_CREATED_AT, COLUMN_STATUS, COLUMN_UPDATED_AT, FILTER_ALL, LOADING_SAVING, STATUS_DISABLED } from "@/lib/ui-strings";

const queryFields: QueryPanelField[] = [{
  key: "status", label: "平台状态", type: "multiSelect", options: [
    { label: FILTER_ALL, value: "" }, { label: "已对接", value: "connected" },
    { label: "测试中", value: "testing" }, { label: STATUS_DISABLED, value: "disabled" },
  ],
}];
const mDiPlatformCoreQueryFieldKeys = ["status"];
const statuses = ["connected", "testing", "disabled"] as const;
const authMethods = ["api_key", "username_password"] as const;
const columns: DataGridColumn<DrugInspectionPlatform>[] = [
  textColumn("platform_code", "平台编码", 150), textColumn("platform_name", "平台名称", 180),
  textColumn("api_url", "API 地址", 280),
  { key: "auth_method", header: "认证方式", width: 150, render: (row) => authMethodLabel(row.auth_method) },
  { key: "credentials", header: "凭证", width: 150, render: (row) => credentialLabel(row) },
  { key: "timeout_seconds", header: "超时（秒）", width: 120, render: (row) => row.timeout_seconds },
  { key: "status", header: COLUMN_STATUS, width: 120, render: (row) => <StatusBadge status={statusVariant(row.status)} label={statusLabel(row.status)} size="sm" /> },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 180, render: (row) => formatDateTime(row.created_at) },
  { key: "updated_at", header: COLUMN_UPDATED_AT, width: 180, render: (row) => formatDateTime(row.updated_at) },
];

type Notice = { type: "success" | "error"; text: string } | null;
type Form = {
  platform_code: string; platform_name: string; api_url: string; auth_method: string;
  api_key_alias: string; username: string; password_alias: string; timeout_seconds: string; status: string;
};

export function DrugInspectionPlatformPage({ currentUser }: { currentUser: CurrentUser }) {
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>({ status: "" });
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>({ status: "" });
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [dialog, setDialog] = React.useState<"create" | "edit" | "status" | null>(null);
  const [form, setForm] = React.useState<Form>(defaultForm());
  const [notice, setNotice] = React.useState<Notice>(null);
  const status = queryString(appliedQuery.status);
  const platformsQuery = useDrugInspectionPlatformsQuery(status);
  const upsertMutation = useUpsertDrugInspectionPlatformMutation();
  const statusMutation = useChangeDrugInspectionPlatformStatusMutation();
  const rows = platformsQuery.data?.data ?? [];
  const selected = rows.find((row) => row.id === selectedRowKeys[0]);
  const busy = upsertMutation.isPending || statusMutation.isPending;
  const querySummaryItems = React.useMemo(() => buildQueryPanelSummaryItems(queryFields, appliedQuery), [appliedQuery]);

  const refreshAction: DataGridRefreshAction = { label: BUTTON_REFRESH, description: "刷新当前货主药检平台列表", disabled: platformsQuery.isFetching, onClick: () => void refresh() };
  const createAction: DataGridCreateAction = { label: "新增平台", description: "新增药检平台对接配置", disabled: busy, onClick: () => openCreate() };
  const editAction: DataGridEditAction = { label: "编辑", description: "编辑选中的药检平台配置", disabled: (context) => context.selectedRowKeys.length !== 1 || busy, onClick: () => selected && openEdit(selected) };
  const toolbarActions: DataGridToolbarAction[] = [{
    key: "status", label: "维护状态", description: "维护选中平台状态", icon: <PlugZap className="size-4" aria-hidden />,
    disabled: (context) => context.selectedRowKeys.length !== 1 || busy,
    onClick: () => selected && openStatus(selected),
  }];

  async function refresh() {
    const result = await platformsQuery.refetch();
    setNotice(result.error ? { type: "error", text: errorMessage(result.error, "读取药检平台列表失败") } : { type: "success", text: "药检平台列表已刷新" });
  }
  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validateDrugInspectionForm(form);
    if (error) { setNotice({ type: "error", text: error }); return; }
    try {
      await upsertMutation.mutateAsync(toRequest(form));
      setDialog(null); setSelectedRowKeys([]); setNotice({ type: "success", text: "药检平台配置已保存" });
    } catch (error) { setNotice({ type: "error", text: errorMessage(error, "保存药检平台配置失败") }); }
  }
  async function submitStatus(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selected) return;
    try {
      await statusMutation.mutateAsync({ id: selected.id, body: { status: form.status } });
      setDialog(null); setSelectedRowKeys([]); setNotice({ type: "success", text: "药检平台状态已更新" });
    } catch (error) { setNotice({ type: "error", text: errorMessage(error, "更新药检平台状态失败") }); }
  }
  function update(key: keyof Form, value: string) { setForm((current) => ({ ...current, [key]: value })); }
  function openCreate() { setNotice(null); setForm(defaultForm()); setDialog("create"); }
  // 状态弹窗必须回填选中平台的当前状态，否则用户未改动直接保存会把平台改成 defaultForm 的 testing
  function openStatus(row: DrugInspectionPlatform) { setNotice(null); setForm({ ...defaultForm(), status: row.status }); setDialog("status"); }
  function openEdit(row: DrugInspectionPlatform) {
    setNotice(null); setForm({ ...defaultForm(), platform_code: row.platform_code, platform_name: row.platform_name, api_url: row.api_url, auth_method: row.auth_method, username: row.username ?? "", timeout_seconds: String(row.timeout_seconds), status: row.status }); setDialog("edit");
  }

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader title="M-DI 药检平台对接配置" subtitle={`当前货主 ${currentUser.owner_code} · 列表由真实药检平台配置 API 返回`} />
    {notice && <div className={cn("rounded-md border px-3 py-2 text-sm", notice.type === "success" ? "border-wms-success/30 bg-wms-success/10 text-wms-success" : "border-destructive/30 bg-destructive/10 text-destructive")} role={notice.type === "success" ? "status" : "alert"}>{notice.text}</div>}
    <QueryPanel fields={queryFields} defaultVisibleFieldKeys={mDiPlatformCoreQueryFieldKeys} value={draftQuery} onValueChange={setDraftQuery} onQuery={() => setAppliedQuery(draftQuery)} onReset={() => { setDraftQuery({ status: "" }); setAppliedQuery({ status: "" }); }} />
    <Card className="rounded-lg shadow-sm"><CardContent className="p-5"><DataGrid storageKey="m-di.drug-inspection-platforms" columns={columns} data={rows} rowKey={(row) => row.id} selectable selectedRowKeys={selectedRowKeys} onSelectedRowKeysChange={setSelectedRowKeys} caption={platformsQuery.isPending ? "加载药检平台..." : undefined} emptyTitle={platformsQuery.isError ? "读取药检平台失败" : "暂无药检平台配置"} emptyDescription={platformsQuery.isError ? errorMessage(platformsQuery.error, "请检查鉴权和数据库连接") : "请新增药检平台对接配置"} refreshAction={refreshAction} createAction={createAction} editAction={editAction} toolbarActions={toolbarActions} queryState={appliedQuery} querySummaryItems={querySummaryItems} onApplyQueryState={(value) => { const next = { status: queryString((value as QueryPanelValue).status) }; setDraftQuery(next); setAppliedQuery(next); }} onClearQueryState={() => { setDraftQuery({ status: "" }); setAppliedQuery({ status: "" }); }} /></CardContent></Card>
    {dialog && <Dialog open onOpenChange={(open) => !busy && !open && setDialog(null)}><DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
      {dialog === "status" ? <form className="grid gap-4" onSubmit={submitStatus}><DialogHeader><DialogTitle>维护药检平台状态</DialogTitle><DialogDescription>{selected?.platform_name}。状态变更会写入审计追踪。</DialogDescription></DialogHeader><Field label={COLUMN_STATUS}><select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.status} onChange={(event) => update("status", event.target.value)}><option value="connected">已对接</option><option value="testing">测试中</option><option value="disabled">{STATUS_DISABLED}</option></select></Field><DialogFooter><DialogClose asChild><Button type="button" variant="outline">取消</Button></DialogClose><Button type="submit" disabled={busy}>{busy ? LOADING_SAVING : "保存状态"}</Button></DialogFooter></form> : <form className="grid gap-4" onSubmit={submit}><DialogHeader><DialogTitle>{dialog === "create" ? "新增药检平台" : "编辑药检平台"}</DialogTitle><DialogDescription>凭证只提交 Vault 引用；列表和编辑表单不会回显敏感值。编辑时请重新填写凭证引用。</DialogDescription></DialogHeader><div className="grid gap-4 sm:grid-cols-2"><Field label="平台编码"><Input required value={form.platform_code} onChange={(event) => update("platform_code", event.target.value)} /></Field><Field label="平台名称"><Input required value={form.platform_name} onChange={(event) => update("platform_name", event.target.value)} /></Field><Field label="API 地址" className="sm:col-span-2"><Input required type="url" placeholder="https://inspection.example/api" value={form.api_url} onChange={(event) => update("api_url", event.target.value)} /></Field><Field label="认证方式"><select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.auth_method} onChange={(event) => update("auth_method", event.target.value)}><option value="api_key">API Key</option><option value="username_password">账号密码</option></select></Field><Field label="超时（秒）"><Input required type="number" min="1" max="300" value={form.timeout_seconds} onChange={(event) => update("timeout_seconds", event.target.value)} /></Field>{form.auth_method === "api_key" ? <Field label="API Key Vault 引用" className="sm:col-span-2"><Input required placeholder="vault://wms/di/platform/api-key" value={form.api_key_alias} onChange={(event) => update("api_key_alias", event.target.value)} /></Field> : <><Field label="账号"><Input required value={form.username} onChange={(event) => update("username", event.target.value)} /></Field><Field label="密码 Vault 引用"><Input required placeholder="vault://wms/di/platform/password" value={form.password_alias} onChange={(event) => update("password_alias", event.target.value)} /></Field></>}<Field label={COLUMN_STATUS}><select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.status} onChange={(event) => update("status", event.target.value)}><option value="connected">已对接</option><option value="testing">测试中</option><option value="disabled">{STATUS_DISABLED}</option></select></Field></div><DialogFooter><DialogClose asChild><Button type="button" variant="outline">取消</Button></DialogClose><Button type="submit" disabled={busy}>{busy ? LOADING_SAVING : "保存配置"}</Button></DialogFooter></form>}
    </DialogContent></Dialog>}
  </section>;
}

export function validateDrugInspectionForm(form: Form) {
  if (!form.platform_code.trim() || !form.platform_name.trim()) return "平台编码和平台名称不能为空";
  try { const url = new URL(form.api_url); if (!/^https?:$/.test(url.protocol) || !url.hostname) return "API 地址必须是带主机的 HTTP 或 HTTPS 地址"; } catch { return "API 地址必须是带主机的 HTTP 或 HTTPS 地址"; }
  if (!authMethods.includes(form.auth_method as (typeof authMethods)[number])) return "认证方式无效";
  const timeout = Number(form.timeout_seconds); if (!Number.isInteger(timeout) || timeout < 1 || timeout > 300) return "超时必须在 1 到 300 秒之间";
  if (!statuses.includes(form.status as (typeof statuses)[number])) return "平台状态无效";
  if (form.auth_method === "api_key" && !isVaultRef(form.api_key_alias)) return "API Key 必须使用 Vault 引用";
  if (form.auth_method === "username_password" && (!form.username.trim() || !isVaultRef(form.password_alias))) return "账号和密码 Vault 引用不能为空";
  return null;
}

function toRequest(form: Form): UpsertDrugInspectionPlatformRequest { return { platform_code: form.platform_code.trim(), platform_name: form.platform_name.trim(), api_url: form.api_url.trim(), auth_method: form.auth_method, api_key_alias: form.auth_method === "api_key" ? form.api_key_alias.trim() : null, username: form.auth_method === "username_password" ? form.username.trim() : null, password_alias: form.auth_method === "username_password" ? form.password_alias.trim() : null, timeout_seconds: Number(form.timeout_seconds), status: form.status }; }
function defaultForm(): Form { return { platform_code: "", platform_name: "", api_url: "", auth_method: "api_key", api_key_alias: "", username: "", password_alias: "", timeout_seconds: "30", status: "testing" }; }
function isVaultRef(value: string) { return /^vault:\/\/\S+$/.test(value.trim()); }
function textColumn(key: keyof DrugInspectionPlatform, header: string, width: number): DataGridColumn<DrugInspectionPlatform> { return { key: String(key), header, width, render: (row) => <span className={key === "platform_code" ? "font-mono" : undefined}>{String(row[key] ?? "-")}</span> }; }
function authMethodLabel(value: string) { return value === "api_key" ? "API Key" : value === "username_password" ? "账号密码" : value; }
function credentialLabel(row: DrugInspectionPlatform) { return row.auth_method === "api_key" ? row.api_key_configured ? "API Key 已配置" : "未配置" : row.password_configured ? "密码已配置" : "未配置"; }
function statusLabel(value: string) { return value === "connected" ? "已对接" : value === "testing" ? "测试中" : STATUS_DISABLED; }
function statusVariant(value: string): "completed" | "isolated" | "expired" { return value === "connected" ? "completed" : value === "testing" ? "isolated" : "expired"; }
function queryString(value: QueryPanelValue[string]) { return typeof value === "string" ? value : ""; }
function formatDateTime(value: string) { const date = new Date(value); return Number.isNaN(date.getTime()) ? "-" : new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(date); }
function errorMessage(error: unknown, fallback: string) { return error instanceof Error ? error.message : fallback; }
function Field({ label, children, className }: { label: string; children: React.ReactNode; className?: string }) { return <label className={cn("grid gap-1 text-sm", className)}>{label}{children}</label>; }
