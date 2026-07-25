/**
 * MrcReconciliationIsolationDialog — 对账隔离引用确认弹窗
 *
 * 层级：Layer 3 页面私有组件
 * 关联故事：US-RC-002
 * 职责：解释共享隔离引用语义，并承载隔离或释放写操作的确认与错误反馈。
 */

import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@wms/ui";

interface MrcReconciliationIsolationDialogProps {
  intent: boolean | null;
  selectedCount: number;
  pending: boolean;
  errorMessage: string | null;
  onOpenChange: (open: boolean) => void;
  onSubmit: () => void;
}

export function MrcReconciliationIsolationDialog({
  intent,
  selectedCount,
  pending,
  errorMessage,
  onOpenChange,
  onSubmit,
}: MrcReconciliationIsolationDialogProps) {
  const isolate = intent !== false;

  return (
    <Dialog open={intent !== null} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{isolate ? "确认对账隔离" : "确认释放隔离"}</DialogTitle>
          <DialogDescription>
            {isolate
              ? `将为已选 ${selectedCount} 条差异对应的合格库存批次建立本次对账隔离引用。`
              : `将释放已选 ${selectedCount} 条差异的本次对账隔离引用；仍被其他对账引用的库存不会恢复可用。`}
          </DialogDescription>
        </DialogHeader>
        {errorMessage && (
          <div className="text-sm text-destructive" role="alert">
            {errorMessage}
          </div>
        )}
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline" disabled={pending}>取消</Button>
          </DialogClose>
          <Button type="button" disabled={pending || selectedCount === 0} onClick={onSubmit}>
            {pending ? "处理中..." : isolate ? "确认隔离" : "确认释放"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
