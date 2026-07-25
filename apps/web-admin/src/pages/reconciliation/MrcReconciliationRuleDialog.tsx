/**
 * MrcReconciliationRuleDialog — 定时对账频率维护弹窗
 *
 * 层级：Layer 3 页面私有组件
 * 关联故事：US-RC-001
 * 职责：承载对账间隔、启用状态以及保存失败反馈。
 */

import type { FormEvent } from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
} from "@wms/ui";

interface MrcReconciliationRuleDialogProps {
  open: boolean;
  interval: string;
  enabled: boolean;
  pending: boolean;
  errorMessage: string | null;
  onOpenChange: (open: boolean) => void;
  onIntervalChange: (value: string) => void;
  onEnabledChange: (value: boolean) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}

export function MrcReconciliationRuleDialog({
  open,
  interval,
  enabled,
  pending,
  errorMessage,
  onOpenChange,
  onIntervalChange,
  onEnabledChange,
  onSubmit,
}: MrcReconciliationRuleDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <form className="grid gap-4" onSubmit={onSubmit}>
          <DialogHeader>
            <DialogTitle>维护定时对账频率</DialogTitle>
            <DialogDescription>默认每 24 小时执行一次；由外部 H-SCH 调度 Worker 单次拉取。</DialogDescription>
          </DialogHeader>
          <label className="grid gap-1 text-sm">
            对账间隔（小时）
            <Input type="number" min="1" max="168" required value={interval} onChange={(event) => onIntervalChange(event.target.value)} />
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={enabled} onChange={(event) => onEnabledChange(event.target.checked)} />
            启用定时对账
          </label>
          {errorMessage && <div className="text-sm text-destructive" role="alert">{errorMessage}</div>}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={pending}>取消</Button>
            </DialogClose>
            <Button type="submit" disabled={pending}>{pending ? "保存中..." : "保存"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
