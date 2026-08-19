import * as React from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  StatusBadge,
} from "@wms/ui";

import {
  useInventoryBatchTraceQuery,
  type InventoryBatch,
  type InventoryBatchTrace,
} from "@/features/inventory/inventory-queries";
import {
  availableQty,
  ExpiryDateCell,
  formatDateTime,
  qualityStatusKey,
  qualityStatusLabel,
  type QualityStatusOption,
} from "./M3BatchViewHelpers";

export function M3BatchDetailDialog({
  batch,
  expiryWarningDays,
  qualityStatusOptions,
  open,
  onOpenChange,
}: {
  batch: InventoryBatch | null;
  expiryWarningDays: number;
  qualityStatusOptions: QualityStatusOption[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const traceQuery = useInventoryBatchTraceQuery(batch?.id ?? "", open && Boolean(batch));
  if (!batch) return null;

  const available = availableQty(batch);
  const identityRows: Array<[string, string]> = [
    ["批号", batch.batch_no],
    ["商品编码", batch.product_code],
    ["库位", batch.location_code],
  ];
  const quantityRows: Array<[string, string]> = [
    ["现存", `${batch.qty_on_hand} 件`],
    ["锁定", `${batch.qty_frozen} 件`],
    ["可用", `${available} 件`],
  ];
  const qualityRows: Array<[string, React.ReactNode]> = [
    [
      "质量状态",
      <StatusBadge
        key="qs"
        status={qualityStatusKey(batch.status, batch.recall_flag)}
        label={qualityStatusLabel(batch.status, qualityStatusOptions)}
        size="sm"
      />,
    ],
    [
      "召回",
      batch.recall_flag ? (
        <StatusBadge key="rf" status="isolated" label="已标记" size="sm" />
      ) : (
        "未标记"
      ),
    ],
  ];
  const dateRows: Array<[string, React.ReactNode]> = [
    ["生产日期", batch.production_date || "-"],
    [
      "有效期",
      <ExpiryDateCell
        key="exp"
        expiryDate={batch.expiry_date}
        warningDays={expiryWarningDays}
      />,
    ],
    ["创建时间", formatDateTime(batch.created_at)],
    ["更新时间", formatDateTime(batch.updated_at)],
  ];
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>批号详情</DialogTitle>
          <DialogDescription>
            {batch.batch_no} · {batch.product_code} · {batch.location_code}
          </DialogDescription>
        </DialogHeader>

        <DetailSection title="批号 / 商品 / 库位">
          <DetailOverview rows={identityRows} />
        </DetailSection>
        <DetailSection title="数量">
          <DetailOverview rows={quantityRows} />
        </DetailSection>
        <DetailSection title="质量状态 / 召回">
          <DetailOverviewNode rows={qualityRows} />
        </DetailSection>
        <DetailSection title="效期与时间">
          <DetailOverviewNode rows={dateRows} />
        </DetailSection>
        <DetailSection title="流转追踪">
          {traceQuery.isPending ? (
            <p className="text-sm text-muted-foreground">加载追溯记录...</p>
          ) : traceQuery.error ? (
            <p className="text-sm text-destructive" role="alert">
              {traceQuery.error.message}
            </p>
          ) : (
            <TraceOverview trace={traceQuery.data} qualityStatusOptions={qualityStatusOptions} />
          )}
        </DetailSection>
      </DialogContent>
    </Dialog>
  );
}

function TraceOverview({
  trace,
  qualityStatusOptions,
}: {
  trace?: InventoryBatchTrace;
  qualityStatusOptions: QualityStatusOption[];
}) {
  if (!trace) return <p className="text-sm text-muted-foreground">暂无追溯记录</p>;
  return (
    <div className="grid gap-2 rounded-md border bg-muted/20 p-3 text-sm">
      <div>库存 movement：{trace.movements.length} 条</div>
      {trace.movements.map((movement) => (
        <div key={movement.id} className="grid gap-1 border-b pb-2 last:border-b-0 last:pb-0">
          <span>{movement.movement_type} · {movement.qty_delta} 件</span>
          <span className="text-xs text-muted-foreground">
            来源 {movement.source_document_type} / {movement.source_document_id}
          </span>
        </div>
      ))}
      <div>状态变更：{trace.status_changes.length} 条</div>
      {trace.status_changes.map((change) => (
        <div key={change.id} className="text-xs text-muted-foreground">
          {qualityStatusLabel(change.from_status, qualityStatusOptions)} → {qualityStatusLabel(change.to_status, qualityStatusOptions)} · {change.approval_source} / {change.approval_id}
        </div>
      ))}
    </div>
  );
}

function DetailSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="grid gap-2">
      <div className="text-sm font-semibold">{title}</div>
      {children}
    </section>
  );
}

function DetailOverview({ rows }: { rows: Array<[string, string]> }) {
  return (
    <section className="rounded-md border bg-muted/20">
      <div className="grid divide-y sm:grid-cols-3 sm:divide-x sm:divide-y-0">
        {rows.map(([label, value]) => (
          <div key={label} className="px-4 py-3">
            <div className="text-xs text-muted-foreground">{label}</div>
            <div className="mt-1 truncate text-sm font-semibold">{value}</div>
          </div>
        ))}
      </div>
    </section>
  );
}

function DetailOverviewNode({ rows }: { rows: Array<[string, React.ReactNode]> }) {
  return (
    <section className="rounded-md border bg-muted/20">
      <div className="grid divide-y sm:grid-cols-2 lg:grid-cols-4 sm:divide-x sm:divide-y-0">
        {rows.map(([label, value]) => (
          <div key={label} className="px-4 py-3">
            <div className="text-xs text-muted-foreground">{label}</div>
            <div className="mt-1 text-sm font-semibold">{value}</div>
          </div>
        ))}
      </div>
    </section>
  );
}
