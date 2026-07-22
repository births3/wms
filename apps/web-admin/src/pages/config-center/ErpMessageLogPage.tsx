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
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  type DataGridColumn,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
} from "@wms/ui";
import { Eye, FileJson, RefreshCw, RotateCcw } from "lucide-react";

import {
  useErpMessageDetailQuery,
  useDecryptH8PayloadMutation,
  useErpMessagesQuery,
  useErpMessageStatsQuery,
  useReplayErpMessageMutation,
  type H8ErpMessage,
} from "@/features/config-center/erp-message-queries";
import { useCurrentUserQuery } from "@/features/auth/auth-queries";
import { H8WorkerRuntimePanel } from "./H8WorkerRuntimePanel";

export const H8_ERP_CONNECTOR_READ = "h8.erp_connector.read";
export const H8_ERP_CONNECTOR_WRITE = "h8.erp_connector.write";

const messageTypeOptions = [
  { label: "预到货通知（ASN）", value: "asn" },
  { label: "出库订单", value: "outbound_order" },
  { label: "退货申请", value: "return_order" },
  { label: "商品主数据", value: "product_master" },
  { label: "商品主数据变更", value: "product_change" },
  { label: "入库完成", value: "putaway_complete" },
  { label: "库存状态", value: "inventory_status" },
  { label: "库存调整", value: "stock_adjustment" },
  { label: "档案修订", value: "archive_revision" },
  { label: "对账差异", value: "reconciliation_diff" },
  { label: "发货确认", value: "shipment_confirm" },
  { label: "库存快照", value: "inventory_snapshot" },
];

export const h8ErpMessageQueryFields: QueryPanelField[] = [
  {
    key: "direction",
    label: "方向",
    type: "select",
    options: [
      { label: "入站", value: "inbound" },
      { label: "出站", value: "outbound" },
    ],
  },
  {
    key: "message_type",
    label: "消息类型",
    type: "select",
    options: messageTypeOptions,
  },
  {
    key: "channel",
    label: "通道",
    type: "select",
    options: [
      { label: "REST", value: "rest" },
      { label: "接口表", value: "interface_table" },
    ],
  },
  {
    key: "status",
    label: "状态",
    type: "select",
    options: [
      { label: "待处理", value: "pending" },
      { label: "处理中", value: "processing" },
      { label: "成功", value: "succeeded" },
      { label: "等待回执", value: "awaiting_receipt" },
      { label: "失败", value: "failed" },
      { label: "死信", value: "dead" },
      { label: "已回执", value: "acked" },
    ],
  },
  { key: "connector_code", label: "连接编码", type: "text" },
  { key: "warehouse_id", label: "仓库", type: "select", options: [] },
  { key: "external_ref", label: "外部业务标识", type: "text" },
  { key: "idempotency_key", label: "幂等键（Idempotency-Key）", type: "text" },
  { key: "correlation_id", label: "关联标识（Correlation）", type: "text" },
  { key: "created_at", label: "创建时间", type: "dateRange" },
];

export const h8ErpMessageCoreQueryFieldKeys = ["direction", "message_type", "status"];

function messageStatusLabel(status: string): string {
  return (
    {
      pending: "待处理",
      processing: "处理中",
      succeeded: "成功",
      awaiting_receipt: "等待回执",
      failed: "失败",
      dead: "死信",
      acked: "已回执",
    }[status] ?? status
  );
}

function messageTypeLabel(messageType: string): string {
  return messageTypeOptions.find((option) => option.value === messageType)?.label ?? messageType;
}

function messageChannelLabel(channel: string): string {
  return channel === "rest" ? "REST" : channel === "interface_table" ? "接口表" : channel;
}

function attemptResultLabel(result: string): string {
  return (
    {
      succeeded: "成功",
      failed: "失败",
      dead: "死信",
      replayed: "已重放",
      claimed: "已认领",
      archived: "已归档",
    }[result] ?? result
  );
}

const columns: DataGridColumn<H8ErpMessage>[] = [
  {
    key: "direction",
    header: "方向",
    width: 90,
    render: (row) => (row.direction === "inbound" ? "入站" : "出站"),
  },
  {
    key: "message_type",
    header: "消息类型",
    width: 160,
    render: (row) => messageTypeLabel(row.message_type),
  },
  {
    key: "sync_status",
    header: "状态",
    width: 100,
    render: (row) => messageStatusLabel(row.sync_status),
  },
  {
    key: "channel",
    header: "通道",
    width: 110,
    render: (row) => messageChannelLabel(row.channel),
  },
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
    render: (row) => formatDateTime(row.created_at),
  },
];

function defaultQuery(): QueryPanelValue {
  return {
    direction: "",
    message_type: "",
    status: "",
    channel: "",
    connector_code: "",
    warehouse_id: "",
    external_ref: "",
    idempotency_key: "",
    correlation_id: "",
    created_at: {},
  };
}

function queryRange(value: QueryPanelValue[string]): QueryPanelRangeValue {
  return value && typeof value === "object" && !Array.isArray(value) ? value : {};
}

function toIsoDay(value: string | undefined, end = false): string | undefined {
  if (!value) return undefined;
  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(
    year,
    month - 1,
    day,
    end ? 23 : 0,
    end ? 59 : 0,
    end ? 59 : 0,
    end ? 999 : 0,
  );
  return Number.isNaN(date.getTime()) ? undefined : date.toISOString();
}

function formatDateTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false });
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
  const [notice, setNotice] = React.useState<{ message: string; isError: boolean } | null>(null);
  const [payloadOpen, setPayloadOpen] = React.useState(false);
  const createdAt = queryRange(appliedQuery.created_at);

  const listQuery = useErpMessagesQuery({
    direction: String(appliedQuery.direction || ""),
    message_type: String(appliedQuery.message_type || ""),
    status: String(appliedQuery.status || ""),
    connector_code: String(appliedQuery.connector_code || ""),
    channel: String(appliedQuery.channel || ""),
    warehouse_id: String(appliedQuery.warehouse_id || ""),
    external_ref: String(appliedQuery.external_ref || ""),
    idempotency_key: String(appliedQuery.idempotency_key || ""),
    correlation_id: String(appliedQuery.correlation_id || ""),
    created_from: toIsoDay(createdAt.from),
    created_to: toIsoDay(createdAt.to, true),
  });
  const statsQuery = useErpMessageStatsQuery({
    connector_code: String(appliedQuery.connector_code || ""),
    channel: String(appliedQuery.channel || ""),
    message_type: String(appliedQuery.message_type || ""),
  });
  const detailQuery = useErpMessageDetailQuery(detailId);
  const replayMutation = useReplayErpMessageMutation();
  const payloadMutation = useDecryptH8PayloadMutation();

  const rows = React.useMemo(
    () => listQuery.data?.pages.flatMap((page) => page.data) ?? [],
    [listQuery.data],
  );
  const warehouseOptions = React.useMemo(
    () =>
      Array.from(
        new Set(
          rows
            .map((message) => message.warehouse_id)
            .filter((value): value is string => Boolean(value)),
        ),
      ).map((value) => ({ label: value, value })),
    [rows],
  );
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
        <div
          role={notice.isError ? "alert" : "status"}
          className={`rounded-md border px-3 py-2 text-sm ${
            notice.isError
              ? "border-destructive/30 bg-destructive/10 text-destructive"
              : "border-emerald-200 bg-emerald-50 text-emerald-900"
          }`}
        >
          {notice.message}
        </div>
      ) : null}

      <Tabs defaultValue="messages" className="flex min-h-0 flex-1 flex-col gap-3">
        <TabsList className="w-fit">
          <TabsTrigger value="messages">消息记录</TabsTrigger>
          <TabsTrigger value="workers">Worker 状态</TabsTrigger>
        </TabsList>
        <TabsContent value="messages" className="mt-0 grid min-h-0 gap-3">
          <QueryPanel
            fields={h8ErpMessageQueryFields}
            fieldOptions={{ warehouse_id: warehouseOptions }}
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
          {listQuery.isFetchNextPageError ? (
            <p role="alert" className="text-center text-sm text-destructive">
              加载下一页失败，请重试。
            </p>
          ) : null}
          {listQuery.hasNextPage ? (
            <div className="flex justify-center">
              <Button
                type="button"
                variant="outline"
                onClick={() => void listQuery.fetchNextPage()}
                disabled={listQuery.isFetchingNextPage}
              >
                {listQuery.isFetchingNextPage ? "加载中…" : "加载更多"}
              </Button>
            </div>
          ) : null}
        </TabsContent>
        <TabsContent value="workers" className="mt-0 min-h-0">
          <H8WorkerRuntimePanel
            canWrite={canWrite}
            onNotice={(message, isError = false) => setNotice({ message, isError })}
          />
        </TabsContent>
      </Tabs>

      <Dialog
        open={detailId != null}
        onOpenChange={(open) => {
          if (!open) {
            setDetailId(null);
            setPayloadOpen(false);
            payloadMutation.reset();
          }
        }}
      >
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>消息详情</DialogTitle>
            <DialogDescription>
              默认仅展示脱敏摘要；完整报文仅在启用保留且授权后按需解密。
            </DialogDescription>
          </DialogHeader>
          {detailQuery.data ? (
            <div className="grid max-h-[60vh] gap-3 overflow-auto text-sm">
              <div className="grid grid-cols-2 gap-2">
                <div>状态：{messageStatusLabel(detailQuery.data.message.sync_status)}</div>
                <div>类型：{messageTypeLabel(detailQuery.data.message.message_type)}</div>
                <div>通道：{messageChannelLabel(detailQuery.data.message.channel)}</div>
                <div>外部标识：{detailQuery.data.message.external_ref}</div>
                <div className="col-span-2">
                  幂等键（Idempotency-Key）：{detailQuery.data.message.idempotency_key}
                </div>
                <div className="col-span-2">
                  关联标识（correlation）：{detailQuery.data.message.correlation_id}
                </div>
                <div className="col-span-2">
                  摘要：{detailQuery.data.message.last_error_summary ?? "—"}
                </div>
                <div className="col-span-2">
                  报文摘要（digest）：{detailQuery.data.message.payload_digest}
                </div>
                <div className="col-span-2">
                  完整报文：
                  {detailQuery.data.payload_retained
                    ? `加密保留至 ${formatDateTime(detailQuery.data.payload_expires_at ?? "")}`
                    : "未保留"}
                </div>
              </div>
              <div>
                <div className="mb-1 font-medium">尝试记录</div>
                <ul className="space-y-1">
                  {detailQuery.data.attempts.map((a) => (
                    <li key={a.id} className="rounded border px-2 py-1">
                      #{a.attempt_no} {attemptResultLabel(a.result)} · {a.actor}
                      {a.error_summary ? ` · ${a.error_summary}` : ""}
                    </li>
                  ))}
                  {detailQuery.data.attempts.length === 0 ? <li>无尝试记录</li> : null}
                </ul>
              </div>
            </div>
          ) : detailQuery.isError ? (
            <p role="alert" className="text-sm text-destructive">
              消息详情加载失败，请关闭后重试。
            </p>
          ) : (
            <p className="text-sm text-muted-foreground">加载中…</p>
          )}
          <DialogFooter>
            {canWrite && detailQuery.data?.payload_retained ? (
              <Button
                type="button"
                onClick={() => {
                  if (!detailId) return;
                  void payloadMutation
                    .mutateAsync(detailId)
                    .then(() => {
                      setDetailId(null);
                      setPayloadOpen(true);
                    })
                    .catch(() =>
                      setNotice({ message: "完整报文解密失败，请重试", isError: true }),
                    );
                }}
                disabled={payloadMutation.isPending}
              >
                <FileJson className="size-4" aria-hidden />
                {payloadMutation.isPending ? "解密中…" : "查看完整报文"}
              </Button>
            ) : null}
            <DialogClose asChild>
              <Button type="button" variant="outline">
                关闭
              </Button>
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={payloadOpen}
        onOpenChange={(open) => {
          setPayloadOpen(open);
          if (!open) payloadMutation.reset();
        }}
      >
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>完整报文</DialogTitle>
            <DialogDescription>本次按需解密已写入 H2 审计；关闭后清除页面缓存。</DialogDescription>
          </DialogHeader>
          <pre className="max-h-[60vh] overflow-auto whitespace-pre-wrap break-all rounded-md bg-muted p-3 text-xs">
            {payloadMutation.data?.payload ?? "加载中…"}
          </pre>
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
                try {
                  await replayMutation.mutateAsync({
                    id: selected.id,
                    body: { reason: replayReason.trim(), confirmed: true },
                  });
                  setReplayOpen(false);
                  setNotice({ message: "已提交重放（复用原幂等键）", isError: false });
                  void listQuery.refetch();
                } catch {
                  setNotice({ message: "消息重放失败，请重试", isError: true });
                }
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
