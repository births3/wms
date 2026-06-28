/**
 * M2InboundFilterBar — 入库列表筛选区
 *
 * 层级：Layer 3 页面局部组件
 * 关联故事：US-M2-008
 * Wave：Wave 6
 * 业务约束：筛选和重置在页面内直接操作，不使用弹窗。
 *
 * @example
 *   <M2InboundFilterBar statusFilter={["receiving"]} ... />
 */

import * as React from "react";
import { Button, Card, CardContent, Checkbox, Input } from "@wms/ui";
import { Search } from "lucide-react";

import type { InboundDocumentType, InboundDocumentTypeFilter } from "./m2-inbound-document-type";

export type StatusFilterValue = "receiving" | "inspecting" | "putaway" | "completed" | "closed_rejected";
export type StatusFilter = StatusFilterValue[];

const documentTypeOptions: Array<{ value: InboundDocumentType; label: string }> = [
  { value: "purchase_inbound", label: "采购入库" },
  { value: "sales_return", label: "销售退货" },
];

const statusOptions: Array<{ value: StatusFilterValue; label: string }> = [
  { value: "receiving", label: "待收货/收货中" },
  { value: "inspecting", label: "验收中" },
  { value: "putaway", label: "上架中" },
  { value: "completed", label: "已完成" },
  { value: "closed_rejected", label: "已关闭(拒收)" },
];

interface M2InboundFilterBarProps {
  keyword: string;
  ownerKeyword: string;
  documentTypeFilter: InboundDocumentTypeFilter;
  statusFilter: StatusFilter;
  arrivalDate: string;
  createdAtFrom: string;
  createdAtTo: string;
  onKeywordChange: (value: string) => void;
  onOwnerKeywordChange: (value: string) => void;
  onDocumentTypeFilterChange: (value: InboundDocumentTypeFilter) => void;
  onStatusFilterChange: (value: StatusFilter) => void;
  onArrivalDateChange: (value: string) => void;
  onCreatedAtFromChange: (value: string) => void;
  onCreatedAtToChange: (value: string) => void;
  onQuery: () => void;
  onReset: () => void;
}

export function M2InboundFilterBar({
  keyword,
  ownerKeyword,
  documentTypeFilter,
  statusFilter,
  arrivalDate,
  createdAtFrom,
  createdAtTo,
  onKeywordChange,
  onOwnerKeywordChange,
  onDocumentTypeFilterChange,
  onStatusFilterChange,
  onArrivalDateChange,
  onCreatedAtFromChange,
  onCreatedAtToChange,
  onQuery,
  onReset,
}: M2InboundFilterBarProps) {
  return (
    <Card className="rounded-lg shadow-sm">
      <CardContent className="grid gap-3 p-4 md:grid-cols-2 xl:grid-cols-[minmax(14rem,1fr)_10rem_9rem_10rem_9rem_18rem_auto] xl:items-end">
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">关键字</label>
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" aria-hidden />
            <Input
              className="pl-9"
              value={keyword}
              onChange={(event) => onKeywordChange(event.target.value)}
              placeholder="ASN / 商品 / 批号 / 单据类型"
            />
          </div>
        </div>
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">货主</label>
          <Input
            value={ownerKeyword}
            onChange={(event) => onOwnerKeywordChange(event.target.value)}
            placeholder="货主编码 / ID"
          />
        </div>
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">单据类型</label>
          <MultiSelectFilter
            label="单据类型"
            options={documentTypeOptions}
            value={documentTypeFilter}
            onChange={(value) => onDocumentTypeFilterChange(value as InboundDocumentTypeFilter)}
          />
        </div>
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">状态</label>
          <MultiSelectFilter
            label="状态"
            options={statusOptions}
            value={statusFilter}
            onChange={(value) => onStatusFilterChange(value as StatusFilter)}
          />
        </div>
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">预计到货</label>
          <Input type="date" value={arrivalDate} onChange={(event) => onArrivalDateChange(event.target.value)} />
        </div>
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">创建时间（默认近90天）</label>
          <div className="grid grid-cols-2 gap-2">
            <Input type="date" value={createdAtFrom} onChange={(event) => onCreatedAtFromChange(event.target.value)} />
            <Input type="date" value={createdAtTo} onChange={(event) => onCreatedAtToChange(event.target.value)} />
          </div>
        </div>
        <div className="grid grid-cols-2 gap-2 md:grid-cols-1 xl:grid-cols-2">
          <Button type="button" className="w-full whitespace-nowrap" onClick={onQuery}>
            <Search className="size-4" aria-hidden />
            查询
          </Button>
          <Button type="button" variant="outline" className="w-full whitespace-nowrap" onClick={onReset}>
            重置
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function MultiSelectFilter<T extends string>({
  label,
  options,
  value,
  onChange,
}: {
  label: string;
  options: Array<{ value: T; label: string }>;
  value: T[];
  onChange: (value: T[]) => void;
}) {
  const rootRef = React.useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = React.useState(false);
  const selectedLabels = options.filter((option) => value.includes(option.value)).map((option) => option.label);

  React.useEffect(() => {
    if (!open) return;
    function close(event: PointerEvent) {
      const target = event.target instanceof Element ? event.target : null;
      if (target && rootRef.current?.contains(target)) return;
      setOpen(false);
    }
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [open]);

  function toggle(optionValue: T, checked: boolean) {
    const next = new Set(value);
    if (checked) next.add(optionValue);
    else next.delete(optionValue);
    onChange(Array.from(next));
  }

  return (
    <div ref={rootRef} className="relative">
      <Button
        type="button"
        variant="outline"
        className="h-10 w-full justify-between px-3 font-normal"
        aria-label={`筛选${label}`}
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        <span className="truncate">{selectedLabels.length > 0 ? selectedLabels.join("、") : "全部"}</span>
      </Button>
      {open && (
        <div className="absolute z-30 mt-2 grid w-full min-w-48 gap-2 rounded-md border bg-background p-3 text-sm shadow-lg">
          {options.map((option) => {
            const checked = value.includes(option.value);
            const checkboxId = `${label}-${option.value}`;
            return (
              <label key={option.value} htmlFor={checkboxId} className="flex items-center gap-2 text-muted-foreground">
                <Checkbox id={checkboxId} checked={checked} onCheckedChange={(next) => toggle(option.value, next === true)} />
                <span className="truncate">{option.label}</span>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
}
