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

import type { InventoryBatch, MarkInventoryRecallRequest } from "@/features/inventory/inventory-queries";
import { LOADING_SUBMITTING } from "@/lib/ui-strings";

const emptyForm = {
  approval_source: "M-QL",
  approval_id: "",
  reason: "",
};

export function M3BatchRecallDialog({
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
  onSubmit: (request: MarkInventoryRecallRequest) => Promise<void>;
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
      approval_source: form.approval_source,
      approval_id: form.approval_id.trim(),
      reason: form.reason.trim(),
    });
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-lg">
        <form className="grid gap-4" onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>标记召回</DialogTitle>
            <DialogDescription>
              将 {selectedBatch.batch_no} 标记为召回并隔离。该操作不可撤销，必须提供审批依据。
            </DialogDescription>
          </DialogHeader>
          <label className="grid gap-1 text-sm">
            审批来源
            <select
              className="h-9 rounded-md border border-input bg-background px-3 text-sm"
              aria-label="审批来源"
              value={form.approval_source}
              onChange={(event) => setForm((value) => ({ ...value, approval_source: event.target.value }))}
            >
              <option value="M-QL">质量管理</option>
              <option value="M-TC">温控事件</option>
            </select>
          </label>
          <label className="grid gap-1 text-sm">
            审批编号
            <Input
              required
              value={form.approval_id}
              onChange={(event) => setForm((value) => ({ ...value, approval_id: event.target.value }))}
              placeholder="例如 RECALL-20260713-001"
            />
          </label>
          <label className="grid gap-1 text-sm">
            召回原因
            <Input
              required
              value={form.reason}
              onChange={(event) => setForm((value) => ({ ...value, reason: event.target.value }))}
              placeholder="说明召回原因"
            />
          </label>
          {errorMessage && <p className="text-sm text-destructive" role="alert">{errorMessage}</p>}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={pending}>取消</Button>
            </DialogClose>
            <Button type="submit" disabled={pending}>
              {pending ? LOADING_SUBMITTING : "确认召回"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
