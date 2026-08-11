import * as React from "react";
import {
  Button, Card, CardContent, DataGrid, Dialog, DialogClose, DialogContent, DialogDescription,
  DialogFooter, DialogHeader, DialogTitle, Input, PageHeader, QueryPanel, Select, SelectContent,
  SelectItem, SelectTrigger, SelectValue, StatusBadge, buildQueryPanelSummaryItems, formatDateTime,
  type DataGridColumn, type DataGridCreateAction, type DataGridEditAction,
  type DataGridRefreshAction, type QueryPanelField, type QueryPanelValue,
} from "@wms/ui";

import {
  useTaskEngineWarehousesQuery, useTaskEngineWarehouseZonesQuery, useTaskGroupsQuery,
  useTaskWorkersQuery, useUpsertTaskGroupMutation,
  type TaskGroup, type UpsertTaskGroupRequest,
} from "@/features/task-engine/task-engine-queries";
import { useTaskTypesQuery } from "@/features/task-engine/task-type-queries";
import { errorText } from "@/lib/error-text";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

const mteTaskGroupQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "任务组编码 / 名称" },
  { key: "status", label: "状态", type: "multiSelect", options: [{ label: "全部", value: "" }, { label: "启用", value: "enabled" }, { label: "停用", value: "disabled" }] },
  { key: "warehouseId", label: "仓库 ID", type: "text", placeholder: "按 warehouse_id 筛选" },
];
const mteTaskGroupCoreQueryFieldKeys = ["keyword", "status"];

type FormState = {
  code: string;
  name: string;
  warehouseId: string;
  zoneIds: string[];
  taskTypeCodes: string[];
  memberUserIds: string[];
  memberQualifications: Array<{ userId: string; validUntil: string; maxActiveTasks: string }>;
  enabled: boolean;
};
type Notice = { kind: "success" | "error"; text: string } | null;

/**
 * 页面设计契约：配置型列表；主信息载体为 QueryPanel + DataGrid；标准新增、编辑、刷新动作放在 DataGrid；
 * 任务类型与成员资格在 Dialog 中维护；不常驻成员明细、审计轨迹或写入表单。
 */
export function TaskGroupConfigPage() {
  const groupsQuery = useTaskGroupsQuery();
  const taskTypesQuery = useTaskTypesQuery();
  const warehousesQuery = useTaskEngineWarehousesQuery();
  const zonesQuery = useTaskEngineWarehouseZonesQuery();
  const usersQuery = useTaskWorkersQuery();
  const save = useUpsertTaskGroupMutation();
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const [selected, setSelected] = React.useState<string[]>([]);
  const {
    open: dialogOpen,
    target: editing,
    setOpen: setDialogOpen,
    setTarget: setEditing,
  } = useDialogState<TaskGroup>();
  const [form, setForm] = React.useState<FormState>(emptyForm);
  const [notice, setNotice] = React.useState<Notice>(null);
  const warehouseNames = new Map((warehousesQuery.data ?? []).map((item) => [item.id, `${item.warehouse_code} · ${item.warehouse_name}`]));
  const rows = (groupsQuery.data?.data ?? []).filter((row) => {
    const keyword = queryString(appliedQuery.keyword).trim().toLowerCase();
    const status = queryString(appliedQuery.status);
    const warehouseId = queryString(appliedQuery.warehouseId).trim();
    return (!keyword || `${row.task_group_code} ${row.task_group_name}`.toLowerCase().includes(keyword))
      && (!status || (status === "enabled" ? row.enabled : !row.enabled))
      && (!warehouseId || row.warehouse_id === warehouseId);
  });
  const selectedRow = rows.find((row) => row.id === selected[0]);
  const columns = React.useMemo<DataGridColumn<TaskGroup>[]>(() => [
    { key: "task_group_code", header: "任务组编码", width: 190, minWidth: 150, mono: true, sortable: true, sortValue: (row) => row.task_group_code, filterValue: (row) => row.task_group_code, copyValue: (row) => row.task_group_code, filter: { type: "text" } },
    { key: "task_group_name", header: "任务组名称", width: 180, minWidth: 140, sortable: true, sortValue: (row) => row.task_group_name, filterValue: (row) => row.task_group_name, copyValue: (row) => row.task_group_name, filter: { type: "text" } },
    { key: "warehouse_id", header: "适用仓库", width: 220, minWidth: 170, render: (row) => warehouseNames.get(row.warehouse_id) ?? row.warehouse_id, copyValue: (row) => row.warehouse_id },
    { key: "zone_ids", header: "适用库区", width: 110, minWidth: 90, render: (row) => row.zone_ids.length > 0 ? `${row.zone_ids.length} 个` : "全仓" },
    { key: "task_type_codes", header: "任务类型", width: 220, minWidth: 160, render: (row) => row.task_type_codes.join("、"), copyValue: (row) => row.task_type_codes.join(",") },
    { key: "member_user_ids", header: "有效成员", width: 110, minWidth: 90, sortable: true, sortValue: (row) => row.member_user_ids.length, render: (row) => `${row.member_user_ids.length} 人` },
    { key: "enabled", header: "状态", width: 100, minWidth: 90, render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? "启用" : "停用"} size="sm" /> },
    { key: "created_at", header: "创建时间", width: 180, minWidth: 150, sortable: true, sortValue: (row) => row.created_at, render: (row) => formatDateTime(row.created_at) },
    { key: "updated_at", header: "更新时间", width: 180, minWidth: 150, sortable: true, sortValue: (row) => row.updated_at, render: (row) => formatDateTime(row.updated_at) },
  ], [warehouseNames]);
  const refreshAction: DataGridRefreshAction = { label: "刷新", description: "刷新任务组与人员资格", disabled: groupsQuery.isFetching, onClick: () => void groupsQuery.refetch() };
  const createAction: DataGridCreateAction = { label: "新增任务组", description: "按仓库、类型与人员配置任务组", disabled: save.isPending, onClick: () => openDialog(null) };
  const editAction: DataGridEditAction = { label: "编辑资格", description: "维护选中任务组的适用范围与成员", disabled: (context) => context.selectedRowKeys.length !== 1 || save.isPending, onClick: () => selectedRow && openDialog(selectedRow) };

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader title="M-TE 任务组与人员资格" />
    {notice && <div className={notice.kind === "error" ? "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" : "rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success"} role={notice.kind === "error" ? "alert" : "status"}>{notice.text}</div>}
    <QueryPanel fields={mteTaskGroupQueryFields} defaultVisibleFieldKeys={mteTaskGroupCoreQueryFieldKeys} value={draftQuery} onValueChange={setDraftQuery} onQuery={() => applyQuery(draftQuery)} onReset={resetQuery} />
    <Card><CardContent className="p-5"><DataGrid storageKey="mte.task-groups" columns={columns} data={rows} rowKey={(row) => row.id} selectable selectedRowKeys={selected} onSelectedRowKeysChange={setSelected} caption={groupsQuery.isPending ? "加载任务组..." : undefined} emptyTitle={groupsQuery.isError ? "读取任务组失败" : "暂无任务组"} emptyDescription={groupsQuery.isError ? errorText(groupsQuery.error, "请检查鉴权和 API 服务") : "业务创建首个任务时会生成仓库默认任务组，也可手工新增"} refreshAction={refreshAction} createAction={createAction} editAction={editAction} exportFileBaseName="M-TE-task-groups" queryState={appliedQuery} querySummaryItems={buildQueryPanelSummaryItems(mteTaskGroupQueryFields, appliedQuery)} onApplyQueryState={applyGridQueryState} onClearQueryState={clearGridQueryState} /></CardContent></Card>
    <TaskGroupDialog open={dialogOpen} editing={editing} form={form} pending={save.isPending} errorMessage={save.error?.message} warehouses={warehousesQuery.data ?? []} zones={zonesQuery.data ?? []} taskTypes={taskTypesQuery.data?.data ?? []} users={usersQuery.data?.data ?? []} onOpenChange={setDialogOpen} onFormChange={setForm} onSubmit={submit} />
  </section>;

  function applyGridQueryState(value: unknown) { applyQuery(queryValueFromUnknown(value)); }
  function clearGridQueryState() { resetQuery(); }
  function openDialog(row: TaskGroup | null) {
    setEditing(row); setForm(row ? formFor(row) : emptyForm()); setDialogOpen(true); setNotice(null);
  }
  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validate(form, Boolean(editing));
    if (error) { setNotice({ kind: "error", text: error }); return; }
    const body: UpsertTaskGroupRequest = {
      task_group_name: form.name.trim(), warehouse_id: form.warehouseId, zone_ids: form.zoneIds,
      task_type_codes: form.taskTypeCodes, member_user_ids: form.memberUserIds,
      member_qualifications: form.memberQualifications.map((item) => ({
        user_id: item.userId,
        valid_until: item.validUntil ? new Date(item.validUntil).toISOString() : null,
        max_active_tasks: item.maxActiveTasks ? Number(item.maxActiveTasks) : null,
      })),
      enabled: form.enabled,
    };
    try {
      await save.mutateAsync({ code: form.code.trim().toLowerCase(), body });
      setDialogOpen(false); setSelected([]); setNotice({ kind: "success", text: `任务组 ${form.code.trim().toLowerCase()} 已保存` });
    } catch { /* mutation error remains visible in dialog */ }
  }
}

function TaskGroupDialog({ open, editing, form, pending, errorMessage, warehouses, zones, taskTypes, users, onOpenChange, onFormChange, onSubmit }: {
  open: boolean; editing: TaskGroup | null; form: FormState; pending: boolean; errorMessage?: string;
  warehouses: Array<{ id: string; warehouse_code: string; warehouse_name: string }>;
  zones: Array<{ id: string; warehouse_id: string; zone_code: string; zone_name: string; status: string }>;
  taskTypes: Array<{ task_type_code: string; task_type_name: string; enabled: boolean }>;
  users: Array<{ user_id: string; username: string; display_name: string }>;
  onOpenChange: (open: boolean) => void; onFormChange: (form: FormState) => void; onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
}) {
  const patch = (value: Partial<FormState>) => onFormChange({ ...form, ...value });
  const toggle = (key: "zoneIds" | "taskTypeCodes", value: string, checked: boolean) => patch({ [key]: checked ? [...form[key], value] : form[key].filter((item) => item !== value) });
  const toggleMember = (userId: string, checked: boolean) => patch(checked ? {
    memberUserIds: [...form.memberUserIds, userId],
    memberQualifications: [...form.memberQualifications, { userId, validUntil: "", maxActiveTasks: "" }],
  } : {
    memberUserIds: form.memberUserIds.filter((item) => item !== userId),
    memberQualifications: form.memberQualifications.filter((item) => item.userId !== userId),
  });
  const patchQualification = (userId: string, value: Partial<{ validUntil: string; maxActiveTasks: string }>) => patch({
    memberQualifications: form.memberQualifications.map((item) => item.userId === userId ? { ...item, ...value } : item),
  });
  const warehouseZones = zones.filter((zone) => zone.warehouse_id === form.warehouseId && (zone.status === "active" || form.zoneIds.includes(zone.id)));
  return <Dialog open={open} onOpenChange={(value) => !pending && onOpenChange(value)}><DialogContent className="max-h-[85vh] overflow-y-auto sm:max-w-2xl"><form className="grid gap-4" onSubmit={onSubmit}><DialogHeader><DialogTitle>{editing ? "编辑任务组资格" : "新增任务组"}</DialogTitle><DialogDescription>分派时同时校验货主、仓库、任务类型和成员资格。</DialogDescription></DialogHeader>
    <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1 text-sm">任务组编码<Input required readOnly={Boolean(editing)} pattern="[A-Za-z0-9][A-Za-z0-9_.-]{0,63}" value={form.code} onChange={(event) => patch({ code: event.target.value })} /></label><label className="grid gap-1 text-sm">任务组名称<Input required maxLength={128} value={form.name} onChange={(event) => patch({ name: event.target.value })} /></label></div>
    <label className="grid gap-1 text-sm">适用仓库<Select value={form.warehouseId} onValueChange={(warehouseId) => patch({ warehouseId, zoneIds: [] })} disabled={Boolean(editing)}><SelectTrigger><SelectValue placeholder="请选择仓库" /></SelectTrigger><SelectContent>{warehouses.map((warehouse) => <SelectItem key={warehouse.id} value={warehouse.id}>{warehouse.warehouse_code} · {warehouse.warehouse_name}</SelectItem>)}</SelectContent></Select></label>
    <fieldset className="grid gap-2 rounded-md border p-3"><legend className="px-1 text-sm font-medium">适用库区</legend><div className="grid gap-2 sm:grid-cols-2">{warehouseZones.map((zone) => <label key={zone.id} className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.zoneIds.includes(zone.id)} onChange={(event) => toggle("zoneIds", zone.id, event.target.checked)} />{zone.zone_code} · {zone.zone_name}</label>)}</div>{warehouseZones.length === 0 && <div className="text-sm text-muted-foreground">未选择表示全仓适用。</div>}</fieldset>
    <fieldset className="grid gap-2 rounded-md border p-3"><legend className="px-1 text-sm font-medium">适用任务类型</legend><div className="grid gap-2 sm:grid-cols-2">{taskTypes.filter((item) => item.enabled || form.taskTypeCodes.includes(item.task_type_code)).map((item) => <label key={item.task_type_code} className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.taskTypeCodes.includes(item.task_type_code)} onChange={(event) => toggle("taskTypeCodes", item.task_type_code, event.target.checked)} />{item.task_type_name}（{item.task_type_code}）</label>)}</div></fieldset>
    <fieldset className="grid gap-3 rounded-md border p-3"><legend className="px-1 text-sm font-medium">任务组成员</legend><div className="grid max-h-48 gap-2 overflow-y-auto sm:grid-cols-2">{users.map((user) => <label key={user.user_id} className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.memberUserIds.includes(user.user_id)} onChange={(event) => toggleMember(user.user_id, event.target.checked)} />{user.display_name}（{user.username}）</label>)}</div>
      {form.memberQualifications.length > 0 && <div className="grid gap-3 border-t pt-3">{form.memberQualifications.map((qualification) => {
        const user = users.find((item) => item.user_id === qualification.userId);
        const name = user?.display_name ?? qualification.userId;
        return <div key={qualification.userId} className="grid gap-2 rounded-md bg-muted/30 p-3 sm:grid-cols-2"><div className="text-sm font-medium sm:col-span-2">{name}</div><label className="grid gap-1 text-sm">{name} 资格有效期<Input type="datetime-local" value={qualification.validUntil} onChange={(event) => patchQualification(qualification.userId, { validUntil: event.target.value })} /></label><label className="grid gap-1 text-sm">{name} 同时在手上限<Input type="number" min={1} step={1} placeholder="不限" value={qualification.maxActiveTasks} onChange={(event) => patchQualification(qualification.userId, { maxActiveTasks: event.target.value })} /></label></div>;
      })}</div>}
    </fieldset>
    <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={form.enabled} onChange={(event) => patch({ enabled: event.target.checked })} />启用任务组</label>
    {errorMessage && <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">{errorMessage}</div>}<DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose><Button type="submit" disabled={pending || !form.warehouseId || form.taskTypeCodes.length === 0}>{pending ? "保存中..." : "保存"}</Button></DialogFooter>
  </form></DialogContent></Dialog>;
}

function emptyForm(): FormState { return { code: "", name: "", warehouseId: "", zoneIds: [], taskTypeCodes: [], memberUserIds: [], memberQualifications: [], enabled: true }; }
function formFor(row: TaskGroup): FormState { return { code: row.task_group_code, name: row.task_group_name, warehouseId: row.warehouse_id, zoneIds: row.zone_ids, taskTypeCodes: row.task_type_codes, memberUserIds: row.member_user_ids, memberQualifications: row.member_qualifications.map((item) => ({ userId: item.user_id, validUntil: toDateTimeLocal(item.valid_until), maxActiveTasks: item.max_active_tasks?.toString() ?? "" })), enabled: row.enabled }; }
function validate(form: FormState, editing: boolean) { if (!editing && !/^[A-Za-z0-9][A-Za-z0-9_.-]{0,63}$/.test(form.code.trim())) return "任务组编码非法"; if (!form.name.trim() || !form.warehouseId || form.taskTypeCodes.length === 0) return "请填写名称、仓库并至少选择一个任务类型"; if (form.memberQualifications.some((item) => item.maxActiveTasks && (!Number.isInteger(Number(item.maxActiveTasks)) || Number(item.maxActiveTasks) <= 0))) return "同时在手上限必须为正整数"; return null; }
function defaultQuery(): QueryPanelValue { return { keyword: "", status: "", warehouseId: "" }; }
function normalizeQuery(value: unknown): QueryPanelValue { const row = queryValueFromUnknown(value); return { keyword: queryString(row.keyword), status: queryString(row.status), warehouseId: queryString(row.warehouseId) }; }
function toDateTimeLocal(value?: string | null) { if (!value) return ""; const date = new Date(value); const offset = date.getTimezoneOffset() * 60_000; return new Date(date.getTime() - offset).toISOString().slice(0, 16); }
