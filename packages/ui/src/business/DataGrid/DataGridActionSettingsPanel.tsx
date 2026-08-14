import * as React from "react";
import { createPortal } from "react-dom";
import { cn } from "../../lib/utils";
import { Checkbox } from "../../ui/checkbox";
import type { DataGridFloatingPanelPosition } from "./data-grid-logic";

/**
 * DataGridActionSettingsPanel — DataGrid 按钮功能显示设置
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端 DataGrid 横向能力
 * Wave：Wave 6 管理端表格增强
 * 业务约束：只控制 DataGrid 工具栏按钮显隐；设置入口本身不允许隐藏。
 *
 * @example
 *   <DataGridActionSettingsPanel open actions={actions} />
 */
export interface DataGridActionSettingItem {
  key: string;
  label: string;
  description?: string;
  visible: boolean;
}

export interface DataGridActionSettingsPanelProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  open: boolean;
  panelId: string;
  position: DataGridFloatingPanelPosition | null;
  actions: DataGridActionSettingItem[];
  onActionVisibleChange: (key: string, visible: boolean) => void;
}

export const DataGridActionSettingsPanel = React.forwardRef<HTMLDivElement, DataGridActionSettingsPanelProps>(
  ({ open, panelId, position, actions, onActionVisibleChange, className, ...rest }, ref) => {
    if (!open || !position || typeof document === "undefined") return null;
    const allVisible = actions.length > 0 && actions.every((action) => action.visible);
    const someVisible = actions.some((action) => action.visible);

    function setAllActionsVisible(visible: boolean) {
      for (const action of actions) {
        if (action.visible !== visible) onActionVisibleChange(action.key, visible);
      }
    }

    return createPortal(
      <div
        ref={ref}
        id={panelId}
        className={cn(
          "fixed z-50 w-72 overflow-auto rounded-md border border-border bg-popover p-2 text-left text-sm text-popover-foreground shadow-xl",
          className,
        )}
        // 动态：按钮功能浮层跟随触发按钮位置和视口高度。
        style={{ top: position.top, left: position.left, maxHeight: position.maxHeight }}
        data-datagrid-popover
        {...rest}
      >
        <div className="mb-1 flex items-center justify-between gap-2 px-2 py-1">
          <div className="text-xs font-medium text-muted-foreground">按钮功能显示设置</div>
          <Checkbox
            aria-label="全选或取消按钮功能"
            title={allVisible ? "取消" : "全选"}
            checked={allVisible || (someVisible ? "indeterminate" : false)}
            onCheckedChange={(value) => setAllActionsVisible(value === true)}
          />
        </div>
        {actions.map((action) => {
          const checkboxId = `${panelId}-${action.key}`;
          return (
            <label
              key={action.key}
              htmlFor={checkboxId}
              className="flex items-center gap-2 rounded-sm px-2 py-1.5 hover:bg-muted/60"
              title={action.description ?? action.label}
            >
              <Checkbox
                id={checkboxId}
                checked={action.visible}
                onCheckedChange={(value) => onActionVisibleChange(action.key, value === true)}
              />
              <span className="min-w-0 flex-1 truncate">{action.label}</span>
            </label>
          );
        })}
      </div>,
      document.body,
    );
  },
);
DataGridActionSettingsPanel.displayName = "DataGridActionSettingsPanel";
