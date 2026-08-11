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
  cn,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridDisableAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { KeyRound, RotateCw } from "lucide-react";

import type { CurrentUser } from "@/features/auth/auth-queries";
import {
  useApiKeysQuery,
  useCreateApiKeyMutation,
  useRevokeApiKeyMutation,
  useRotateApiKeyMutation,
  type ApiKey,
  type CreateApiKeyRequest,
} from "@/features/api-key/api-key-queries";
import { errorText } from "@/lib/error-text";
import { formatDateTime } from "@/lib/format";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import { usePageQueryState } from "@/lib/use-page-query-state";

const apiKeyQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: "关键字",
    type: "text",
    placeholder: "调用方名称 / 用途",
    ariaLabel: "搜索 API Key",
  },
  {
    key: "status",
    label: "状态",
    type: "multiSelect",
    options: [
      { label: "全部", value: "" },
      { label: "启用", value: "active" },
      { label: "临时禁用", value: "temporarily_disabled" },
      { label: "已吊销", value: "revoked" },
    ],
  },
];
const apiKeyCoreQueryFieldKeys = ["keyword", "status"];

const apiKeyColumns: DataGridColumn<ApiKey>[] = [
  {
    key: "caller_name",
    header: "调用方",
    width: 170,
    minWidth: 140,
    sortable: true,
    sortValue: (row) => row.caller_name,
    filterValue: (row) => row.caller_name,
    copyValue: (row) => row.caller_name,
    filter: { type: "text" },
  },
  {
    key: "purpose",
    header: "用途",
    width: 220,
    minWidth: 160,
    sortable: true,
    sortValue: (row) => row.purpose,
    filterValue: (row) => row.purpose,
    copyValue: (row) => row.purpose,
    filter: { type: "text" },
  },
  {
    key: "scopes",
    header: "作用域",
    width: 240,
    minWidth: 180,
    copyValue: (row) => row.scopes.join(", "),
    filter: { type: "text" },
    filterValue: (row) => row.scopes.join(" "),
    render: (row) => <span className="font-mono text-xs">{row.scopes.join(" · ")}</span>,
  },
  {
    key: "status",
    header: "状态",
    width: 130,
    minWidth: 110,
    sortable: true,
    sortValue: (row) => row.status,
    filterValue: (row) => row.status,
    copyValue: (row) => statusLabel(row),
    filter: { type: "multiSelect", options: [{ label: "启用", value: "active" }, { label: "临时禁用", value: "temporarily_disabled" }, { label: "已吊销", value: "revoked" }] },
    render: (row) => <StatusBadge status={statusVariant(row)} label={statusLabel(row)} size="sm" />,
  },
  {
    key: "expires_at",
    header: "过期时间",
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.expires_at,
    filterValue: (row) => row.expires_at,
    copyValue: (row) => formatDateTime(row.expires_at),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.expires_at),
  },
  {
    key: "grace_expires_at",
    header: "旧 Key 宽限至",
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.grace_expires_at ?? "",
    filterValue: (row) => row.grace_expires_at ?? "",
    copyValue: (row) => row.grace_expires_at ? formatDateTime(row.grace_expires_at) : "",
    filter: { type: "text" },
    render: (row) => row.grace_expires_at ? formatDateTime(row.grace_expires_at) : "-",
  },
  {
    key: "updated_at",
    header: "更新时间",
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.updated_at,
    filterValue: (row) => row.updated_at,
    copyValue: (row) => formatDateTime(row.updated_at),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.updated_at),
  },
  {
    key: "created_at",
    header: "创建时间",
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.created_at,
    filterValue: (row) => row.created_at,
    copyValue: (row) => formatDateTime(row.created_at),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.created_at),
  },
];

type Notice = { type: "success" | "error"; text: string } | null;
type CreateForm = {
  callerName: string;
  purpose: string;
  warehouseIds: string;
  scopes: string[];
  expiresAt: string;
};

export function H1ApiKeyPage({ currentUser }: { currentUser: CurrentUser }) {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [pageIndex, setPageIndex] = React.useState(0);
  const [pageSize, setPageSize] = React.useState(20);
  const [createOpen, setCreateOpen] = React.useState(false);
  const [rotateOpen, setRotateOpen] = React.useState(false);
  const [rotateDays, setRotateDays] = React.useState("7");
  const [createForm, setCreateForm] = React.useState<CreateForm>(() => defaultCreateForm());
  const [notice, setNotice] = React.useState<Notice>(null);
  const keyword = queryString(appliedQuery.keyword);
  const status = queryString(appliedQuery.status);
  const apiKeysQuery = useApiKeysQuery({ keyword, status, page: pageIndex + 1, pageSize });
  const createMutation = useCreateApiKeyMutation();
  const rotateMutation = useRotateApiKeyMutation();
  const revokeMutation = useRevokeApiKeyMutation();
  const rows = apiKeysQuery.data?.data ?? [];
  const total = apiKeysQuery.data?.page.total ?? rows.length;
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(apiKeyQueryFields, appliedQuery),
    [appliedQuery],
  );
  const selectedRow = rows.find((row) => row.key_id === selectedRowKeys[0]);
  const busy = createMutation.isPending || rotateMutation.isPending || revokeMutation.isPending;

  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新当前货主 API Key 列表",
    disabled: apiKeysQuery.isFetching,
    onClick: () => void refresh(),
  };
  const createAction: DataGridCreateAction = {
    label: "创建 Key",
    description: "创建 API Key，明文只在首次响应展示",
    disabled: busy,
    onClick: () => {
      setNotice(null);
      setCreateForm(defaultCreateForm());
      setCreateOpen(true);
    },
  };
  const revokeAction: DataGridDisableAction = {
    label: "吊销",
    description: "立即吊销选中的 API Key",
    disabled: (context) => context.selectedRowKeys.length !== 1 || busy || selectedRow?.status === "revoked",
    onClick: (context) => {
      const row = rows.find((item) => item.key_id === context.selectedRowKeys[0]);
      if (!row || !window.confirm(`确认立即吊销 ${row.caller_name} 的 API Key？`)) return;
      void revoke(row.key_id);
    },
  };
  const toolbarActions: DataGridToolbarAction[] = [
    {
      key: "rotate",
      label: "轮换",
      description: "生成新 Key 并保留旧 Key 宽限期",
      icon: <RotateCw className="size-4" aria-hidden />,
      disabled: (context) => context.selectedRowKeys.length !== 1 || busy || selectedRow?.status === "revoked",
      onClick: () => {
        setNotice(null);
        setRotateDays("7");
        setRotateOpen(true);
      },
    },
  ];

  function applyGridQueryState(queryState: unknown) {
    applyQuery(queryValueFromUnknown(queryState));
    setPageIndex(0);
  }

  function clearGridQueryState() {
    resetQuery();
    setPageIndex(0);
  }

  async function refresh() {
    const result = await apiKeysQuery.refetch();
    setNotice(result.error ? { type: "error", text: result.error.message } : { type: "success", text: "API Key 列表已刷新" });
  }

  async function revoke(id: string) {
    setNotice(null);
    try {
      await revokeMutation.mutateAsync(id);
      setSelectedRowKeys([]);
      setNotice({ type: "success", text: "API Key 已吊销；重复吊销保持幂等" });
    } catch (error) {
      setNotice({ type: "error", text: errorText(error, "吊销 API Key 失败") });
    }
  }

  async function submitCreate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const body: CreateApiKeyRequest = {
      caller_name: createForm.callerName.trim(),
      purpose: createForm.purpose.trim(),
      warehouse_ids: splitIds(createForm.warehouseIds),
      scopes: createForm.scopes,
      expires_at: createForm.expiresAt ? new Date(createForm.expiresAt).toISOString() : null,
      responsible_user_id: currentUser.user_id,
    };
    try {
      const created = await createMutation.mutateAsync(body);
      setCreateOpen(false);
      setNotice({ type: "success", text: `创建成功。明文 secret 只展示一次：${created.secret ?? "未返回"}` });
    } catch (error) {
      setNotice({ type: "error", text: errorText(error, "创建 API Key 失败") });
    }
  }

  async function submitRotate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedRow) return;
    const days = Number(rotateDays);
    if (!Number.isInteger(days) || days < 0) {
      setNotice({ type: "error", text: "宽限期必须是大于等于 0 的整数天" });
      return;
    }
    try {
      const rotated = await rotateMutation.mutateAsync({ id: selectedRow.key_id, body: { grace_period_days: days, expires_at: null } });
      setRotateOpen(false);
      setSelectedRowKeys([]);
      setNotice({ type: "success", text: `轮换成功。新 secret 只展示一次：${rotated.new_key.secret ?? "未返回"}` });
    } catch (error) {
      setNotice({ type: "error", text: errorText(error, "轮换 API Key 失败") });
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="H1 API Key 生命周期"
      />
      <NoticePanel notice={notice} />
      <QueryPanel
        fields={apiKeyQueryFields}
        defaultVisibleFieldKeys={apiKeyCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => { applyQuery(draftQuery); setPageIndex(0); }}
        onReset={() => { resetQuery(); setPageIndex(0); }}
      />
      <Card className="rounded-lg shadow-sm">
        <CardContent className="p-5">
          <DataGrid
            storageKey="h1.auth.api-keys"
            columns={apiKeyColumns}
            data={rows}
            rowKey={(row) => row.key_id}
            selectable
            selectedRowKeys={selectedRowKeys}
            onSelectedRowKeysChange={setSelectedRowKeys}
            serverPagination={{
              pageIndex,
              pageSize,
              total,
              onPageChange: (next) => { setPageIndex(next); setSelectedRowKeys([]); },
              onPageSizeChange: (next) => { setPageSize(next); setPageIndex(0); setSelectedRowKeys([]); },
            }}
            caption={apiKeysQuery.isPending ? "加载 API Key..." : undefined}
            emptyTitle={apiKeysQuery.isError ? "读取 API Key 失败" : "暂无 API Key"}
            emptyDescription={apiKeysQuery.isError ? errorText(apiKeysQuery.error, "请检查鉴权和数据库连接") : "请使用创建 Key 录入受控调用方"}
            exportFileBaseName="H1-API-Key"
            refreshAction={refreshAction}
            createAction={createAction}
            disableAction={revokeAction}
            toolbarActions={toolbarActions}
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            onApplyQueryState={applyGridQueryState}
            onClearQueryState={clearGridQueryState}
          />
        </CardContent>
      </Card>
      <Card className="rounded-lg border-dashed shadow-none">
        <CardContent className="flex items-start gap-3 p-4 text-sm text-muted-foreground">
          <KeyRound className="mt-0.5 size-4 shrink-0 text-primary" aria-hidden />
          <p>secret 不会进入列表、审计或幂等重放响应；请在创建或轮换成功后立即交给调用方安全保存。</p>
        </CardContent>
      </Card>

      <Dialog open={createOpen} onOpenChange={(open) => !createMutation.isPending && setCreateOpen(open)}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
          <form className="grid gap-4" onSubmit={submitCreate}>
            <DialogHeader>
              <DialogTitle>创建 API Key</DialogTitle>
              <DialogDescription>owner 和负责人由当前登录态派生；secret 只在首次响应展示。</DialogDescription>
            </DialogHeader>
            <label className="grid gap-1 text-sm">调用方名称<Input required value={createForm.callerName} onChange={(event) => updateCreateForm("callerName", event.target.value)} /></label>
            <label className="grid gap-1 text-sm">用途<Input required value={createForm.purpose} onChange={(event) => updateCreateForm("purpose", event.target.value)} /></label>
            <label className="grid gap-1 text-sm">仓库范围<span className="text-xs text-muted-foreground">可留空表示当前货主全部仓库；多个 UUID 用逗号分隔</span><Input value={createForm.warehouseIds} onChange={(event) => updateCreateForm("warehouseIds", event.target.value)} placeholder="warehouse-uuid-1, warehouse-uuid-2" /></label>
            <label className="grid gap-1 text-sm">作用域<select required multiple className="min-h-28 rounded-md border border-input bg-background px-3 py-2 text-sm" value={createForm.scopes} onChange={(event) => setCreateForm((current) => ({ ...current, scopes: Array.from(event.target.selectedOptions, (option) => option.value) }))} aria-label="API Key 作用域">
              <option value="master-data:write">master-data:write</option>
              <option value="inbound:push">inbound:push</option>
              <option value="outbound:push">outbound:push</option>
              <option value="return:push">return:push</option>
              <option value="tms:callback">tms:callback</option>
            </select></label>
            <label className="grid gap-1 text-sm">过期时间<span className="text-xs text-muted-foreground">留空使用默认 180 天</span><Input type="datetime-local" value={createForm.expiresAt} onChange={(event) => updateCreateForm("expiresAt", event.target.value)} /></label>
            <DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={createMutation.isPending}>取消</Button></DialogClose><Button type="submit" disabled={createMutation.isPending}>{createMutation.isPending ? "创建中..." : "确认创建"}</Button></DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={rotateOpen} onOpenChange={(open) => !rotateMutation.isPending && setRotateOpen(open)}>
        <DialogContent className="sm:max-w-md">
          <form className="grid gap-4" onSubmit={submitRotate}>
            <DialogHeader><DialogTitle>轮换 API Key</DialogTitle><DialogDescription>旧 Key 在宽限期内可用，宽限期结束后立即失效。</DialogDescription></DialogHeader>
            <label className="grid gap-1 text-sm">旧 Key 宽限期（天）<Input type="number" min="0" step="1" value={rotateDays} onChange={(event) => setRotateDays(event.target.value)} aria-label="旧 Key 宽限期（天）" /></label>
            <DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={rotateMutation.isPending}>取消</Button></DialogClose><Button type="submit" disabled={rotateMutation.isPending}>{rotateMutation.isPending ? "轮换中..." : "确认轮换"}</Button></DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </section>
  );

  function updateCreateForm(key: keyof CreateForm, value: string) {
    setCreateForm((current) => ({ ...current, [key]: value }));
  }
}

function defaultQuery(): QueryPanelValue {
  return { keyword: "", status: "" };
}

function normalizeQuery(value: QueryPanelValue): QueryPanelValue {
  return { keyword: queryString(value.keyword), status: queryString(value.status) };
}

function defaultCreateForm(): CreateForm {
  return { callerName: "", purpose: "", warehouseIds: "", scopes: ["inbound:push"], expiresAt: "" };
}

function splitIds(value: string) {
  return value.split(/[\s,]+/).map((item) => item.trim()).filter(Boolean);
}

function statusLabel(row: ApiKey) {
  if (row.status === "revoked") return "已吊销";
  if (row.status === "temporarily_disabled") return "临时禁用";
  if (row.grace_expires_at) return "轮换宽限";
  return "启用";
}

function statusVariant(row: ApiKey): "completed" | "isolated" | "expired" {
  if (row.status === "revoked") return "expired";
  if (row.status === "temporarily_disabled") return "isolated";
  return "completed";
}

function NoticePanel({ notice }: { notice: Notice }) {
  if (!notice) return null;
  return <div className={cn("rounded-md border px-3 py-2 text-sm", notice.type === "success" ? "border-wms-success/30 bg-wms-success/10 text-wms-success" : "border-destructive/30 bg-destructive/10 text-destructive")} role={notice.type === "success" ? "status" : "alert"}>{notice.text}</div>;
}
