import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  PageHeader,
  QueryPanel,
  StatusBadge,
  type DataGridColumn,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
} from "@wms/ui";

import {
  useReceivingDashboardQuery,
  useReceivingOrdersQuery,
  type ReceivingOrder,
  type ReceivingDashboard,
} from "@/features/inbound/inbound-queries";
import { M2InboundDetailDialog } from "./M2InboundDetailDialog";
import { localDayRange, statusKey, statusLabel, type OwnerContext } from "./m2-inbound-page-helpers";
import type { InboundDetailStage } from "./m2-inbound-detail-view-model";
import { queryString } from "@/lib/query-value";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

const dashboardQueryFields: QueryPanelField[] = [
  { key: "supplierId", label: "供应商 ID", type: "text", placeholder: "可选 UUID" },
  { key: "productCode", label: "商品编码", type: "text", placeholder: "可选商品编码" },
  { key: "arrivalRange", label: "预计到货时间", type: "dateRange" },
];

const dashboardCoreQueryFieldKeys = ["supplierId", "productCode"];

const refreshOptions = [
  { value: "0", label: "关闭" },
  { value: "15", label: "15 秒" },
  { value: "30", label: "30 秒" },
  { value: "60", label: "60 秒" },
] as const;

function defaultQuery(): QueryPanelValue {
  const today = localDayRange();
  return {
    supplierId: "",
    productCode: "",
    arrivalRange: today,
  };
}

interface M2InboundDashboardPageProps {
  currentOwner: OwnerContext;
  onBack: () => void;
}

export function M2InboundDashboardPage({ currentOwner, onBack }: M2InboundDashboardPageProps) {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery);
  const [refreshSeconds, setRefreshSeconds] = React.useState("30");
  const statusDialog = useDialogState<string>();
  const detailDialog = useDialogState<ReceivingOrder>();
  const dashboardFilters = React.useMemo(() => toDashboardQuery(appliedQuery), [appliedQuery]);
  const dashboardQuery = useReceivingDashboardQuery(
    dashboardFilters,
    refreshSeconds === "0" ? false : Number(refreshSeconds) * 1000,
  );
  const ordersQuery = useReceivingOrdersQuery();
  const selectedStatus = statusDialog.open ? statusDialog.target : null;
  const statusOrders = React.useMemo(
    () => ordersQuery.data?.filter((order) => order.status === selectedStatus && matchesDashboardFilters(order, dashboardFilters)) ?? [],
    [dashboardFilters, ordersQuery.data, selectedStatus],
  );

  const columns = React.useMemo<DataGridColumn<ReceivingDashboard["data"][number]>[]>(
    () => [
      { key: "created_at", header: "创建时间", width: 190, copyValue: (row) => row.created_at },
      {
        key: "status",
        header: "状态",
        width: 180,
        copyValue: (row) => statusLabel(row.status),
        render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" />,
      },
      { key: "order_count", header: "单据数", width: 120, align: "right" },
      { key: "expected_qty", header: "预报数量", width: 140, align: "right" },
      {
        key: "abnormal",
        header: "异常",
        width: 100,
        render: (row) => row.abnormal
          ? <span className="font-medium text-destructive">需关注</span>
          : <span className="text-muted-foreground">正常</span>,
      },
    ],
    [],
  );

  function openStatusOrders(row: ReceivingDashboard["data"][number]) {
    statusDialog.openWith(row.status);
  }

  function openOrderDetail(order: ReceivingOrder) {
    statusDialog.close();
    detailDialog.openWith(order);
  }

  return (
    <section className="space-y-4 p-6">
      <PageHeader
        title="M2 入库进度看板"
        subtitle="US-M2-008 · 真实接口只读聚合，异常状态高亮"
        actions={
          <div className="flex items-center gap-2">
            <label className="flex items-center gap-2 text-xs text-muted-foreground">
              刷新间隔
              <select
                aria-label="刷新间隔"
                className="h-9 rounded-md border border-input bg-background px-2 text-sm text-foreground"
                value={refreshSeconds}
                onChange={(event) => setRefreshSeconds(event.target.value)}
              >
                {refreshOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
              </select>
            </label>
            <Button variant="outline" onClick={onBack}>返回</Button>
          </div>
        }
      />
      <QueryPanel
        fields={dashboardQueryFields}
        defaultVisibleFieldKeys={dashboardCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => applyQuery(draftQuery)}
        onReset={resetQuery}
      />
      {dashboardQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {dashboardQuery.error.message}
        </div>
      )}
      <DataGrid
        columns={columns}
        data={dashboardQuery.data?.data ?? []}
        rowKey={(row) => row.status}
        caption={dashboardQuery.isPending ? "正在读取入库进度..." : `刷新于 ${dashboardQuery.data?.refreshed_at ?? "—"}`}
        emptyTitle="暂无入库进度"
        emptyDescription="调整筛选条件后重试。"
        storageKey="m2-inbound-dashboard-datagrid"
        exportFileBaseName="M2 入库进度看板"
        onRowClick={openStatusOrders}
        tableClassName="min-w-[720px]"
      />

      <Dialog open={statusDialog.open} onOpenChange={statusDialog.setOpen}>
        <DialogContent className="max-h-[80vh] overflow-y-auto sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>状态单据</DialogTitle>
            <DialogDescription>{selectedStatus ? statusLabel(selectedStatus) : "-"} · 点击单号查看详情</DialogDescription>
          </DialogHeader>
          <div className="grid gap-2">
            {ordersQuery.isPending && <p className="text-sm text-muted-foreground">正在读取单据...</p>}
            {!ordersQuery.isPending && statusOrders.length === 0 && (
              <p className="text-sm text-muted-foreground">当前筛选范围没有可查看的单据。</p>
            )}
            {statusOrders.map((order) => (
              <button
                key={order.id}
                type="button"
                className="flex items-center justify-between rounded-md border px-3 py-2 text-left text-sm hover:bg-muted"
                onClick={() => openOrderDetail(order)}
              >
                <span className="font-medium text-primary">{order.receipt_no}</span>
                <span className="text-muted-foreground">{order.lines?.length ?? 0} 行 · {statusLabel(order.status)}</span>
              </button>
            ))}
          </div>
        </DialogContent>
      </Dialog>

      <M2InboundDetailDialog
        order={detailDialog.target}
        currentOwner={currentOwner}
        defaultStage={detailStageForStatus(detailDialog.target?.status)}
        open={detailDialog.open}
        onOpenChange={detailDialog.setOpen}
      />
    </section>
  );
}

function detailStageForStatus(status: string | null | undefined): InboundDetailStage {
  if (status === "completed") return "completed";
  if (status?.includes("putaway")) return "putaway";
  if (status?.includes("inspect")) return "inspection";
  return "receiving";
}

function toDashboardQuery(value: QueryPanelValue) {
  const arrivalRange = value.arrivalRange as QueryPanelRangeValue | undefined;
  return {
    supplierId: queryString(value.supplierId),
    productCode: queryString(value.productCode),
    from: arrivalRange?.from ?? "",
    to: arrivalRange?.to ?? "",
  };
}

function matchesDashboardFilters(
  order: ReceivingOrder,
  filters: ReturnType<typeof toDashboardQuery>,
) {
  if (filters.supplierId && order.supplier_id !== filters.supplierId) return false;
  if (filters.productCode && !(order.lines ?? []).some((line) => line.product_code === filters.productCode)) {
    return false;
  }
  // 日期区间按日期字符串比较（同 m4 dateInRange）：Date.parse 会把 "YYYY-MM-DD" 当 UTC 零点，
  // 导致 to 边界当天晚些到货的单据被错误排除。
  const arrivalDate = order.expected_arrival_at?.slice(0, 10) ?? "";
  if (filters.from && (!arrivalDate || arrivalDate < filters.from.slice(0, 10))) return false;
  if (filters.to && (!arrivalDate || arrivalDate > filters.to.slice(0, 10))) return false;
  return true;
}

/* 看板聚合查询负责统计和刷新，单据查询复用已有详情弹窗，避免扩展看板 API 契约。 */
