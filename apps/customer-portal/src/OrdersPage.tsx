import { useEffect, useMemo, useRef, useState } from "react";
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
  const orderCount = orders.data?.length ?? 0;
  const availableCount =
    orders.data?.reduce((total, order) => total + order.available_report_count, 0) ?? 0;
  const pendingCount =
    orders.data?.reduce((total, order) => total + order.pending_report_count, 0) ?? 0;
  const toggleAll = (checked: boolean) => {
    const visibleIds = orders.data?.map((order) => order.id) ?? [];
    setSelected((current) =>
      checked
        ? Array.from(new Set([...current, ...visibleIds]))
        : current.filter((id) => !visibleIds.includes(id)),
    );
  };

  return (
    <div className="portal-page" data-testid="portal-orders-page">
      <section className="portal-page-header">
        <div>
          <div className="portal-eyebrow">订单资料</div>
          <h1 className="portal-page-title">订单与药检单</h1>
          <p className="portal-page-description">
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

      <section className="portal-summary-grid" aria-label="订单资料概览">
        <SummaryMetric
          icon={<Package className="size-5" />}
          label="当前订单"
          value={orderCount}
          suffix="单"
        />
        <SummaryMetric
          icon={<FileText className="size-5" />}
          label="可下载资料"
          value={availableCount}
          suffix="份"
          tone="success"
        />
        <SummaryMetric
          icon={<FileClock className="size-5" />}
          label="暂缺或处理中"
          value={pendingCount}
          suffix="项"
          tone={pendingCount ? "warning" : "neutral"}
        />
      </section>

      <section className="portal-filter-panel">
        <div className="portal-filter-grid">
          <label className="portal-filter-field">
            <span>关键词</span>
            <span className="relative">
              <Search className="portal-input-icon" />
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
            </span>
          </label>
          <label className="portal-filter-field">
            <span>客户地址</span>
            <select
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
          </label>
          <label className="portal-filter-field">
            <span>订单状态</span>
            <select
              value={status}
              onChange={(event) => setStatus(event.target.value)}
              aria-label="订单状态"
            >
              <option value="">全部状态</option>
              <option value="shipped">已发货</option>
              <option value="signed">已签收</option>
            </select>
          </label>
          <Button
            type="button"
            className="portal-filter-action"
            onClick={() => setKeyword(keywordInput.trim())}
          >
            查询
          </Button>
        </div>
      </section>

      {message ? (
        <div className="portal-alert portal-alert-success">
          <span className="flex items-center gap-2">
            <CheckCircle2 className="size-4" />
            {message}
          </span>
          <button type="button" onClick={() => setMessage("")} aria-label="关闭提示">
            <X className="size-4" />
          </button>
        </div>
      ) : null}

      <section className="portal-table-shell">
        <div className="portal-table-toolbar">
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
            <table className="portal-table portal-responsive-table">
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

function SummaryMetric(props: {
  icon: React.ReactNode;
  label: string;
  value: number;
  suffix: string;
  tone?: "neutral" | "success" | "warning";
}) {
  return (
    <article className="portal-summary-card" data-tone={props.tone ?? "neutral"}>
      <span className="portal-summary-icon">{props.icon}</span>
      <span>
        <span className="portal-summary-label">{props.label}</span>
        <strong className="portal-summary-value">
          {props.value}
          <small>{props.suffix}</small>
        </strong>
      </span>
    </article>
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
      <td data-mobile="select">
        <Checkbox
          checked={props.checked}
          onCheckedChange={(value) => props.onCheck(value === true)}
          aria-label={`选择订单 ${props.order.order_no}`}
        />
      </td>
      <td data-label="订单号" className="font-medium text-slate-900">
        {props.order.order_no}
      </td>
      <td data-label="客户地址">
        <span className="flex items-center gap-1.5">
          <MapPin className="size-4 text-slate-400" />
          {props.order.address_name}
        </span>
      </td>
      <td data-label="状态">
        <StatusBadge
          status={props.order.status === "signed" ? "completed" : "in_progress"}
          size="sm"
          label={props.order.status === "signed" ? "已签收" : "已发货"}
        />
      </td>
      <td data-label="发货时间">{formatTime(props.order.shipped_at)}</td>
      <td data-label="资料状态">
        <div className="flex flex-wrap gap-2">
          {available ? (
            <span className="text-xs font-medium text-emerald-700">{available} 份可下载</span>
          ) : null}
          {pending ? (
            <span className="text-xs font-medium text-amber-700">{pending} 项暂缺/处理中</span>
          ) : null}
        </div>
      </td>
      <td data-mobile="action">
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
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
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
  useEffect(() => {
    const previousFocus = document.activeElement as HTMLElement | null;
    const previousOverflow = document.body.style.overflow;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        props.onClose();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
        'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])',
      );
      if (!focusable?.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.body.style.overflow = "hidden";
    document.addEventListener("keydown", closeOnEscape);
    closeButtonRef.current?.focus();
    return () => {
      document.body.style.overflow = previousOverflow;
      document.removeEventListener("keydown", closeOnEscape);
      previousFocus?.focus();
    };
  }, [props.onClose]);

  return (
    <div
      className="portal-dialog-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="portal-order-detail-title"
      aria-describedby="portal-order-detail-description"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) props.onClose();
      }}
    >
      <div className="portal-dialog-panel" ref={dialogRef}>
        <div className="portal-dialog-header">
          <div>
            <div className="portal-eyebrow">订单批号详情</div>
            <h2 id="portal-order-detail-title" className="portal-dialog-title">
              {detail.data?.order_no ?? "正在读取…"}
            </h2>
            <p id="portal-order-detail-description" className="portal-dialog-description">
              资料下载前会再次校验客户、地址、订单、商品和批号范围。
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            aria-label="关闭订单详情"
            onClick={props.onClose}
            ref={closeButtonRef}
          >
            <X className="size-4" />
          </Button>
        </div>
        <div className="portal-dialog-body">
          {detail.isPending ? <EmptyState text="正在读取订单批号…" /> : null}
          {detail.error ? <ErrorBanner error={detail.error} /> : null}
          {downloadError ? <ErrorBanner error={new Error(downloadError)} /> : null}
          {detail.data?.lines.map((line) => (
            <article
              key={line.id}
              className="portal-batch-card"
              data-testid={`portal-batch-${line.batch_no}`}
            >
              <div className="flex flex-col justify-between gap-3 sm:flex-row">
                <div className="flex gap-3">
                  <div className="portal-batch-icon">
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
    <div className="portal-report-card" data-testid={`portal-report-v${report.version_number}`}>
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
    <div className="portal-empty-state">
      {props.text}
    </div>
  );
}

function ErrorBanner(props: { error: unknown }) {
  return (
    <div role="alert" className="portal-alert portal-alert-error m-4">
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
