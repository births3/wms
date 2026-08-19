import * as React from "react";
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

import type { CancelInventoryRecallRequest, InventoryBatch } from "@/features/inventory/inventory-queries";

const emptyForm = { approvalId: "", secondApproverId: "", reason: "" };

export function M3BatchRecallCancelDialog({
  batch,
  open,
  pending,
  errorMessage,
  onOpenChange,
  onSubmit,
}: {
  batch: InventoryBatch | null;
  open: boolean;
  pending: boolean;
  errorMessage?: string;
  onOpenChange: (open: boolean) => void;
  onSubmit: (request: CancelInventoryRecallRequest) => Promise<void>;
}) {
  const [form, setForm] = React.useState(emptyForm);

  React.useEffect(() => {
    if (open) setForm(emptyForm);
  }, [batch, open]);

  if (!batch) return null;
  const selectedBatch = batch;

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await onSubmit({
      batch_id: selectedBatch.id,
      approval_id: form.approvalId.trim(),
      second_approver_id: form.secondApproverId.trim(),
      reason: form.reason.trim(),
    });
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <form className="grid gap-4" onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>取消召回</DialogTitle>
            <DialogDescription>
              {selectedBatch.batch_no} 需要仓库主管和质量负责人两名不同人员审批后才能取消召回。
            </DialogDescription>
          </DialogHeader>
          <label className="grid gap-1 text-sm">
            取消审批编号
            <Input
              required
              value={form.approvalId}
              onChange={(event) => setForm((value) => ({ ...value, approvalId: event.target.value }))}
              placeholder="例如 RECALL-CANCEL-001"
            />
          </label>
          <label className="grid gap-1 text-sm">
            质量审批人 ID
            <Input
              required
              value={form.secondApproverId}
              onChange={(event) => setForm((value) => ({ ...value, secondApproverId: event.target.value }))}
              placeholder="填写质量负责人的用户 ID"
            />
          </label>
          <label className="grid gap-1 text-sm">
            取消原因
            <Input
              required
              value={form.reason}
              onChange={(event) => setForm((value) => ({ ...value, reason: event.target.value }))}
              placeholder="说明取消召回原因"
            />
          </label>
          {errorMessage && <p className="text-sm text-destructive" role="alert">{errorMessage}</p>}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={pending}>取消</Button>
            </DialogClose>
            <Button type="submit" disabled={pending}>
              {pending ? "提交中..." : "确认取消"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
