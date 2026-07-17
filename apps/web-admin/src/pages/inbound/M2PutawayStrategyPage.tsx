import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridEditAction,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import type { CurrentUser } from "@/features/auth/auth-queries";
import {
  usePutawayStrategyProfilesQuery,
  useUpsertPutawayStrategyProfileMutation,
  type PutawayStrategyProfile,
} from "@/features/inbound/putaway-strategy-queries";

/** 上架策略规则目录（与 US-M2-010 验收标准一致）。 */
export const PUTAWAY_RULE_CATALOG: Array<{ code: string; label: string }> = [
  { code: "temperature_match", label: "温区匹配" },
  { code: "owner_isolation", label: "货主隔离" },
  { code: "capacity_match", label: "容积匹配" },
  { code: "same_product_cluster", label: "同品聚集" },
  { code: "abc_class", label: "ABC 分类" },
  { code: "category_zone", label: "品类分区" },
  { code: "expiry_isolation", label: "效期隔离" },
  { code: "empty_location_first", label: "空库位优先" },
  { code: "quality_color_match", label: "质量色标匹配" },
];

export const queryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "方案编码 / 名称 / 品类" },
  {
    key: "status",
    label: "状态",
    type: "select",
    options: [
      { label: "全部", value: "" },
      { label: "启用", value: "active" },
      { label: "停用", value: "disabled" },
    ],
  },
  {
    key: "isDefault",
    label: "默认方案",
    type: "select",
    options: [
      { label: "全部", value: "" },
      { label: "是", value: "yes" },
      { label: "否", value: "no" },
    ],
  },
];

export const defaultVisibleFieldKeys = ["keyword", "status", "isDefault"];

type Form = {
  profileCode: string;
  profileName: string;
  isDefault: boolean;
  topN: string;
  warehouseId: string;
  productCategory: string;
  notifyOnNoLocation: boolean;
  status: "active" | "disabled";
  enabledRules: Record<string, boolean>;
  rulePriority: string[];
};

type Notice = { kind: "success" | "error"; text: string } | null;

const columns: DataGridColumn<PutawayStrategyProfile>[] = [
  {
    key: "profile_code",
    header: "方案编码",
    width: 140,
    mono: true,
    sortable: true,
    filterValue: (row) => row.profile_code,
    copyValue: (row) => row.profile_code,
    filter: { type: "text" },
  },
  {
    key: "profile_name",
    header: "方案名称",
    width: 160,
    sortable: true,
    filterValue: (row) => row.profile_name,
    copyValue: (row) => row.profile_name,
    filter: { type: "text" },
  },
  {
    key: "is_default",
    header: "默认",
    width: 80,
    render: (row) => (row.is_default ? "是" : "否"),
    filterValue: (row) => (row.is_default ? "yes" : "no"),
    filter: {
      type: "multiSelect",
      options: [
        { label: "是", value: "yes" },
        { label: "否", value: "no" },
      ],
    },
  },
  {
    key: "top_n",
    header: "Top N",
    width: 80,
    mono: true,
    sortable: true,
    sortValue: (row) => row.top_n,
    copyValue: (row) => String(row.top_n),
  },
  {
    key: "product_category",
    header: "品类绑定",
    width: 120,
    render: (row) => row.product_category?.trim() || "不限",
    filterValue: (row) => row.product_category ?? "",
    copyValue: (row) => row.product_category ?? "",
    filter: { type: "text" },
  },
  {
    key: "warehouse_id",
    header: "仓库绑定",
    width: 160,
    mono: true,
    render: (row) => row.warehouse_id ?? "货主通用",
    filterValue: (row) => row.warehouse_id ?? "",
    copyValue: (row) => row.warehouse_id ?? "",
    filter: { type: "text" },
  },
  {
    key: "notify_on_no_location",
    header: "无库位通知",
    width: 110,
    render: (row) => (row.notify_on_no_location ? "开启" : "关闭"),
    filterValue: (row) => (row.notify_on_no_location ? "yes" : "no"),
  },
  {
    key: "status",
    header: "状态",
    width: 100,
    render: (row) => (
      <StatusBadge
        status={row.status === "active" ? "completed" : "isolated"}
        label={row.status === "active" ? "启用" : "停用"}
        size="sm"
      />
    ),
    filterValue: (row) => row.status,
    filter: {
      type: "multiSelect",
      options: [
        { label: "启用", value: "active" },
        { label: "停用", value: "disabled" },
      ],
    },
  },
  {
    key: "created_at",
    header: "创建时间",
    width: 180,
    sortable: true,
    sortValue: (row) => row.created_at,
    copyValue: (row) => row.created_at,
    render: (row) => formatDate(row.created_at),
    filter: { type: "dateRange" },
  },
  {
    key: "updated_at",
    header: "更新时间",
    width: 180,
    sortable: true,
    sortValue: (row) => row.updated_at,
    copyValue: (row) => row.updated_at,
    render: (row) => formatDate(row.updated_at),
    filter: { type: "dateRange" },
  },
];

export function M2PutawayStrategyPage({ currentUser }: { currentUser: CurrentUser }) {
  const profilesQuery = usePutawayStrategyProfilesQuery();
  const saveMutation = useUpsertPutawayStrategyProfileMutation();
  const [selected, setSelected] = React.useState<string[]>([]);
  const [query, setQuery] = React.useState<QueryPanelValue>(defaultQuery());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(defaultQuery());
  const [form, setForm] = React.useState<Form>(emptyForm());
  const [editing, setEditing] = React.useState<PutawayStrategyProfile | null>(null);
  const [dialogOpen, setDialogOpen] = React.useState(false);
  const [notice, setNotice] = React.useState<Notice>(null);
  const [dragCode, setDragCode] = React.useState<string | null>(null);

  const rows = React.useMemo(
    () => filterRows(profilesQuery.data ?? [], appliedQuery),
    [appliedQuery, profilesQuery.data],
  );
  const busy = saveMutation.isPending;
  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新上架策略方案",
    disabled: profilesQuery.isFetching,
    onClick: () => void profilesQuery.refetch(),
  };
  const createAction: DataGridCreateAction = {
    label: "新增",
    description: "新增上架策略方案",
    disabled: busy,
    onClick: () => openDialog(null),
  };
  const editAction: DataGridEditAction = {
    label: "修改",
    description: "修改选中的上架策略方案",
    disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
    onClick: (ctx) => openDialog(rows.find((row) => row.id === ctx.selectedRowKeys[0]) ?? null),
  };

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const error = validate(form);
    if (error) {
      setNotice({ kind: "error", text: error });
      return;
    }
    try {
      const saved = await saveMutation.mutateAsync({
        profile_code: form.profileCode.trim(),
        profile_name: form.profileName.trim(),
        is_default: form.isDefault,
        top_n: Number(form.topN),
        enabled_rules: form.enabledRules,
        rule_priority: form.rulePriority,
        warehouse_id: form.warehouseId.trim() || null,
        product_category: form.productCategory.trim() || null,
        notify_on_no_location: form.notifyOnNoLocation,
        status: form.status,
      });
      setDialogOpen(false);
      setSelected([]);
      setNotice({ kind: "success", text: `方案 ${saved.profile_code} 已保存` });
    } catch (errorValue) {
      setNotice({ kind: "error", text: errorText(errorValue, "保存上架策略方案失败") });
    }
  }

  function openDialog(row: PutawayStrategyProfile | null) {
    setNotice(null);
    setEditing(row);
    setForm(row ? formFromRow(row) : emptyForm());
    setDialogOpen(true);
  }

  function moveRule(code: string, direction: -1 | 1) {
    setForm((current) => {
      const index = current.rulePriority.indexOf(code);
      if (index < 0) return current;
      const nextIndex = index + direction;
      if (nextIndex < 0 || nextIndex >= current.rulePriority.length) return current;
      const next = [...current.rulePriority];
      const [item] = next.splice(index, 1);
      next.splice(nextIndex, 0, item);
      return { ...current, rulePriority: next };
    });
  }

  function onDropRule(targetCode: string) {
    if (!dragCode || dragCode === targetCode) {
      setDragCode(null);
      return;
    }
    setForm((current) => {
      const from = current.rulePriority.indexOf(dragCode);
      const to = current.rulePriority.indexOf(targetCode);
      if (from < 0 || to < 0) return current;
      const next = [...current.rulePriority];
      const [item] = next.splice(from, 1);
      next.splice(to, 0, item);
      return { ...current, rulePriority: next };
    });
    setDragCode(null);
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8" data-testid="m2-putaway-strategy-page">
      <PageHeader
        title="M2 上架策略"
        subtitle={`当前货主 ${currentUser.owner_code} · 配置规则优先级、启停、Top N 与仓库/品类绑定`}
      />
      {notice && (
        <div
          className={
            notice.kind === "error"
              ? "rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
              : "rounded-md border border-wms-success/30 bg-wms-success/10 px-3 py-2 text-sm text-wms-success"
          }
          role={notice.kind === "error" ? "alert" : "status"}
        >
          {notice.text}
        </div>
      )}
      <QueryPanel
        fields={queryFields}
        defaultVisibleFieldKeys={defaultVisibleFieldKeys}
        value={query}
        onValueChange={setQuery}
        onQuery={() => setAppliedQuery(query)}
        onReset={() => {
          const next = defaultQuery();
          setQuery(next);
          setAppliedQuery(next);
        }}
      />
      <Card>
        <CardContent className="p-5">
          <DataGrid
            storageKey="m2-putaway-strategy-datagrid"
            columns={columns}
            data={rows}
            rowKey={(row) => row.id}
            selectable
            selectedRowKeys={selected}
            onSelectedRowKeysChange={setSelected}
            caption={profilesQuery.isPending ? "加载上架策略方案..." : undefined}
            emptyTitle={profilesQuery.isError ? "读取上架策略失败" : "暂无上架策略方案"}
            emptyDescription={
              profilesQuery.isError
                ? errorText(profilesQuery.error, "请检查鉴权和 API 服务")
                : "请新增默认通用方案，规则默认全部启用"
            }
            exportFileBaseName="M2-putaway-strategy-profiles"
            refreshAction={refreshAction}
            createAction={createAction}
            editAction={editAction}
            queryState={appliedQuery}
            querySummaryItems={buildQueryPanelSummaryItems(queryFields, appliedQuery)}
            onApplyQueryState={(value) => {
              const next = normalizeQuery(value);
              setQuery(next);
              setAppliedQuery(next);
            }}
            onClearQueryState={() => {
              const next = defaultQuery();
              setQuery(next);
              setAppliedQuery(next);
            }}
          />
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={(open) => !busy && setDialogOpen(open)}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl" data-testid="m2-putaway-strategy-dialog">
          <form className="grid gap-4" onSubmit={submit}>
            <DialogHeader>
              <DialogTitle>{editing ? "修改上架策略方案" : "新增上架策略方案"}</DialogTitle>
              <DialogDescription>
                规则可启用/停用，并支持拖拽或上下调整优先级。默认优先级：温区 &gt; 货主 &gt; 容积 &gt; 同品 &gt; ABC &gt; 品类 &gt; 效期 &gt; 空库位。
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field label="方案编码">
                <Input
                  required
                  value={form.profileCode}
                  disabled={Boolean(editing)}
                  onChange={(event) => setForm((current) => ({ ...current, profileCode: event.target.value }))}
                  placeholder="default"
                />
              </Field>
              <Field label="方案名称">
                <Input
                  required
                  value={form.profileName}
                  onChange={(event) => setForm((current) => ({ ...current, profileName: event.target.value }))}
                  placeholder="通用方案"
                />
              </Field>
              <Field label="Top N">
                <Input
                  required
                  type="number"
                  min={1}
                  max={50}
                  value={form.topN}
                  onChange={(event) => setForm((current) => ({ ...current, topN: event.target.value }))}
                />
              </Field>
              <Field label="状态">
                <select
                  className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                  value={form.status}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      status: event.target.value === "disabled" ? "disabled" : "active",
                    }))
                  }
                >
                  <option value="active">启用</option>
                  <option value="disabled">停用</option>
                </select>
              </Field>
              <Field label="仓库绑定（UUID，可空）">
                <Input
                  value={form.warehouseId}
                  onChange={(event) => setForm((current) => ({ ...current, warehouseId: event.target.value }))}
                  placeholder="空=货主通用"
                />
              </Field>
              <Field label="品类绑定（可空）">
                <Input
                  value={form.productCategory}
                  onChange={(event) => setForm((current) => ({ ...current, productCategory: event.target.value }))}
                  placeholder="如 western_medicine"
                />
              </Field>
            </div>
            <div className="flex flex-wrap gap-4 text-sm">
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={form.isDefault}
                  onChange={(event) => setForm((current) => ({ ...current, isDefault: event.target.checked }))}
                />
                设为默认方案
              </label>
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={form.notifyOnNoLocation}
                  onChange={(event) =>
                    setForm((current) => ({ ...current, notifyOnNoLocation: event.target.checked }))
                  }
                />
                无可用库位时通知仓库主管
              </label>
            </div>

            <div className="grid gap-2" data-testid="m2-putaway-rule-priority">
              <div className="text-sm font-medium">规则优先级（可拖拽排序）</div>
              <ul className="grid gap-2">
                {form.rulePriority.map((code, index) => {
                  const meta = PUTAWAY_RULE_CATALOG.find((item) => item.code === code);
                  const enabled = form.enabledRules[code] !== false;
                  return (
                    <li
                      key={code}
                      draggable
                      onDragStart={() => setDragCode(code)}
                      onDragOver={(event) => event.preventDefault()}
                      onDrop={() => onDropRule(code)}
                      className="flex items-center gap-2 rounded-md border border-border bg-background px-3 py-2 text-sm"
                      data-rule-code={code}
                    >
                      <span className="w-6 text-muted-foreground">{index + 1}</span>
                      <span className="flex-1 font-medium">{meta?.label ?? code}</span>
                      <label className="flex items-center gap-1 text-xs">
                        <input
                          type="checkbox"
                          checked={enabled}
                          onChange={(event) =>
                            setForm((current) => ({
                              ...current,
                              enabledRules: { ...current.enabledRules, [code]: event.target.checked },
                            }))
                          }
                        />
                        启用
                      </label>
                      <Button type="button" variant="outline" size="sm" onClick={() => moveRule(code, -1)} disabled={index === 0}>
                        上移
                      </Button>
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => moveRule(code, 1)}
                        disabled={index === form.rulePriority.length - 1}
                      >
                        下移
                      </Button>
                    </li>
                  );
                })}
              </ul>
            </div>

            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline" disabled={busy}>
                  取消
                </Button>
              </DialogClose>
              <Button type="submit" disabled={busy}>
                {busy ? "保存中..." : "保存"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="grid gap-1 text-sm">
      {label}
      {children}
    </label>
  );
}

function defaultQuery(): QueryPanelValue {
  return { keyword: "", status: "", isDefault: "" };
}

function emptyForm(): Form {
  const rulePriority = PUTAWAY_RULE_CATALOG.map((item) => item.code);
  const enabledRules = Object.fromEntries(rulePriority.map((code) => [code, true]));
  return {
    profileCode: "default",
    profileName: "通用方案",
    isDefault: true,
    topN: "3",
    warehouseId: "",
    productCategory: "",
    notifyOnNoLocation: true,
    status: "active",
    enabledRules,
    rulePriority,
  };
}

function formFromRow(row: PutawayStrategyProfile): Form {
  const defaults = emptyForm();
  const enabledRules = {
    ...defaults.enabledRules,
    ...(asRecord(row.enabled_rules) as Record<string, boolean>),
  };
  const priority = asStringArray(row.rule_priority);
  const rulePriority =
    priority.length > 0
      ? [
          ...priority.filter((code) => defaults.rulePriority.includes(code)),
          ...defaults.rulePriority.filter((code) => !priority.includes(code)),
        ]
      : defaults.rulePriority;
  return {
    profileCode: row.profile_code,
    profileName: row.profile_name,
    isDefault: row.is_default,
    topN: String(row.top_n),
    warehouseId: row.warehouse_id ?? "",
    productCategory: row.product_category ?? "",
    notifyOnNoLocation: row.notify_on_no_location !== false,
    status: row.status === "disabled" ? "disabled" : "active",
    enabledRules,
    rulePriority,
  };
}

function filterRows(rows: PutawayStrategyProfile[], query: QueryPanelValue) {
  const keyword = text(query.keyword).trim().toLowerCase();
  const status = text(query.status);
  const isDefault = text(query.isDefault);
  return rows.filter((row) => {
    if (
      keyword &&
      ![row.profile_code, row.profile_name, row.product_category ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(keyword)
    ) {
      return false;
    }
    if (status && row.status !== status) return false;
    if (isDefault === "yes" && !row.is_default) return false;
    if (isDefault === "no" && row.is_default) return false;
    return true;
  });
}

function validate(form: Form) {
  if (!form.profileCode.trim() || !form.profileName.trim()) return "方案编码和名称不能为空";
  const topN = Number(form.topN);
  if (!Number.isInteger(topN) || topN < 1 || topN > 50) return "Top N 必须是 1-50 的整数";
  if (form.rulePriority.length === 0) return "至少保留一条规则";
  return null;
}

function normalizeQuery(value: unknown): QueryPanelValue {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  return {
    keyword: text(record.keyword),
    status: text(record.status),
    isDefault: text(record.isDefault),
  };
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string");
}

function text(value: unknown) {
  return typeof value === "string" ? value : "";
}

function formatDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false });
}

function errorText(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}
