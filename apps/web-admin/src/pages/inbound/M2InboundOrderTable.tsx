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
  type DataGridRefreshAction,
  type DataGridToolbarAction,
} from "@wms/ui";
import { CheckCircle2, ClipboardCheck, PackageCheck } from "lucide-react";

import type { ReceivingOrder } from "@/features/inbound/inbound-queries";
import type { InboundDialog } from "./M2InboundDialogs";
import { inboundDocumentTypeLabel, inboundDocumentTypeOf } from "./m2-inbound-document-type";
import {
  canReceiveOrReject,
  formatDateTime,
  ownerLabel,
  statusKey,
  statusLabel,
  totalExpectedQty,
  workFieldHeader,
  workFieldText,
  type M2InboundMode,
  type OwnerContext,
} from "./m2-inbound-page-helpers";

interface M2InboundOrderTableProps {
  mode: M2InboundMode;
  currentOwner: OwnerContext;
  orders: ReceivingOrder[];
  selectedId: string | null;
  isPending: boolean;
  onSelectOrder: (id: string | null) => void;
  onOpenDetail: (id: string) => void;
  onOpenDialog: (id: string, dialog: InboundDialog) => void;
  refreshAction?: DataGridRefreshAction;
  createAction?: DataGridCreateAction;
}

export function M2InboundOrderTable({
  mode,
  currentOwner,
  orders,
  selectedId,
  isPending,
  onSelectOrder,
  onOpenDetail,
  onOpenDialog,
  refreshAction,
  createAction,
}: M2InboundOrderTableProps) {
  const selectedOrder = orders.find((item) => item.id === selectedId) ?? null;
  const detailAction: DataGridDetailAction = {
    label: "详情",
    disabled: !selectedOrder,
    onClick: () => {
      if (selectedOrder) onOpenDetail(selectedOrder.id);
    },
  };
  const privateActions = inboundPrivateActions(mode, selectedOrder, onOpenDialog);
  const orderColumns: DataGridColumn<ReceivingOrder>[] = [
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
      render: (row) => <span className="text-primary">{row.receipt_no}</span>,
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
      width: 150,
      minWidth: 140,
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
      key: "product",
      header: "商品 / 数量",
      width: 200,
      minWidth: 180,
      copyValue: (row) => {
        const line = row.lines[0];
        return `${line?.product_code ?? "-"} ${totalExpectedQty(row)} 件`;
      },
      filterValue: (row) => {
        const line = row.lines[0];
        return [line?.product_code ?? "", line?.batch_no ?? "", totalExpectedQty(row)].join(" ");
      },
      filter: { type: "text" },
      render: (row) => (
        <div className="text-sm">
          <div className="font-medium">{row.lines[0]?.product_code ?? "-"}</div>
          <div className="text-xs text-muted-foreground">{totalExpectedQty(row)} 件</div>
        </div>
      ),
    },
    {
      key: "work_fields",
      header: workFieldHeader(mode),
      width: 440,
      minWidth: 360,
      filterValue: (row) => workFieldText(row, mode),
      copyValue: (row) => workFieldText(row, mode),
      filter: { type: "text" },
      render: (row) => <WorkFieldSummary order={row} mode={mode} />,
    },
    {
      key: "expected_arrival_at",
      header: "预计到货",
      width: 190,
      minWidth: 180,
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
      width: 190,
      minWidth: 180,
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
      width: 170,
      minWidth: 150,
      sortable: true,
      sortValue: (row) => statusLabel(row.status),
      filterValue: (row) => row.status,
      copyValue: (row) => statusLabel(row.status),
      filter: {
        type: "multiSelect",
        options: [
          { label: "待处理", value: "pending" },
          { label: "待收货", value: "released" },
          { label: "收货中", value: "receiving" },
          { label: "验收中", value: "inspecting" },
          { label: "上架中", value: "putaway" },
          { label: "已完成", value: "completed" },
          { label: "已关闭(拒收)", value: "closed_rejected" },
        ],
      },
      render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" />,
    },
  ];

  return (
    <DataGrid
      columns={orderColumns}
      data={orders}
      rowKey={(row) => row.id}
      selectedKey={selectedId ?? undefined}
      onRowClick={(row) => onSelectOrder(row.id)}
      selectedRowKeys={selectedId ? [selectedId] : []}
      onSelectedRowKeysChange={(keys) => onSelectOrder(keys.at(-1) ?? null)}
      caption={isPending ? "加载入库单..." : undefined}
      emptyTitle="暂无入库单"
      storageKey="m2-inbound-datagrid"
      tableClassName="min-w-[1840px]"
      refreshAction={refreshAction}
      createAction={createAction}
      detailAction={detailAction}
      toolbarActions={privateActions}
      selectable
    />
  );
}

function inboundPrivateActions(
  mode: M2InboundMode,
  selectedOrder: ReceivingOrder | null,
  onOpenDialog: (id: string, dialog: InboundDialog) => void,
): DataGridToolbarAction[] {
  if (mode === "receiving") {
    return [
      {
        key: "receive",
        label: "收货",
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
        label: "验收",
        icon: <ClipboardCheck className="size-4" aria-hidden />,
        disabled: !selectedOrder,
        onClick: () => {
          if (selectedOrder) onOpenDialog(selectedOrder.id, "inspect");
        },
      },
    ];
  }

  if (mode === "putaway") {
    return [
      {
        key: "putaway",
        label: "上架",
        icon: <PackageCheck className="size-4" aria-hidden />,
        disabled: !selectedOrder,
        onClick: () => {
          if (selectedOrder) onOpenDialog(selectedOrder.id, "putaway");
        },
      },
    ];
  }

  return [];
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
