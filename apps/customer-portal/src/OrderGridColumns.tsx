import { Button, StatusBadge, type DataGridColumn } from "@wms/ui";

import type { OrderSummary } from "./types";

export function buildOrderGridColumns(
  onOpen: (orderId: string) => void,
): DataGridColumn<OrderSummary>[] {
  return [
    {
      key: "order_no",
      header: "订单号",
      width: 190,
      minWidth: 160,
      mono: true,
      sortable: true,
      sortValue: (order) => order.order_no,
      filterValue: (order) => order.order_no,
      filter: { type: "text" },
    },
    {
      key: "customer_code",
      header: "客户编码",
      width: 130,
      minWidth: 110,
      sortable: true,
      sortValue: (order) => order.customer_code,
      filterValue: (order) => order.customer_code,
      filter: { type: "text" },
    },
    {
      key: "customer_name",
      header: "客户名称",
      width: 180,
      minWidth: 140,
      sortable: true,
      sortValue: (order) => order.customer_name,
      filterValue: (order) => order.customer_name,
      filter: { type: "text" },
    },
    {
      key: "address_code",
      header: "地址编码",
      width: 120,
      minWidth: 100,
      sortable: true,
      sortValue: (order) => order.address_code,
      filterValue: (order) => order.address_code,
      filter: { type: "text" },
    },
    {
      key: "address_name",
      header: "送货地址",
      width: 200,
      minWidth: 160,
      sortable: true,
      sortValue: (order) => order.address_name,
      filterValue: (order) => order.address_name,
      filter: { type: "text" },
    },
    {
      key: "product_codes",
      header: "商品编码",
      width: 160,
      minWidth: 130,
      render: (order) => order.product_codes.join("、"),
      filterValue: (order) => order.product_codes.join("、"),
      copyValue: (order) => order.product_codes.join("、"),
      filter: { type: "text" },
    },
    {
      key: "product_names",
      header: "商品名称",
      width: 220,
      minWidth: 170,
      render: (order) => order.product_names.join("、"),
      filterValue: (order) => order.product_names.join("、"),
      copyValue: (order) => order.product_names.join("、"),
      filter: { type: "text" },
    },
    {
      key: "batch_nos",
      header: "批号",
      width: 150,
      minWidth: 120,
      render: (order) => order.batch_nos.join("、"),
      filterValue: (order) => order.batch_nos.join("、"),
      copyValue: (order) => order.batch_nos.join("、"),
      filter: { type: "text" },
    },
    {
      key: "quantities",
      header: "数量",
      width: 110,
      minWidth: 90,
      align: "right",
      render: (order) => order.quantities.join("、"),
      copyValue: (order) => order.quantities.join("、"),
    },
    {
      key: "status",
      header: "状态",
      width: 110,
      minWidth: 100,
      render: (order) => (
        <StatusBadge
          status={order.status === "signed" ? "completed" : "in_progress"}
          size="sm"
          label={order.status === "signed" ? "已签收" : "已发货"}
        />
      ),
      sortValue: (order) => order.status,
      filterValue: (order) => order.status,
      filter: {
        type: "select",
        options: [
          { label: "已发货", value: "shipped" },
          { label: "已签收", value: "signed" },
        ],
      },
    },
    {
      key: "shipped_at",
      header: "发货时间",
      width: 180,
      minWidth: 160,
      sortable: true,
      sortValue: (order) => order.shipped_at,
      render: (order) => formatPortalTime(order.shipped_at),
      filterValue: (order) => order.shipped_at,
      filter: { type: "dateRange" },
    },
    {
      key: "signed_at",
      header: "签收时间",
      width: 180,
      minWidth: 160,
      sortable: true,
      sortValue: (order) => order.signed_at ?? "",
      render: (order) => order.signed_at ? formatPortalTime(order.signed_at) : "未签收",
      filterValue: (order) => order.signed_at ?? "",
      filter: { type: "dateRange" },
    },
    {
      key: "report_status",
      header: "资料状态",
      width: 190,
      minWidth: 160,
      copyable: false,
      render: (order) => (
        <div className="flex flex-wrap gap-2">
          {order.available_report_count ? (
            <span className="text-xs font-medium text-emerald-700">
              {order.available_report_count} 份可下载
            </span>
          ) : null}
          {order.pending_report_count ? (
            <span className="text-xs font-medium text-amber-700">
              {order.pending_report_count} 项暂缺/处理中
            </span>
          ) : null}
        </div>
      ),
    },
    {
      key: "actions",
      header: "操作",
      width: 120,
      minWidth: 110,
      hideable: false,
      copyable: false,
      render: (order) => (
        <Button type="button" variant="outline" size="sm" onClick={() => onOpen(order.id)}>
          查看资料
        </Button>
      ),
    },
  ];
}

export function formatPortalTime(value: string) {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}
