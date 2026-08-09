/**
 * @wms/ui — 日期时间工具（统一各应用内重复的本地实现）
 */

/**
 * 格式化为中文日期时间（24 小时制）。
 * 空值或解析失败（new Date(value) 无效）时返回 "-"，保证表格渲染不崩溃
 * （与 apps/web-admin/src/lib/format.ts 的历史契约一致）。
 */
export function formatDateTime(value: string | null | undefined): string {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}

/**
 * 格式化为中文日期时间（yyyy/MM/dd HH:mm），与现有 Intl.DateTimeFormat("zh-CN", …) 用法保持一致。
 * 解析失败（new Date(value) 无效）时原样返回入参，便于表格直接展示原始值。
 */
export function formatZhDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}
