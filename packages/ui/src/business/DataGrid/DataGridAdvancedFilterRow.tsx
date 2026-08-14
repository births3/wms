import * as React from "react";
import { Trash2 } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Input } from "../../ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../../ui/select";
import { Checkbox } from "../../ui/checkbox";
import {
  getOperatorsForFilterType,
  getOperatorLabel,
  operatorRequiresNoValue,
  getDefaultOperatorForFilterType,
  type DataGridFilterItem,
  type DataGridFilterOperator,
} from "./data-grid-operators";
import type { DataGridColumn } from "./data-grid-types";
import type { DataGridRangeFilter } from "./data-grid-logic";

/**
 * DataGridAdvancedFilterRow — 高级筛选器单行条件构建器
 *
 * 层级：Layer 2 业务复合内部组件
 * 关联故事：H7 / M2 管理端表格增强
 * Wave：Wave 6 管理端表格增强
 * 业务约束：按三段式渲染字段、操作符与动态值输入器
 *
 * @example
 *   <DataGridAdvancedFilterRow item={item} columns={columns} onChange={setItem} onRemove={removeItem} />
 */
export interface DataGridAdvancedFilterRowProps<T> {
  item: DataGridFilterItem;
  columns: DataGridColumn<T>[];
  className?: string;
  onChange: (item: DataGridFilterItem) => void;
  onRemove: () => void;
}

export function DataGridAdvancedFilterRow<T>({
  item,
  columns,
  className,
  onChange,
  onRemove,
}: DataGridAdvancedFilterRowProps<T>) {
  const filterableColumns = React.useMemo(
    () => columns.filter((col) => col.hideable !== false && col.filter !== false),
    [columns],
  );

  const currentColumn = React.useMemo(
    () => filterableColumns.find((col) => col.key === item.columnKey) ?? filterableColumns[0],
    [filterableColumns, item.columnKey],
  );

  const filterType = currentColumn?.filter === false ? "text" : currentColumn?.filter?.type ?? "text";
  const availableOperators = React.useMemo(() => getOperatorsForFilterType(filterType), [filterType]);

  const handleColumnChange = (nextKey: string) => {
    const nextCol = filterableColumns.find((col) => col.key === nextKey);
    const nextType = nextCol?.filter === false ? "text" : nextCol?.filter?.type ?? "text";
    const defaultOp = getDefaultOperatorForFilterType(nextType);
    onChange({
      ...item,
      columnKey: nextKey,
      operator: defaultOp,
      value: undefined,
    });
  };

  const handleOperatorChange = (nextOp: string) => {
    const operator = nextOp as DataGridFilterOperator;
    onChange({
      ...item,
      operator,
      value: operatorRequiresNoValue(operator) ? undefined : item.value,
    });
  };

  const noValueNeeded = operatorRequiresNoValue(item.operator);

  return (
    <div className={cn("flex flex-wrap items-center gap-2 rounded-md border bg-background/60 p-2 text-xs", className)}>
      {/* 1. 字段选择器 */}
      <div className="w-36 shrink-0">
        <Select value={item.columnKey} onValueChange={handleColumnChange}>
          <SelectTrigger className="h-8 text-xs font-medium" aria-label="选择筛选字段">
            <SelectValue placeholder="选择字段" />
          </SelectTrigger>
          <SelectContent>
            {filterableColumns.map((col) => (
              <SelectItem key={col.key} value={col.key}>
                {typeof col.header === "string" ? col.header : col.key}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* 2. 操作符选择器 */}
      <div className="w-32 shrink-0">
        <Select value={item.operator} onValueChange={handleOperatorChange}>
          <SelectTrigger className="h-8 text-xs" aria-label="选择操作符">
            <SelectValue placeholder="选择条件" />
          </SelectTrigger>
          <SelectContent>
            {availableOperators.map((op) => (
              <SelectItem key={op} value={op}>
                {getOperatorLabel(op)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* 3. 动态值输入器 */}
      <div className="min-w-44 flex-1">
        {noValueNeeded ? (
          <div className="flex h-8 items-center px-2 text-xs italic text-muted-foreground">
            (此条件无需输入数值)
          </div>
        ) : (
          renderFilterValueInput({
            column: currentColumn,
            operator: item.operator,
            value: item.value,
            onChange: (value) => onChange({ ...item, value }),
          })
        )}
      </div>

      {/* 4. 删除按钮 */}
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="size-8 shrink-0 text-muted-foreground hover:text-destructive"
        aria-label="删除筛选条件"
        onClick={onRemove}
      >
        <Trash2 className="size-4" />
      </Button>
    </div>
  );
}

interface FilterValueInputProps<T> {
  column?: DataGridColumn<T>;
  operator: DataGridFilterOperator;
  value: unknown;
  onChange: (val: unknown) => void;
}

function renderFilterValueInput<T>({ column, operator, value, onChange }: FilterValueInputProps<T>) {
  const filter = column?.filter === false ? { type: "text" as const } : column?.filter ?? { type: "text" as const };
  const label = typeof column?.header === "string" ? column.header : column?.key ?? "值";

  if (filter.type === "numberRange") {
    if (operator === "between") {
      const range = (typeof value === "object" && value !== null ? value : {}) as DataGridRangeFilter;
      return (
        <div className="flex items-center gap-1.5">
          <Input
            type="number"
            inputMode="decimal"
            value={range.from ?? ""}
            onChange={(e) => onChange({ ...range, from: e.target.value })}
            placeholder="最小值"
            aria-label={`${label}最小值`}
            className="h-8 text-xs"
          />
          <span className="shrink-0 text-muted-foreground">至</span>
          <Input
            type="number"
            inputMode="decimal"
            value={range.to ?? ""}
            onChange={(e) => onChange({ ...range, to: e.target.value })}
            placeholder="最大值"
            aria-label={`${label}最大值`}
            className="h-8 text-xs"
          />
        </div>
      );
    }
    return (
      <Input
        type="number"
        inputMode="decimal"
        value={typeof value === "number" || typeof value === "string" ? value : ""}
        onChange={(e) => onChange(e.target.value)}
        placeholder={`输入${label}`}
        aria-label={`输入${label}`}
        className="h-8 text-xs"
      />
    );
  }

  if (filter.type === "dateRange") {
    if (operator === "between") {
      const range = (typeof value === "object" && value !== null ? value : {}) as DataGridRangeFilter;
      return (
        <div className="flex items-center gap-1.5">
          <Input
            type="date"
            value={range.from ?? ""}
            onChange={(e) => onChange({ ...range, from: e.target.value })}
            aria-label={`${label}起始日期`}
            className="h-8 text-xs"
          />
          <span className="shrink-0 text-muted-foreground">至</span>
          <Input
            type="date"
            value={range.to ?? ""}
            onChange={(e) => onChange({ ...range, to: e.target.value })}
            aria-label={`${label}截止日期`}
            className="h-8 text-xs"
          />
        </div>
      );
    }
    return (
      <Input
        type="date"
        value={typeof value === "string" ? value : ""}
        onChange={(e) => onChange(e.target.value)}
        aria-label={`输入${label}`}
        className="h-8 text-xs"
      />
    );
  }

  if (filter.type === "select") {
    if (operator === "isAnyOf" || operator === "isNoneOf") {
      const selected = Array.isArray(value) ? value : [];
      return (
        <div className="flex max-h-24 flex-wrap gap-2 overflow-auto rounded border bg-muted/20 p-1.5">
          {(filter.options ?? []).map((opt) => {
            const checked = selected.includes(opt.value);
            const checkId = `adv-${column?.key}-${opt.value}`;
            return (
              <label key={opt.value} htmlFor={checkId} className="flex cursor-pointer items-center gap-1 text-xs">
                <Checkbox
                  id={checkId}
                  checked={checked}
                  onCheckedChange={(nextChecked) => {
                    const next = new Set(selected);
                    if (nextChecked) next.add(opt.value);
                    else next.delete(opt.value);
                    onChange(Array.from(next));
                  }}
                />
                <span className="select-none">{opt.label}</span>
              </label>
            );
          })}
        </div>
      );
    }
    return (
      <Select value={typeof value === "string" ? value : undefined} onValueChange={onChange}>
        <SelectTrigger className="h-8 text-xs" aria-label={`选择${label}`}>
          <SelectValue placeholder={`选择${label}`} />
        </SelectTrigger>
        <SelectContent>
          {(filter.options ?? []).map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    );
  }

  if (filter.type === "multiSelect") {
    const selected = Array.isArray(value) ? value : [];
    return (
      <div className="flex max-h-24 flex-wrap gap-2 overflow-auto rounded border bg-muted/20 p-1.5">
        {(filter.options ?? []).map((opt) => {
          const checked = selected.includes(opt.value);
          const checkId = `adv-multi-${column?.key}-${opt.value}`;
          return (
            <label key={opt.value} htmlFor={checkId} className="flex cursor-pointer items-center gap-1 text-xs">
              <Checkbox
                id={checkId}
                checked={checked}
                onCheckedChange={(nextChecked) => {
                  const next = new Set(selected);
                  if (nextChecked) next.add(opt.value);
                  else next.delete(opt.value);
                  onChange(Array.from(next));
                }}
              />
              <span className="select-none">{opt.label}</span>
            </label>
          );
        })}
      </div>
    );
  }

  return (
    <Input
      value={typeof value === "string" ? value : ""}
      onChange={(e) => onChange(e.target.value)}
      placeholder={`输入${label}`}
      aria-label={`输入${label}`}
      className="h-8 text-xs"
    />
  );
}
