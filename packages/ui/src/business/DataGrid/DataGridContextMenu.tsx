import * as React from "react";
import { createPortal } from "react-dom";
import { Calculator, CheckSquare, ClipboardPaste, Copy, Grid2X2 } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import { useDataGridPopoverDismiss } from "./data-grid-popover-dismiss";

/**
 * DataGridContextMenu — DataGrid 单元格右键菜单
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端 DataGrid 横向能力
 * Wave：Wave 6 管理端表格增强
 * 业务约束：提供行复制和区域复制，不把复制逻辑下放到页面。
 *
 * @example
 *   <DataGridContextMenu open position={{x: 100, y: 100}} />
 */
export interface DataGridContextMenuPosition {
  x: number;
  y: number;
}

export interface DataGridContextMenuProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  open: boolean;
  position: DataGridContextMenuPosition | null;
  areaSelectionEnabled: boolean;
  hasSelectedArea: boolean;
  onClose: () => void;
  onCopyRow: () => void;
  onCopyRowWithHeader: () => void;
  canPaste: boolean;
  canColumnPaste: boolean;
  onPaste: () => void;
  onColumnPaste: () => void;
  areaSumText: string | null;
  onStartAreaSelection: () => void;
  onCloseAreaSelection: () => void;
  onCopyArea: () => void;
  onCopyAreaWithHeader: () => void;
  onCopyAreaSum: () => void;
}

export const DataGridContextMenu = React.forwardRef<HTMLDivElement, DataGridContextMenuProps>(
  (
    {
      open,
      position,
      areaSelectionEnabled,
      hasSelectedArea,
      onClose,
      onCopyRow,
      onCopyRowWithHeader,
      canPaste,
      canColumnPaste,
      onPaste,
      onColumnPaste,
      areaSumText,
      onStartAreaSelection,
      onCloseAreaSelection,
      onCopyArea,
      onCopyAreaWithHeader,
      onCopyAreaSum,
      className,
      ...rest
    },
    ref,
  ) => {
    useDataGridPopoverDismiss({ open, onDismiss: onClose });
    if (!open || !position || typeof document === "undefined") return null;

    return createPortal(
      <div
        ref={ref}
        role="menu"
        className={cn("fixed z-50 w-56 rounded-md border bg-background p-1 text-sm shadow-lg", className)}
        style={{ left: position.x, top: position.y }}
        data-datagrid-popover
        {...rest}
      >
        <MenuButton onClick={onCopyRow}>
          <Copy className="size-4" aria-hidden />
          行复制
        </MenuButton>
        <MenuButton onClick={onCopyRowWithHeader}>
          <Copy className="size-4" aria-hidden />
          行复制加表头
        </MenuButton>
        <MenuButton onClick={onPaste} disabled={!canPaste}>
          <ClipboardPaste className="size-4" aria-hidden />
          粘贴
        </MenuButton>
        <MenuButton onClick={onColumnPaste} disabled={!canColumnPaste}>
          <ClipboardPaste className="size-4" aria-hidden />
          列粘贴
        </MenuButton>
        <MenuButton onClick={onStartAreaSelection} disabled={areaSelectionEnabled}>
          <CheckSquare className="size-4" aria-hidden />
          启动区域选择
        </MenuButton>
        <MenuButton onClick={onCloseAreaSelection} disabled={!areaSelectionEnabled}>
          <CheckSquare className="size-4" aria-hidden />
          关闭区域选择
        </MenuButton>
        <MenuButton onClick={onCopyArea} disabled={!hasSelectedArea}>
          <Grid2X2 className="size-4" aria-hidden />
          复制区域
        </MenuButton>
        <MenuButton onClick={onCopyAreaWithHeader} disabled={!hasSelectedArea}>
          <Grid2X2 className="size-4" aria-hidden />
          复制区域加表头
        </MenuButton>
        <MenuButton onClick={onCopyAreaSum} disabled={!areaSumText}>
          <Calculator className="size-4" aria-hidden />
          区域求和{areaSumText ? `：${areaSumText}` : ""}
        </MenuButton>
      </div>,
      document.body,
    );
  },
);
DataGridContextMenu.displayName = "DataGridContextMenu";

function MenuButton({
  children,
  onClick,
  disabled,
}: {
  children: React.ReactNode;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      className="h-8 w-full justify-start px-2"
      disabled={disabled}
      role="menuitem"
      onClick={onClick}
    >
      {children}
    </Button>
  );
}
