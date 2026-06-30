import type {
  DataGridColumnFilterValue,
  DataGridColumnFilters,
  DataGridFilterConfig,
  DataGridRangeFilter,
} from "./data-grid-logic";

export interface DataGridFilterSummaryField {
  key: string;
  label: string;
  filter?: DataGridFilterConfig | false;
}

export interface DataGridFilterSummaryItem {
  key: string;
  label: string;
  value: string;
  text: string;
}

export function buildDataGridFilterSummaryItems(
  filters: DataGridColumnFilters,
  fields: DataGridFilterSummaryField[],
): DataGridFilterSummaryItem[] {
  const items: DataGridFilterSummaryItem[] = [];

  for (const field of fields) {
    if (field.filter === false) continue;

    const value = filters[field.key];
    if (!filterActive(value)) continue;

    const summaryValue = summarizeFilterValue(value, filterConfig(field.filter));
    if (!summaryValue) continue;

    items.push({
      key: field.key,
      label: field.label,
      value: summaryValue,
      text: `${field.label}：${summaryValue}`,
    });
  }

  return items;
}

export function clearDataGridFilterKey(
  filters: DataGridColumnFilters,
  key: string,
): DataGridColumnFilters {
  if (!Object.prototype.hasOwnProperty.call(filters, key)) return filters;
  const next = { ...filters };
  delete next[key];
  return next;
}

function filterConfig(filter: DataGridFilterConfig | false | undefined): DataGridFilterConfig {
  return filter === false ? { type: "text" } : filter ?? { type: "text" };
}

function filterActive(value: DataGridColumnFilterValue | undefined): boolean {
  if (typeof value === "string") return value.trim().length > 0;
  if (Array.isArray(value)) return value.length > 0;
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  return Boolean(value.from?.trim() || value.to?.trim());
}

function summarizeFilterValue(
  value: DataGridColumnFilterValue,
  filter: DataGridFilterConfig,
): string {
  if (filter.type === "select") return summarizeSelectValue(value, filter);
  if (filter.type === "multiSelect") return summarizeMultiSelectValue(value, filter);
  if (filter.type === "dateRange" || filter.type === "numberRange") {
    return summarizeRangeValue(value);
  }
  return typeof value === "string" ? value.trim() : "";
}

function summarizeSelectValue(
  value: DataGridColumnFilterValue,
  filter: DataGridFilterConfig,
): string {
  if (typeof value !== "string") return "";
  const selected = value.trim();
  return optionLabel(selected, filter) ?? selected;
}

function summarizeMultiSelectValue(
  value: DataGridColumnFilterValue,
  filter: DataGridFilterConfig,
): string {
  if (!Array.isArray(value)) return "";
  return value
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => optionLabel(item, filter) ?? item)
    .join("、");
}

function summarizeRangeValue(value: DataGridColumnFilterValue): string {
  const range = rangeFilterValue(value);
  if (!range) return "";

  const from = range.from?.trim();
  const to = range.to?.trim();
  if (from && to) return `${from} 至 ${to}`;
  if (from) return `>= ${from}`;
  if (to) return `<= ${to}`;
  return "";
}

function optionLabel(value: string, filter: DataGridFilterConfig): string | undefined {
  return filter.options?.find((option) => option.value === value)?.label;
}

function rangeFilterValue(value: DataGridColumnFilterValue): DataGridRangeFilter | null {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? value : null;
}
