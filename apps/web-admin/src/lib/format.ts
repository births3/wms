/** 日期时间格式化唯一来源：非法输入一律返回 "-"，不许抛异常导致整表崩溃。 */

export function formatDateTime(value: string | null | undefined): string {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}
