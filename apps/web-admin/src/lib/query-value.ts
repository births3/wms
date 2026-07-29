import type { QueryPanelRangeValue, QueryPanelValue } from "@wms/ui";

/** QueryPanelValue 防御性取值工具唯一来源（此前十余个页面各自维护一份，且行为已漂移）。 */

export function queryString(value: QueryPanelValue[string]): string {
  return typeof value === "string" ? value : "";
}

export function queryStringArray(value: QueryPanelValue[string]): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

export function queryRange(value: QueryPanelValue[string]): QueryPanelRangeValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) return { from: "", to: "" };
  return {
    from: typeof value.from === "string" ? value.from : "",
    to: typeof value.to === "string" ? value.to : "",
  };
}

export function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as QueryPanelValue) : {};
}
