/**
 * US-H8-003：H8 ERP 消息列表 / 详情 / 重放。
 * 列表型：QueryPanel + DataGrid；详情与重放仅弹窗，不常驻完整报文。
 */
import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  QueryPanel,
  type DataGridColumn,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Eye, RefreshCw, RotateCcw } from "lucide-react";

import {
  useErpMessageDetailQuery,
  useErpMessagesQuery,
  useErpMessageStatsQuery,
  useReplayErpMessageMutation,
  type H8ErpMessage,
} from "@/features/config-center/erp-message-queries";
import { useCurrentUserQuery } from "@/features/auth/auth-queries";

export const H8_ERP_CONNECTOR_READ = "h8.erp_connector.read";
export const H8_ERP_CONNECTOR_WRITE = "h8.erp_connector.write";

export const h8ErpMessageQueryFields: QueryPanelField[] = [
  {
    key: "direction",
    label: "方向",
    type: "select",
    options: [
      { label: "全部", value: "" },
      { label: "inbound", value: "inbound" },
      { label: "outbound", value: "outbound" },
    ],
  },
  {
    key: "message_type",
    label: "消息类型",
    type: "text",
    placeholder: "asn / putaway_complete …",
  },
  {
    key: "status",
    label: "状态",
    type: "select",
    options: [
      { label: "全部", value: "" },
      { label: "pending", value: "pending" },
      { label: "processing", value: "processing" },
      { label: "succeeded", value: "succeeded" },
      { label: "failed", value: "failed" },
      { label: "dead", value: "dead" },
      { label: "acked", value: "acked" },
    ],
  },
  { key: "connector_code", label: "连接编码", type: "text" },
  { key: "external_ref", label: "外部业务标识", type: "text" },
  { key: "idempotency_key", label: "Idempotency-Key", type: "text" },
  { key: "correlation_id", label: "correlation", type: "text" },
];

export const h8ErpMessageCoreQueryFieldKeys = ["direction", "message_type", "status"];

const columns: DataGridColumn<H8ErpMessage>[] = [
  { key: "direction", header: "方向", width: 90 },
  { key: "message_type", header: "消息类型", width: 140 },
  { key: "sync_status", header: "状态", width: 100 },
  { key: "channel", header: "通道", width: 110 },
  {
    key: "connector_code",
    header: "连接",
    width: 120,
    render: (row) => row.connector_code ?? "—",
  },
  { key: "external_ref", header: "外部标识", width: 140 },
  { key: "retry_count", header: "重试", width: 70 },
  {
    key: "last_error_summary",
    header: "错误摘要",
    width: 200,
    render: (row) => row.last_error_summary ?? "—",
  },
  {
    key: "created_at",
    header: "创建时间",
    width: 170,
    render: (row) => new Date(row.created_at).toLocaleString(),
  },
];

function defaultQuery(): QueryPanelValue {
  return {
    direction: "",
    message_type: "",
    status: "",
    connector_code: "",
    external_ref: "",
    idempotency_key: "",
    correlation_id: "",
  };
}

function filterClient(rows: H8ErpMessage[], query: QueryPanelValue): H8ErpMessage[] {
  return rows.filter((row) => {
    if (query.connector_code && !(row.connector_code ?? "").includes(String(query.connector_code))) {
      return false;
    }
    if (query.external_ref && !row.external_ref.includes(String(query.external_ref))) {
      return false;
    }
    if (query.idempotency_key && !row.idempotency_key.includes(String(query.idempotency_key))) {
      return false;
    }
    if (query.correlation_id && !row.correlation_id.includes(String(query.correlation_id))) {
      return false;
    }
    return true;
  });
}

export function ErpMessageLogPage() {
  const { data: currentUser } = useCurrentUserQuery(true);
  const canWrite = Boolean(currentUser?.permissions.includes(H8_ERP_CONNECTOR_WRITE));
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultQuery());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultQuery());
  const [selectedKeys, setSelectedKeys] = React.useState<string[]>([]);
  const [detailId, setDetailId] = React.useState<string | null>(null);
  const [replayOpen, setReplayOpen] = React.useState(false);
  const [replayReason, setReplayReason] = React.useState("");
  const [notice, setNotice] = React.useState<string | null>(null);

  const listQuery = useErpMessagesQuery({
    direction: String(appliedQuery.direction || ""),
    message_type: String(appliedQuery.message_type || ""),
    status: String(appliedQuery.status || ""),
  });
  const statsQuery = useErpMessageStatsQuery();
  const detailQuery = useErpMessageDetailQuery(detailId);
  const replayMutation = useReplayErpMessageMutation();

  const rows = filterClient(listQuery.data?.data ?? [], appliedQuery);
  const selected = rows.find((r) => r.id === selectedKeys[0]);
  const busy = replayMutation.isPending;

  const toolbarActions: DataGridToolbarAction[] = [
    {
      key: "refresh",
      label: "刷新",
      icon: <RefreshCw className="size-4" aria-hidden />,
      onClick: () => {
        void listQuery.refetch();
        void statsQuery.refetch();
      },
    },
    {
      key: "detail",
      label: "详情",
      icon: <Eye className="size-4" aria-hidden />,
      disabled: (ctx) => ctx.selectedRowKeys.length !== 1,
      onClick: () => {
        if (selected) setDetailId(selected.id);
      },
    },
    ...(canWrite
      ? [
          {
            key: "replay",
            label: "重放",
            icon: <RotateCcw className="size-4" aria-hidden />,
            disabled: (ctx: { selectedRowKeys: string[] }) => {
              if (ctx.selectedRowKeys.length !== 1 || !selected) return true;
              return selected.sync_status !== "failed" && selected.sync_status !== "dead";
            },
            onClick: () => {
              setReplayReason("");
              setReplayOpen(true);
            },
          } satisfies DataGridToolbarAction,
        ]
      : []),
  ];

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 p-4">
      <header className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <h1 className="text-xl font-semibold">H8 ERP 消息</h1>
          <p className="text-sm text-muted-foreground">
            运行日志与死信重放 · {canWrite ? "可重放" : "只读"}
          </p>
        </div>
        {statsQuery.data ? (
          <div className="flex flex-wrap gap-3 text-sm text-muted-foreground">
            <span>合计 {statsQuery.data.total}</span>
            <span>成功 {statsQuery.data.succeeded}</span>
            <span>失败 {statsQuery.data.failed}</span>
            <span>死信 {statsQuery.data.dead}</span>
            <span>重试累计 {statsQuery.data.retry_total}</span>
          </div>
        ) : null}
      </header>

      {notice ? (
        <div className="rounded-md border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900">
          {notice}
        </div>
      ) : null}

      <QueryPanel
        fields={h8ErpMessageQueryFields}
        defaultVisibleFieldKeys={h8ErpMessageCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => setAppliedQuery(draftQuery)}
        onReset={() => {
          const q = defaultQuery();
          setDraftQuery(q);
          setAppliedQuery(q);
        }}
      />

      <DataGrid
        storageKey="h8.erp-messages"
        columns={columns}
        data={rows}
        rowKey={(row) => row.id}
        selectable
        selectedRowKeys={selectedKeys}
        onSelectedRowKeysChange={setSelectedKeys}
        toolbarActions={toolbarActions}
        emptyTitle={listQuery.isError ? "加载失败" : "暂无消息"}
        emptyDescription="失败与死信消息可在此查询与重放"
        caption={listQuery.isLoading ? "加载 ERP 消息..." : undefined}
      />

      <Dialog open={detailId != null} onOpenChange={(open) => !open && setDetailId(null)}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>消息详情</DialogTitle>
            <DialogDescription>仅展示脱敏摘要与尝试时间线，不含完整报文。</DialogDescription>
          </DialogHeader>
          {detailQuery.data ? (
            <div className="grid max-h-[60vh] gap-3 overflow-auto text-sm">
              <div className="grid grid-cols-2 gap-2">
                <div>状态：{detailQuery.data.message.sync_status}</div>
                <div>类型：{detailQuery.data.message.message_type}</div>
                <div>通道：{detailQuery.data.message.channel}</div>
                <div>外部标识：{detailQuery.data.message.external_ref}</div>
                <div className="col-span-2">Idempotency：{detailQuery.data.message.idempotency_key}</div>
                <div className="col-span-2">correlation：{detailQuery.data.message.correlation_id}</div>
                <div className="col-span-2">
                  摘要：{detailQuery.data.message.last_error_summary ?? "—"}
                </div>
                <div className="col-span-2">digest：{detailQuery.data.message.payload_digest}</div>
              </div>
              <div>
                <div className="mb-1 font-medium">尝试记录</div>
                <ul className="space-y-1">
                  {detailQuery.data.attempts.map((a) => (
                    <li key={a.id} className="rounded border px-2 py-1">
                      #{a.attempt_no} {a.result} · {a.actor}
                      {a.error_summary ? ` · ${a.error_summary}` : ""}
                    </li>
                  ))}
                  {detailQuery.data.attempts.length === 0 ? <li>无尝试记录</li> : null}
                </ul>
              </div>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">加载中…</p>
          )}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">
                关闭
              </Button>
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={replayOpen} onOpenChange={(open) => !busy && setReplayOpen(open)}>
        <DialogContent className="sm:max-w-md">
          <form
            className="grid gap-3"
            onSubmit={(event) => {
              event.preventDefault();
              if (!selected) return;
              void (async () => {
                await replayMutation.mutateAsync({
                  id: selected.id,
                  body: { reason: replayReason.trim(), confirmed: true },
                });
                setReplayOpen(false);
                setNotice("已提交重放（复用原幂等键）");
                void listQuery.refetch();
              })();
            }}
          >
            <DialogHeader>
              <DialogTitle>确认重放</DialogTitle>
              <DialogDescription>
                仅 failed/dead 可重放；复用原 Idempotency-Key，不复制新业务消息。
              </DialogDescription>
            </DialogHeader>
            <label className="grid gap-1 text-sm">
              原因（必填）
              <Input
                required
                value={replayReason}
                onChange={(e) => setReplayReason(e.target.value)}
                placeholder="说明重放原因"
              />
            </label>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  取消
                </Button>
              </DialogClose>
              <Button type="submit" disabled={busy || !replayReason.trim()}>
                {busy ? "提交中…" : "确认重放"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
