import * as React from "react";
import { createPortal } from "react-dom";
import { ArrowDown, ArrowUp, GripVertical } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { Checkbox } from "../../ui/checkbox";
import type { DataGridColumn } from "./DataGrid";
import type { DataGridFloatingPanelPosition } from "./data-grid-logic";

/**
 * DataGridFieldSettingsPanel — DataGrid 字段设置面板
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2 收货管理列表
 * Wave：Wave 6 M2 管理端表格增强
 * 业务约束：字段显示、复制开关、冻结列和字段顺序设置集中维护
 *
 * @example
 *   <DataGridFieldSettingsPanel open columns={columns} />
 */
export interface DataGridFieldSettingsPanelProps<T> {
  open: boolean;
  panelId: string;
  position: DataGridFloatingPanelPosition | null;
  columns: DataGridColumn<T>[];
  visibleKeys: Set<string>;
  copyableKeys: Set<string>;
  frozenKeys: Set<string>;
  visibleHideableCount: number;
  draggingColumnKey: string | null;
  className?: string;
  onDraggingColumnKeyChange: (key: string | null) => void;
  onColumnVisibleChange: (key: string, visible: boolean) => void;
  onColumnCopyableChange: (key: string, copyable: boolean) => void;
  onColumnFrozenChange: (key: string, frozen: boolean) => void;
  onMoveColumn: (key: string, beforeKey: string) => void;
  onMoveColumnByStep: (key: string, step: -1 | 1) => void;
}

export function DataGridFieldSettingsPanel<T>({
  open,
  panelId,
  position,
  columns,
  visibleKeys,
  copyableKeys,
  frozenKeys,
  visibleHideableCount,
  draggingColumnKey,
  className,
  onDraggingColumnKeyChange,
  onColumnVisibleChange,
  onColumnCopyableChange,
  onColumnFrozenChange,
  onMoveColumn,
  onMoveColumnByStep,
}: DataGridFieldSettingsPanelProps<T>) {
  if (!open || !position || typeof document === "undefined") return null;

  return createPortal(
    <div
      id={panelId}
      className={cn(
        "fixed z-50 w-80 overflow-auto rounded-md border border-primary/30 bg-background p-2 text-left text-sm shadow-lg",
        className,
      )}
      // 动态：字段设置浮层跟随触发按钮位置和视口高度。
      style={{ top: position.top, left: position.left, maxHeight: position.maxHeight }}
      data-datagrid-popover
    >
      {columns.map((item, index) => {
        const checked = visibleKeys.has(item.key);
        const copyable = copyableKeys.has(item.key);
        const frozen = frozenKeys.has(item.key);
        const disabled = checked && visibleHideableCount <= 1;
        const checkboxId = `${panelId}-${item.key}`;
        const copyCheckboxId = `${panelId}-${item.key}-copy`;
        const freezeCheckboxId = `${panelId}-${item.key}-freeze`;
        const canMoveUp = index > 0;
        const canMoveDown = index < columns.length - 1;

        return (
          <div
            key={item.key}
            draggable
            tabIndex={0}
            onDragStart={() => onDraggingColumnKeyChange(item.key)}
            onDragOver={(event) => event.preventDefault()}
            onDrop={() => {
              if (draggingColumnKey) onMoveColumn(draggingColumnKey, item.key);
              onDraggingColumnKeyChange(null);
            }}
            onDragEnd={() => onDraggingColumnKeyChange(null)}
            onKeyDown={(event) => {
              if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
              event.preventDefault();
              onMoveColumnByStep(item.key, event.key === "ArrowUp" ? -1 : 1);
            }}
            className={cn(
              "flex items-center gap-2 rounded-sm px-2 py-1.5 outline-none focus-visible:ring-2 focus-visible:ring-ring",
              draggingColumnKey === item.key ? "bg-muted" : "hover:bg-muted/60",
            )}
          >
            <GripVertical className="size-4 shrink-0 cursor-grab text-muted-foreground" aria-hidden />
            <Checkbox
              id={checkboxId}
              checked={checked}
              disabled={disabled}
              onCheckedChange={(value) => onColumnVisibleChange(item.key, value === true)}
            />
            <label htmlFor={checkboxId} className="min-w-0 flex-1 truncate text-muted-foreground">
              {columnLabel(item)}
            </label>
            <div className="flex shrink-0 items-center gap-1">
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-6"
                aria-label={`上移${columnLabel(item)}`}
                disabled={!canMoveUp}
                onClick={() => onMoveColumnByStep(item.key, -1)}
              >
                <ArrowUp className="size-3.5" aria-hidden />
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="size-6"
                aria-label={`下移${columnLabel(item)}`}
                disabled={!canMoveDown}
                onClick={() => onMoveColumnByStep(item.key, 1)}
              >
                <ArrowDown className="size-3.5" aria-hidden />
              </Button>
            </div>
            <label htmlFor={copyCheckboxId} className="flex shrink-0 items-center gap-1 text-xs text-muted-foreground">
              <Checkbox
                id={copyCheckboxId}
                checked={copyable}
                disabled={item.copyable === false}
                onCheckedChange={(value) => onColumnCopyableChange(item.key, value === true)}
              />
              复制
            </label>
            <label htmlFor={freezeCheckboxId} className="flex shrink-0 items-center gap-1 text-xs text-muted-foreground">
              <Checkbox
                id={freezeCheckboxId}
                checked={frozen}
                onCheckedChange={(value) => onColumnFrozenChange(item.key, value === true)}
              />
              冻结
            </label>
          </div>
        );
      })}
    </div>,
    document.body,
  );
}

function columnLabel<T>(column: DataGridColumn<T>): string {
  return typeof column.header === "string" ? column.header : column.key;
}
