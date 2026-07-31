import { StatusBadge, type DataGridColumn } from "@wms/ui";

import {
  BatchNoCell,
  CustomerCell,
  OrderNoSummary,
  ProductSummary,
  ReviewSummary,
  TwoLine,
  ValidationBadge,
  purchaseReturnDocumentTypeLabel,
} from "./M4OutboundPageParts";
import type {
  OutboundOrder,
  OutboundWave,
  PurchaseReturnOrder,
} from "./m4-outbound-page-model";
import {
  statusKey,
  statusLabel,
  statusOptions,
  type M4OutboundMode,
} from "./m4-outbound-page-model";

export function outboundOrderColumns(
  mode: M4OutboundMode,
  openDetail: (id: string) => void,
): DataGridColumn<OutboundOrder>[] {
  if (mode === "review") {
    return [
      { key: "wms_order_no", header: "单号 / 类型", mono: true, minWidth: 220, width: 240, onDoubleClick: (row) => openDetail(row.id), render: (row) => <OrderNoSummary order={row} /> },
      { key: "product", header: "计划件数", minWidth: 120, render: (row) => <ReviewSummary order={row} /> },
      { key: "customer_id", header: "客户 / 配送", minWidth: 160, render: (row) => (
        <div className="text-sm">
          <CustomerCell customerId={row.customer_id} />
          <div className="mt-0.5 text-xs text-muted-foreground">配送 第三方快递</div>
        </div>
      ) },
      { key: "required_ship_at", header: "包裹 / 车牌", minWidth: 150, render: () => <TwoLine top="包裹数量 1" bottom="车牌号 沪A-12345" /> },
      createdAtColumn<OutboundOrder>((row) => row.created_at),
      statusColumn<OutboundOrder>(mode, (row) => row.status),
    ];
  }
  return [
    { key: "wms_order_no", header: "单号 / 类型", mono: true, minWidth: 220, width: 240, onDoubleClick: (row) => openDetail(row.id), render: (row) => <OrderNoSummary order={row} /> },
    { key: "product", header: "商品 / 数量", minWidth: 140, render: (row) => <ProductSummary order={row} /> },
    { key: "batch_no", header: "批号", mono: true, minWidth: 150, render: (row) => <BatchNoCell order={row} /> },
    { key: "validation", header: "校验", minWidth: 100, render: (row) => <ValidationBadge order={row} /> },
    { key: "customer_id", header: "客户 / 门店", minWidth: 150, render: (row) => <CustomerCell customerId={row.customer_id} /> },
    { key: "required_ship_at", header: "要求发货", minWidth: 150, render: (row) => formatDate(row.required_ship_at) },
    createdAtColumn<OutboundOrder>((row) => row.created_at),
    statusColumn<OutboundOrder>(mode, (row) => row.status),
  ];
}

export function outboundWaveColumns(
  mode: M4OutboundMode,
  orders: OutboundOrder[],
  openDetail: (id: string) => void,
): DataGridColumn<OutboundWave>[] {
  return [
    { key: "wave_no", header: "波次号", mono: true, minWidth: 180, onDoubleClick: (row) => openDetail(row.id), render: (row) => <span className="text-primary">{row.wave_no}</span> },
    { key: "orders", header: "订单 / 明细", minWidth: 140, render: (row) => `${(row.order_ids ?? []).length} 单 / ${waveLineCount(row, orders)} 行` },
    { key: "qty", header: "件数 / 温区", align: "right", minWidth: 130, render: (row) => `${waveQty(row, orders)} 件 / 常温` },
    { key: "route", header: "路径策略 / 容量", minWidth: 180, render: () => <TwoLine top="S 型最短路径" bottom="容量上限 100 单 / 10000 件" /> },
    createdAtColumn<OutboundWave>((row) => row.created_at),
    statusColumn<OutboundWave>(mode, (row) => row.status),
  ];
}

export function purchaseReturnColumns(
  mode: M4OutboundMode,
  openDetail: (id: string) => void,
): DataGridColumn<PurchaseReturnOrder>[] {
  return [
    { key: "return_no", header: "采购退货单 / 类型", mono: true, minWidth: 240, width: 260, onDoubleClick: (row) => openDetail(row.id), render: (row) => <TwoLine top={row.return_no} bottom={purchaseReturnDocumentTypeLabel(row.document_type)} /> },
    { key: "source_purchase_order_no", header: "原采购入库单", mono: true, minWidth: 180 },
    { key: "supplier_name", header: "供应商 / 原因", minWidth: 200, render: (row) => <TwoLine top={row.supplier_name} bottom={row.reason} /> },
    { key: "product", header: "商品 / 数量", minWidth: 160, render: (row) => <TwoLine top={row.product_code} bottom={`${row.qty} 件`} /> },
    createdAtColumn<PurchaseReturnOrder>((row) => row.created_at),
    statusColumn<PurchaseReturnOrder>(mode, (row) => row.status),
  ];
}

function createdAtColumn<Row>(value: (row: Row) => string): DataGridColumn<Row> {
  return { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(value(row)) };
}

function statusColumn<Row>(
  mode: M4OutboundMode,
  value: (row: Row) => string,
): DataGridColumn<Row> {
  return {
    key: "status",
    header: "状态",
    minWidth: 130,
    filter: { type: "multiSelect", options: statusOptions(mode).map(([option, label]) => ({ value: option, label })) },
    render: (row) => <StatusBadge status={statusKey(value(row))} label={statusLabel(value(row))} size="sm" />,
  };
}

function waveQty(wave: OutboundWave, orders: OutboundOrder[]) {
  const orderIds = wave.order_ids ?? [];
  return orders
    .filter((order) => orderIds.includes(order.id))
    .reduce((sum, order) => sum + (order.lines ?? []).reduce((lineSum, line) => lineSum + line.planned_qty, 0), 0);
}

function waveLineCount(wave: OutboundWave, orders: OutboundOrder[]) {
  const orderIds = wave.order_ids ?? [];
  return orders
    .filter((order) => orderIds.includes(order.id))
    .reduce((sum, order) => sum + (order.lines ?? []).length, 0);
}

function formatDate(value: string | null | undefined) {
  return value?.slice(0, 10) || "-";
}
