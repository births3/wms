import * as React from "react";
import { Button, Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, Input, ListPageTemplate, StatusBadge, type DataGridColumn, type DataGridCreateAction, type DataGridEditAction, type DataGridRefreshAction, type QueryPanelField, type QueryPanelValue } from "@wms/ui";
import type { CurrentUser } from "@/features/auth/auth-queries";
import { useSystemDictionaryItemOptionsQuery } from "@/features/master-data/master-data-queries";
import { useInventoryStatusTransitionsQuery, useUpsertInventoryStatusTransitionMutation, type InventoryStatusTransition } from "@/features/inventory/inventory-status-config-queries";
import { errorText } from "@/lib/error-text";
import { formatDateTime } from "@/lib/format";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import { BUTTON_ADD, BUTTON_REFRESH, COLUMN_CREATED_AT, COLUMN_STATUS, COLUMN_UPDATED_AT, FIELD_KEYWORD, FIELD_SCOPE, LOADING_SAVING } from "@/lib/ui-strings";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

type Form = { scope: "owner" | "global"; fromStatus: string; toStatus: string; approvalSources: string; enabled: boolean };
type Notice = { kind: "success" | "error"; text: string } | null;
const queryFields: QueryPanelField[] = [
  { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "起始状态 / 目标状态 / 审批来源" },
  { key: "scope", label: FIELD_SCOPE, type: "multiSelect", options: [{ label: "全部", value: "" }, { label: "当前货主", value: "owner" }, { label: "全局", value: "global" }] },
  { key: "status", label: COLUMN_STATUS, type: "multiSelect", options: [{ label: "全部", value: "" }, { label: "允许", value: "enabled" }, { label: "禁止", value: "disabled" }] },
];
const defaultVisibleFieldKeys = ["keyword", "scope", "status"];
const columns: DataGridColumn<InventoryStatusTransition>[] = [
  { key: "scope", header: FIELD_SCOPE, width: 110, render: (row) => row.owner_id ? "当前货主" : "全局", filterValue: (row) => row.owner_id ? "owner" : "global", copyValue: (row) => row.owner_id ? "当前货主" : "全局", filter: { type: "multiSelect", options: [{ label: "当前货主", value: "owner" }, { label: "全局", value: "global" }] } },
  { key: "from_status", header: "起始状态", width: 160, mono: true, sortable: true, filterValue: (row) => row.from_status, copyValue: (row) => row.from_status, filter: { type: "text" } },
  { key: "to_status", header: "目标状态", width: 160, mono: true, sortable: true, filterValue: (row) => row.to_status, copyValue: (row) => row.to_status, filter: { type: "text" } },
  { key: "approval_sources", header: "原因/审批要求", width: 260, render: (row) => row.approval_sources.join("、"), filterValue: (row) => row.approval_sources.join(" "), copyValue: (row) => row.approval_sources.join(", "), filter: { type: "text" } },
  { key: "enabled", header: COLUMN_STATUS, width: 100, render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? "允许" : "禁止"} size="sm" />, filterValue: (row) => row.enabled ? "enabled" : "disabled", filter: { type: "multiSelect", options: [{ label: "允许", value: "enabled" }, { label: "禁止", value: "disabled" }] } },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 180, sortable: true, sortValue: (row) => row.created_at, copyValue: (row) => row.created_at, render: (row) => formatDateTime(row.created_at), filter: { type: "dateRange" } },
  { key: "updated_at", header: COLUMN_UPDATED_AT, width: 180, sortable: true, sortValue: (row) => row.updated_at, copyValue: (row) => row.updated_at, render: (row) => formatDateTime(row.updated_at), filter: { type: "dateRange" } },
];

export function M3InventoryStatusConfigPage({ currentUser }: { currentUser: CurrentUser }) {
  const transitionsQuery = useInventoryStatusTransitionsQuery();
  const statusesQuery = useSystemDictionaryItemOptionsQuery("inventory_quality_status");
  const saveMutation = useUpsertInventoryStatusTransitionMutation();
  const [selected, setSelected] = React.useState<string[]>([]);
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const [form, setForm] = React.useState<Form>(emptyForm());
  const { open: dialogOpen, target: editing, setOpen: setDialogOpen, setTarget: setEditing } =
    useDialogState<InventoryStatusTransition>();
  const [notice, setNotice] = React.useState<Notice>(null);
  const statusOptions = (statusesQuery.data ?? []).map(([value, label]) => ({ value, label: typeof label === "string" && label.trim() ? label : value }));
  const rows = React.useMemo(() => filterRows(transitionsQuery.data?.data ?? [], appliedQuery), [appliedQuery, transitionsQuery.data]);
  const busy = saveMutation.isPending;
  const refreshAction: DataGridRefreshAction = { label: BUTTON_REFRESH, description: "刷新库存状态转换规则", disabled: transitionsQuery.isFetching, onClick: () => void transitionsQuery.refetch() };
  const createAction: DataGridCreateAction = { label: BUTTON_ADD, description: "新增库存状态转换规则", disabled: busy || statusOptions.length === 0, onClick: () => openDialog(null) };
  const editAction: DataGridEditAction = { label: "修改", description: "修改选中的库存状态转换规则", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: (ctx) => openDialog(rows.find((row) => row.id === ctx.selectedRowKeys[0]) ?? null) };
  const statusError = statusesQuery.isError ? errorText(statusesQuery.error, "库存质量状态字典读取失败") : statusesQuery.data?.length === 0 ? "没有可用的库存质量状态字典项" : null;

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validate(form, statusOptions.length > 0);
    if (error) { setNotice({ kind: "error", text: error }); return; }
    try {
      const saved = await saveMutation.mutateAsync({ fromStatus: form.fromStatus.trim(), toStatus: form.toStatus.trim(), body: { owner_id: form.scope === "global" ? null : currentUser.owner_id, approval_sources: form.approvalSources.split(",").map((value) => value.trim()).filter(Boolean), enabled: form.enabled } });
      setDialogOpen(false); setSelected([]); setNotice({ kind: "success", text: `${saved.from_status} → ${saved.to_status} 规则已保存` });
    } catch (errorValue) { setNotice({ kind: "error", text: errorText(errorValue, "保存库存状态转换规则失败") }); }
  }
  function openDialog(row: InventoryStatusTransition | null) { setNotice(null); setEditing(row); setForm(row ? { scope: row.owner_id ? "owner" : "global", fromStatus: row.from_status, toStatus: row.to_status, approvalSources: row.approval_sources.join(", "), enabled: row.enabled } : emptyForm(statusOptions[0]?.value)); setDialogOpen(true); }
  function update(key: keyof Form, value: string | boolean) { setForm((current) => ({ ...current, [key]: value })); }

  return (
    <ListPageTemplate
      notice={notice}
      banner={
        statusError ? (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">
            {statusError}
          </div>
        ) : null
      }
      queryFields={queryFields}
      coreQueryFieldKeys={defaultVisibleFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={setDraftQuery}
      onQuery={() => applyQuery(draftQuery)}
      onReset={resetQuery}
      gridProps={{
        storageKey: "m3.inventory-status-config",
        columns,
        data: rows,
        rowKey: (row) => row.id,
        selectable: true,
        selectedRowKeys: selected,
        onSelectedRowKeysChange: setSelected,
        caption: transitionsQuery.isPending ? "加载状态转换规则..." : undefined,
        emptyTitle: transitionsQuery.isError ? "读取状态转换规则失败" : "暂无状态转换规则",
        emptyDescription: transitionsQuery.isError ? errorText(transitionsQuery.error, "请检查鉴权和 API 服务") : "请新增全局或当前货主状态转换规则",
        exportFileBaseName: "M3-inventory-status-transitions",
        refreshAction,
        createAction,
        editAction,
        queryState: appliedQuery,
        onApplyQueryState: (value) => applyQuery(queryValueFromUnknown(value)),
        onClearQueryState: resetQuery,
      }}
      dialogs={
        <Dialog open={dialogOpen} onOpenChange={(open) => !busy && setDialogOpen(open)}>
          <DialogContent className="sm:max-w-xl">
            <form className="grid gap-4" onSubmit={submit}>
              <DialogHeader>
                <DialogTitle>{editing ? "修改库存状态转换规则" : "新增库存状态转换规则"}</DialogTitle>
                <DialogDescription>状态必须来自库存质量状态字典；原因/审批要求对应后端 approval_sources。</DialogDescription>
              </DialogHeader>
              <div className="grid gap-4 sm:grid-cols-2">
                <Field label={FIELD_SCOPE}>
                  <select
                    className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                    value={form.scope}
                    onChange={(event) => update("scope", event.target.value)}
                  >
                    <option value="owner">当前货主（{currentUser.owner_code}）</option>
                    <option value="global">全局</option>
                  </select>
                </Field>
                <Field label="允许转换">
                  <label className="flex h-10 items-center gap-2">
                    <input
                      type="checkbox"
                      checked={form.enabled}
                      onChange={(event) => update("enabled", event.target.checked)}
                    />
                    允许
                  </label>
                </Field>
                <Field label="起始状态">
                  <select
                    required
                    className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                    value={form.fromStatus}
                    onChange={(event) => update("fromStatus", event.target.value)}
                  >
                    <option value="">请选择</option>
                    {statusOptions.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}（{option.value}）
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="目标状态">
                  <select
                    required
                    className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                    value={form.toStatus}
                    onChange={(event) => update("toStatus", event.target.value)}
                  >
                    <option value="">请选择</option>
                    {statusOptions.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}（{option.value}）
                      </option>
                    ))}
                  </select>
                </Field>
              </div>
              <Field label="原因/审批来源">
                <Input
                  required
                  value={form.approvalSources}
                  onChange={(event) => update("approvalSources", event.target.value)}
                  placeholder="多个来源用英文逗号分隔"
                />
              </Field>
              <DialogFooter>
                <DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose>
                <Button type="submit" disabled={busy || Boolean(statusError)}>{busy ? LOADING_SAVING : "保存"}</Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
      }
    />
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) { return <label className="grid gap-1 text-sm">{label}{children}</label>; }
function defaultQuery(): QueryPanelValue { return { keyword: "", scope: "", status: "" }; }
function emptyForm(fromStatus = ""): Form { return { scope: "owner", fromStatus, toStatus: "", approvalSources: "", enabled: true }; }
function normalizeQuery(value: QueryPanelValue): QueryPanelValue { return { keyword: queryString(value.keyword), scope: queryString(value.scope), status: queryString(value.status) }; }
function filterRows(rows: InventoryStatusTransition[], query: QueryPanelValue) { const keyword = queryString(query.keyword).trim().toLowerCase(); const scope = queryString(query.scope); const status = queryString(query.status); return rows.filter((row) => (!keyword || [row.from_status, row.to_status, ...row.approval_sources].join(" ").toLowerCase().includes(keyword)) && (!scope || (row.owner_id ? "owner" : "global") === scope) && (!status || (row.enabled ? "enabled" : "disabled") === status)); }
function validate(form: Form, hasStatuses: boolean) { if (!hasStatuses) return "库存质量状态字典不可用，不能保存规则"; if (!form.fromStatus || !form.toStatus) return "起始状态和目标状态不能为空"; if (form.fromStatus === form.toStatus) return "起始状态和目标状态不能相同"; if (!form.approvalSources.split(",").some((value) => value.trim())) return "至少填写一个原因/审批来源"; return null; }
