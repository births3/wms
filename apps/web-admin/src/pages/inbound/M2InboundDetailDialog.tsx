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
  batchInfoRows,
  inboundDetailStageIndex,
  inboundDetailStages,
  orderLicenseRows,
  productInfoRows,
  processDetail,
  type InboundDetailStage,
  type ProcessState,
} from "./m2-inbound-detail-view-model";
import { inboundDocumentTypeLabel, inboundDocumentTypeOf } from "./m2-inbound-document-type";
import { formatDateTime, ownerLabel, totalExpectedQty, type OwnerContext } from "./m2-inbound-page-helpers";

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

  if (!order) return null;
  const line = order.lines[0];
  const lineSummary = order.lines.length > 1 ? ` 等 ${order.lines.length} 行` : "";
  const expectedQty = totalExpectedQty(order);
  const documentType = inboundDocumentTypeOf(order);
  const currentStage = inboundDetailStageIndex(order.status);
  const selectedProcess = processDetail(selectedStage, expectedQty, currentStage);
  const orderRows: Array<[string, string]> = [
    ["单据状态", statusLabel(order.status)],
    ["单据类型", inboundDocumentTypeLabel(documentType)],
    ["货主", ownerLabel(order.owner_id, currentOwner)],
    ["供应商", shortId(order.supplier_id)],
    ["采购员", "采购员 0101"],
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

        <ProductInfoBlock order={order} />

        <Section title="订单信息">
          <OverviewGrid rows={orderRows} />
          <BatchInfoBlock order={order} />
        </Section>

        <Section title="收货信息">
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
    <Section title="商品信息">
      <div className="rounded-md border">
        <div className="overflow-x-auto">
          <table className="w-full min-w-[1180px] text-sm">
            <thead className="bg-muted/20 text-xs text-muted-foreground">
              <tr>
                <th className="px-4 py-2 text-left font-medium">商品编码</th>
                <th className="px-4 py-2 text-left font-medium">品名</th>
                <th className="px-4 py-2 text-left font-medium">规格</th>
                <th className="px-4 py-2 text-left font-medium">生产厂家</th>
                <th className="px-4 py-2 text-right font-medium">订单数量</th>
                <th className="px-4 py-2 text-left font-medium">单位</th>
                <th className="px-4 py-2 text-right font-medium">件数</th>
                <th className="px-4 py-2 text-right font-medium">零数</th>
                <th className="px-4 py-2 text-left font-medium">中包数量</th>
                <th className="px-4 py-2 text-left font-medium">件包数量</th>
              </tr>
            </thead>
            <tbody className="divide-y">
              {rows.map((item) => (
                <tr key={item.key}>
                  <td className="px-4 py-2 font-medium">{item.productCode}</td>
                  <td className="px-4 py-2">{item.productName}</td>
                  <td className="px-4 py-2">{item.specification}</td>
                  <td className="px-4 py-2">{item.manufacturer}</td>
                  <td className="px-4 py-2 text-right">{item.orderQty}</td>
                  <td className="px-4 py-2">{item.unit}</td>
                  <td className="px-4 py-2 text-right">{item.caseQty}</td>
                  <td className="px-4 py-2 text-right">{item.looseQty}</td>
                  <td className="px-4 py-2">{item.middlePackQty}</td>
                  <td className="px-4 py-2">{item.casePackQty}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </Section>
  );
}

function BatchInfoBlock({ order }: { order: ReceivingOrder }) {
  const rows = batchInfoRows(order);
  return (
    <div className="rounded-md border">
      <div className="border-b bg-muted/40 px-4 py-2.5 text-xs font-medium text-muted-foreground">批号明细</div>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[1280px] text-sm">
          <thead className="bg-muted/20 text-xs text-muted-foreground">
            <tr>
              <th className="px-4 py-2 text-left font-medium">行号</th>
              <th className="px-4 py-2 text-left font-medium">批号</th>
              <th className="px-4 py-2 text-left font-medium">批准文号</th>
              <th className="px-4 py-2 text-left font-medium">进口注册证</th>
              <th className="px-4 py-2 text-left font-medium">上市持有人</th>
              <th className="px-4 py-2 text-right font-medium">批号数量</th>
              <th className="px-4 py-2 text-left font-medium">批号件包装</th>
              <th className="px-4 py-2 text-left font-medium">生产日期</th>
              <th className="px-4 py-2 text-left font-medium">有效期</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {rows.map((item) => (
              <tr key={item.key}>
                <td className="px-4 py-2 text-muted-foreground">{item.lineNo}</td>
                <td className="px-4 py-2">{item.batchNo}</td>
                <td className="px-4 py-2">{item.approvalNo}</td>
                <td className="px-4 py-2">{item.importRegistrationCertificate}</td>
                <td className="px-4 py-2">{item.marketingAuthorizationHolder}</td>
                <td className="px-4 py-2 text-right">{item.batchQty}</td>
                <td className="px-4 py-2">{item.batchCasePackage}</td>
                <td className="px-4 py-2">{item.productionDate}</td>
                <td className="px-4 py-2">{item.expiryDate}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function statusLabel(status: string) {
  const labels: Record<string, string> = {
    pending: "待处理",
    released: "待收货",
    receiving: "收货中",
    inspecting: "验收中",
    putaway: "上架中",
    completed: "已完成",
    closed_rejected: "已关闭(拒收)",
  };
  return labels[status] ?? status;
}

function shortId(value: string | null | undefined) {
  if (!value) return "-";
  return value.slice(0, 8);
}
