/**
 * M2InboundOrderTable — 入库订单 DataGrid
 *
 * 层级：Layer 3 页面局部组件
 * 关联故事：US-M2-002, US-M2-003, US-M2-005, US-M2-008
 * Wave：Wave 6
 */

import * as React from "react";
import { Button, DataGrid, StatusBadge, type DataGridColumn } from "@wms/ui";
import { CheckCircle2, ClipboardCheck, Eye, PackageCheck } from "lucide-react";

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
  onSelectOrder: (id: string) => void;
  onOpenDetail: (id: string) => void;
  onOpenDialog: (id: string, dialog: InboundDialog) => void;
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
}: M2InboundOrderTableProps) {
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
    {
      key: "actions",
      header: "操作",
      align: "right",
      width: 230,
      minWidth: 220,
      hideable: false,
      copyable: false,
      render: (row) => (
        <div className="flex justify-end gap-2">
          <RowButton
            icon={<Eye className="size-4" aria-hidden />}
            label="详情"
            onClick={() => onOpenDetail(row.id)}
          />
          {mode === "receiving" && canReceiveOrReject(row.status) && (
            <RowButton
              icon={<CheckCircle2 className="size-4" aria-hidden />}
              label="收货"
              onClick={() => onOpenDialog(row.id, "receive")}
            />
          )}
          {mode === "inspecting" && (
            <RowButton
              icon={<ClipboardCheck className="size-4" aria-hidden />}
              label="验收"
              onClick={() => onOpenDialog(row.id, "inspect")}
            />
          )}
          {mode === "putaway" && (
            <RowButton
              icon={<PackageCheck className="size-4" aria-hidden />}
              label="上架"
              onClick={() => onOpenDialog(row.id, "putaway")}
            />
          )}
        </div>
      ),
    },
  ];

  return (
    <DataGrid
      columns={orderColumns}
      data={orders}
      rowKey={(row) => row.id}
      selectedKey={selectedId ?? undefined}
      onRowClick={(row) => onSelectOrder(row.id)}
      caption={isPending ? "加载入库单..." : undefined}
      emptyTitle="暂无入库单"
      storageKey="m2-inbound-datagrid"
      tableClassName="min-w-[2070px]"
      selectable
    />
  );
}

function RowButton({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
    >
      {icon}
      {label}
    </Button>
  );
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
