import * as React from "react";
import { Calculator } from "lucide-react";
import { Button } from "../../ui/button";
import { Checkbox } from "../../ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../../ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../ui/select";
import type { DataGridColumn } from "./DataGrid";
import {
  type DataGridSummarySelection,
  type DataGridSummaryType,
} from "./data-grid-summary";

/**
 * DataGridSummaryDialog — DataGrid 汇总统计弹窗
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端 DataGrid 横向能力
 * Wave：Wave 6 管理端表格增强
 * 业务约束：只统计当前筛选结果；字段来自 DataGrid 当前字段定义。
 *
 * @example
 *   <DataGridSummaryDialog open columns={columns} rows={rows} />
 */
export interface DataGridSummaryDialogProps<T>
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  open: boolean;
  columns: DataGridColumn<T>[];
  onOpenChange: (open: boolean) => void;
  onApply: (config: DataGridSummaryConfig) => void;
}

export interface DataGridSummaryConfig {
  groupColumnKeys: string[];
  selections: DataGridSummarySelection[];
}

const summaryTypes: Array<{ value: DataGridSummaryType; label: string }> = [
  { value: "sum", label: "求和" },
  { value: "avg", label: "平均" },
  { value: "max", label: "最大" },
  { value: "min", label: "最小" },
];

export function DataGridSummaryDialog<T>({
  open,
  columns,
  onOpenChange,
  onApply,
}: DataGridSummaryDialogProps<T>) {
  const [groupColumnKeys, setGroupColumnKeys] = React.useState<string[]>([]);
  const [selections, setSelections] = React.useState<DataGridSummarySelection[]>([]);
  const selectableColumns = React.useMemo(
    () => columns.filter((column) => column.hideable !== false),
    [columns],
  );

  React.useEffect(() => {
    if (!open) return;
    setGroupColumnKeys((current) =>
      current.filter((columnKey) => selectableColumns.some((column) => column.key === columnKey)),
    );
    setSelections((current) =>
      current.filter((selection) => selectableColumns.some((column) => column.key === selection.columnKey)),
    );
  }, [open, selectableColumns]);

  function toggleGroupColumn(columnKey: string, checked: boolean) {
    setGroupColumnKeys((current) => {
      if (checked) return current.includes(columnKey) ? current : [...current, columnKey];
      return current.filter((key) => key !== columnKey);
    });
  }

  function toggleColumn(columnKey: string, checked: boolean) {
    setSelections((current) => {
      if (checked) {
        return current.some((selection) => selection.columnKey === columnKey)
          ? current
          : [...current, { columnKey, type: "sum" }];
      }
      return current.filter((selection) => selection.columnKey !== columnKey);
    });
  }

  function updateType(columnKey: string, type: DataGridSummaryType) {
    setSelections((current) =>
      current.map((selection) =>
        selection.columnKey === columnKey ? { ...selection, type } : selection,
      ),
    );
  }

  function applySummary() {
    if (selections.length === 0) return;
    onApply({
      groupColumnKeys: [...groupColumnKeys],
      selections: selections.map((selection) => ({ ...selection })),
    });
    onOpenChange(false);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl" data-datagrid-summary-dialog>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Calculator className="size-5" aria-hidden />
            汇总统计
          </DialogTitle>
          <DialogDescription>勾选分组字段，再为汇总字段选择统计类型，按当前筛选结果计算。</DialogDescription>
        </DialogHeader>
        <div className="max-h-[56vh] space-y-3 overflow-auto rounded-md border p-3">
          <div>
            <div className="mb-2 text-xs font-medium text-muted-foreground">分组字段</div>
            <div className="space-y-1">
              {selectableColumns.map((column) => {
                const checkboxId = `data-grid-summary-group-${column.key}`;
                return (
                  <label key={column.key} htmlFor={checkboxId} className="flex min-w-0 items-center gap-2 rounded-md px-2 py-1 hover:bg-muted/60">
                    <Checkbox
                      id={checkboxId}
                      checked={groupColumnKeys.includes(column.key)}
                      onCheckedChange={(value) => toggleGroupColumn(column.key, value === true)}
                    />
                    <span className="min-w-0 truncate text-sm">{columnLabel(column)}</span>
                  </label>
                );
              })}
            </div>
          </div>
          <div className="border-t pt-3">
            <div className="mb-2 text-xs font-medium text-muted-foreground">汇总字段</div>
            <div className="space-y-1">
              {selectableColumns.map((column) => {
                const selection = selections.find((item) => item.columnKey === column.key);
                const checkboxId = `data-grid-summary-field-${column.key}`;
                return (
                  <div key={column.key} className="grid grid-cols-[1fr_9rem] items-center gap-3 rounded-md px-2 py-1 hover:bg-muted/60">
                    <label htmlFor={checkboxId} className="flex min-w-0 items-center gap-2">
                      <Checkbox
                        id={checkboxId}
                        checked={Boolean(selection)}
                        onCheckedChange={(value) => toggleColumn(column.key, value === true)}
                      />
                      <span className="min-w-0 truncate text-sm">{columnLabel(column)}</span>
                    </label>
                    <Select
                      value={selection?.type ?? "sum"}
                      disabled={!selection}
                      onValueChange={(value) => updateType(column.key, value as DataGridSummaryType)}
                    >
                      <SelectTrigger aria-label={`${columnLabel(column)}统计类型`} className="h-8">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {summaryTypes.map((type) => (
                          <SelectItem key={type.value} value={type.value}>
                            {type.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => {
              setGroupColumnKeys([]);
              setSelections([]);
            }}
            disabled={groupColumnKeys.length === 0 && selections.length === 0}
          >
            清空
          </Button>
          <Button type="button" onClick={applySummary} disabled={selections.length === 0}>
            应用
          </Button>
          <Button type="button" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function columnLabel<T>(column: DataGridColumn<T>): string {
  return typeof column.header === "string" ? column.header : column.key;
}
