/** 日期时间格式化唯一来源：非法输入一律返回 "-"，不许抛异常导致整表崩溃。实现收口到 @wms/ui。 */

export { formatDateTime } from "@wms/ui";

export function formatDate(value: string | null | undefined): string {
  if (!value) return "-";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "-" : value.slice(0, 10);
}
