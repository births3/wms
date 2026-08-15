/**
 * M2InboundOrderTable — 入库订单 DataGrid
 *
 * 层级：Layer 3 页面局部组件
 * 关联故事：US-M2-002, US-M2-003, US-M2-005, US-M2-008
 * Wave：Wave 6
 */

import * as React from "react";
import {
  DataGrid,
  StatusBadge,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridDetailAction,
  type DataGridPrintAction,
  type DataGridQuerySummaryItem,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
} from "@wms/ui";
import { CheckCircle2, ClipboardCheck, PackageCheck, Send } from "lucide-react";

import type { ReceivingOrder } from "@/features/inbound/inbound-queries";
import type { ReceivingOrderListRow } from "@/features/inbound/receiving-order-list-row";
import type { InboundDialog } from "./M2InboundDialogs";
import { inboundDocumentTypeLabel, inboundDocumentTypeOf } from "./m2-inbound-document-type";
import { contactLines, quantityLabel, receiptDetailsOf } from "./m2-inbound-receipt-fields";
import {
  canInspect,
  canPutaway,
  canRelease,
  canReceiveOrReject,
  formatDateTime,
  ownerLabel,
  statusColumnFilterOptions,
  statusKey,
  statusLabel,
  totalExpectedQty,
  workFieldHeader,
  workFieldText,
  type M2InboundQueryValue,
  type M2InboundMode,
  type OwnerContext,
} from "./m2-inbound-page-helpers";

interface M2InboundOrderTableProps {
  mode: M2InboundMode;
  currentOwner: OwnerContext;
  orders: ReceivingOrderListRow[];
  exportFileBaseName: string;
  selectedId: string | null;
  selectedRowKeys: string[];
  isPending: boolean;
  onSelectOrder: (id: string | null) => void;
  onSelectOrderKeys: (keys: string[]) => void;
  onOpenDetail: (id: string) => void;
  onOpenDialog: (id: string, dialog: InboundDialog) => void;
  onOpenPrint: (id: string) => void;
  onRelease: (id: string) => void;
  refreshAction?: DataGridRefreshAction;
  createAction?: DataGridCreateAction;
  queryState?: M2InboundQueryValue;
  querySummaryItems?: DataGridQuerySummaryItem[];
  onApplyQueryState?: (queryState: unknown) => void;
  onClearQueryState?: () => void;
}

export function buildInboundOrderColumns({
  mode,
  currentOwner,
  onOpenDetail,
}: {
  mode: M2InboundMode;
  currentOwner: OwnerContext;
  onOpenDetail: (id: string) => void;
}): DataGridColumn<ReceivingOrderListRow>[] {
  const isReceiving = mode === "receiving";

  const baseColumns: DataGridColumn<ReceivingOrderListRow>[] = [
    {
      key: "receipt_no",
      header: "ASN / 入库单",
      mono: true,
      width: 220,
      minWidth: 210,
      sortable: true,
      sortValue: (row) => row.receipt_no,
      filterValue: (row) => row.receipt_no,
      copyValue: (row) => row.receipt_no,
      filter: { type: "text" },
      onDoubleClick: (row) => onOpenDetail(row.id),
      render: (row) => <span className="text-primary font-medium">{row.receipt_no}</span>,
    },
    {
      key: "owner",
      header: "货主",
      width: 170,
      minWidth: 150,
      sortable: true,
      sortValue: (row) => ownerLabel(row.owner_id, currentOwner),
      filterValue: (row) => [row.owner_id, ownerLabel(row.owner_id, currentOwner)].join(" "),
      copyValue: (row) => ownerLabel(row.owner_id, currentOwner),
      filter: { type: "text" },
      render: (row) => ownerLabel(row.owner_id, currentOwner),
    },
    {
      key: "document_type",
      header: "单据类型",
      width: 140,
      minWidth: 130,
      sortable: true,
      sortValue: (row) => inboundDocumentTypeLabel(inboundDocumentTypeOf(row)),
      filterValue: (row) => inboundDocumentTypeOf(row),
      copyValue: (row) => inboundDocumentTypeLabel(inboundDocumentTypeOf(row)),
      filter: {
        type: "multiSelect",
        options: [
          { label: "采购入库", value: "purchase_inbound" },
          { label: "销售退货", value: "sales_return" },
        ],
      },
      render: (row) => inboundDocumentTypeLabel(inboundDocumentTypeOf(row)),
    },
    {
      key: "product_code",
      header: "商品编码",
      mono: true,
      width: 160,
      minWidth: 140,
      copyValue: (row) => row.lines?.[0]?.product_code ?? "-",
      filterValue: (row) => row.lines?.map((l) => l.product_code).join(" ") ?? "",
      filter: { type: "text" },
      render: (row) => <span className="font-mono text-xs">{row.lines?.[0]?.product_code ?? "-"}</span>,
    },
    {
      key: "expected_qty",
      header: "预报数量",
      width: 120,
      minWidth: 110,
      align: "right",
      sortable: true,
      sortValue: (row) => totalExpectedQty(row),
      filterValue: (row) => String(totalExpectedQty(row)),
      copyValue: (row) => String(totalExpectedQty(row)),
      render: (row) => <span className="font-mono font-medium">{totalExpectedQty(row)} 件</span>,
    },
  ];

  const receivingSpecificColumns: DataGridColumn<ReceivingOrderListRow>[] = isReceiving
    ? [
        ...receivingTextColumns(),
        {
          key: "contact_person",
          header: "送货人 / 电话 / 身份证",
          width: 190,
          minWidth: 160,
          copyValue: (row) => {
            const contact = contactLines(receiptDetailsOf(row).details);
            return [contact.name, contact.phone, contact.idNo].filter(Boolean).join(" / ") || "-";
          },
          render: (row) => {
            const contact = contactLines(receiptDetailsOf(row).details);
            return (
              <div className="text-xs space-y-0.5">
                <div className="font-medium">{contact.name ?? "-"}</div>
                {contact.phone ? <div className="text-muted-foreground font-mono">{contact.phone}</div> : null}
                {contact.idNo ? <div className="text-muted-foreground font-mono">{contact.idNo}</div> : null}
              </div>
            );
          },
        },
        {
          key: "seal_and_filing",
          header: "印章 / 备案件核对",
          width: 150,
          minWidth: 140,
          render: (row) => {
            const { details } = receiptDetailsOf(row);
            return (
              <div className="text-xs">
                <div>印章: {details?.seal_checked || "-"}</div>
                <div className="text-muted-foreground">备案件: {details?.filing_checked || "-"}</div>
              </div>
            );
          },
        },
        {
          key: "departure_arrival_time",
          header: "启运 / 到货时间",
          width: 180,
          minWidth: 160,
          render: (row) => {
            const { details } = receiptDetailsOf(row);
            return (
              <div className="text-xs font-mono">
                <div>启: {details?.departure_at ? formatDateTime(details.departure_at) : "-"}</div>
                <div className="text-muted-foreground">到: {details?.arrival_at ? formatDateTime(details.arrival_at) : "-"}</div>
              </div>
            );
          },
        },
        {
          key: "sales_return_summary",
          header: "销售退货批号 / 拒收明细",
          width: 220,
          minWidth: 180,
          render: (row) => {
            const { batches } = receiptDetailsOf(row);
            if (batches.length === 0) return <span className="text-muted-foreground text-xs">-</span>;
            return (
              <div className="text-xs space-y-0.5">
                {batches.slice(0, 2).map((batch, index) => (
                  <div key={`${batch.batch_no}-${index}`} className="font-mono">
                    {batch.batch_no} × {batch.quantity} 件
                    {Number(batch.rejected_qty) > 0 && <span className="text-destructive ml-1">(拒{batch.rejected_qty})</span>}
                  </div>
                ))}
                {batches.length > 2 && <div className="text-muted-foreground">等共 {batches.length} 批</div>}
              </div>
            );
          },
        },
      ]
    : [
        {
          key: "work_fields",
          header: workFieldHeader(mode),
          width: 380,
          minWidth: 320,
          filterValue: (row) => workFieldText(row, mode),
          copyValue: (row) => workFieldText(row, mode),
          filter: { type: "text" },
          render: (row) => <WorkFieldSummary order={row} mode={mode} />,
        },
      ];

  const tailColumns: DataGridColumn<ReceivingOrderListRow>[] = [
    {
      key: "expected_arrival_at",
      header: "预计到货",
      width: 170,
      minWidth: 160,
      sortable: true,
      sortValue: (row) => row.expected_arrival_at ?? "",
      filterValue: (row) => row.expected_arrival_at,
      copyValue: (row) => formatDateTime(row.expected_arrival_at),
      filter: { type: "dateRange" },
      render: (row) => formatDateTime(row.expected_arrival_at),
    },
    {
      key: "created_at",
      header: "创建时间",
      width: 170,
      minWidth: 160,
      sortable: true,
      sortValue: (row) => row.created_at,
      filterValue: (row) => row.created_at,
      copyValue: (row) => formatDateTime(row.created_at),
      filter: { type: "dateRange" },
      render: (row) => formatDateTime(row.created_at),
    },
    {
      key: "status",
      header: "状态",
      width: 160,
      minWidth: 140,
      sortable: true,
      sortValue: (row) => statusLabel(row.status),
      filterValue: (row) => row.status,
      copyValue: (row) => statusLabel(row.status),
      filter: {
        type: "multiSelect",
        options: statusColumnFilterOptions(mode),
      },
      render: (row) => row.status
        ? <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" />
        : <span className="text-muted-foreground">-</span>,
    },
  ];

  return [...baseColumns, ...receivingSpecificColumns, ...tailColumns];
}

export function M2InboundOrderTable({
  mode,
  currentOwner,
  orders,
  exportFileBaseName,
  selectedId,
  selectedRowKeys,
  isPending,
  onSelectOrder,
  onSelectOrderKeys,
  onOpenDetail,
  onOpenDialog,
  onOpenPrint,
  onRelease,
  refreshAction,
  createAction,
  queryState,
  querySummaryItems,
  onApplyQueryState,
  onClearQueryState,
}: M2InboundOrderTableProps) {
  const selectedOrder = orders.find((item) => item.id === selectedId) ?? null;
  const detailAction: DataGridDetailAction = {
    label: "详情",
    disabled: !selectedOrder,
    onClick: () => {
      if (selectedOrder) onOpenDetail(selectedOrder.id);
    },
  };
  const privateActions = inboundPrivateActions(mode, selectedOrder, onOpenDialog, onRelease);
  const printAction: DataGridPrintAction = {
    label: "打印",
    description: mode === "receiving" ? "打印 ASN 单" : "打印验收记录单",
    disabled: (context) => context.selectedRowKeys.length !== 1 || !selectedOrder,
    onClick: () => {
      if (selectedOrder) onOpenPrint(selectedOrder.id);
    },
  };
  const orderColumns = React.useMemo(
    () => buildInboundOrderColumns({ mode, currentOwner, onOpenDetail }),
    [mode, currentOwner, onOpenDetail],
  );

  return (
    <DataGrid
      columns={orderColumns}
      data={orders}
      rowKey={(row) => row.id}
      selectedKey={selectedId ?? undefined}
      onRowClick={(row) => onSelectOrder(row.id)}
      selectedRowKeys={selectedRowKeys}
      onSelectedRowKeysChange={onSelectOrderKeys}
      caption={isPending ? "加载入库单..." : undefined}
      emptyTitle="暂无入库单"
      storageKey={`m2-inbound-datagrid-${mode}`}
      exportFileBaseName={exportFileBaseName}
      tableClassName="w-full min-w-full"
      refreshAction={refreshAction}
      createAction={createAction}
      detailAction={detailAction}
      printAction={printAction}
      toolbarActions={privateActions}
      queryState={queryState}
      querySummaryItems={querySummaryItems}
      onApplyQueryState={onApplyQueryState}
      onClearQueryState={onClearQueryState}
      selectable
    />
  );
}

export function inboundPrivateActions(
  mode: M2InboundMode,
  selectedOrder: ReceivingOrder | null,
  onOpenDialog: (id: string, dialog: InboundDialog) => void,
  onRelease: (id: string) => void,
): DataGridToolbarAction[] {
  if (mode === "receiving") {
    return [
      {
        key: "release",
        label: "放行",
        description: "放行草稿 ASN",
        icon: <Send className="size-4" aria-hidden />,
        disabled: !selectedOrder || !canRelease(selectedOrder.status),
        onClick: () => {
          if (selectedOrder && canRelease(selectedOrder.status)) onRelease(selectedOrder.id);
        },
      },
      {
        key: "receive",
        label: "收货",
        description: "收货操作",
        icon: <CheckCircle2 className="size-4" aria-hidden />,
        disabled: !selectedOrder || !canReceiveOrReject(selectedOrder.status),
        onClick: () => {
          if (selectedOrder) onOpenDialog(selectedOrder.id, "receive");
        },
      },
    ];
  }

  if (mode === "inspecting") {
    return [
      {
        key: "inspect",
        label: selectedOrder?.status === "awaiting_second_sign" ? "第二签字" : "验收",
        description:
          selectedOrder?.status === "awaiting_second_sign"
            ? "第二人独立登录后完成签字"
            : "验收操作",
        icon: <ClipboardCheck className="size-4" aria-hidden />,
        disabled: !selectedOrder || !canInspect(selectedOrder.status),
        onClick: () => {
          if (selectedOrder && canInspect(selectedOrder.status)) {
            onOpenDialog(selectedOrder.id, "inspect");
          }
        },
      },
    ];
  }

  if (mode === "putaway") {
    return [
      {
        key: "putaway",
        label: "上架",
        description: "上架操作",
        icon: <PackageCheck className="size-4" aria-hidden />,
        disabled: !selectedOrder || !canPutaway(selectedOrder.status),
        onClick: () => {
          if (selectedOrder && canPutaway(selectedOrder.status)) {
            onOpenDialog(selectedOrder.id, "putaway");
          }
        },
      },
    ];
  }

  return [];
}

function receivingTextColumns(): DataGridColumn<ReceivingOrderListRow>[] {
  const columns: Array<{
    key: string;
    header: string;
    width: number;
    minWidth: number;
    align?: "right";
    mono?: boolean;
    filter?: true;
    value: (row: ReceivingOrderListRow) => string;
    className?: string;
  }> = [
    { key: "delivery_qty", header: "送货数量", width: 120, minWidth: 110, align: "right", value: (row) => quantityLabel(receiptDetailsOf(row).details?.delivery_qty) },
    { key: "actual_qty", header: "实际到货数量", width: 130, minWidth: 120, align: "right", className: "font-semibold text-emerald-600 dark:text-emerald-400", value: (row) => quantityLabel(receiptDetailsOf(row).receipt?.actual_qty) },
    { key: "shortage_qty", header: "缺货数量", width: 110, minWidth: 100, align: "right", value: (row) => quantityLabel(receiptDetailsOf(row).receipt?.shortage_qty) },
    { key: "rejected_qty", header: "拒收数量", width: 110, minWidth: 100, align: "right", value: (row) => quantityLabel(receiptDetailsOf(row).receipt?.rejected_qty) },
    { key: "arrival_temperature", header: "到货温度", width: 120, minWidth: 110, value: (row) => {
      const value = receiptDetailsOf(row).receipt?.arrival_temperature_celsius;
      return value != null ? `${value} °C` : "-";
    } },
    { key: "temperature_control_method", header: "温控方式", width: 120, minWidth: 110, value: (row) => receiptDetailsOf(row).details?.temperature_control_method || "-" },
    { key: "carrier", header: "承运商", width: 140, minWidth: 120, filter: true, value: (row) => receiptDetailsOf(row).details?.carrier || "-" },
    { key: "vehicle_no", header: "车牌号", width: 130, minWidth: 120, mono: true, filter: true, value: (row) => receiptDetailsOf(row).details?.vehicle_no || "-" },
    { key: "origin", header: "发运地点", width: 140, minWidth: 120, filter: true, value: (row) => receiptDetailsOf(row).details?.origin || "-" },
    { key: "transport_mode", header: "运输方式", width: 120, minWidth: 110, value: (row) => receiptDetailsOf(row).details?.transport_mode || "-" },
    { key: "storage_at", header: "收货入库时间", width: 170, minWidth: 150, value: (row) => {
      const value = receiptDetailsOf(row).details?.storage_at;
      return value ? formatDateTime(value) : "-";
    } },
    { key: "second_receiver_id", header: "第二收货员", width: 130, minWidth: 120, value: (row) => receiptDetailsOf(row).details?.second_receiver_id || "-" },
    { key: "exception_note", header: "拒收备注", width: 150, minWidth: 130, value: (row) => receiptDetailsOf(row).receipt?.exception_note || "-" },
  ];
  return columns.map((column) => ({
    key: column.key,
    header: column.header,
    width: column.width,
    minWidth: column.minWidth,
    align: column.align,
    mono: column.mono,
    filter: column.filter ? { type: "text" as const } : undefined,
    copyValue: column.value,
    filterValue: column.filter ? column.value : undefined,
    render: (row) => <span className={`text-xs ${column.mono ? "font-mono" : ""} ${column.className ?? ""}`}>{column.value(row)}</span>,
  }));
}

function WorkFieldSummary({ order, mode }: { order: ReceivingOrder; mode: M2InboundMode }) {
  const content = workFieldText(order, mode);
  return (
    <div className="text-sm">
      <div className="font-medium">{content[0]}</div>
      <div className="text-xs text-muted-foreground">{content[1]}</div>
    </div>
  );
}
