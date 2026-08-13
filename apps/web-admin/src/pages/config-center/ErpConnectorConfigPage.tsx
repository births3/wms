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
  formatZhDate,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Pencil, Play, Power, PowerOff, Trash2 } from "lucide-react";

import {
  useActivateErpConnectorMutation,
  useCreateErpConnectorMutation,
  useDeleteErpConnectorMutation,
  useDisableErpConnectorMutation,
  useErpConnectorsQuery,
  useTestErpConnectorMutation,
  useUpdateErpConnectorMutation,
  type CreateH8ErpConnectorRequest,
  type H8ErpConnector,
} from "@/features/config-center/erp-connector-queries";
import type { CurrentUser } from "@/features/auth/auth-queries";
import { useApiKeysQuery } from "@/features/api-key/api-key-queries";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";
import { queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_REFRESH,
  BUTTON_SAVE,
  COLUMN_CREATED_AT,
  COLUMN_STATUS,
  COLUMN_UPDATED_AT,
  COLUMN_VERSION,
  FIELD_KEYWORD,
  LOADING_SAVING,
  STATUS_DEACTIVATED,
  STATUS_DISABLED,
  STATUS_ENABLED,
} from "@/lib/ui-strings";

export const H8_ERP_CONNECTOR_WRITE = "h8.erp_connector.write";

type Notice = { type: "success" | "error"; text: string } | null;
type ConfirmAction = "test" | "activate" | "disable" | "delete";

export const h8ErpConnectorQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: FIELD_KEYWORD,
    type: "text",
    placeholder: "连接编码 / 名称",
    ariaLabel: "搜索 ERP 连接",
  },
  {
    key: "status",
    label: COLUMN_STATUS,
    type: "multiSelect",
    options: [
      { label: "testing", value: "testing" },
      { label: "active", value: "active" },
      { label: "disabled", value: "disabled" },
    ],
  },
];
export const h8ErpConnectorCoreQueryFieldKeys = ["keyword", "status"];

const columns: DataGridColumn<H8ErpConnector>[] = [
  textColumn("connector_code", "连接编码", 170),
  textColumn("connector_name", "名称", 180),
  textColumn("channel_mode", "通道", 140),
  {
    key: "status",
    header: COLUMN_STATUS,
    width: 100,
    render: (row) => (
      <StatusBadge status={statusVariant(row.status)} label={row.status} size="sm" />
    ),
  },
  {
    key: "config_version",
    header: COLUMN_VERSION,
    width: 70,
    filter: false,
    render: (row) => row.config_version,
  },
  {
    key: "last_tested_succeeded",
    header: "最近测试",
    width: 90,
    filter: false,
    render: (row) =>
      row.last_tested_succeeded == null ? "—" : row.last_tested_succeeded ? "通过" : "失败",
  },
  {
    key: "created_at",
    header: COLUMN_CREATED_AT,
    width: 160,
    render: (row) => formatZhDate(row.created_at),
  },
  {
    key: "updated_at",
    header: COLUMN_UPDATED_AT,
    width: 180,
    defaultHidden: true,
    render: (row) => formatZhDate(row.updated_at),
  },
  {
    key: "directions",
    header: "方向",
    width: 120,
    defaultHidden: true,
    render: (row) => row.directions.join(","),
  },
  {
    key: "message_types",
    header: "消息类型",
    width: 180,
    defaultHidden: true,
    render: (row) => row.message_types.join(","),
  },
];

function emptyForm(): CreateH8ErpConnectorRequest {
  return {
    connector_code: "",
    connector_name: "",
    warehouse_ids: [],
    directions: ["inbound"],
    message_types: ["asn"],
    channel_mode: "rest",
    api_base_url: "https://erp.example.com",
    // 必须引用真实存在的 API Key（后端会校验 key 存在且 scopes 覆盖消息类型），由弹窗下拉选择
    api_key_id: "",
    bearer_secret_alias: null,
    interface_db_password_alias: null,
    interface_probe_db_username: null,
    interface_probe_db_password_alias: null,
  };
}

function defaultQuery(): QueryPanelValue {
  return { keyword: "", status: [] };
}

function normalizeQuery(value: QueryPanelValue): QueryPanelValue {
  const status = value.status;
  return {
    keyword: typeof value.keyword === "string" ? value.keyword : "",
    status: Array.isArray(status) ? status.map(String) : [],
  };
}

function filterConnectors(rows: H8ErpConnector[], query: QueryPanelValue): H8ErpConnector[] {
  const keyword = String(query.keyword ?? "")
    .trim()
    .toLowerCase();
  const statuses = Array.isArray(query.status) ? query.status.map(String) : [];
  return rows.filter((row) => {
    if (statuses.length > 0 && !statuses.includes(row.status)) return false;
    if (!keyword) return true;
    const haystack = `${row.connector_code} ${row.connector_name} ${row.channel_mode}`.toLowerCase();
    return haystack.includes(keyword);
  });
}

interface ErpConnectorConfigPageProps {
  currentUser?: CurrentUser;
  onBack?: () => void;
}

/** H8 集成中心 · ERP 连接配置页（US-H8-001，独立菜单 h8-erp-connectors） */
export function ErpConnectorConfigPage({
  currentUser,
  onBack,
}: ErpConnectorConfigPageProps = {}) {
  const listQuery = useErpConnectorsQuery();
  const createMutation = useCreateErpConnectorMutation();
  const updateMutation = useUpdateErpConnectorMutation();
  const testMutation = useTestErpConnectorMutation();
  const activateMutation = useActivateErpConnectorMutation();
  const disableMutation = useDisableErpConnectorMutation();
  const deleteMutation = useDeleteErpConnectorMutation();
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [createOpen, setCreateOpen] = React.useState(false);
  const [editOpen, setEditOpen] = React.useState(false);
  const confirmDialog = useDialogState<ConfirmAction>();
  const [form, setForm] = React.useState<CreateH8ErpConnectorRequest>(() => emptyForm());
  const apiKeysQuery = useApiKeysQuery({ status: "active" });
  const apiKeyOptions = React.useMemo(
    () => (apiKeysQuery.data?.data ?? []).filter((key) => !key.revoked_at),
    [apiKeysQuery.data],
  );
  const defaultApiKeyId = apiKeyOptions[0]?.key_id ?? "";
  React.useEffect(() => {
    // 新建弹窗默认选中第一个可用 Key，避免空引用被后端 422 拒绝
    if (!createOpen || !defaultApiKeyId) return;
    setForm((current) => (current.api_key_id ? current : { ...current, api_key_id: defaultApiKeyId }));
  }, [createOpen, defaultApiKeyId]);
  const [editForm, setEditForm] = React.useState({
    connector_name: "",
    channel_mode: "rest",
    api_base_url: "",
    bearer_secret_alias: "",
    interface_probe_db_username: "",
    interface_probe_db_password_alias: "",
    expected_config_version: 1,
    expected_probe_config_version: 1,
  });
  const [notice, setNotice] = React.useState<Notice>(null);
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);

  // 无 currentUser 时默认不可写，避免权限未就绪时露出写操作
  const canWrite = Boolean(currentUser?.permissions.includes(H8_ERP_CONNECTOR_WRITE));

  const rows = listQuery.data ?? [];
  const filteredRows = React.useMemo(() => filterConnectors(rows, appliedQuery), [rows, appliedQuery]);
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(h8ErpConnectorQueryFields, appliedQuery),
    [appliedQuery],
  );
  const selected = filteredRows.find((row) => row.id === selectedRowKeys[0])
    ?? rows.find((row) => row.id === selectedRowKeys[0]);
  const busy =
    createMutation.isPending ||
    updateMutation.isPending ||
    testMutation.isPending ||
    activateMutation.isPending ||
    disableMutation.isPending ||
    deleteMutation.isPending;

  const refreshAction: DataGridRefreshAction = {
    label: BUTTON_REFRESH,
    description: "刷新 ERP 连接列表",
    disabled: listQuery.isFetching,
    onClick: () => void listQuery.refetch(),
  };
  const createAction: DataGridCreateAction | undefined = canWrite
    ? {
        label: "新建连接",
        description: "创建 testing 状态的 ERP 连接",
        disabled: busy,
        onClick: () => {
          setNotice(null);
          setForm(emptyForm());
          setCreateOpen(true);
        },
      }
    : undefined;
  const toolbarActions: DataGridToolbarAction[] = canWrite
    ? [
        {
          key: "edit",
          label: "编辑",
          description: "修改端点/路由/secret 会使 active 回 testing",
          icon: <Pencil className="size-4" aria-hidden />,
          disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
          onClick: () => {
            if (!selected) return;
            setNotice(null);
            setEditForm({
              connector_name: selected.connector_name,
              channel_mode: selected.channel_mode,
              api_base_url: selected.api_base_url ?? "",
              bearer_secret_alias: selected.bearer_secret_alias ?? "",
              interface_probe_db_username: selected.interface_probe_db_username ?? "",
              // GET 只返回 alias 是否已配置；旧 alias 不回显，留空表示保持现值。
              interface_probe_db_password_alias: "",
              expected_config_version: selected.config_version,
              expected_probe_config_version: selected.interface_probe_config_version,
            });
            setEditOpen(true);
          },
        },
        {
          key: "test",
          label: "测试",
          description: "测试当前版本（不写业务单据）",
          icon: <Play className="size-4" aria-hidden />,
          disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
          onClick: () => confirmDialog.openWith("test"),
        },
        {
          key: "activate",
          label: STATUS_ENABLED,
          description: "当前版本测试通过后启用",
          icon: <Power className="size-4" aria-hidden />,
          disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
          onClick: () => confirmDialog.openWith("activate"),
        },
        {
          key: "disable",
          label: STATUS_DISABLED,
          description: "停用 active 连接并暂停在途",
          icon: <PowerOff className="size-4" aria-hidden />,
          disabled: (ctx) =>
            ctx.selectedRowKeys.length !== 1 || selected?.status !== "active" || busy,
          onClick: () => confirmDialog.openWith("disable"),
        },
        {
          key: "delete",
          label: "删除",
          description: "仅从未启用且无引用可删",
          icon: <Trash2 className="size-4" aria-hidden />,
          disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
          onClick: () => confirmDialog.openWith("delete"),
        },
      ]
    : [];

  async function run(action: () => Promise<unknown>, ok: string) {
    setNotice(null);
    try {
      await action();
      setNotice({ type: "success", text: ok });
      setSelectedRowKeys([]);
      confirmDialog.close();
    } catch (error) {
      confirmDialog.close();
      setNotice({
        type: "error",
        text: error instanceof Error ? error.message : "操作失败",
      });
    }
  }

  function confirmTitle(action: ConfirmAction | null): string {
    switch (action) {
      case "test":
        return "确认测试连接？";
      case "activate":
        return "确认启用连接？";
      case "disable":
        return "确认停用连接？";
      case "delete":
        return "确认删除连接？";
      default:
        return "";
    }
  }

  function applyGridQueryState(queryState: unknown) {
    applyQuery(queryValueFromUnknown(queryState));
  }

  function clearGridQueryState() {
    resetQuery();
  }

  function executeConfirm() {
    const confirmAction = confirmDialog.target;
    if (!selected || !confirmDialog.open || !confirmAction) return;
    const id = selected.id;
    if (confirmAction === "test") {
      void run(() => testMutation.mutateAsync(id), "测试已完成");
    } else if (confirmAction === "activate") {
      void run(() => activateMutation.mutateAsync(id), "已启用");
    } else if (confirmAction === "disable") {
      void run(() => disableMutation.mutateAsync(id), STATUS_DEACTIVATED);
    } else if (confirmAction === "delete") {
      void run(() => deleteMutation.mutateAsync(id), "已删除");
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        actions={
          onBack ? (
            <Button variant="outline" onClick={onBack}>
              返回
            </Button>
          ) : undefined
        }
      />
      {notice && (
        <div
          className={cn(
            "rounded-md border px-3 py-2 text-sm",
            notice.type === "success"
              ? "border-wms-success/30 bg-wms-success/10 text-wms-success"
              : "border-destructive/30 bg-destructive/10 text-destructive",
          )}
          role={notice.type === "success" ? "status" : "alert"}
        >
          {notice.text}
        </div>
      )}
      <QueryPanel
        fields={h8ErpConnectorQueryFields}
        defaultVisibleFieldKeys={h8ErpConnectorCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeQuery(next))}
        onQuery={() => applyQuery(draftQuery)}
        onReset={resetQuery}
      />
      <Card className="rounded-lg shadow-sm">
        <CardContent className="p-5">
          <DataGrid
            storageKey="h8.erp-connectors"
            columns={columns}
            data={filteredRows}
            rowKey={(row) => row.id}
            selectable
            selectedRowKeys={selectedRowKeys}
            onSelectedRowKeysChange={setSelectedRowKeys}
            caption={listQuery.isPending ? "加载 ERP 连接..." : undefined}
            emptyTitle={listQuery.isError ? "读取失败" : "暂无 ERP 连接"}
            emptyDescription="请新建连接并测试通过后启用"
            refreshAction={refreshAction}
            createAction={createAction}
            toolbarActions={toolbarActions}
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            onApplyQueryState={applyGridQueryState}
            onClearQueryState={clearGridQueryState}
          />
        </CardContent>
      </Card>

      <Dialog open={createOpen} onOpenChange={(open) => !busy && setCreateOpen(open)}>
        {/* 表单较长（含 API Key 下拉），限制高度并允许滚动，避免“保存”按钮溢出视口不可点 */}
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
          <form
            className="grid gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              void run(async () => {
                await createMutation.mutateAsync(form);
                setCreateOpen(false);
              }, "已创建（testing）");
            }}
          >
            <DialogHeader>
              <DialogTitle>新建 ERP 连接</DialogTitle>
              <DialogDescription>
                密钥只使用 alias / API Key 引用。新建进入 testing，须测试通过后人工启用。
              </DialogDescription>
            </DialogHeader>
            {/* 提交失败提示必须渲染在弹窗内部，避免被模态遮挡（页面层保留成功提示）。 */}
            <ErrorNotice notice={createOpen ? notice : null} />
            <label className="grid gap-1 text-sm">
              连接编码
              <Input
                required
                value={form.connector_code}
                onChange={(e) => setForm((f) => ({ ...f, connector_code: e.target.value }))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              连接名称
              <Input
                required
                value={form.connector_name}
                onChange={(e) => setForm((f) => ({ ...f, connector_name: e.target.value }))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              方向
              <select
                className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                value={form.directions[0] ?? "inbound"}
                onChange={(e) =>
                  setForm((f) => ({
                    ...f,
                    directions: [e.target.value],
                    // outbound_order 属入站目录（ERP 下发出库单到 WMS）；出站默认用发运确认
                    message_types: e.target.value === "outbound" ? ["shipment_confirm"] : ["asn"],
                  }))
                }
              >
                <option value="inbound">inbound</option>
                <option value="outbound">outbound</option>
              </select>
            </label>
            <label className="grid gap-1 text-sm">
              API Key（入站鉴权）
              <select
                required
                className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                value={form.api_key_id ?? ""}
                onChange={(e) => setForm((f) => ({ ...f, api_key_id: e.target.value }))}
              >
                <option value="" disabled>
                  {apiKeysQuery.isPending ? "API Key 加载中..." : apiKeyOptions.length === 0 ? "无可用 API Key（请先在 H1 创建）" : "请选择 API Key"}
                </option>
                {apiKeyOptions.map((key) => (
                  <option key={key.key_id} value={key.key_id}>
                    {key.caller_name} · {key.purpose}
                  </option>
                ))}
              </select>
            </label>
            <label className="grid gap-1 text-sm">
              通道模式
              <select
                className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                value={form.channel_mode}
                onChange={(e) => setForm((f) => ({ ...f, channel_mode: e.target.value }))}
              >
                <option value="rest">rest</option>
                <option value="interface_table">interface_table</option>
                <option value="rest_primary_table_fallback">rest_primary_table_fallback</option>
              </select>
            </label>
            <label className="grid gap-1 text-sm">
              REST 地址
              <Input
                value={form.api_base_url ?? ""}
                onChange={(e) => setForm((f) => ({ ...f, api_base_url: e.target.value }))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              Bearer secret alias（出站）
              <Input
                placeholder="vault://wms/.../bearer"
                value={form.bearer_secret_alias ?? ""}
                onChange={(e) =>
                  setForm((f) => ({
                    ...f,
                    bearer_secret_alias: e.target.value.trim() ? e.target.value.trim() : null,
                  }))
                }
              />
            </label>
            <label className="grid gap-1 text-sm">
              接口表探查账号（只读）
              <Input
                placeholder="wms_h8_probe"
                value={form.interface_probe_db_username ?? ""}
                onChange={(e) =>
                  setForm((f) => ({
                    ...f,
                    interface_probe_db_username: e.target.value.trim() ? e.target.value.trim() : null,
                  }))
                }
              />
            </label>
            <label className="grid gap-1 text-sm">
              接口表探查密码 alias（只读）
              <Input
                placeholder="vault://wms/.../h8-probe"
                value={form.interface_probe_db_password_alias ?? ""}
                onChange={(e) =>
                  setForm((f) => ({
                    ...f,
                    interface_probe_db_password_alias: e.target.value.trim()
                      ? e.target.value.trim()
                      : null,
                  }))
                }
              />
            </label>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  取消
                </Button>
              </DialogClose>
              <Button type="submit" disabled={busy}>
                {busy ? LOADING_SAVING : BUTTON_SAVE}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={editOpen} onOpenChange={(open) => !busy && setEditOpen(open)}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-lg">
          <form
            className="grid gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              if (!selected) return;
              void run(async () => {
                await updateMutation.mutateAsync({
                  id: selected.id,
                  body: {
                    expected_config_version: editForm.expected_config_version,
                    expected_probe_config_version: editForm.expected_probe_config_version,
                    connector_name: editForm.connector_name,
                    channel_mode: editForm.channel_mode,
                    api_base_url: editForm.api_base_url.trim() ? editForm.api_base_url.trim() : null,
                    bearer_secret_alias: editForm.bearer_secret_alias.trim()
                      ? editForm.bearer_secret_alias.trim()
                      : null,
                    interface_probe_db_username: editForm.interface_probe_db_username.trim()
                      ? editForm.interface_probe_db_username.trim()
                      : null,
                    interface_probe_db_password_alias: editForm.interface_probe_db_password_alias.trim()
                      ? editForm.interface_probe_db_password_alias.trim()
                      : null,
                  },
                });
                setEditOpen(false);
              }, "已保存（运行相关字段变更会回 testing 并需复测）");
            }}
          >
            <DialogHeader>
              <DialogTitle>编辑 ERP 连接</DialogTitle>
              <DialogDescription>
                编码不可改。探查凭据使用独立版本，不会使传输测试失效；传输版本 {editForm.expected_config_version}，探查版本 {editForm.expected_probe_config_version}。
              </DialogDescription>
            </DialogHeader>
            {/* 提交失败提示必须渲染在弹窗内部，避免被模态遮挡（页面层保留成功提示）。 */}
            <ErrorNotice notice={editOpen ? notice : null} />
            <label className="grid gap-1 text-sm">
              连接名称
              <Input
                required
                value={editForm.connector_name}
                onChange={(e) => setEditForm((f) => ({ ...f, connector_name: e.target.value }))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              通道模式
              <select
                className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                value={editForm.channel_mode}
                onChange={(e) => setEditForm((f) => ({ ...f, channel_mode: e.target.value }))}
              >
                <option value="rest">rest</option>
                <option value="interface_table">interface_table</option>
                <option value="rest_primary_table_fallback">rest_primary_table_fallback</option>
              </select>
            </label>
            <label className="grid gap-1 text-sm">
              REST 地址
              <Input
                value={editForm.api_base_url}
                onChange={(e) => setEditForm((f) => ({ ...f, api_base_url: e.target.value }))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              Bearer secret alias
              <Input
                value={editForm.bearer_secret_alias}
                onChange={(e) => setEditForm((f) => ({ ...f, bearer_secret_alias: e.target.value }))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              接口表探查账号（只读）
              <Input
                value={editForm.interface_probe_db_username}
                onChange={(e) => setEditForm((f) => ({ ...f, interface_probe_db_username: e.target.value }))}
              />
            </label>
            <label className="grid gap-1 text-sm">
              接口表探查密码 alias（只读）
              <Input
                placeholder={selected?.interface_probe_db_password_alias_set ? "已配置（留空保持）" : "vault://wms/.../h8-probe"}
                value={editForm.interface_probe_db_password_alias}
                onChange={(e) => setEditForm((f) => ({ ...f, interface_probe_db_password_alias: e.target.value }))}
              />
            </label>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  取消
                </Button>
              </DialogClose>
              <Button type="submit" disabled={busy || !selected}>
                {busy ? LOADING_SAVING : BUTTON_SAVE}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={confirmDialog.open}
        onOpenChange={(open) => !busy && !open && confirmDialog.close()}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{confirmTitle(confirmDialog.target)}</DialogTitle>
            <DialogDescription>
              {selected
                ? `${selected.connector_code} · ${selected.connector_name}`
                : "请先选择一行"}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={busy}>
                取消
              </Button>
            </DialogClose>
            <Button type="button" disabled={busy || !selected} onClick={executeConfirm}>
              {busy ? "处理中..." : "确认"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}

function ErrorNotice({ notice }: { notice: Notice }) {
  if (notice?.type !== "error") return null;
  return (
    <div
      className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
      role="alert"
    >
      {notice.text}
    </div>
  );
}

function textColumn(
  key: keyof H8ErpConnector,
  header: string,
  width: number,
): DataGridColumn<H8ErpConnector> {
  return {
    key: String(key),
    header,
    width,
    render: (row) => String(row[key] ?? "-"),
  };
}

function statusVariant(value: string): "completed" | "isolated" | "expired" {
  return value === "active" ? "completed" : value === "testing" ? "isolated" : "expired";
}

