import * as React from "react";
import { cn } from "../../lib/utils";
import { Checkbox } from "../../ui/checkbox";
import { Input } from "../../ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../../ui/select";
import type { DataGridColumnFilterValue, DataGridFilterConfig, DataGridRangeFilter } from "./data-grid-logic";

/**
 * DataGridColumnFilter — DataGrid 字段筛选输入器（按字段类型渲染文本 / 枚举 / 日期 / 数字筛选）
 *
 * 层级：Layer 2 业务复合内部组件
 * 关联故事：M2 收货管理列表字段筛选
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：只负责筛选输入 UI；筛选语义由 DataGrid 逻辑层统一处理
 *
 * @example
 *   <DataGridColumnFilter columnKey="status" label="状态" filter={{ type: "multiSelect", options }} value={value} onChange={setValue} />
 */
export interface DataGridColumnFilterProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "onChange"> {
  columnKey: string;
  label: string;
  filter: DataGridFilterConfig | false | undefined;
  value: DataGridColumnFilterValue | undefined;
  onChange: (value: DataGridColumnFilterValue) => void;
}

export const DataGridColumnFilter = React.forwardRef<HTMLDivElement, DataGridColumnFilterProps>(
  ({ columnKey, label, filter, value, onChange, className, ...rest }, ref) => {
    return (
      <div ref={ref} className={cn("min-w-0", className)} {...rest}>
        {renderColumnFilter({ columnKey, label, filter, value, onChange })}
      </div>
    );
  },
);
DataGridColumnFilter.displayName = "DataGridColumnFilter";

function renderColumnFilter({
  columnKey,
  label,
  filter,
  value,
  onChange,
}: Pick<DataGridColumnFilterProps, "columnKey" | "label" | "filter" | "value" | "onChange">) {
  const filterConfig = filter === false ? { type: "text" as const } : filter ?? { type: "text" as const };

  if (filterConfig.type === "select") {
    return (
      <Select value={typeof value === "string" && value ? value : undefined} onValueChange={onChange}>
        <SelectTrigger className="h-8 text-xs" aria-label={`筛选${label}`}>
          <SelectValue placeholder={`选择${label}`} />
        </SelectTrigger>
        <SelectContent>
          {(filterConfig.options ?? []).map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    );
  }

  if (filterConfig.type === "multiSelect") {
    const values = arrayFilterValue(value);
    return (
      <div className="grid max-h-48 gap-2 overflow-auto pr-1">
        {(filterConfig.options ?? []).map((option) => {
          const checked = values.includes(option.value);
          const checkboxId = `${columnKey}-${option.value}`;
          return (
            <div key={option.value} className="flex items-center gap-2">
              <Checkbox
                id={checkboxId}
                checked={checked}
                onCheckedChange={(nextChecked) => {
                  const next = new Set(values);
                  if (nextChecked === true) next.add(option.value);
                  else next.delete(option.value);
                  onChange(Array.from(next));
                }}
              />
              <label htmlFor={checkboxId} className="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                {option.label}
              </label>
            </div>
          );
        })}
      </div>
    );
  }

  if (filterConfig.type === "dateRange") {
    const range = rangeFilterValue(value);
    return (
      <div className="grid gap-2">
        <Input
          type="date"
          value={range.from ?? ""}
          onChange={(event) => onChange({ ...range, from: event.target.value })}
          aria-label={`筛选${label}开始日期`}
          className="h-8 text-xs"
          autoFocus
        />
        <Input
          type="date"
          value={range.to ?? ""}
          onChange={(event) => onChange({ ...range, to: event.target.value })}
          aria-label={`筛选${label}结束日期`}
          className="h-8 text-xs"
        />
      </div>
    );
  }

  if (filterConfig.type === "numberRange") {
    const range = rangeFilterValue(value);
    return (
      <div className="grid gap-2">
        <Input
          type="number"
          inputMode="decimal"
          value={range.from ?? ""}
          onChange={(event) => onChange({ ...range, from: event.target.value })}
          placeholder="最小值"
          aria-label={`筛选${label}最小值`}
          className="h-8 text-xs"
          autoFocus
        />
        <Input
          type="number"
          inputMode="decimal"
          value={range.to ?? ""}
          onChange={(event) => onChange({ ...range, to: event.target.value })}
          placeholder="最大值"
          aria-label={`筛选${label}最大值`}
          className="h-8 text-xs"
        />
      </div>
    );
  }

  return (
    <Input
      value={typeof value === "string" ? value : ""}
      onChange={(event) => onChange(event.target.value)}
      placeholder={`筛选${label}`}
      aria-label={`筛选${label}`}
      className="h-8 text-xs"
      autoFocus
    />
  );
}

function rangeFilterValue(value: DataGridColumnFilterValue | undefined): DataGridRangeFilter {
  return typeof value === "object" && value !== null && !Array.isArray(value) ? value : {};
}

function arrayFilterValue(value: DataGridColumnFilterValue | undefined): string[] {
  return Array.isArray(value) ? value : [];
}
