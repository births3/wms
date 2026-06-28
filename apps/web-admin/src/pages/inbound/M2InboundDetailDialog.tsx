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
import { inboundDocumentTypeLabel, inboundDocumentTypeOf } from "./m2-inbound-document-type";

interface M2InboundDetailDialogProps {
  order: ReceivingOrder | null;
  defaultStage: DetailStage;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type DetailStage = "receiving" | "inspection" | "sign" | "putaway" | "completed";

export function M2InboundDetailDialog({ order, defaultStage, open, onOpenChange }: M2InboundDetailDialogProps) {
  const [selectedStage, setSelectedStage] = React.useState<DetailStage>(defaultStage);

  React.useEffect(() => {
    if (open) setSelectedStage(defaultStage);
  }, [defaultStage, open]);

  if (!order) return null;
  const line = order.lines[0];
  const lineSummary = order.lines.length > 1 ? ` 等 ${order.lines.length} 行` : "";
  const expectedQty = totalExpectedQty(order);
  const documentType = inboundDocumentTypeOf(order);
  const isSalesReturn = documentType === "sales_return";
  const currentStage = stageIndex(order.status);
  const selectedProcess = processDetail(selectedStage, expectedQty, currentStage);
  const overviewRows: Array<[string, string]> = [
    ["单据状态", statusLabel(order.status)],
    ["单据类型", inboundDocumentTypeLabel(documentType)],
    ["供应商", shortId(order.supplier_id)],
    ["仓库", shortId(order.warehouse_id)],
    ["预计到货", formatDateTime(order.expected_arrival_at)],
    ["商品概要", line ? `${line.product_code}${lineSummary}` : "-"],
  ];
  if (isSalesReturn) {
    overviewRows.push(["原销售批号", line?.batch_no ? `${line.batch_no}${lineSummary}` : "-"]);
  }
  overviewRows.push(
    ["生产 / 有效期", `${line?.production_date ?? "-"} / ${line?.expiry_date ?? "-"}`],
    ["预报数量", `${expectedQty} 件`],
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>订单详情</DialogTitle>
          <DialogDescription>{order.receipt_no}</DialogDescription>
        </DialogHeader>

        <OverviewGrid rows={overviewRows} />

        <InboundStatusRail currentStage={currentStage} selectedStage={selectedStage} onSelect={setSelectedStage} />

        <ProcessBlock title={selectedProcess.title} state={selectedProcess.state} rows={selectedProcess.rows} />

        <LinesBlock order={order} showBatch={isSalesReturn} />
      </DialogContent>
    </Dialog>
  );
}

function OverviewGrid({ rows }: { rows: Array<[string, string]> }) {
  return (
    <section className="rounded-md border bg-muted/20">
      <div className="grid divide-y sm:grid-cols-4 sm:divide-x sm:divide-y-0">
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
  selectedStage: DetailStage;
  onSelect: (stage: DetailStage) => void;
}) {
  const stages: Array<{ label: string; stage: DetailStage; index: number }> = [
    { label: "收货", stage: "receiving", index: 0 },
    { label: "验收", stage: "inspection", index: 1 },
    { label: "双人签字", stage: "sign", index: 2 },
    { label: "上架", stage: "putaway", index: 3 },
    { label: "完成", stage: "completed", index: 4 },
  ];

  return (
    <div className="grid gap-3 md:grid-cols-5">
      {stages.map(({ label, stage, index }) => {
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
      <div className="grid gap-3 p-4 sm:grid-cols-2">
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

function LinesBlock({ order, showBatch }: { order: ReceivingOrder; showBatch: boolean }) {
  const rowClass = showBatch
    ? "grid gap-2 px-4 py-3 text-sm md:grid-cols-[4rem_1fr_1fr_6rem]"
    : "grid gap-2 px-4 py-3 text-sm md:grid-cols-[4rem_1fr_6rem]";
  return (
    <div className="rounded-md border">
      <div className="border-b bg-muted/40 px-4 py-2.5 text-xs text-muted-foreground">入库明细</div>
      <div className="divide-y">
        {order.lines.map((item) => (
          <div key={item.line_no} className={rowClass}>
            <span className="text-muted-foreground">#{item.line_no}</span>
            <span className="font-medium">{item.product_code}</span>
            {showBatch && <span>{item.batch_no ?? "-"}</span>}
            <span className="text-right">{item.expected_qty} 件</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function totalExpectedQty(order: ReceivingOrder) {
  return order.lines.reduce((sum, line) => sum + line.expected_qty, 0);
}

function stageIndex(status: string) {
  if (status === "completed") return 4;
  if (status.includes("putaway")) return 3;
  if (status.includes("inspect")) return 1;
  if (status.includes("receiv")) return 0;
  return 0;
}

interface ProcessState {
  label: string;
  status: "completed" | "in_progress" | "pending";
}

function processState(index: number, current: number): ProcessState {
  if (index < current) return { label: "已完成", status: "completed" };
  if (index === current) return { label: "当前", status: "in_progress" };
  return { label: "待处理", status: "pending" };
}

function processDetail(stage: DetailStage, expectedQty: number, currentStage: number) {
  const map = {
    receiving: {
      title: "收货信息",
      state: processState(0, currentStage),
      rows: [
        ["承运商 / 车牌", "华东冷链 / 沪A-12345"],
        ["发运地点", "上海配送中心"],
        ["启运 / 到货", "2026-06-27 08:00 / 2026-06-27 10:00"],
        ["入库时间", "2026-06-27 10:15"],
        ["运输 / 温控 / 温度", "冷藏车 / 冷藏车 / 20℃"],
        ["联系人", "张三 / 13800000000 / 310101********0000"],
        ["随货核对", "印章已核对 / 备案件已核对"],
        ["数量闭合", `${expectedQty} / ${expectedQty} / 0 / 0 件`],
        ["第二收货员", "收货员 0102"],
        ["异常备注", "-"],
      ],
    },
    inspection: {
      title: "验收信息",
      state: processState(1, currentStage),
      rows: [
        ["通过 / 拒收", `${expectedQty} / 0 件`],
        ["追溯码", "TC-M2-PC-0001"],
        ["质量状态", "合格"],
        ["四项核对", "外观 / 包装 / 说明书 / 标签均合格"],
        ["验收备注", "-"],
      ],
    },
    sign: {
      title: "双人签字信息",
      state: processState(2, currentStage),
      rows: [
        ["双人策略", "验收节点命中双人扫码"],
        ["签字人", "验收员 0101 / 复核员 0102"],
        ["签字备注", "-"],
      ],
    },
    putaway: {
      title: "上架信息",
      state: processState(3, currentStage),
      rows: [
        ["容器 LPN", "LPN-M2-PC-0001"],
        ["推荐库位", "A-01-01 / A-01-02 / A-02-01"],
        ["实际库位", "待录入"],
        ["校验结果", "待执行"],
        ["上架备注", "-"],
      ],
    },
    completed: {
      title: "完成信息",
      state: processState(4, currentStage),
      rows: [["完成状态", currentStage >= 4 ? "已完成" : "未完成"]],
    },
  } satisfies Record<DetailStage, { title: string; state: ProcessState; rows: Array<[string, string]> }>;
  return map[stage];
}

function statusLabel(status: string) {
  const labels: Record<string, string> = {
    pending: "待处理",
    receiving: "收货中",
    inspecting: "验收中",
    putaway: "上架中",
    completed: "已完成",
  };
  return labels[status] ?? status;
}

function shortId(value: string | null | undefined) {
  if (!value) return "-";
  return value.slice(0, 8);
}

function formatDateTime(value: string | null | undefined) {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { hour12: false });
}
