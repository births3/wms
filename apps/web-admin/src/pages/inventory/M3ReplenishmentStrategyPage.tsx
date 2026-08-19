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
  type DataGridCreateAction,
  type DataGridDisableAction,
  type DataGridEditAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import {
  useBindReplenishmentLocationsMutation,
  useCreateReplenishmentStrategyMutation,
  useDisableReplenishmentStrategyMutation,
  usePreviewReplenishmentStrategyMutation,
  useReplenishmentStrategiesQuery,
  useUpdateReplenishmentStrategyMutation,
  useUpsertReplenishmentLocationGroupMutation,
  type ReplenishmentPreviewItem,
  type ReplenishmentStrategy,
  type UpsertReplenishmentStrategyRequest,
} from "@/features/replenishment/replenishment-strategy-queries";
import { errorText } from "@/lib/error-text";
import { queryString, queryStringArray, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_ADD,
  BUTTON_REFRESH,
  BUTTON_SAVE,
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
import { isUuid } from "@/lib/uuid";

export const queryFields: QueryPanelField[] = [
  { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "策略编码 / 名称" },
  {
    key: "enabled",
    label: COLUMN_STATUS,
    type: "multiSelect",
    options: [
      { label: FILTER_ALL, value: "" },
      { label: STATUS_ENABLED, value: "true" },
      { label: STATUS_DISABLED, value: "false" },
    ],
  },
  {
    key: "scope_type",
    label: "范围类型",
    type: "multiSelect",
    options: [
      { label: FILTER_ALL, value: "" },
      { label: "商品", value: "product" },
      { label: "品类", value: "category" },
      { label: "库位组", value: "location_group" },
    ],
  },
  {
    key: "target_type",
    label: "目标形态",
    type: "multiSelect",
    options: [
      { label: FILTER_ALL, value: "" },
      { label: "整箱拣选", value: "case_pick" },
      { label: "拆零拣选", value: "piece_pick" },
    ],
  },
];
export const defaultVisibleFieldKeys = ["keyword", "enabled"];

type Form = {
  strategyCode: string;
  strategyName: string;
  scopeType: string;
  scopeRef: string;
  sourceType: string;
  targetType: string;
  minSafety: string;
  maxTarget: string;
  triggerMinMax: boolean;
  triggerWaveGap: boolean;
  enabled: boolean;
  locationIds: string;
};
type GroupForm = { groupCode: string; groupName: string; locationIds: string; enabled: boolean };
type Notice = { kind: "success" | "error"; text: string } | null;

const columns: DataGridColumn<ReplenishmentStrategy>[] = [
  { key: "strategy_code", header: "策略编码", width: 140, mono: true, sortable: true, filterValue: (row) => row.strategy_code, copyValue: (row) => row.strategy_code, filter: { type: "text" } },
  { key: "strategy_name", header: "策略名称", width: 160, sortable: true, filterValue: (row) => row.strategy_name, copyValue: (row) => row.strategy_name, filter: { type: "text" } },
  { key: "scope_type", header: "范围", width: 110, filterValue: (row) => row.scope_type, copyValue: (row) => row.scope_type, filter: { type: "multiSelect", options: [{ label: "商品", value: "product" }, { label: "品类", value: "category" }, { label: "库位组", value: "location_group" }] } },
  { key: "source_type", header: "来源形态", width: 110, filterValue: (row) => row.source_type, copyValue: (row) => row.source_type },
  { key: "target_type", header: "目标形态", width: 110, filterValue: (row) => row.target_type, copyValue: (row) => row.target_type },
  { key: "min_safety_threshold", header: "Min", width: 80, mono: true, copyValue: (row) => row.min_safety_threshold },
  { key: "max_replenish_target", header: "Max", width: 80, mono: true, copyValue: (row) => row.max_replenish_target },
  { key: "trigger_modes", header: "触发", width: 140, render: (row) => row.trigger_modes.join("、"), filterValue: (row) => row.trigger_modes.join(" ") },
  { key: "enabled", header: COLUMN_STATUS, width: 90, render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? STATUS_ENABLED : STATUS_DISABLED} size="sm" />, filterValue: (row) => (row.enabled ? "true" : "false") },
];

export function M3ReplenishmentStrategyPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const listQuery = useReplenishmentStrategiesQuery(toStrategyFilters(appliedQuery));
  const createMutation = useCreateReplenishmentStrategyMutation();
  const updateMutation = useUpdateReplenishmentStrategyMutation();
  const disableMutation = useDisableReplenishmentStrategyMutation();
  const previewMutation = usePreviewReplenishmentStrategyMutation();
  const bindMutation = useBindReplenishmentLocationsMutation();
  const groupMutation = useUpsertReplenishmentLocationGroupMutation();
  const [selected, setSelected] = React.useState<string[]>([]);
  const [form, setForm] = React.useState<Form>(emptyForm());
  const [groupForm, setGroupForm] = React.useState<GroupForm>(emptyGroupForm());
  const [previewRows, setPreviewRows] = React.useState<ReplenishmentPreviewItem[]>([]);
  const editDialog = useDialogState<ReplenishmentStrategy>();
  const disableDialog = useDialogState<ReplenishmentStrategy>();
  const previewDialog = useDialogState<ReplenishmentStrategy>();
  const bindDialog = useDialogState<ReplenishmentStrategy>();
  const groupDialog = useDialogState<null>();
  const [notice, setNotice] = React.useState<Notice>(null);
  const rows = listQuery.data?.data ?? [];
  const selectedRow = rows.find((row) => row.id === selected[0]);
  const busy = createMutation.isPending || updateMutation.isPending || disableMutation.isPending || bindMutation.isPending || groupMutation.isPending;
  const refreshAction: DataGridRefreshAction = { label: BUTTON_REFRESH, description: "刷新补货策略", disabled: listQuery.isFetching, onClick: () => void listQuery.refetch() };
  const createAction: DataGridCreateAction = { label: BUTTON_ADD, description: "新增补货策略", disabled: busy, onClick: () => openEdit(null) };
  const editAction: DataGridEditAction = { label: "修改", description: "修改选中的补货策略", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: (ctx) => openEdit(rows.find((row) => row.id === ctx.selectedRowKeys[0]) ?? null) };
  const disableAction: DataGridDisableAction = {
    label: selectedRow?.enabled ? STATUS_DISABLED : STATUS_ENABLED,
    description: "启停补货策略",
    disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
    onClick: () => selectedRow && disableDialog.openWith(selectedRow),
  };
  const toolbarActions: DataGridToolbarAction[] = [
    { key: "preview", label: "命中预览", description: "预览命中位", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: () => selectedRow && void openPreview(selectedRow) },
    { key: "bind", label: "挂接拣选位", description: "绑定拣选位", disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy, onClick: () => selectedRow && openBind(selectedRow) },
    { key: "groups", label: "维护库位组", description: "维护库位组", disabled: busy, onClick: () => { setGroupForm(emptyGroupForm()); groupDialog.openWith(null); } },
  ];

  return (
    <ListPageTemplate
      data-testid="m3-replenishment-strategy-page"
      notice={notice}
      queryFields={queryFields}
      coreQueryFieldKeys={defaultVisibleFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={setDraftQuery}
      onQuery={() => applyQuery(draftQuery)}
      onReset={resetQuery}
      gridProps={{
        storageKey: ["m3", "replenishment-strategies"].join("."),
        columns,
        data: rows,
        rowKey: (row) => row.id,
        selectable: true,
        selectedRowKeys: selected,
        onSelectedRowKeysChange: setSelected,
        caption: listQuery.isPending ? "加载补货策略..." : undefined,
        emptyTitle: listQuery.isError ? "读取补货策略失败" : "暂无补货策略",
        emptyDescription: listQuery.isError ? errorText(listQuery.error, ERROR_AUTH_API_CHECK) : "请新增 Min-Max / 波次缺口策略",
        exportFileBaseName: "M3-replenishment-strategies",
        refreshAction,
        createAction,
        editAction,
        disableAction,
        toolbarActions,
        queryState: appliedQuery,
        onApplyQueryState: (value) => applyQuery(queryValueFromUnknown(value)),
        onClearQueryState: resetQuery,
      }}
      dialogs={
        <>
          <Dialog open={editDialog.open} onOpenChange={(open) => !busy && editDialog.setOpen(open)}>
            <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
              <form className="grid gap-4" onSubmit={submitStrategy}>
                <DialogHeader>
                  <DialogTitle>{editDialog.target ? "修改补货策略" : "新增补货策略"}</DialogTitle>
                  <DialogDescription>配置型分区：基本信息、动线与 scope、Min-Max 与触发模式、挂接拣选位、启停、命中预览。</DialogDescription>
                </DialogHeader>
                <Section title="基本信息">
                  <Field label="策略编码"><Input required value={form.strategyCode} disabled={Boolean(editDialog.target)} onChange={(event) => update("strategyCode", event.target.value)} /></Field>
                  <Field label="策略名称"><Input required value={form.strategyName} onChange={(event) => update("strategyName", event.target.value)} /></Field>
                </Section>
                <Section title="动线与 scope">
                  <Field label="范围类型">
                    <select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.scopeType} onChange={(event) => update("scopeType", event.target.value)}>
                      <option value="product">商品</option>
                      <option value="category">品类</option>
                      <option value="location_group">库位组</option>
                    </select>
                  </Field>
                  <Field label="范围引用"><Input required value={form.scopeRef} onChange={(event) => update("scopeRef", event.target.value)} placeholder="UUID" /></Field>
                  <Field label="来源形态">
                    <select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.sourceType} onChange={(event) => update("sourceType", event.target.value)}>
                      <option value="storage">存储</option>
                      <option value="case_pick">整箱拣选</option>
                    </select>
                  </Field>
                  <Field label="目标形态">
                    <select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={form.targetType} onChange={(event) => update("targetType", event.target.value)}>
                      <option value="case_pick">整箱拣选</option>
                      <option value="piece_pick">拆零拣选</option>
                    </select>
                  </Field>
                </Section>
                <Section title="Min-Max 与触发模式">
                  <Field label="安全水位 Min"><Input required value={form.minSafety} onChange={(event) => update("minSafety", event.target.value)} /></Field>
                  <Field label="补货上限 Max"><Input required value={form.maxTarget} onChange={(event) => update("maxTarget", event.target.value)} /></Field>
                  <label className="flex h-10 items-center gap-2 text-sm"><input type="checkbox" checked={form.triggerMinMax} onChange={(event) => update("triggerMinMax", event.target.checked)} />min_max</label>
                  <label className="flex h-10 items-center gap-2 text-sm"><input type="checkbox" checked={form.triggerWaveGap} onChange={(event) => update("triggerWaveGap", event.target.checked)} />wave_gap</label>
                </Section>
                <Section title="挂接拣选位">
                  <Field label="拣选位 ID（保存策略后在「挂接拣选位」动作提交）"><Input value={form.locationIds} onChange={(event) => update("locationIds", event.target.value)} placeholder="逗号分隔 UUID" /></Field>
                </Section>
                <Section title="启停">
                  <label className="flex h-10 items-center gap-2 text-sm"><input type="checkbox" checked={form.enabled} onChange={(event) => update("enabled", event.target.checked)} />启用</label>
                </Section>
                <Section title="命中预览">
                  <p className="text-sm text-muted-foreground">保存后选中策略，使用工具栏「命中预览」查看当前可用量。</p>
                </Section>
                <DialogFooter>
                  <DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose>
                  <Button type="submit" disabled={busy}>{busy ? LOADING_SAVING : BUTTON_SAVE}</Button>
                </DialogFooter>
              </form>
            </DialogContent>
          </Dialog>
          <Dialog open={disableDialog.open} onOpenChange={(open) => !busy && disableDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-md">
              <DialogHeader>
                <DialogTitle>{disableDialog.target?.enabled ? "停用补货策略" : "启用补货策略"}</DialogTitle>
                <DialogDescription>确认{disableDialog.target?.enabled ? "停用" : "启用"}策略「{disableDialog.target?.strategy_name ?? ""}」？</DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose>
                <Button type="button" disabled={busy || !disableDialog.target} onClick={() => void confirmDisable()}>{busy ? LOADING_PROCESSING : "确认"}</Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
          <Dialog open={previewDialog.open} onOpenChange={previewDialog.setOpen}>
            <DialogContent className="sm:max-w-xl">
              <DialogHeader>
                <DialogTitle>命中预览</DialogTitle>
                <DialogDescription>策略 {previewDialog.target?.strategy_code} 当前挂接拣选位可用量。</DialogDescription>
              </DialogHeader>
              <div className="grid gap-2 text-sm">
                {previewRows.length === 0 ? <p>暂无命中位</p> : previewRows.map((item) => (
                  <div key={item.location_id} className="flex justify-between gap-2 border-b py-1">
                    <span>{item.location_code}</span>
                    <span>可用 {item.available_qty}{item.would_trigger ? " · 将触发" : ""}</span>
                  </div>
                ))}
              </div>
            </DialogContent>
          </Dialog>
          <Dialog open={bindDialog.open} onOpenChange={(open) => !busy && bindDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-lg">
              <form className="grid gap-4" onSubmit={submitBind}>
                <DialogHeader>
                  <DialogTitle>挂接拣选位</DialogTitle>
                  <DialogDescription>替换策略 {bindDialog.target?.strategy_code} 的拣选位挂接。</DialogDescription>
                </DialogHeader>
                <Field label="拣选位 ID"><Input required value={form.locationIds} onChange={(event) => update("locationIds", event.target.value)} placeholder="逗号分隔 UUID" /></Field>
                <DialogFooter>
                  <DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose>
                  <Button type="submit" disabled={busy}>{busy ? LOADING_SAVING : BUTTON_SAVE}</Button>
                </DialogFooter>
              </form>
            </DialogContent>
          </Dialog>
          <Dialog open={groupDialog.open} onOpenChange={(open) => !busy && groupDialog.setOpen(open)}>
            <DialogContent className="sm:max-w-lg">
              <form className="grid gap-4" onSubmit={submitGroup}>
                <DialogHeader>
                  <DialogTitle>维护库位组</DialogTitle>
                  <DialogDescription>按组编码全量替换成员。</DialogDescription>
                </DialogHeader>
                <Field label="组编码"><Input required value={groupForm.groupCode} onChange={(event) => setGroupForm((current) => ({ ...current, groupCode: event.target.value }))} /></Field>
                <Field label="组名称"><Input required value={groupForm.groupName} onChange={(event) => setGroupForm((current) => ({ ...current, groupName: event.target.value }))} /></Field>
                <Field label="成员库位 ID"><Input value={groupForm.locationIds} onChange={(event) => setGroupForm((current) => ({ ...current, locationIds: event.target.value }))} placeholder="逗号分隔 UUID" /></Field>
                <label className="flex h-10 items-center gap-2 text-sm"><input type="checkbox" checked={groupForm.enabled} onChange={(event) => setGroupForm((current) => ({ ...current, enabled: event.target.checked }))} />启用</label>
                <DialogFooter>
                  <DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose>
                  <Button type="submit" disabled={busy}>{busy ? LOADING_SAVING : BUTTON_SAVE}</Button>
                </DialogFooter>
              </form>
            </DialogContent>
          </Dialog>
        </>
      }
    />
  );

  function update(key: keyof Form, value: string | boolean) {
    setForm((current) => ({ ...current, [key]: value }));
  }
  function openEdit(row: ReplenishmentStrategy | null) {
    setNotice(null);
    setForm(row ? formFromRow(row) : emptyForm());
    editDialog.setTarget(row);
    editDialog.setOpen(true);
  }
  function openBind(row: ReplenishmentStrategy) {
    setNotice(null);
    setForm((current) => ({ ...current, locationIds: "" }));
    bindDialog.openWith(row);
  }
  async function openPreview(row: ReplenishmentStrategy) {
    setNotice(null);
    try {
      const preview = await previewMutation.mutateAsync(row.id);
      setPreviewRows(preview.data);
      previewDialog.openWith(row);
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "预览命中位失败") });
    }
  }
  async function submitStrategy(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validate(form);
    if (error) { setNotice({ kind: "error", text: error }); return; }
    const body = toRequest(form);
    try {
      if (editDialog.target) {
        await updateMutation.mutateAsync({ id: editDialog.target.id, body });
      } else {
        await createMutation.mutateAsync(body);
      }
      editDialog.setOpen(false);
      setSelected([]);
      setNotice({ kind: "success", text: `策略 ${body.strategy_code} 已保存` });
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "保存补货策略失败") });
    }
  }
  async function confirmDisable() {
    const row = disableDialog.target;
    if (!row) return;
    try {
      if (row.enabled) {
        await disableMutation.mutateAsync(row.id);
      } else {
        await updateMutation.mutateAsync({ id: row.id, body: toRequest(formFromRow({ ...row, enabled: true })) });
      }
      disableDialog.setOpen(false);
      setSelected([]);
      setNotice({ kind: "success", text: `${row.strategy_name} 已${row.enabled ? "停用" : "启用"}` });
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "启停补货策略失败") });
    }
  }
  async function submitBind(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!bindDialog.target) return;
    const locationIds = parseIds(form.locationIds);
    if (locationIds.some((id) => !isUuid(id))) { setNotice({ kind: "error", text: "拣选位必须是合法 UUID" }); return; }
    try {
      await bindMutation.mutateAsync({ id: bindDialog.target.id, location_ids: locationIds });
      bindDialog.setOpen(false);
      setNotice({ kind: "success", text: `策略 ${bindDialog.target.strategy_code} 已挂接 ${locationIds.length} 个拣选位` });
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "挂接拣选位失败") });
    }
  }
  async function submitGroup(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const locationIds = parseIds(groupForm.locationIds);
    if (locationIds.some((id) => !isUuid(id))) { setNotice({ kind: "error", text: "库位组成员必须是合法 UUID" }); return; }
    try {
      const saved = await groupMutation.mutateAsync({
        group_code: groupForm.groupCode.trim(),
        group_name: groupForm.groupName.trim(),
        enabled: groupForm.enabled,
        location_ids: locationIds,
      });
      groupDialog.setOpen(false);
      setNotice({ kind: "success", text: `库位组 ${saved.group_code} 已保存` });
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "保存库位组失败") });
    }
  }
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return <section className="grid gap-3 rounded-md border p-3"><h3 className="text-sm font-medium">{title}</h3><div className="grid gap-3 sm:grid-cols-2">{children}</div></section>;
}
function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="grid gap-1 text-sm">{label}{children}</label>;
}
function defaultQuery(): QueryPanelValue { return { keyword: "", enabled: [], scope_type: [], target_type: [] }; }
function normalizeQuery(value: QueryPanelValue): QueryPanelValue {
  return { keyword: queryString(value.keyword), enabled: queryStringArray(value.enabled), scope_type: queryStringArray(value.scope_type), target_type: queryStringArray(value.target_type) };
}
function emptyForm(): Form {
  return { strategyCode: "", strategyName: "", scopeType: "product", scopeRef: "", sourceType: "storage", targetType: "case_pick", minSafety: "10", maxTarget: "50", triggerMinMax: true, triggerWaveGap: false, enabled: true, locationIds: "" };
}
function emptyGroupForm(): GroupForm { return { groupCode: "", groupName: "", locationIds: "", enabled: true }; }
function formFromRow(row: ReplenishmentStrategy): Form {
  return { strategyCode: row.strategy_code, strategyName: row.strategy_name, scopeType: row.scope_type, scopeRef: row.scope_ref, sourceType: row.source_type, targetType: row.target_type, minSafety: row.min_safety_threshold, maxTarget: row.max_replenish_target, triggerMinMax: row.trigger_modes.includes("min_max"), triggerWaveGap: row.trigger_modes.includes("wave_gap"), enabled: row.enabled, locationIds: "" };
}
function toRequest(form: Form): UpsertReplenishmentStrategyRequest {
  const trigger_modes = [form.triggerMinMax ? "min_max" : "", form.triggerWaveGap ? "wave_gap" : ""].filter(Boolean);
  return { strategy_code: form.strategyCode.trim(), strategy_name: form.strategyName.trim(), scope_type: form.scopeType, scope_ref: form.scopeRef.trim(), source_type: form.sourceType, target_type: form.targetType, min_safety_threshold: form.minSafety.trim(), max_replenish_target: form.maxTarget.trim(), trigger_modes, enabled: form.enabled };
}
function parseIds(value: string): string[] {
  return value.split(/[,，\s]+/).map((item) => item.trim()).filter(Boolean);
}
function toStrategyFilters(query: QueryPanelValue) {
  const enabled = queryStringArray(query.enabled).find(Boolean);
  return {
    keyword: queryString(query.keyword),
    enabled: enabled === "true" ? true : enabled === "false" ? false : undefined,
    scope_type: queryStringArray(query.scope_type).find(Boolean) ?? "",
    target_type: queryStringArray(query.target_type).find(Boolean) ?? "",
  };
}
function validate(form: Form) {
  if (!form.strategyCode.trim() || !form.strategyName.trim()) return "策略编码和名称不能为空";
  if (!isUuid(form.scopeRef.trim())) return "范围引用必须是合法 UUID";
  const min = Number(form.minSafety);
  const max = Number(form.maxTarget);
  if (!Number.isFinite(min) || !Number.isFinite(max) || max <= min) return "Max 必须大于 Min";
  if (!form.triggerMinMax && !form.triggerWaveGap) return "至少选择一种触发模式";
  return null;
}
