import * as React from "react";
import { ChevronDown, ChevronUp, Search } from "lucide-react";

import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Card, CardContent } from "../../ui/card";
import { Checkbox } from "../../ui/checkbox";
import { Input } from "../../ui/input";

/**
 * QueryPanel — 管理页通用查询条件区
 *
 * 层级：Layer 2 业务复合
 * 关联故事：所有 PC 管理页列表查询
 * Wave：Wave 6
 * 业务约束：只负责查询条件布局和动作入口；筛选状态、查询语义和数据来源由页面或 DataGrid 持有。
 *
 * @example
 *   <QueryPanel fields={fields} value={draftQuery} onValueChange={setDraftQuery} onQuery={applyQuery} />
 */
export type QueryPanelFieldType = "text" | "select" | "multiSelect" | "dateRange" | "numberRange";

export interface QueryPanelOption {
  label: string;
  value: string;
  disabled?: boolean;
}

export interface QueryPanelRangeValue {
  from?: string;
  to?: string;
}

export type QueryPanelFieldValue = string | string[] | QueryPanelRangeValue | undefined;
export type QueryPanelValue = Record<string, QueryPanelFieldValue>;

export interface QueryPanelField {
  key: string;
  label: string;
  type: QueryPanelFieldType;
  placeholder?: string;
  ariaLabel?: string;
  options?: QueryPanelOption[];
}

export interface QueryPanelSummaryItem {
  key: string;
  label: string;
  value: string;
  text: string;
}

export interface QueryPanelProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "onReset"> {
  keyword?: string;
  keywordPlaceholder?: string;
  keywordAriaLabel?: string;
  onKeywordChange?: (value: string) => void;
  fields?: QueryPanelField[];
  fieldOptions?: Record<string, QueryPanelOption[]>;
  defaultVisibleFieldKeys?: string[];
  value?: QueryPanelValue;
  onValueChange?: (value: QueryPanelValue) => void;
  onQuery?: () => void;
  onReset?: () => void;
  queryLabel?: string;
  resetLabel?: string;
  actions?: React.ReactNode;
  contentClassName?: string;
}

export const QueryPanel = React.forwardRef<HTMLDivElement, QueryPanelProps>(
  (
    {
      keyword,
      keywordPlaceholder = "搜索关键字",
      keywordAriaLabel = "搜索关键字",
      onKeywordChange,
      fields = [],
      fieldOptions,
      defaultVisibleFieldKeys,
      value = {},
      onValueChange,
      onQuery,
      onReset,
      queryLabel = "查询",
      resetLabel = "重置",
      actions,
      children,
      className,
      contentClassName,
      ...rest
    },
    ref,
  ) => {
    const [expanded, setExpanded] = React.useState(false);
    const hasKeyword = onKeywordChange || keyword !== undefined;
    const hasFields = fields.length > 0;
    const hasActions = Boolean(onQuery || onReset || actions);
    const defaultVisibleKeys = React.useMemo(
      () => new Set(defaultVisibleFieldKeys ?? fields.map((field) => field.key)),
      [defaultVisibleFieldKeys, fields],
    );
    const visibleFields = defaultVisibleFieldKeys && !expanded
      ? fields.filter((field) => defaultVisibleKeys.has(field.key))
      : fields;
    const hasMoreFields = defaultVisibleFieldKeys
      ? fields.some((field) => !defaultVisibleKeys.has(field.key))
      : false;
    const gridColumns =
      hasFields || hasKeyword ? "md:grid-cols-[minmax(0,1fr)_auto]" : "md:grid-cols-[1fr_auto]";

    function updateField(key: string, nextValue: QueryPanelFieldValue) {
      onValueChange?.({ ...value, [key]: nextValue });
    }

    return (
      <Card ref={ref} className={cn("rounded-lg shadow-sm", className)} {...rest}>
        <CardContent
          className={cn(
            "grid gap-3 p-4 md:items-end",
            gridColumns,
            contentClassName,
          )}
        >
          {(hasKeyword || hasFields || children) && (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              {hasKeyword && (
                <label className="relative block">
                  <span className="mb-1 block text-xs text-muted-foreground">{keywordAriaLabel}</span>
                  <Search className="pointer-events-none absolute left-2.5 top-8 size-4 text-muted-foreground" aria-hidden />
                  <Input
                    value={keyword ?? ""}
                    onChange={(event) => onKeywordChange?.(event.target.value)}
                    placeholder={keywordPlaceholder}
                    aria-label={keywordAriaLabel}
                    className="h-9 pl-9 text-sm"
                  />
                </label>
              )}
              {visibleFields.map((field) => (
                <QueryPanelFieldControl
                  key={field.key}
                  field={fieldOptions?.[field.key] ? { ...field, options: fieldOptions[field.key] } : field}
                  value={value[field.key]}
                  onChange={(nextValue) => updateField(field.key, nextValue)}
                />
              ))}
              {children}
            </div>
          )}
          {hasActions && (
            <div className="flex flex-wrap gap-2 md:justify-end">
              {onQuery && (
                <Button type="button" onClick={onQuery}>
                  <Search className="size-4" aria-hidden />
                  {queryLabel}
                </Button>
              )}
              {onReset && (
                <Button type="button" variant="outline" onClick={onReset}>
                  {resetLabel}
                </Button>
              )}
              {hasMoreFields && (
                <Button
                  type="button"
                  variant="ghost"
                  aria-expanded={expanded}
                  onClick={() => setExpanded((current) => !current)}
                >
                  {expanded ? <ChevronUp className="size-4" aria-hidden /> : <ChevronDown className="size-4" aria-hidden />}
                  {expanded ? "收起" : "展开"}
                </Button>
              )}
              {actions}
            </div>
          )}
        </CardContent>
      </Card>
    );
  },
);
QueryPanel.displayName = "QueryPanel";

export function buildQueryPanelSummaryItems(
  fields: QueryPanelField[],
  value: QueryPanelValue,
): QueryPanelSummaryItem[] {
  return fields.flatMap((field) => {
    const summaryValue = summarizeQueryPanelValue(field, value[field.key]);
    return summaryValue ? [{ key: field.key, label: field.label, value: summaryValue, text: `${field.label}：${summaryValue}` }] : [];
  });
}

function QueryPanelFieldControl({
  field,
  value,
  onChange,
}: {
  field: QueryPanelField;
  value: QueryPanelFieldValue;
  onChange: (value: QueryPanelFieldValue) => void;
}) {
  const id = React.useId();
  const label = field.ariaLabel ?? field.label;
  const rangeValue = asRangeValue(value);

  if (field.type === "dateRange" || field.type === "numberRange") {
    const inputType = field.type === "dateRange" ? "date" : "number";
    return (
      <div>
        <label className="mb-1 block text-xs text-muted-foreground">{field.label}</label>
        <div className="grid grid-cols-2 gap-2">
          <Input
            type={inputType}
            value={rangeValue.from ?? ""}
            aria-label={`${label}开始`}
            onChange={(event) => onChange({ ...rangeValue, from: event.target.value })}
            className="h-9 text-sm"
          />
          <Input
            type={inputType}
            value={rangeValue.to ?? ""}
            aria-label={`${label}结束`}
            onChange={(event) => onChange({ ...rangeValue, to: event.target.value })}
            className="h-9 text-sm"
          />
        </div>
      </div>
    );
  }

  if (field.type === "multiSelect") {
    return (
      <QueryPanelMultiSelect
        field={field}
        value={Array.isArray(value) ? value : []}
        onChange={onChange}
      />
    );
  }

  if (field.type === "select") {
    return (
      <div>
        <label htmlFor={id} className="mb-1 block text-xs text-muted-foreground">
          {field.label}
        </label>
        <select
          id={id}
          value={typeof value === "string" ? value : ""}
          aria-label={label}
          onChange={(event) => onChange(event.target.value)}
          className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <option value="">全部</option>
          {(field.options ?? []).map((option) => (
            <option key={option.value} value={option.value} disabled={option.disabled}>
              {option.label}
            </option>
          ))}
        </select>
      </div>
    );
  }

  return (
    <label className="relative block">
      <span className="mb-1 block text-xs text-muted-foreground">{field.label}</span>
      <Search className="pointer-events-none absolute left-2.5 top-8 size-4 text-muted-foreground" aria-hidden />
      <Input
        value={typeof value === "string" ? value : ""}
        onChange={(event) => onChange(event.target.value)}
        placeholder={field.placeholder}
        aria-label={label}
        className="h-9 pl-9 text-sm"
      />
    </label>
  );
}

function QueryPanelMultiSelect({
  field,
  value,
  onChange,
}: {
  field: QueryPanelField;
  value: string[];
  onChange: (value: string[]) => void;
}) {
  const selectedLabels = (field.options ?? [])
    .filter((option) => value.includes(option.value))
    .map((option) => option.label);

  function toggle(optionValue: string, checked: boolean) {
    const next = new Set(value);
    if (checked) next.add(optionValue);
    else next.delete(optionValue);
    onChange(Array.from(next));
  }

  return (
    <div>
      <label className="mb-1 block text-xs text-muted-foreground">{field.label}</label>
      <details className="group relative">
        <summary
          className={cn(
            "flex h-9 w-full cursor-pointer list-none items-center rounded-md border border-input",
            "bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring",
            "[&::-webkit-details-marker]:hidden",
          )}
          aria-label={field.ariaLabel ?? field.label}
        >
          <span className="truncate">{selectedLabels.length > 0 ? selectedLabels.join("、") : "全部"}</span>
        </summary>
        <div className="absolute z-40 mt-2 grid w-full min-w-48 gap-2 rounded-md border bg-background p-3 text-sm shadow-lg">
          {(field.options ?? []).map((option) => {
            const checkboxId = `${field.key}-${option.value}`;
            return (
              <label key={option.value} htmlFor={checkboxId} className="flex items-center gap-2 text-muted-foreground">
                <Checkbox
                  id={checkboxId}
                  checked={value.includes(option.value)}
                  onCheckedChange={(checked) => toggle(option.value, checked === true)}
                />
                <span className="truncate">{option.label}</span>
              </label>
            );
          })}
        </div>
      </details>
    </div>
  );
}

function summarizeQueryPanelValue(field: QueryPanelField, value: QueryPanelFieldValue): string {
  if (typeof value === "string") return field.type === "select" ? optionLabel(field, value.trim()) ?? value.trim() : value.trim();
  if (Array.isArray(value)) {
    return value
      .map((item) => optionLabel(field, item) ?? item)
      .filter(Boolean)
      .join("、");
  }
  const range = asRangeValue(value);
  const from = range.from?.trim();
  const to = range.to?.trim();
  if (from && to) return `${from} 至 ${to}`;
  if (from) return `>= ${from}`;
  if (to) return `<= ${to}`;
  return "";
}

function optionLabel(field: QueryPanelField, value: string) {
  if (!value) return "";
  return field.options?.find((option) => option.value === value)?.label;
}

function asRangeValue(value: QueryPanelFieldValue): QueryPanelRangeValue {
  return value && typeof value === "object" && !Array.isArray(value) ? value : {};
}
