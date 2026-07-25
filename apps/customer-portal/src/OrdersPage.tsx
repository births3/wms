import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertCircle,
  Archive,
  CheckCircle2,
  Download,
  FileClock,
  FileText,
  MapPin,
  Package,
  Search,
  X,
} from "lucide-react";
import {
  Button,
  Checkbox,
  Input,
  StatusBadge,
} from "@wms/ui";

import {
  authorizeReportDownload,
  createExport,
  getOrder,
  listAddresses,
  listOrders,
} from "./api";
import type { LoginResponse, OrderSummary, ReportSummary } from "./types";

export function OrdersPage(props: {
  session: LoginResponse;
  onOpenExports: () => void;
}) {
  const token = props.session.access_token;
  const queryClient = useQueryClient();
  const [keywordInput, setKeywordInput] = useState("");
  const [keyword, setKeyword] = useState("");
  const [addressId, setAddressId] = useState("");
  const [status, setStatus] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [includeHistory, setIncludeHistory] = useState(false);
  const [message, setMessage] = useState("");

  const addresses = useQuery({
    queryKey: ["portal-addresses"],
    queryFn: () => listAddresses(token),
  });
  const orders = useQuery({
    queryKey: ["portal-orders", addressId, status, keyword],
    queryFn: () =>
      listOrders(token, {
        addressId: addressId || undefined,
        status: status || undefined,
        keyword: keyword || undefined,
      }),
  });
  const exportMutation = useMutation({
    mutationFn: () => createExport(token, selected, includeHistory),
    onSuccess: async () => {
      setMessage("批量任务已创建，可在导出中心查看进度");
      setSelected([]);
      await queryClient.invalidateQueries({ queryKey: ["portal-exports"] });
    },
  });

  const allVisibleSelected =
    Boolean(orders.data?.length) &&
    orders.data?.every((order) => selected.includes(order.id));
  const toggleAll = (checked: boolean) => {
    const visibleIds = orders.data?.map((order) => order.id) ?? [];
    setSelected((current) =>
      checked
        ? Array.from(new Set([...current, ...visibleIds]))
        : current.filter((id) => !visibleIds.includes(id)),
    );
  };

  return (
    <div className="space-y-5" data-testid="portal-orders-page">
      <section className="flex flex-col justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <div className="text-sm font-medium text-emerald-700">订单资料</div>
          <h1 className="mt-1 text-2xl font-semibold">订单与药检单</h1>
          <p className="mt-1 text-sm text-slate-500">
            从已发货或已签收订单进入商品批号资料，不提供跨订单批号查询。
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          onClick={props.onOpenExports}
        >
          <Archive className="mr-2 size-4" />
          导出中心
        </Button>
      </section>

      <section className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm">
        <div className="grid gap-3 md:grid-cols-[minmax(220px,1fr)_220px_160px_auto]">
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-2.5 size-4 text-slate-400" />
            <Input
              value={keywordInput}
              onChange={(event) => setKeywordInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") setKeyword(keywordInput.trim());
              }}
              className="pl-9"
              placeholder="订单号 / 商品 / 批号"
              aria-label="订单关键词"
            />
          </div>
          <select
            className="h-10 rounded-md border border-input bg-background px-3 text-sm"
            value={addressId}
            onChange={(event) => setAddressId(event.target.value)}
            aria-label="客户地址"
            data-testid="portal-address-filter"
          >
            <option value="">全部授权地址</option>
            {addresses.data?.map((address) => (
              <option key={address.id} value={address.id}>
                {address.address_code} · {address.address_name}
              </option>
            ))}
          </select>
          <select
            className="h-10 rounded-md border border-input bg-background px-3 text-sm"
            value={status}
            onChange={(event) => setStatus(event.target.value)}
            aria-label="订单状态"
          >
            <option value="">全部状态</option>
            <option value="shipped">已发货</option>
            <option value="signed">已签收</option>
          </select>
          <Button type="button" onClick={() => setKeyword(keywordInput.trim())}>
            查询
          </Button>
        </div>
      </section>

      {message ? (
        <div className="flex items-center justify-between rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
          <span className="flex items-center gap-2">
            <CheckCircle2 className="size-4" />
            {message}
          </span>
          <button type="button" onClick={() => setMessage("")} aria-label="关闭提示">
            <X className="size-4" />
          </button>
        </div>
      ) : null}

      <section className="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm">
        <div className="flex flex-col justify-between gap-3 border-b border-slate-200 px-4 py-3 sm:flex-row sm:items-center">
          <div className="flex items-center gap-3 text-sm text-slate-600">
            <Checkbox
              checked={allVisibleSelected}
              onCheckedChange={(value) => toggleAll(value === true)}
              aria-label="选择当前结果全部订单"
            />
            <span>
              共 {orders.data?.length ?? 0} 个订单，已选 {selected.length} 个
            </span>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            {props.session.user.can_view_report_history ? (
              <label className="flex items-center gap-2 text-sm text-slate-600">
                <Checkbox
                  checked={includeHistory}
                  onCheckedChange={(value) => setIncludeHistory(value === true)}
                />
                批量包含历史版本
              </label>
            ) : null}
            <Button
              type="button"
              disabled={!selected.length || exportMutation.isPending}
              onClick={() => exportMutation.mutate()}
              data-testid="portal-create-export"
            >
              <Archive className="mr-2 size-4" />
              {exportMutation.isPending ? "创建中…" : "批量下载"}
            </Button>
          </div>
        </div>
        {exportMutation.error ? (
          <ErrorBanner error={exportMutation.error} />
        ) : null}
        {orders.isPending ? (
          <EmptyState text="正在读取独立查询库…" />
        ) : orders.error ? (
          <ErrorBanner error={orders.error} />
        ) : orders.data?.length ? (
          <div className="overflow-x-auto">
            <table className="portal-table">
              <thead>
                <tr>
                  <th className="w-12">选择</th>
                  <th>订单号</th>
                  <th>客户地址</th>
                  <th>状态</th>
                  <th>发货时间</th>
                  <th>资料状态</th>
                  <th className="w-24">操作</th>
                </tr>
              </thead>
              <tbody>
                {orders.data.map((order) => (
                  <OrderRow
                    key={order.id}
                    order={order}
                    checked={selected.includes(order.id)}
                    onCheck={(checked) =>
                      setSelected((current) =>
                        checked
                          ? [...current, order.id]
                          : current.filter((id) => id !== order.id),
                      )
                    }
                    onOpen={() => setDetailId(order.id)}
                  />
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyState text="当前账号和筛选范围内没有可查询订单" />
        )}
      </section>

      {detailId ? (
        <OrderDetailPanel
          token={token}
          orderId={detailId}
          onClose={() => setDetailId(null)}
        />
      ) : null}
    </div>
  );
}

function OrderRow(props: {
  order: OrderSummary;
  checked: boolean;
  onCheck: (checked: boolean) => void;
  onOpen: () => void;
}) {
  const available = props.order.available_report_count;
  const pending = props.order.pending_report_count;
  return (
    <tr data-testid={`portal-order-${props.order.order_no}`}>
      <td>
        <Checkbox
          checked={props.checked}
          onCheckedChange={(value) => props.onCheck(value === true)}
          aria-label={`选择订单 ${props.order.order_no}`}
        />
      </td>
      <td className="font-medium text-slate-900">{props.order.order_no}</td>
      <td>
        <span className="flex items-center gap-1.5">
          <MapPin className="size-4 text-slate-400" />
          {props.order.address_name}
        </span>
      </td>
      <td>
        <StatusBadge
          status={props.order.status === "signed" ? "completed" : "in_progress"}
          size="sm"
          label={props.order.status === "signed" ? "已签收" : "已发货"}
        />
      </td>
      <td>{formatTime(props.order.shipped_at)}</td>
      <td>
        <div className="flex flex-wrap gap-2">
          {available ? (
            <span className="text-xs font-medium text-emerald-700">{available} 份可下载</span>
          ) : null}
          {pending ? (
            <span className="text-xs font-medium text-amber-700">{pending} 项暂缺/处理中</span>
          ) : null}
        </div>
      </td>
      <td>
        <Button type="button" variant="outline" size="sm" onClick={props.onOpen}>
          查看资料
        </Button>
      </td>
    </tr>
  );
}

function OrderDetailPanel(props: {
  token: string;
  orderId: string;
  onClose: () => void;
}) {
  const detail = useQuery({
    queryKey: ["portal-order", props.orderId],
    queryFn: () => getOrder(props.token, props.orderId),
  });
  const [downloadError, setDownloadError] = useState("");
  const download = async (report: ReportSummary) => {
    setDownloadError("");
    try {
      const authorized = await authorizeReportDownload(props.token, report.id);
      window.location.assign(authorized.url);
    } catch (error) {
      setDownloadError(error instanceof Error ? error.message : "下载授权失败");
    }
  };
  const reports = useMemo(
    () => detail.data?.lines.flatMap((line) => line.reports) ?? [],
    [detail.data],
  );

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/45 p-3 sm:p-8" role="dialog" aria-modal="true">
      <div className="ml-auto flex h-full w-full max-w-3xl flex-col overflow-hidden rounded-2xl bg-white shadow-2xl">
        <div className="flex items-start justify-between border-b border-slate-200 px-6 py-5">
          <div>
            <div className="text-sm font-medium text-emerald-700">订单批号详情</div>
            <h2 className="mt-1 text-xl font-semibold">
              {detail.data?.order_no ?? "正在读取…"}
            </h2>
            <p className="mt-1 text-sm text-slate-500">
              资料下载前会再次校验客户、地址、订单、商品和批号范围。
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            aria-label="关闭订单详情"
            onClick={props.onClose}
          >
            <X className="size-4" />
          </Button>
        </div>
        <div className="flex-1 space-y-4 overflow-y-auto bg-slate-50 p-6">
          {detail.isPending ? <EmptyState text="正在读取订单批号…" /> : null}
          {detail.error ? <ErrorBanner error={detail.error} /> : null}
          {downloadError ? <ErrorBanner error={new Error(downloadError)} /> : null}
          {detail.data?.lines.map((line) => (
            <article
              key={line.id}
              className="rounded-2xl border border-slate-200 bg-white p-5"
              data-testid={`portal-batch-${line.batch_no}`}
            >
              <div className="flex flex-col justify-between gap-3 sm:flex-row">
                <div className="flex gap-3">
                  <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-emerald-50 text-emerald-700">
                    <Package className="size-5" />
                  </div>
                  <div>
                    <div className="font-medium">{line.product_name}</div>
                    <div className="mt-1 text-sm text-slate-500">
                      {line.product_code} · 批号 {line.batch_no} · 数量 {line.quantity}
                    </div>
                  </div>
                </div>
              </div>
              <div className="mt-4 space-y-3">
                {line.reports.length ? (
                  line.reports.map((report) => (
                    <ReportRow key={report.id} report={report} onDownload={() => download(report)} />
                  ))
                ) : (
                  <div className="flex items-center gap-2 rounded-xl bg-amber-50 px-4 py-3 text-sm text-amber-800">
                    <FileClock className="size-4" />
                    资料暂缺
                  </div>
                )}
              </div>
            </article>
          ))}
          {detail.data && !reports.length ? (
            <div className="rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-800">
              当前订单暂无可用客户药检单副本，不影响订单发货状态。
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function ReportRow(props: { report: ReportSummary; onDownload: () => void }) {
  const report = props.report;
  const available = report.customer_copy_status === "available";
  const label = {
    available: "可下载",
    processing: "处理中",
    queued: "排队中",
    failed: "生成失败",
  }[report.customer_copy_status];
  return (
    <div className="rounded-xl border border-slate-200 p-4" data-testid={`portal-report-v${report.version_number}`}>
      <div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
        <div className="flex items-start gap-3">
          <FileText className="mt-0.5 size-5 text-emerald-700" />
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-medium">{report.report_no}</span>
              <StatusBadge
                status={available ? "completed" : report.customer_copy_status === "failed" ? "unqualified" : "pending"}
                size="sm"
                label={label}
              />
              {report.is_current ? (
                <span className="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-600">当前版本</span>
              ) : (
                <span className="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-600">历史版本</span>
              )}
            </div>
            <div className="mt-1 text-xs text-slate-500">
              版本 V{report.version_number} · 确认于 {formatTime(report.confirmed_at)}
            </div>
            {report.modification_reason ? (
              <div className="mt-1 text-xs text-slate-500">
                更正原因：{report.modification_reason}
              </div>
            ) : null}
            {report.digitally_signed_original ? (
              <div className="mt-2 flex items-center gap-1 text-xs text-amber-700">
                <AlertCircle className="size-3.5" />
                客户分发副本可能使原数字签名失效
              </div>
            ) : null}
          </div>
        </div>
        <Button type="button" size="sm" disabled={!available} onClick={props.onDownload}>
          <Download className="mr-2 size-4" />
          {available ? "下载 PDF" : label}
        </Button>
      </div>
    </div>
  );
}

function EmptyState(props: { text: string }) {
  return (
    <div className="grid min-h-40 place-items-center px-6 py-10 text-sm text-slate-500">
      {props.text}
    </div>
  );
}

function ErrorBanner(props: { error: unknown }) {
  return (
    <div role="alert" className="m-4 rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700">
      {props.error instanceof Error ? props.error.message : "请求失败"}
    </div>
  );
}

function formatTime(value: string) {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}
