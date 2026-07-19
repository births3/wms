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
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Play, Power, PowerOff, Trash2 } from "lucide-react";

import {
  useActivateErpConnectorMutation,
  useCreateErpConnectorMutation,
  useDeleteErpConnectorMutation,
  useDisableErpConnectorMutation,
  useErpConnectorsQuery,
  useTestErpConnectorMutation,
  type CreateH8ErpConnectorRequest,
  type H8ErpConnector,
} from "@/features/config-center/erp-connector-queries";
import type { CurrentUser } from "@/features/auth/auth-queries";

export const H8_ERP_CONNECTOR_READ = "h8.erp_connector.read";
export const H8_ERP_CONNECTOR_WRITE = "h8.erp_connector.write";

type Notice = { type: "success" | "error"; text: string } | null;
type ConfirmAction = "test" | "activate" | "disable" | "delete" | null;

export const h8ErpConnectorQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: "关键字",
    type: "text",
    placeholder: "连接编码 / 名称",
    ariaLabel: "搜索 ERP 连接",
  },
  {
    key: "status",
    label: "状态",
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
  textColumn("connector_code", "连接编码", 140),
  textColumn("connector_name", "名称", 160),
  textColumn("channel_mode", "通道", 180),
  {
    key: "status",
    header: "状态",
    width: 110,
    render: (row) => (
      <StatusBadge status={statusVariant(row.status)} label={row.status} size="sm" />
    ),
  },
  {
    key: "config_version",
    header: "版本",
    width: 80,
    render: (row) => row.config_version,
  },
  {
    key: "last_tested_succeeded",
    header: "最近测试",
    width: 100,
    render: (row) =>
      row.last_tested_succeeded == null ? "—" : row.last_tested_succeeded ? "通过" : "失败",
  },
  {
    key: "created_at",
    header: "创建时间",
    width: 180,
    render: (row) => formatDateTime(row.created_at),
  },
  {
    key: "updated_at",
    header: "更新时间",
    width: 180,
    render: (row) => formatDateTime(row.updated_at),
  },
  {
    key: "directions",
    header: "方向",
    width: 120,
    render: (row) => row.directions.join(","),
  },
  {
    key: "message_types",
    header: "消息类型",
    width: 180,
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
    api_key_id: crypto.randomUUID(),
    bearer_secret_alias: null,
    interface_db_password_alias: null,
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
  const testMutation = useTestErpConnectorMutation();
  const activateMutation = useActivateErpConnectorMutation();
  const disableMutation = useDisableErpConnectorMutation();
  const deleteMutation = useDeleteErpConnectorMutation();
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [createOpen, setCreateOpen] = React.useState(false);
  const [confirmAction, setConfirmAction] = React.useState<ConfirmAction>(null);
  const [form, setForm] = React.useState<CreateH8ErpConnectorRequest>(() => emptyForm());
  const [notice, setNotice] = React.useState<Notice>(null);
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultQuery());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultQuery());

  const canWrite =
    currentUser?.permissions.includes(H8_ERP_CONNECTOR_WRITE) ??
    // 未注入用户时（仅 dev 兜底）按可写展示，真实壳层始终注入 currentUser
    true;

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
    testMutation.isPending ||
    activateMutation.isPending ||
    disableMutation.isPending ||
    deleteMutation.isPending;

  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
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
          setForm(emptyForm());
          setCreateOpen(true);
        },
      }
    : undefined;
  const toolbarActions: DataGridToolbarAction[] = canWrite
    ? [
        {
          key: "test",
          label: "测试",
          description: "测试当前版本（不写业务单据）",
          icon: <Play className="size-4" aria-hidden />,
          disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
          onClick: () => setConfirmAction("test"),
        },
        {
          key: "activate",
          label: "启用",
          description: "当前版本测试通过后启用",
          icon: <Power className="size-4" aria-hidden />,
          disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
          onClick: () => setConfirmAction("activate"),
        },
        {
          key: "disable",
          label: "停用",
          description: "停用 active 连接并暂停在途",
          icon: <PowerOff className="size-4" aria-hidden />,
          disabled: (ctx) =>
            ctx.selectedRowKeys.length !== 1 || selected?.status !== "active" || busy,
          onClick: () => setConfirmAction("disable"),
        },
        {
          key: "delete",
          label: "删除",
          description: "仅从未启用且无引用可删",
          icon: <Trash2 className="size-4" aria-hidden />,
          disabled: (ctx) => ctx.selectedRowKeys.length !== 1 || busy,
          onClick: () => setConfirmAction("delete"),
        },
      ]
    : [];

  async function run(action: () => Promise<unknown>, ok: string) {
    setNotice(null);
    try {
      await action();
      setNotice({ type: "success", text: ok });
      setSelectedRowKeys([]);
      setConfirmAction(null);
    } catch (error) {
      setConfirmAction(null);
      setNotice({
        type: "error",
        text: error instanceof Error ? error.message : "操作失败",
      });
    }
  }

  function confirmTitle(action: ConfirmAction): string {
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

  function executeConfirm() {
    if (!selected || !confirmAction) return;
    const id = selected.id;
    if (confirmAction === "test") {
      void run(() => testMutation.mutateAsync(id), "测试已完成");
    } else if (confirmAction === "activate") {
      void run(() => activateMutation.mutateAsync(id), "已启用");
    } else if (confirmAction === "disable") {
      void run(() => disableMutation.mutateAsync(id), "已停用");
    } else if (confirmAction === "delete") {
      void run(() => deleteMutation.mutateAsync(id), "已删除");
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="H8 ERP 连接"
        subtitle={`集成中心 · US-H8-001 · ${filteredRows.length}/${rows.length} 条 · ${
          canWrite ? "可维护" : "只读"
        } · 不落明文凭据`}
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
        onQuery={() => setAppliedQuery(normalizeQuery(draftQuery))}
        onReset={() => {
          const next = defaultQuery();
          setDraftQuery(next);
          setAppliedQuery(next);
        }}
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
            onApplyQueryState={(queryState) => {
              const next = normalizeQuery(queryState as QueryPanelValue);
              setDraftQuery(next);
              setAppliedQuery(next);
            }}
            onClearQueryState={() => {
              const next = defaultQuery();
              setDraftQuery(next);
              setAppliedQuery(next);
            }}
          />
        </CardContent>
      </Card>

      <Dialog open={createOpen} onOpenChange={(open) => !busy && setCreateOpen(open)}>
        <DialogContent className="sm:max-w-lg">
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
                    message_types: e.target.value === "outbound" ? ["outbound_order"] : ["asn"],
                  }))
                }
              >
                <option value="inbound">inbound</option>
                <option value="outbound">outbound</option>
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
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
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

      <Dialog
        open={confirmAction != null}
        onOpenChange={(open) => !busy && !open && setConfirmAction(null)}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>{confirmTitle(confirmAction)}</DialogTitle>
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

/** @deprecated 使用 ErpConnectorConfigPage；保留别名避免旧 import 瞬时失败 */
export const ErpConnectorConfigPanel = ErpConnectorConfigPage;

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

function formatDateTime(value: string) {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
