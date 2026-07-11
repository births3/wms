import { StatusBadge, cn, type DataGridColumn, type StatusKey } from "@wms/ui";

import type {
  H4NotificationConfig,
  H4NotificationRecord,
} from "@/features/wechat-notify/wechat-notify-queries";

export const configColumns: DataGridColumn<H4NotificationConfig>[] = [
  {
    key: "event_type",
    header: "事件类型",
    width: 220,
    minWidth: 160,
    mono: true,
    sortable: true,
    sortValue: (row) => row.event_type,
    filterValue: (row) => row.event_type,
    copyValue: (row) => row.event_type,
    filter: { type: "text" },
  },
  {
    key: "enabled",
    header: "状态",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => (row.enabled ? 1 : 0),
    filterValue: (row) => (row.enabled ? "true" : "false"),
    copyValue: (row) => (row.enabled ? "启用" : "停用"),
    filter: { type: "multiSelect", options: [{ label: "启用", value: "true" }, { label: "停用", value: "false" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? "启用" : "停用"} size="sm" />,
  },
  {
    key: "channels",
    header: "通知方式",
    width: 160,
    minWidth: 120,
    copyValue: (row) => row.channels.join("、"),
    render: (row) => row.channels.join("、"),
  },
  {
    key: "recipient_rule",
    header: "接收规则",
    width: 260,
    minWidth: 180,
    filterValue: (row) => recipientRuleText(row.recipient_rule),
    copyValue: (row) => recipientRuleText(row.recipient_rule),
    filter: { type: "text" },
    render: (row) => {
      const text = recipientRuleText(row.recipient_rule);
      return <span className="block truncate" title={text}>{text}</span>;
    },
  },
  {
    key: "template",
    header: "通知模板",
    width: 360,
    minWidth: 220,
    filterValue: (row) => row.template,
    copyValue: (row) => row.template,
    filter: { type: "text" },
    render: (row) => <span className="block truncate" title={row.template}>{row.template}</span>,
  },
  {
    key: "version",
    header: "版本",
    width: 100,
    minWidth: 80,
    sortable: true,
    sortValue: (row) => row.version,
    render: (row) => `v${row.version}`,
  },
  {
    key: "updated_at",
    header: "更新时间",
    width: 190,
    minWidth: 160,
    sortable: true,
    sortValue: (row) => row.updated_at,
    filterValue: (row) => row.updated_at,
    copyValue: (row) => formatDateTime(row.updated_at),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.updated_at),
  },
];

export const recordColumns: DataGridColumn<H4NotificationRecord>[] = [
  {
    key: "event_type",
    header: "事件类型",
    width: 220,
    minWidth: 160,
    mono: true,
    sortable: true,
    sortValue: (row) => row.event_type,
    filterValue: (row) => row.event_type,
    copyValue: (row) => row.event_type,
    filter: { type: "text" },
  },
  {
    key: "created_at",
    header: "时间",
    width: 190,
    minWidth: 160,
    sortable: true,
    sortValue: (row) => row.created_at,
    filterValue: (row) => row.created_at,
    copyValue: (row) => formatDateTime(row.created_at),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.created_at),
  },
  {
    key: "recipient",
    header: "接收人",
    width: 180,
    minWidth: 140,
    filterValue: (row) => row.recipient,
    copyValue: (row) => row.recipient,
    filter: { type: "text" },
  },
  {
    key: "content_summary",
    header: "内容摘要",
    width: 360,
    minWidth: 220,
    filterValue: (row) => row.content_summary,
    copyValue: (row) => row.content_summary,
    filter: { type: "text" },
    render: (row) => (
      <span className="block truncate" title={row.content_summary || undefined}>
        {row.content_summary || "-"}
      </span>
    ),
  },
  {
    key: "status",
    header: "状态",
    width: 130,
    minWidth: 110,
    sortable: true,
    sortValue: (row) => row.status,
    filterValue: (row) => row.status,
    copyValue: (row) => statusLabel(row.status),
    filter: { type: "multiSelect", options: [{ label: "成功", value: "success" }, { label: "失败", value: "failed" }, { label: "重试中", value: "retrying" }] },
    render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" />,
  },
  {
    key: "retry_count",
    header: "重试次数",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.retry_count,
    filterValue: (row) => row.retry_count,
    filter: { type: "numberRange" },
  },
  {
    key: "failure_reason",
    header: "失败原因",
    width: 280,
    minWidth: 180,
    filterValue: (row) => failureReasonText(row),
    copyValue: (row) => failureReasonText(row),
    filter: { type: "text" },
    render: (row) => {
      const text = failureReasonText(row);
      return (
        <span
          className={cn(
            "block truncate",
            row.status === "failed" || row.status === "retrying" ? "text-destructive" : "text-muted-foreground",
          )}
          title={text}
        >
          {text}
        </span>
      );
    },
  },
  {
    key: "dedupe_key",
    header: "去重键",
    width: 220,
    minWidth: 160,
    mono: true,
    filterValue: (row) => row.dedupe_key,
    copyValue: (row) => row.dedupe_key,
    filter: { type: "text" },
  },
];

export function recipientRuleText(rule: Record<string, unknown>) {
  const users = stringArray(rule.users);
  const roles = stringArray(rule.roles);
  return [`用户 ${users.join("、") || "-"}`, `角色 ${roles.join("、") || "-"}`].join("；");
}

export function statusLabel(status: string) {
  if (status === "success") return "成功";
  if (status === "failed") return "失败";
  if (status === "retrying") return "重试中";
  return status;
}

export function failureReasonText(row: H4NotificationRecord) {
  const reason = row.failure_reason?.trim();
  if (reason) return reason;
  if (row.status === "failed") return "未返回失败原因";
  if (row.status === "retrying") return "重试中，暂无失败原因";
  return "-";
}

function statusKey(status: string): StatusKey {
  if (status === "success") return "completed";
  if (status === "failed") return "unqualified";
  return "pending";
}

function stringArray(value: unknown) {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string" && item.trim().length > 0) : [];
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}
