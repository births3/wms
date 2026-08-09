import * as React from "react";
import {
  Button, Card, CardContent, CardHeader, CardTitle, DataGrid, Dialog, DialogClose,
  DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input,
  PageHeader, QueryPanel, StatusBadge, buildQueryPanelSummaryItems, formatDateTime, type DataGridColumn,
  type DataGridRefreshAction, type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";

import {
  downloadAlertExport, useActiveAlertsQuery, useAlertActionMutation,
  useAlertStatisticsQuery, useCreateAlertExportMutation, useGspAlertReportQuery,
  type AlertInstance, type AlertInstanceFilters,
} from "@/features/alert-engine/alert-runtime-queries";
import { errorText } from "@/lib/error-text";
import { queryRange, queryString, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_REFRESH,
  COLUMN_CREATED_AT,
  COLUMN_STATUS,
  COLUMN_WAREHOUSE,
  FIELD_WAREHOUSE_ID,
  FILTER_ALL,
  LOADING_SUBMITTING,
} from "@/lib/ui-strings";
import { usePageQueryState } from "@/lib/use-page-query-state";

export const alertDashboardQueryFields: QueryPanelField[] = [
  { key: "alertCode", label: "告警类型", type: "text", placeholder: "告警编码" },
  { key: "severity", label: "级别", type: "select", options: severityOptions(true) },
  { key: "status", label: COLUMN_STATUS, type: "select", options: statusOptions(true) },
  { key: "warehouseId", label: COLUMN_WAREHOUSE, type: "text", placeholder: FIELD_WAREHOUSE_ID },
  { key: "triggeredAt", label: "触发时间", type: "dateRange" },
];
export const alertDashboardCoreQueryFieldKeys = ["alertCode", "severity", "status"];

type AlertOperation = "acknowledge" | "handling" | "close" | "ignore";
type ActionDialog = { operation: AlertOperation; alert: AlertInstance } | null;
type Notice = { kind: "success" | "error"; text: string } | null;

const alertColumns: DataGridColumn<AlertInstance>[] = [
  { key: "severity", header: "级别", width: 95, minWidth: 85, sortable: true, sortValue: (row) => severityOrder(row.severity), render: (row) => <StatusBadge status={row.severity === "critical" ? "unqualified" : row.severity === "warning" ? "pending" : "completed"} label={severityLabel(row.severity)} size="sm" /> },
  { key: "alert_name", header: "告警", width: 210, minWidth: 150, sortable: true, sortValue: (row) => row.alert_name, filterValue: (row) => `${row.alert_code} ${row.alert_name}`, copyValue: (row) => row.alert_code, filter: { type: "text" }, render: (row) => <div><div>{row.alert_name}</div><div className="font-mono text-xs text-muted-foreground">{row.alert_code}</div></div> },
  { key: "resource", header: "业务对象", width: 210, minWidth: 150, filterValue: (row) => `${row.resource_type} ${row.resource_id}`, copyValue: (row) => row.resource_id, filter: { type: "text" }, render: (row) => row.resource_path ? <a className="text-primary underline-offset-4 hover:underline" href={row.resource_path}>{row.resource_type} · {row.resource_id}</a> : <span>{row.resource_type} · {row.resource_id}</span> },
  { key: "warehouse_id", header: COLUMN_WAREHOUSE, width: 155, minWidth: 120, mono: true, render: (row) => row.warehouse_id ?? "-" },
  { key: "status", header: COLUMN_STATUS, width: 115, minWidth: 95, render: (row) => <StatusBadge status={statusTone(row.status)} label={statusLabel(row.status)} size="sm" /> },
  { key: "escalation_level", header: "升级", width: 85, minWidth: 75, sortable: true, sortValue: (row) => row.escalation_level, render: (row) => row.escalation_level ? `L${row.escalation_level}` : "未升级" },
  { key: "recipients", header: "接收人", width: 210, minWidth: 150, render: (row) => row.recipients.join("、") || "-", copyValue: (row) => row.recipients.join(",") },
  { key: "waiting", header: "已等待", width: 105, minWidth: 90, sortable: true, sortValue: (row) => Date.now() - new Date(row.triggered_at).getTime(), render: (row) => elapsedSince(row.triggered_at, row.closed_at) },
  { key: "triggered_at", header: "触发时间", width: 175, minWidth: 150, sortable: true, sortValue: (row) => row.triggered_at, filterValue: (row) => row.triggered_at, filter: { type: "dateRange" }, render: (row) => formatDateTime(row.triggered_at) },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 175, minWidth: 150, sortable: true, sortValue: (row) => row.created_at, render: (row) => formatDateTime(row.created_at) },
];

const gspColumns: DataGridColumn<AlertInstance>[] = [
  alertColumns[1], alertColumns[2], alertColumns[4], alertColumns[8], alertColumns[9],
  { key: "acknowledged_at", header: "确认时间", width: 175, minWidth: 150, render: (row) => formatOptionalDate(row.acknowledged_at) },
  { key: "closed_at", header: "关闭时间", width: 175, minWidth: 150, render: (row) => formatOptionalDate(row.closed_at) },
  { key: "close_reason", header: "关闭原因", width: 220, minWidth: 160, render: (row) => row.close_reason ?? "-" },
];

/** 页面设计契约：监控列表型页面；活动告警、统计、GSP 生命周期和导出共享同一组受控筛选条件。 */
export function AlertDashboardPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const [selected, setSelected] = React.useState<string[]>([]);
  const [action, setAction] = React.useState<ActionDialog>(null);
  const [description, setDescription] = React.useState("");
  const [recipientEmail, setRecipientEmail] = React.useState("");
  const [notice, setNotice] = React.useState<Notice>(null);
  const filters = toFilters(appliedQuery);
  const active = useActiveAlertsQuery({ ...filters, active_only: true, limit: 500 });
  const statistics = useAlertStatisticsQuery(filters);
  const gspReport = useGspAlertReportQuery({ ...filters, limit: 500 });
  const alertAction = useAlertActionMutation();
  const createExport = useCreateAlertExportMutation();
  const rows = active.data?.data ?? [];
  const selectedAlert = rows.find((row) => row.id === selected[0]);
  const refreshAction: DataGridRefreshAction = { label: BUTTON_REFRESH, description: "刷新活动告警", disabled: active.isFetching, onClick: () => void refreshAll() };

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader title="H-AL 告警看板" subtitle="活动告警每 5 秒刷新；按当前用户、货主和授权仓库隔离，严重告警优先" />
    {notice && <div role={notice.kind === "error" ? "alert" : "status"} className={notice.kind === "error" ? "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" : "rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success"}>{notice.text}</div>}
    <QueryPanel fields={alertDashboardQueryFields} defaultVisibleFieldKeys={alertDashboardCoreQueryFieldKeys} value={draftQuery} onValueChange={setDraftQuery} onQuery={() => applyQuery(draftQuery)} onReset={resetQuery} />
    <Card><CardHeader><div className="flex flex-wrap items-center justify-between gap-3"><CardTitle>活动告警</CardTitle><div className="flex flex-wrap gap-2"><Button variant="outline" disabled={!selectedAlert || alertAction.isPending} onClick={() => openAction("acknowledge")}>确认接警</Button><Button variant="outline" disabled={!selectedAlert || alertAction.isPending} onClick={() => openAction("handling")}>记录处理</Button><Button disabled={!selectedAlert || alertAction.isPending} onClick={() => openAction("close")}>关闭</Button><Button variant="destructive" disabled={!selectedAlert || alertAction.isPending} onClick={() => openAction("ignore")}>忽略</Button></div></div></CardHeader><CardContent><DataGrid storageKey="hal.active-alerts" columns={alertColumns} data={rows} rowKey={(row) => row.id} selectable selectedRowKeys={selected} onSelectedRowKeysChange={setSelected} caption={active.isPending ? "加载活动告警..." : "严重告警优先，数据最长延迟 5 秒"} emptyTitle={active.isError ? "读取活动告警失败" : "当前没有活动告警"} emptyDescription={active.isError ? errorText(active.error, "请检查权限和仓库范围") : "新告警会在触发后自动出现"} tableClassName="min-w-[1430px]" exportFileBaseName="H-AL 活动告警" refreshAction={refreshAction} queryState={appliedQuery} querySummaryItems={buildQueryPanelSummaryItems(alertDashboardQueryFields, appliedQuery)} onApplyQueryState={applyGridQueryState} onClearQueryState={clearGridQueryState} /></CardContent></Card>
    <StatisticsSection data={statistics.data} loading={statistics.isPending} error={statistics.error} />
    <Card><CardHeader><div className="flex flex-wrap items-end justify-between gap-3"><div><CardTitle>GSP 告警生命周期报表</CardTitle><p className="mt-1 text-sm text-muted-foreground">只包含强制告警，查询与导出均写入审计日志</p></div><div className="flex flex-wrap items-end gap-2"><label className="grid gap-1 text-xs text-muted-foreground">异步导出通知邮箱<Input className="w-56" type="email" placeholder="可选" value={recipientEmail} onChange={(event) => setRecipientEmail(event.target.value)} /></label><Button variant="outline" disabled={createExport.isPending} onClick={() => void exportReport("excel")}>导出 Excel</Button><Button variant="outline" disabled={createExport.isPending} onClick={() => void exportReport("pdf")}>导出 PDF</Button></div></div></CardHeader><CardContent><DataGrid storageKey="hal.gsp-alert-report" columns={gspColumns} data={(gspReport.data?.data ?? []).map((record) => record.alert)} rowKey={(row) => row.id} caption={gspReport.isPending ? "加载 GSP 生命周期..." : undefined} emptyTitle={gspReport.isError ? "读取 GSP 报表失败" : "暂无 GSP 强制告警"} emptyDescription={gspReport.isError ? errorText(gspReport.error, "请检查报表权限") : "当前筛选范围内没有记录"} tableClassName="min-w-[1200px]" exportFileBaseName="H-AL GSP 生命周期" /></CardContent></Card>
    <AlertActionDialog action={action} description={description} pending={alertAction.isPending} errorMessage={alertAction.error?.message} onDescriptionChange={setDescription} onOpenChange={(open) => !open && setAction(null)} onConfirm={() => void submitAction()} />
  </section>;

  function applyGridQueryState(value: unknown) { applyQuery(queryValueFromUnknown(value)); }
  function clearGridQueryState() { resetQuery(); }
  function openAction(operation: AlertOperation) { if (!selectedAlert) return; setDescription(""); setNotice(null); alertAction.reset(); setAction({ operation, alert: selectedAlert }); }
  async function submitAction() {
    if (!action) return;
    if (action.operation !== "acknowledge" && !description.trim()) { setNotice({ kind: "error", text: "处理、关闭或忽略必须填写说明" }); return; }
    try { await alertAction.mutateAsync({ id: action.alert.id, operation: action.operation, description: description.trim() }); setAction(null); setSelected([]); setNotice({ kind: "success", text: `${actionLabel(action.operation)}已记录并写入审计日志` }); }
    catch (error) { setNotice({ kind: "error", text: errorText(error, "告警操作失败") }); }
  }
  async function exportReport(format: "excel" | "pdf") {
    setNotice(null);
    try {
      const job = await createExport.mutateAsync({ format, filters, recipientEmail });
      if (job.download_url) { await downloadAlertExport(job.download_url, format); setNotice({ kind: "success", text: `${format.toUpperCase()} 报表已生成` }); }
      else setNotice({ kind: "success", text: `导出任务已进入队列（${job.row_count} 行），完成后通过 H4 通知` });
    } catch (error) { setNotice({ kind: "error", text: errorText(error, "导出失败") }); }
  }
  async function refreshAll() { await Promise.all([active.refetch(), statistics.refetch(), gspReport.refetch()]); }
}

function StatisticsSection({ data, loading, error }: { data?: ReturnType<typeof useAlertStatisticsQuery>["data"]; loading: boolean; error: unknown }) {
  const latest = data?.monthly.at(-1);
  return <div className="grid gap-4 xl:grid-cols-3"><MetricCard title="本月触发" value={latest ? String(latest.triggered_count) : loading ? "加载中" : "0"} detail="按筛选范围汇总" /><MetricCard title="确认率" value={latest ? percent(latest.acknowledgement_rate) : "-"} detail={latest?.average_response_seconds == null ? "暂无响应数据" : `平均响应 ${duration(latest.average_response_seconds)}`} /><MetricCard title="升级率" value={latest ? percent(latest.escalation_rate) : "-"} detail={error ? errorText(error, "统计读取失败") : "月度口径，最长查询一年"} /><RankingCard title="告警类型 Top 10" rows={data?.alert_type_top10 ?? []} /><RankingCard title="接收人 Top 10" rows={data?.recipient_top10 ?? []} /><Card><CardHeader><CardTitle>月度趋势</CardTitle></CardHeader><CardContent><div className="space-y-2 text-sm">{data?.monthly.length ? data.monthly.map((row) => <div key={row.month} className="grid grid-cols-4 gap-2 border-b pb-2"><span>{row.month}</span><span>{row.triggered_count} 次</span><span>确认 {percent(row.acknowledgement_rate)}</span><span>升级 {percent(row.escalation_rate)}</span></div>) : <p className="text-muted-foreground">暂无统计数据</p>}</div></CardContent></Card></div>;
}

function MetricCard({ title, value, detail }: { title: string; value: string; detail: string }) { return <Card><CardHeader><CardTitle className="text-sm text-muted-foreground">{title}</CardTitle></CardHeader><CardContent><div className="text-3xl font-semibold">{value}</div><p className="mt-2 text-sm text-muted-foreground">{detail}</p></CardContent></Card>; }
function RankingCard({ title, rows }: { title: string; rows: Array<{ key: string; count: number; unacknowledged_count: number }> }) { return <Card><CardHeader><CardTitle>{title}</CardTitle></CardHeader><CardContent><ol className="space-y-2 text-sm">{rows.length ? rows.map((row, index) => <li key={row.key} className="flex justify-between gap-3"><span>{index + 1}. {row.key}</span><span>{row.count} 次 / 未确认 {row.unacknowledged_count}</span></li>) : <li className="text-muted-foreground">暂无排行数据</li>}</ol></CardContent></Card>; }

function AlertActionDialog({ action, description, pending, errorMessage, onDescriptionChange, onOpenChange, onConfirm }: { action: ActionDialog; description: string; pending: boolean; errorMessage?: string; onDescriptionChange: (value: string) => void; onOpenChange: (open: boolean) => void; onConfirm: () => void }) {
  const needsDescription = action?.operation !== "acknowledge";
  return <Dialog open={Boolean(action)} onOpenChange={(open) => !pending && onOpenChange(open)}><DialogContent className="sm:max-w-lg"><DialogHeader><DialogTitle>{action ? actionLabel(action.operation) : "处理告警"}</DialogTitle><DialogDescription>{action?.alert.alert_name} · {action?.alert.resource_type} {action?.alert.resource_id}</DialogDescription></DialogHeader>{needsDescription && <label className="grid gap-1 text-sm">处理说明<textarea autoFocus rows={4} maxLength={1000} className="rounded-md border border-input bg-background px-3 py-2" value={description} onChange={(event) => onDescriptionChange(event.target.value)} /></label>}{errorMessage && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{errorMessage}</div>}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="button" variant={action?.operation === "ignore" ? "destructive" : "default"} disabled={pending || (needsDescription && !description.trim())} onClick={onConfirm}>{pending ? LOADING_SUBMITTING : "确认"}</Button></DialogFooter></DialogContent></Dialog>;
}

function defaultQuery(): QueryPanelValue { return { alertCode: "", severity: "", status: "", warehouseId: "", triggeredAt: { from: "", to: "" } }; }
function normalizeQuery(value: unknown): QueryPanelValue { const row = queryValueFromUnknown(value); return { alertCode: queryString(row.alertCode), severity: queryString(row.severity), status: queryString(row.status), warehouseId: queryString(row.warehouseId), triggeredAt: queryRange(row.triggeredAt) }; }
function toFilters(value: QueryPanelValue): AlertInstanceFilters { const dates = queryRange(value.triggeredAt); return { alert_code: optional(value.alertCode), severity: optional(value.severity), status: optional(value.status), warehouse_id: optional(value.warehouseId), from: dateBoundary(dates.from, false), to: dateBoundary(dates.to, true) }; }
function dateBoundary(value: string | undefined, end: boolean) { return value ? new Date(`${value}T${end ? "23:59:59.999" : "00:00:00"}`).toISOString() : undefined; }
function optional(value: QueryPanelValue[string]) { const result = queryString(value).trim(); return result || undefined; }
function severityOptions(all: boolean) { return [...(all ? [{ label: FILTER_ALL, value: "" }] : []), { label: "提示", value: "info" }, { label: "警告", value: "warning" }, { label: "严重", value: "critical" }]; }
function statusOptions(all: boolean) { return [...(all ? [{ label: FILTER_ALL, value: "" }] : []), ...["triggered", "notified", "notification_failed", "timed_out", "escalated", "acknowledged", "handling"].map((value) => ({ label: statusLabel(value), value }))]; }
function statusTone(value: string): "completed" | "unqualified" | "pending" | "isolated" { if (["notification_failed", "timed_out"].includes(value)) return "unqualified"; if (["acknowledged", "handling"].includes(value)) return "completed"; if (value === "escalated") return "isolated"; return "pending"; }
function statusLabel(value: string) { return ({ triggered: "已触发", notified: "已通知", notification_failed: "通知失败", timed_out: "已超时", escalated: "已升级", acknowledged: "已确认", handling: "处理中", closed: "已关闭", ignored: "已忽略" } as Record<string, string>)[value] ?? value; }
function severityLabel(value: string) { return ({ info: "提示", warning: "警告", critical: "严重" } as Record<string, string>)[value] ?? value; }
function severityOrder(value: string) { return ({ critical: 3, warning: 2, info: 1 } as Record<string, number>)[value] ?? 0; }
function actionLabel(value: AlertOperation) { return ({ acknowledge: "确认接警", handling: "记录处理", close: "关闭告警", ignore: "忽略告警" } as const)[value]; }
function percent(value: number) { return `${(value * 100).toFixed(1)}%`; }
function duration(seconds: number) { return seconds < 60 ? `${Math.round(seconds)} 秒` : `${(seconds / 60).toFixed(1)} 分钟`; }
function elapsedSince(from: string, to?: string | null) { const seconds = Math.max(0, (new Date(to ?? Date.now()).getTime() - new Date(from).getTime()) / 1000); if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟`; if (seconds < 86400) return `${(seconds / 3600).toFixed(1)} 小时`; return `${(seconds / 86400).toFixed(1)} 天`; }
function formatOptionalDate(value?: string | null) { return value ? formatDateTime(value) : "-"; }
