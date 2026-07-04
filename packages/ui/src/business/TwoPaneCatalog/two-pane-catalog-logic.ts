import type * as React from "react";

export interface TwoPaneCatalogItemBase {
  code: string;
  name: string;
  enabled?: boolean;
}

export interface TwoPaneCatalogGroup<TItem extends TwoPaneCatalogItemBase = TwoPaneCatalogItemBase> {
  code: string;
  name: string;
  items: TItem[];
}

export interface TwoPaneCatalogField<TItem extends TwoPaneCatalogItemBase = TwoPaneCatalogItemBase> {
  key: string;
  label: string;
  defaultVisible?: boolean;
  className?: string;
  layout?: "column" | "detail";
  render?: (item: TItem) => React.ReactNode;
  copyText?: (item: TItem) => string;
}

export interface TwoPaneCatalogPreference {
  selectedGroupCode: string;
  groupQuery: string;
  itemQuery: string;
  hiddenFieldKeys: string[];
}

export interface TwoPaneCatalogGroupSummary {
  code: string;
  name: string;
  enabledCount: number;
  totalCount: number;
}

export function getTwoPaneCatalogSelectedGroup<TItem extends TwoPaneCatalogItemBase>(
  groups: TwoPaneCatalogGroup<TItem>[],
  selectedCode?: string
) {
  return groups.find((group) => group.code === selectedCode) ?? groups[0];
}

export function summarizeTwoPaneCatalogGroup<TItem extends TwoPaneCatalogItemBase>(
  group: TwoPaneCatalogGroup<TItem>
): TwoPaneCatalogGroupSummary {
  return {
    code: group.code,
    name: group.name,
    enabledCount: group.items.filter((item) => item.enabled !== false).length,
    totalCount: group.items.length,
  };
}

export function filterTwoPaneCatalogGroups<TItem extends TwoPaneCatalogItemBase>(
  groups: TwoPaneCatalogGroup<TItem>[],
  query: string
) {
  const normalized = normalizeTwoPaneCatalogQuery(query);
  if (!normalized) return groups;
  return groups.filter((group) => includesText([group.code, group.name], normalized));
}

export function filterTwoPaneCatalogItems<TItem extends TwoPaneCatalogItemBase>(
  items: TItem[],
  query: string,
  getSearchText: (item: TItem) => readonly unknown[] = defaultItemSearchText
) {
  const normalized = normalizeTwoPaneCatalogQuery(query);
  if (!normalized) return items;
  return items.filter((item) => includesText(getSearchText(item), normalized));
}

export function toggleTwoPaneCatalogSelection(selectedKeys: readonly string[], key: string, selected: boolean) {
  const current = selectedKeys.filter((item, index, source) => source.indexOf(item) === index);
  if (selected) return current.includes(key) ? current : [...current, key];
  return current.filter((item) => item !== key);
}

export function normalizeTwoPaneCatalogFields<TItem extends TwoPaneCatalogItemBase>(
  fields: readonly TwoPaneCatalogField<TItem>[],
  hiddenFieldKeys: readonly string[] = []
) {
  const hidden = new Set(hiddenFieldKeys);
  return fields.filter((field) => field.defaultVisible !== false && !hidden.has(field.key)).map((field) => field.key);
}

export function splitTwoPaneCatalogFields<TItem extends TwoPaneCatalogItemBase>(
  fields: readonly TwoPaneCatalogField<TItem>[],
  visibleFieldKeys: readonly string[]
) {
  const visible = new Set(visibleFieldKeys);
  const selected = fields.filter((field) => visible.has(field.key));
  return {
    columns: selected.filter((field) => field.layout !== "detail"),
    details: selected.filter((field) => field.layout === "detail"),
  };
}

export function normalizeTwoPaneCatalogPreference<TItem extends TwoPaneCatalogItemBase>(
  value: unknown,
  groups: readonly TwoPaneCatalogGroup<TItem>[],
  fieldKeys: readonly string[]
): TwoPaneCatalogPreference {
  const record = isRecord(value) ? value : {};
  const groupCodes = new Set(groups.map((group) => group.code));
  const selectedGroupCode =
    typeof record.selectedGroupCode === "string" && groupCodes.has(record.selectedGroupCode)
      ? record.selectedGroupCode
      : groups[0]?.code ?? "";
  const validFieldKeys = new Set(fieldKeys);

  return {
    selectedGroupCode,
    groupQuery: normalizeTwoPaneCatalogQuery(record.groupQuery),
    itemQuery: normalizeTwoPaneCatalogQuery(record.itemQuery),
    hiddenFieldKeys: Array.isArray(record.hiddenFieldKeys)
      ? record.hiddenFieldKeys.filter((key): key is string => typeof key === "string" && validFieldKeys.has(key))
      : [],
  };
}

export function normalizeTwoPaneCatalogQuery(value: unknown) {
  return typeof value === "string" ? value.trim().toLowerCase() : "";
}

export function twoPaneCatalogText(value: unknown) {
  if (typeof value === "string") return value.trim() || "-";
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (value === null || value === undefined) return "-";
  return JSON.stringify(value);
}

export function readTwoPaneCatalogFieldText<TItem extends TwoPaneCatalogItemBase>(item: TItem, key: string) {
  return twoPaneCatalogText((item as unknown as Record<string, unknown>)[key]);
}

export function buildTwoPaneCatalogCopyTitle(value: string | undefined) {
  return value ? `复制 ${value}` : "复制";
}

function defaultItemSearchText<TItem extends TwoPaneCatalogItemBase>(item: TItem) {
  return [item.code, item.name];
}

function includesText(values: readonly unknown[], query: string) {
  return values.some((value) => twoPaneCatalogText(value).toLowerCase().includes(query));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
