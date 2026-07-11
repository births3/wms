/**
 * M4OutboundDetailDialog — 出库详情弹窗
 *
 * 层级：Layer 3 页面局部组件
 * 关联故事：US-M4-001, US-M4-002, US-M4-004, US-M4-006, US-M4-008
 * Wave：Wave 6
 * 业务约束：单据流程、明细和节点状态只在详情弹窗中查看。
 *
 * @example
 *   <M4OutboundDetailDialog target={target} open onOpenChange={() => void 0} />
 */

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  StatusBadge,
} from "@wms/ui";
import type { components } from "@wms/api-client";

export type OutboundOrder = components["schemas"]["OutboundOrder"];
export type OutboundWave = components["schemas"]["OutboundWave"];

export interface PurchaseReturnOrder {
  id: string;
  return_no: string;
  document_type: "purchase_return_outbound";
  source_purchase_order_no: string;
  supplier_name: string;
  reason: string;
  approval_source: "purchase_return_approval";
  status: string;
  product_code: string;
  qty: number;
  created_at: string;
  updated_at: string;
}

export type DetailTarget =
  | { kind: "order"; value: OutboundOrder }
  | { kind: "wave"; value: OutboundWave; orders: OutboundOrder[] }
  | { kind: "return"; value: PurchaseReturnOrder };

interface M4OutboundDetailDialogProps {
  target: DetailTarget | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function M4OutboundDetailDialog({ target, open, onOpenChange }: M4OutboundDetailDialogProps) {
  if (!target) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>{detailTitle(target)}</DialogTitle>
          <DialogDescription>{detailDescription(target)}</DialogDescription>
        </DialogHeader>

        <StatusRail labels={stageLabels(target.kind)} activeIndex={stageIndex(target)} />

        {target.kind === "order" && <OrderDetail order={target.value} />}
        {target.kind === "wave" && <WaveDetail wave={target.value} orders={target.orders} />}
        {target.kind === "return" && <ReturnDetail returnOrder={target.value} />}
      </DialogContent>
    </Dialog>
  );
}

function OrderDetail({ order }: { order: OutboundOrder }) {
  return (
    <section className="grid gap-4">
      <div className="grid gap-4 md:grid-cols-2">
        <DetailBlock
          title="单据信息"
          rows={[
            ["WMS 单号", order.wms_order_no],
            ["ERP 单号", order.erp_order_no ?? "-"],
            ["订单类型", "销售出库"],
            ["客户 / 门店", `${shortId(order.customer_id)} / 门店A`],
            ["要求发货", formatDateTime(order.required_ship_at)],
          ]}
        />
        <DetailBlock
          title="执行摘要"
          rows={[
            ["当前状态", statusLabel(order.status)],
            ["明细行数", `${order.lines.length} 行`],
            ["计划数量", `${totalPlannedQty(order)} 件`],
            ["短拣标识", order.short_pick ? "是" : "否"],
          ]}
        />
        <DetailBlock
          title="校验与配送"
          rows={[
            ["校验结果", order.status === "validation_exception" ? "校验异常" : "指定批号库存充足"],
            ["复核模式", "包装站复核"],
            ["配送方", "第三方快递"],
            ["包裹数量", "1"],
          ]}
        />
        <DetailBlock
          title="交接字段"
          rows={[
            ["交接时间", order.status === "shipped" ? formatDateTime(order.updated_at) : "—"],
            ["车牌号", order.status === "shipped" ? "沪A-12345" : "—"],
            ["装车温度", "—"],
            ["签字", order.status === "shipped" ? "已签字" : "—"],
          ]}
        />
      </div>
      <Lines title="出库明细" lines={order.lines.map((line) => ({
        key: String(line.line_no),
        cells: [`#${line.line_no}`, line.product_code, line.batch_no, `${line.planned_qty} 件`],
      }))} />
    </section>
  );
}

function WaveDetail({ wave, orders }: { wave: OutboundWave; orders: OutboundOrder[] }) {
  return (
    <section className="grid gap-4">
      <DetailBlock
        title="波次信息"
        rows={[
          ["波次号", wave.wave_no],
          ["当前状态", statusLabel(wave.status)],
          ["订单数", `${wave.order_ids.length}`],
          ["明细行数", `${orders.reduce((sum, order) => sum + order.lines.length, 0)}`],
          ["路径策略", "S 型最短路径"],
          ["温区", "常温"],
          ["容量上限", "100 单 / 10000 件"],
          ["创建时间", formatDateTime(wave.created_at)],
        ]}
      />
      <Lines title="波次订单" lines={orders.map((order) => ({
        key: order.id,
        cells: [order.wms_order_no, order.erp_order_no ?? "-", `${totalPlannedQty(order)} 件`, statusLabel(order.status)],
      }))} />
    </section>
  );
}

function ReturnDetail({ returnOrder }: { returnOrder: PurchaseReturnOrder }) {
  return (
    <section className="grid gap-4">
      <div className="grid gap-4 md:grid-cols-2">
        <DetailBlock
          title="采购退货信息"
          rows={[
            ["采购退货单号", returnOrder.return_no],
            ["单据类型", purchaseReturnDocumentTypeLabel(returnOrder.document_type)],
            ["原采购入库单", returnOrder.source_purchase_order_no],
            ["供应商", returnOrder.supplier_name],
            ["退货原因", returnOrder.reason],
            ["审批来源", purchaseReturnApprovalSourceLabel(returnOrder.approval_source)],
          ]}
        />
        <DetailBlock
          title="商品与数量"
          rows={[
            ["当前状态", statusLabel(returnOrder.status)],
            ["商品编码", returnOrder.product_code],
            ["数量", `${returnOrder.qty} 件`],
          ]}
        />
        <DetailBlock
          title="出库执行"
          rows={[
            ["审批记录", returnOrder.status === "pending_approval" ? "待审批" : returnOrder.status === "cancelled" ? "已驳回" : "已审批"],
            ["拣货记录", returnOrder.status === "picking" || returnOrder.status === "reviewed" || returnOrder.status === "shipped" ? "已拣货" : "—"],
            ["复核记录", returnOrder.status === "reviewed" || returnOrder.status === "shipped" ? "已复核" : "—"],
            ["出库交接", returnOrder.status === "shipped" ? "已交接" : "—"],
          ]}
        />
      </div>
    </section>
  );
}

function StatusRail({ labels, activeIndex }: { labels: string[]; activeIndex: number }) {
  return (
    <div className="grid gap-3 md:grid-cols-5">
      {labels.map((label, index) => {
        const done = index < activeIndex;
        const active = index === activeIndex;
        return (
          <div
            key={label}
            className={
              active
                ? "rounded-md border border-primary bg-primary/10 p-3"
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
          </div>
        );
      })}
    </div>
  );
}

function DetailBlock({ title, rows }: { title: string; rows: Array<[string, string]> }) {
  return (
    <div className="rounded-md border">
      <div className="border-b bg-muted/40 px-4 py-2.5 text-xs text-muted-foreground">{title}</div>
      <div className="divide-y">
        {rows.map(([label, value]) => (
          <div key={label} className="flex items-center justify-between gap-3 px-4 py-3 text-sm">
            <span className="text-muted-foreground">{label}</span>
            <span className="truncate font-medium">{value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function Lines({ title, lines }: { title: string; lines: Array<{ key: string; cells: string[] }> }) {
  return (
    <div className="rounded-md border">
      <div className="border-b bg-muted/40 px-4 py-2.5 text-xs text-muted-foreground">{title}</div>
      <div className="divide-y">
        {lines.map((line) => (
          <div key={line.key} className="grid gap-2 px-4 py-3 text-sm md:grid-cols-4">
            {line.cells.map((cell) => (
              <span key={cell} className="truncate">{cell}</span>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

function detailTitle(target: DetailTarget) {
  if (target.kind === "wave") return "波次详情";
  if (target.kind === "return") return "采购退货详情";
  return "订单详情";
}

function detailDescription(target: DetailTarget) {
  if (target.kind === "wave") return target.value.wave_no;
  if (target.kind === "return") return target.value.return_no;
  return target.value.wms_order_no;
}

function stageLabels(kind: DetailTarget["kind"]) {
  if (kind === "return") return ["申请", "审批", "拣货", "复核", "发货"];
  if (kind === "wave") return ["创建", "下发", "锁定", "拣选", "完成"];
  return ["校验", "入波次", "拣选", "复核", "发货"];
}

function stageIndex(target: DetailTarget) {
  const status = target.value.status;
  if (target.kind === "return") {
    if (status === "shipped") return 4;
    if (status === "reviewed") return 3;
    if (status === "picking") return 2;
    if (status === "approved") return 1;
    return 0;
  }
  if (target.kind === "wave") {
    if (status === "completed") return 4;
    if (status === "picking") return 3;
    if (status === "inventory_locked") return 2;
    if (status === "released") return 1;
    return 0;
  }
  if (status === "shipped" || status === "signed") return 4;
  if (status === "reviewed") return 3;
  if (status === "inventory_locked") return 2;
  if (status === "in_wave") return 1;
  return 0;
}

export function statusLabel(status: string) {
  const labels: Record<string, string> = {
    pending_validation: "待校验",
    validation_exception: "校验异常",
    confirmed: "已确认",
    in_wave: "已入波次",
    inventory_locked: "库存锁定",
    reviewed: "已复核",
    shipped: "已发货",
    signed: "已签收",
    void_requested: "作废申请",
    draft: "待下发",
    released: "已下发",
    cancelled: "已取消",
    picking: "拣选中",
    pending_approval: "待审批",
    approved: "已审批",
    pickup: "提货中",
    inspecting: "验收中",
    completed: "已完成",
  };
  return labels[status] ?? status;
}

function purchaseReturnDocumentTypeLabel(value: PurchaseReturnOrder["document_type"]) {
  return value === "purchase_return_outbound" ? "采购退货出库" : value;
}

function purchaseReturnApprovalSourceLabel(value: PurchaseReturnOrder["approval_source"] | string) {
  if (value === "purchase_return_approval") return "采购退货审批";
  return value;
}

function totalPlannedQty(order: OutboundOrder) {
  return order.lines.reduce((sum, line) => sum + line.planned_qty, 0);
}

function shortId(value: string) {
  return value.slice(0, 8);
}

function formatDateTime(value: string | null | undefined) {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { hour12: false });
}
