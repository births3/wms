/**
 * M2InboundDetailDialog — 入库单详情弹窗
 *
 * 层级：Layer 3 页面局部组件
 * 关联故事：US-M2-002, US-M2-003, US-M2-004, US-M2-005
 * Wave：Wave 6
 * 业务约束：只读单据详情和入库节点状态在弹窗内查看。
 *
 * @example
 *   <M2InboundDetailDialog order={order} open onOpenChange={() => void 0} />
 */

import * as React from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  StatusBadge,
} from "@wms/ui";

import type { ReceivingOrder } from "@/features/inbound/inbound-queries";
import {
  batchInfoFieldDefinitions,
  batchInfoRows,
  inboundDetailFieldSections,
  inboundDetailStageIndex,
  inboundDetailStages,
  orderLicenseRows,
  productInfoFieldDefinitions,
  productInfoRows,
  processDetail,
  type InboundDetailStage,
  type ProcessState,
} from "./m2-inbound-detail-view-model";
import { inboundDocumentTypeLabel, inboundDocumentTypeOf } from "./m2-inbound-document-type";
import {
  formatDateTime,
  ownerLabel,
  statusLabel,
  totalExpectedQty,
  type OwnerContext,
} from "./m2-inbound-page-helpers";

interface M2InboundDetailDialogProps {
  order: ReceivingOrder | null;
  currentOwner: OwnerContext;
  defaultStage: InboundDetailStage;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function M2InboundDetailDialog({ order, currentOwner, defaultStage, open, onOpenChange }: M2InboundDetailDialogProps) {
  const [selectedStage, setSelectedStage] = React.useState<InboundDetailStage>(defaultStage);

  React.useEffect(() => {
    if (open) setSelectedStage(defaultStage);
  }, [defaultStage, open]);

  // 关闭且无单时不渲染；有单时也必须容忍 list 摘要缺 lines/status
  if (!order) return null;
  const line = order.lines?.[0];
  const lineCount = order.lines?.length ?? 0;
  const lineSummary = lineCount > 1 ? ` 等 ${lineCount} 行` : "";
  const expectedQty = totalExpectedQty(order);
  const documentType = inboundDocumentTypeOf(order);
  const currentStage = inboundDetailStageIndex(order.status ?? "");
  const selectedProcess = processDetail(selectedStage, expectedQty, currentStage);
  const orderRows: Array<[string, string]> = [
    ["单据状态", statusLabel(order.status)],
    ["单据类型", inboundDocumentTypeLabel(documentType)],
    ["货主", ownerLabel(order.owner_id, currentOwner)],
    ["供应商", shortId(order.supplier_id)],
    // 采购员尚未随入库单返回：缺数据展示「-」，不得虚构账号
    ["采购员", "-"],
    ["仓库", shortId(order.warehouse_id)],
    ["预计到货", formatDateTime(order.expected_arrival_at)],
    ["预报数量", `${expectedQty} 件`],
    ...orderLicenseRows(order),
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] w-[calc(100vw-2rem)] max-w-none overflow-y-auto p-4 sm:p-6 lg:w-[92vw] 2xl:w-[1440px]">
        <DialogHeader>
          <DialogTitle>订单详情</DialogTitle>
          <DialogDescription>{order.receipt_no} · {line ? `${line.product_code}${lineSummary}` : "-"}</DialogDescription>
        </DialogHeader>

        <Section title={inboundDetailFieldSections.product.title}>
          <ProductInfoBlock order={order} />
        </Section>

        <Section title={inboundDetailFieldSections.order.title}>
          <OverviewGrid rows={orderRows} />
        </Section>

        <Section title={inboundDetailFieldSections.batch.title}>
          <BatchInfoBlock order={order} />
        </Section>

        <Section title={inboundDetailFieldSections.process.title}>
          <InboundStatusRail currentStage={currentStage} selectedStage={selectedStage} onSelect={setSelectedStage} />

          <ProcessBlock title={selectedProcess.title} state={selectedProcess.state} rows={selectedProcess.rows} />
        </Section>
      </DialogContent>
    </Dialog>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="grid gap-3">
      <div className="text-sm font-semibold">{title}</div>
      {children}
    </section>
  );
}

function OverviewGrid({ rows }: { rows: Array<[string, string]> }) {
  return (
    <section className="rounded-md border bg-muted/20">
      <div className="grid divide-y sm:grid-cols-3 lg:grid-cols-6 sm:divide-x sm:divide-y-0">
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

function InboundStatusRail({
  currentStage,
  selectedStage,
  onSelect,
}: {
  currentStage: number;
  selectedStage: InboundDetailStage;
  onSelect: (stage: InboundDetailStage) => void;
}) {
  return (
    <div className="grid gap-3 md:grid-cols-4">
      {inboundDetailStages.map(({ label, stage, index }) => {
        const done = index < currentStage;
        const active = index === currentStage;
        const selected = stage === selectedStage;
        return (
          <button
            key={label}
            type="button"
            onClick={() => onSelect(stage)}
            className={
              selected
                ? "rounded-md border border-primary bg-primary/10 p-3"
                : active
                  ? "rounded-md border border-primary/40 bg-background p-3"
                : done
                  ? "rounded-md border border-wms-success/30 bg-wms-success/10 p-3"
                  : "rounded-md border bg-background p-3"
            }
          >
            <div className="flex items-center justify-between gap-2">
              <span className="text-sm font-semibold">{label}</span>
              <StatusBadge
                status={done ? "completed" : active ? "in_progress" : "pending"}
                label={done ? "已完成" : active ? "当前" : "待处理"}
                size="sm"
              />
            </div>
          </button>
        );
      })}
    </div>
  );
}

function ProcessBlock({ title, state, rows }: { title: string; state: ProcessState; rows: Array<[string, string]> }) {
  return (
    <section className="rounded-md border">
      <div className="flex items-center justify-between gap-3 border-b bg-muted/40 px-4 py-2.5">
        <div className="text-xs font-medium text-muted-foreground">{title}</div>
        <StatusBadge status={state.status} label={state.label} size="sm" />
      </div>
      <div className="grid gap-3 p-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
        {rows.map(([label, value]) => (
          <div key={label} className="rounded-md border bg-background px-3 py-2 text-sm">
            <div className="text-xs text-muted-foreground">{label}</div>
            <div className="mt-1 font-medium">{value}</div>
          </div>
        ))}
      </div>
    </section>
  );
}

function ProductInfoBlock({ order }: { order: ReceivingOrder }) {
  const rows = productInfoRows(order);
  return (
    <div className="rounded-md border">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[1180px] text-sm">
          <thead className="bg-muted/20 text-xs text-muted-foreground">
            <tr>
              {productInfoFieldDefinitions.map((field) => (
                <th key={field.key} className={`px-4 py-2 font-medium ${field.align === "right" ? "text-right" : "text-left"}`}>
                  {field.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y">
            {rows.map((item) => (
              <tr key={item.key}>
                {productInfoFieldDefinitions.map((field) => (
                  <td key={field.key} className={`px-4 py-2 ${field.key === "productCode" ? "font-medium" : ""} ${field.align === "right" ? "text-right" : ""}`}>
                    {item[field.key]}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function BatchInfoBlock({ order }: { order: ReceivingOrder }) {
  const rows = batchInfoRows(order);
  return (
    <div className="rounded-md border">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[1280px] text-sm">
          <thead className="bg-muted/20 text-xs text-muted-foreground">
            <tr>
              {batchInfoFieldDefinitions.map((field) => (
                <th key={field.key} className={`px-4 py-2 font-medium ${field.align === "right" ? "text-right" : "text-left"}`}>
                  {field.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y">
            {rows.map((item) => (
              <tr key={item.key}>
                {batchInfoFieldDefinitions.map((field) => (
                  <td key={field.key} className={`px-4 py-2 ${field.key === "lineNo" ? "text-muted-foreground" : ""} ${field.align === "right" ? "text-right" : ""}`}>
                    {item[field.key]}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function shortId(value: string | null | undefined) {
  if (!value) return "-";
  return value.slice(0, 8);
}
