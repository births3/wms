import * as React from "react";
import { cn } from "../../lib/utils";
import { Button } from "../../ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../../ui/dialog";
import { Input } from "../../ui/input";
import { Label } from "../../ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../../ui/select";
import type { DataGridExportFormat } from "./data-grid-export";

/**
 * DataGridExportDialog — DataGrid 导出设置弹窗
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 导入导出横向能力
 * Wave：Wave 6 管理端表格增强
 * 业务约束：导出文件名和格式由用户确认，实际导出内容由 DataGrid 公共导出工具生成
 *
 * @example
 *   <DataGridExportDialog open fileName="M2收货管理_2607040934" format="xlsx" />
 */
export interface DataGridExportDialogProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "onChange"> {
  open: boolean;
  fileName: string;
  format: DataGridExportFormat;
  rowCount: number;
  onOpenChange: (open: boolean) => void;
  onFileNameChange: (fileName: string) => void;
  onFormatChange: (format: DataGridExportFormat) => void;
  onConfirm: () => void;
}

export const DataGridExportDialog = React.forwardRef<HTMLDivElement, DataGridExportDialogProps>(
  (
    {
      open,
      fileName,
      format,
      rowCount,
      className,
      onOpenChange,
      onFileNameChange,
      onFormatChange,
      onConfirm,
      ...rest
    },
    ref,
  ) => {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent ref={ref} className={cn("max-h-[90vh] overflow-y-auto sm:max-w-lg", className)} {...rest}>
        <DialogHeader>
          <DialogTitle>导出列表</DialogTitle>
          <DialogDescription>确认文件名和导出格式，当前筛选结果共 {rowCount} 条。</DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 py-2">
          <div className="grid gap-2">
            <Label htmlFor="data-grid-export-file-name">文件名</Label>
            <Input
              id="data-grid-export-file-name"
              value={fileName}
              onChange={(event) => onFileNameChange(event.target.value)}
              placeholder="请输入导出文件名"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="data-grid-export-format">格式</Label>
            <Select value={format} onValueChange={(value) => onFormatChange(value as DataGridExportFormat)}>
              <SelectTrigger id="data-grid-export-format" aria-label="导出格式">
                <SelectValue placeholder="选择格式" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="xls">xls</SelectItem>
                <SelectItem value="xlsx">xlsx</SelectItem>
                <SelectItem value="csv">csv</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline">
              取消
            </Button>
          </DialogClose>
          <Button type="button" onClick={onConfirm} disabled={!fileName.trim() || rowCount === 0}>
            导出
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
  },
);

DataGridExportDialog.displayName = "DataGridExportDialog";
