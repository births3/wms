import { useMutation, useQuery } from "@tanstack/react-query";
import {
  Archive,
  CheckCircle2,
  Clock3,
  Download,
  FileWarning,
  RefreshCw,
} from "lucide-react";
import { Button, StatusBadge } from "@wms/ui";

import { authorizeExportDownload, listExports } from "./api";
import type { ExportJob, LoginResponse } from "./types";

export function ExportsPage(props: { session: LoginResponse }) {
  const token = props.session.access_token;
  const exportsQuery = useQuery({
    queryKey: ["portal-exports"],
    queryFn: () => listExports(token),
    refetchInterval: (query) =>
      query.state.data?.some((job) => ["queued", "processing"].includes(job.status))
        ? 1_000
        : false,
  });
  const download = useMutation({
    mutationFn: (jobId: string) => authorizeExportDownload(token, jobId),
    onSuccess: (authorized) => window.location.assign(authorized.url),
  });

  return (
    <div className="space-y-5" data-testid="portal-exports-page">
      <section className="flex flex-col justify-between gap-3 sm:flex-row sm:items-end">
        <div>
          <div className="text-sm font-medium text-emerald-700">异步批量处理</div>
          <h1 className="mt-1 text-2xl font-semibold">导出中心</h1>
          <p className="mt-1 text-sm text-slate-500">
            ZIP 保留 7 天，每次点击下载都会重新生成 15 分钟有效的授权地址。
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          onClick={() => exportsQuery.refetch()}
          disabled={exportsQuery.isFetching}
        >
          <RefreshCw className={`mr-2 size-4 ${exportsQuery.isFetching ? "animate-spin" : ""}`} />
          刷新
        </Button>
      </section>

      {download.error ? (
        <div role="alert" className="rounded-xl bg-red-50 px-4 py-3 text-sm text-red-700">
          {download.error.message}
        </div>
      ) : null}

      <section className="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm">
        {exportsQuery.isPending ? (
          <div className="grid min-h-48 place-items-center text-sm text-slate-500">
            正在读取导出任务…
          </div>
        ) : exportsQuery.data?.length ? (
          <div className="overflow-x-auto">
            <table className="portal-table">
              <thead>
                <tr>
                  <th>创建时间</th>
                  <th>订单数</th>
                  <th>药检单</th>
                  <th>缺失项</th>
                  <th>状态</th>
                  <th>保留至</th>
                  <th className="w-28">操作</th>
                </tr>
              </thead>
              <tbody>
                {exportsQuery.data.map((job) => (
                  <ExportRow
                    key={job.id}
                    job={job}
                    downloading={download.isPending && download.variables === job.id}
                    onDownload={() => download.mutate(job.id)}
                  />
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="grid min-h-56 place-items-center px-6 text-center">
            <div>
              <Archive className="mx-auto size-10 text-slate-300" />
              <div className="mt-3 text-sm font-medium text-slate-700">暂无导出任务</div>
              <p className="mt-1 text-xs text-slate-500">
                请到“订单与药检单”勾选订单后创建批量下载。
              </p>
            </div>
          </div>
        )}
      </section>
      <div className="rounded-xl border border-slate-200 bg-white p-4 text-xs leading-6 text-slate-500">
        同一报告版本被多个订单引用时，ZIP 仅放一份 PDF；清单会保留每个订单、商品和批号，
        资料暂缺项不会被静默忽略。单次最多 200 份且不超过 2GB。
      </div>
    </div>
  );
}

function ExportRow(props: {
  job: ExportJob;
  downloading: boolean;
  onDownload: () => void;
}) {
  const statusMeta = {
    queued: { status: "pending" as const, label: "排队中", icon: Clock3 },
    processing: { status: "in_progress" as const, label: "处理中", icon: RefreshCw },
    completed: { status: "completed" as const, label: "已完成", icon: CheckCircle2 },
    failed: { status: "unqualified" as const, label: "失败", icon: FileWarning },
  }[props.job.status];
  const StatusIcon = statusMeta.icon;
  return (
    <tr data-testid={`portal-export-${props.job.status}`}>
      <td>{formatTime(props.job.created_at)}</td>
      <td>{props.job.requested_order_count}</td>
      <td>{props.job.report_file_count} 份</td>
      <td>
        <span className={props.job.missing_count ? "font-medium text-amber-700" : ""}>
          {props.job.missing_count} 项
        </span>
      </td>
      <td>
        <StatusBadge status={statusMeta.status} size="sm" label={statusMeta.label} />
        {props.job.last_error ? (
          <div className="mt-1 max-w-xs text-xs text-red-600">{props.job.last_error}</div>
        ) : null}
      </td>
      <td>{props.job.expires_at ? formatTime(props.job.expires_at) : "—"}</td>
      <td>
        <Button
          type="button"
          size="sm"
          disabled={props.job.status !== "completed" || props.downloading}
          onClick={props.onDownload}
        >
          {props.job.status === "completed" ? (
            <Download className="mr-2 size-4" />
          ) : (
            <StatusIcon className="mr-2 size-4" />
          )}
          {props.downloading ? "授权中" : "下载 ZIP"}
        </Button>
      </td>
    </tr>
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
