import * as React from "react";
import { PlusCircle } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Checkbox } from "../../ui/checkbox";
import type { DataGridFilterOption } from "./data-grid-logic";

/**
 * DataGridFacetedFilter — 分面快捷筛选胶囊组件
 *
 * 层级：Layer 2 业务复合内部组件
 * 关联故事：H7 / M2 管理端表格增强
 * Wave：Wave 6 管理端表格增强
 * 业务约束：在工具栏展示快捷分面下拉菜单，支持多选、已选徽标和快速清空。
 *
 * @example
 *   <DataGridFacetedFilter title="状态" options={statusOptions} selectedValues={selected} onSelectChange={setSelected} />
 */
export interface DataGridFacetedFilterProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "onChange"> {
  title: string;
  options: DataGridFilterOption[];
  selectedValues: string[] | undefined;
  onSelectChange: (values: string[]) => void;
  className?: string;
}

export const DataGridFacetedFilter = React.forwardRef<HTMLDivElement, DataGridFacetedFilterProps>(
  ({ title, options, selectedValues = [], onSelectChange, className, ...rest }, ref) => {
    const [open, setOpen] = React.useState(false);
    const containerRef = React.useRef<HTMLDivElement>(null);

    React.useImperativeHandle(ref, () => containerRef.current as HTMLDivElement);

  React.useEffect(() => {
    if (!open) return;
    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [open]);

  const selectedCount = selectedValues.length;

  const handleToggle = (value: string) => {
    const next = new Set(selectedValues);
    if (next.has(value)) next.delete(value);
    else next.add(value);
    onSelectChange(Array.from(next));
  };

  const handleClear = (e: React.MouseEvent) => {
    e.stopPropagation();
    onSelectChange([]);
  };

  return (
    <div ref={containerRef} {...rest} className={cn("relative inline-block text-xs", className)}>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className={cn(
          "h-8 gap-1.5 border-dashed text-xs font-normal",
          selectedCount > 0 && "border-solid bg-accent text-accent-foreground font-medium",
        )}
        onClick={() => setOpen((prev) => !prev)}
      >
        <PlusCircle className="size-3.5" />
        <span>{title}</span>
        {selectedCount > 0 && (
          <>
            <span className="h-4 w-px bg-border" />
            <span className="rounded bg-primary/10 px-1.5 py-0.2 text-[10px] font-semibold text-primary">
              {selectedCount}
            </span>
          </>
        )}
      </Button>

      {open && (
        <div className="absolute left-0 top-9 z-50 min-w-48 rounded-md border bg-popover p-2 text-popover-foreground shadow-md animate-in fade-in-50">
          <div className="mb-1.5 flex items-center justify-between px-1 text-[11px] font-semibold text-muted-foreground">
            <span>筛选 {title}</span>
            {selectedCount > 0 && (
              <button
                type="button"
                className="text-xs text-muted-foreground hover:text-foreground"
                onClick={handleClear}
              >
                清空
              </button>
            )}
          </div>
          <div className="max-h-56 space-y-1 overflow-y-auto">
            {options.map((option) => {
              const checked = selectedValues.includes(option.value);
              return (
                <div
                  key={option.value}
                  className="flex cursor-pointer items-center gap-2 rounded px-1.5 py-1 text-xs hover:bg-accent"
                  onClick={() => handleToggle(option.value)}
                >
                  <Checkbox checked={checked} />
                  <span className="flex-1 truncate">{option.label}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
});
DataGridFacetedFilter.displayName = "DataGridFacetedFilter";
