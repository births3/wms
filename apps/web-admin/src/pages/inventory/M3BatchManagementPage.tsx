/**
 * M3BatchManagementPage — 库内业务批号列表切片
 *
 * 层级：Layer 3 页面
 * 关联故事：US-M3-001, US-M3-002, US-BA-004
 * Wave：Wave 6
 */

import * as React from "react";
import {
  DataGrid,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridDetailAction,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
  type StatusKey,
} from "@wms/ui";

import { useInventoryBatchesQuery, type InventoryBatch } from "@/features/inventory/inventory-queries";

interface M3BatchManagementPageProps {
  onBack: () => void;
}

/** 近效期阈值：有效期在 90 天内（含）视为近效期 */
const NEAR_EXPIRY_DAYS = 90;

const qualityStatusLabels: Record<string, string> = {
  qualified: "合格",
  quarantined: "隔离",
  quarantine: "隔离",
  unqualified: "不合格",
  pending_destruction: "待销毁",
  loss_deducted: "报损扣减",
};

const qualityStatusOptions = [
  { label: "合格", value: "qualified" },
  { label: "隔离", value: "quarantined" },
  { label: "不合格", value: "unqualified" },
  { label: "待销毁", value: "pending_destruction" },
  { label: "报损扣减", value: "loss_deducted" },
];

const m3BatchQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: "关键字",
    type: "text",
    placeholder: "批号 / 商品编码 / 库位",
  },
  {
    key: "qualityStatus",
    label: "质量状态",
    type: "multiSelect",
    options: qualityStatusOptions,
  },
  {
    key: "recallFlag",
    label: "召回",
    type: "multiSelect",
    options: [
      { label: "已标记", value: "true" },
      { label: "未标记", value: "false" },
    ],
  },
  { key: "productionDate", label: "生产日期", type: "dateRange" },
  { key: "expiryDate", label: "有效期", type: "dateRange" },
  { key: "createdAt", label: "创建时间", type: "dateRange" },
];
const m3BatchCoreQueryFieldKeys = ["keyword", "qualityStatus"];

export function M3BatchManagementPage({}: M3BatchManagementPageProps) {
  const batchesQuery = useInventoryBatchesQuery();
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultM3BatchQueryValue());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultM3BatchQueryValue());
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [detailOpen, setDetailOpen] = React.useState(false);
  const [detailBatch, setDetailBatch] = React.useState<InventoryBatch | null>(null);
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const batches = React.useMemo(
    () => filterBatches(batchesQuery.data ?? [], normalizeM3BatchQueryValue(appliedQuery)),
    [batchesQuery.data, appliedQuery],
  );
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(m3BatchQueryFields, appliedQuery),
    [appliedQuery],
  );

  function openBatchDetail(id: string) {
    const batch = (batchesQuery.data ?? []).find((item) => item.id === id) ?? batches.find((item) => item.id === id);
    if (!batch) return;
    setSelectedId(id);
    setDetailBatch(batch);
    setDetailOpen(true);
  }

  async function refreshBatches() {
    const result = await batchesQuery.refetch();
    setLastEvent(result.error ? null : "批号列表已刷新");
  }

  const gridRefreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新批号列表",
    disabled: batchesQuery.isFetching,
    onClick: () => {
      void refreshBatches();
    },
  };

  const gridDetailAction: DataGridDetailAction = {
    label: "详情",
    description: "查看选中批号详情",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1,
    onClick: ({ selectedRowKeys }) => {
      const id = selectedRowKeys[0];
      if (id) openBatchDetail(id);
    },
  };

  const columns = buildBatchColumns(openBatchDetail);

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="M3 批号管理"
        subtitle="库存批次、效期、质量状态与库位分布"
        actions={
          <div className="flex flex-wrap gap-2">
            {lastEvent && (
              <span className="self-center text-sm text-muted-foreground" role="status">
                {lastEvent}
              </span>
            )}
          </div>
        }
      />

      <QueryPanel
        fields={m3BatchQueryFields}
        defaultVisibleFieldKeys={m3BatchCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeM3BatchQueryValue(next))}
        onQuery={() => {
          setAppliedQuery(normalizeM3BatchQueryValue(draftQuery));
          setLastEvent("批号列表已查询");
        }}
        onReset={() => {
          const next = defaultM3BatchQueryValue();
          setDraftQuery(next);
          setAppliedQuery(next);
          setSelectedId(null);
        }}
      />

      {batchesQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {batchesQuery.error.message}
        </div>
      )}

      <DataGrid
        columns={columns}
        data={batches}
        rowKey={(row) => row.id}
        caption={batchesQuery.isPending ? "加载库存批次..." : undefined}
        emptyTitle="暂无库存批次"
        emptyDescription="调整筛选条件后重试，或等待库存同步完成"
        storageKey="m3-batches-datagrid"
        exportFileBaseName="M3 批号管理"
        refreshAction={gridRefreshAction}
        detailAction={gridDetailAction}
        selectedKey={selectedId ?? undefined}
        selectedRowKeys={selectedId ? [selectedId] : []}
        onSelectedRowKeysChange={(keys) => setSelectedId(keys.at(-1) ?? null)}
        onRowClick={(row) => setSelectedId(row.id)}
        selectable
        tableClassName="min-w-[1670px]"
        queryState={appliedQuery}
        querySummaryItems={querySummaryItems}
        onApplyQueryState={(queryState) => {
          const next = normalizeM3BatchQueryValue(queryValueFromUnknown(queryState));
          setDraftQuery(next);
          setAppliedQuery(next);
          setSelectedId(null);
        }}
        onClearQueryState={() => {
          const next = defaultM3BatchQueryValue();
          setDraftQuery(next);
          setAppliedQuery(next);
          setSelectedId(null);
        }}
      />

      <M3BatchDetailDialog
        batch={detailBatch}
        open={detailOpen}
        onOpenChange={(open) => {
          setDetailOpen(open);
          if (!open) setDetailBatch(null);
        }}
      />
    </section>
  );
}

function buildBatchColumns(onOpenDetail: (id: string) => void): DataGridColumn<InventoryBatch>[] {
  return [
    {
      key: "batch_no",
      header: "批号",
      mono: true,
      width: 190,
      minWidth: 170,
      sortable: true,
      sortValue: (row) => row.batch_no,
      filterValue: (row) => row.batch_no,
      copyValue: (row) => row.batch_no,
      filter: { type: "text" },
      onDoubleClick: (row) => onOpenDetail(row.id),
      render: (row) => <span className="text-primary">{row.batch_no}</span>,
    },
    {
      key: "product_code",
      header: "商品编码",
      mono: true,
      width: 170,
      minWidth: 150,
      sortable: true,
      sortValue: (row) => row.product_code,
      filterValue: (row) => row.product_code,
      copyValue: (row) => row.product_code,
      filter: { type: "text" },
    },
    {
      key: "location_code",
      header: "库位",
      mono: true,
      width: 150,
      minWidth: 130,
      sortable: true,
      sortValue: (row) => row.location_code,
      filterValue: (row) => row.location_code,
      copyValue: (row) => row.location_code,
      filter: { type: "text" },
    },
    {
      key: "quantity",
      header: "数量",
      width: 210,
      minWidth: 190,
      sortable: true,
      sortValue: (row) => availableQty(row),
      filterValue: (row) => availableQty(row),
      copyValue: (row) => `现存 ${row.qty_on_hand} / 锁定 ${row.qty_locked} / 可用 ${availableQty(row)}`,
      filter: { type: "numberRange" },
      render: (row) => (
        <div className="text-sm">
          <div className="font-medium">{row.qty_on_hand} 件</div>
          <div className="text-xs text-muted-foreground">锁定 {row.qty_locked} / 可用 {availableQty(row)}</div>
        </div>
      ),
    },
    {
      key: "quality_status",
      header: "质量状态",
      width: 150,
      minWidth: 130,
      sortable: true,
      sortValue: (row) => qualityStatusLabel(row.quality_status),
      filterValue: (row) => row.quality_status,
      copyValue: (row) => qualityStatusLabel(row.quality_status),
      filter: {
        type: "multiSelect",
        options: qualityStatusOptions,
      },
      render: (row) => (
        <StatusBadge status={qualityStatusKey(row.quality_status, row.recall_flag)} label={qualityStatusLabel(row.quality_status)} size="sm" />
      ),
    },
    {
      key: "recall_flag",
      header: "召回",
      width: 120,
      minWidth: 110,
      sortable: true,
      sortValue: (row) => (row.recall_flag ? 1 : 0),
      filterValue: (row) => (row.recall_flag ? "true" : "false"),
      copyValue: (row) => (row.recall_flag ? "已标记" : "未标记"),
      filter: {
        type: "multiSelect",
        options: [
          { label: "已标记", value: "true" },
          { label: "未标记", value: "false" },
        ],
      },
      render: (row) => row.recall_flag ? <StatusBadge status="isolated" label="已标记" size="sm" /> : <span className="text-muted-foreground">未标记</span>,
    },
    {
      key: "production_date",
      header: "生产日期",
      width: 150,
      minWidth: 130,
      sortable: true,
      sortValue: (row) => row.production_date,
      filterValue: (row) => row.production_date,
      copyValue: (row) => row.production_date,
      filter: { type: "dateRange" },
    },
    {
      key: "expiry_date",
      header: "有效期",
      width: 170,
      minWidth: 150,
      sortable: true,
      sortValue: (row) => row.expiry_date,
      filterValue: (row) => row.expiry_date,
      copyValue: (row) => expiryCopyValue(row.expiry_date),
      filter: { type: "dateRange" },
      render: (row) => <ExpiryDateCell expiryDate={row.expiry_date} />,
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
      key: "updated_at",
      header: "更新时间",
      width: 190,
      minWidth: 180,
      sortable: true,
      sortValue: (row) => row.updated_at,
      filterValue: (row) => row.updated_at,
      copyValue: (row) => formatDateTime(row.updated_at),
      filter: { type: "dateRange" },
      render: (row) => formatDateTime(row.updated_at),
    },
  ];
}

function M3BatchDetailDialog({
  batch,
  open,
  onOpenChange,
}: {
  batch: InventoryBatch | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  if (!batch) return null;

  const available = availableQty(batch);
  const identityRows: Array<[string, string]> = [
    ["批号", batch.batch_no],
    ["商品编码", batch.product_code],
    ["库位", batch.location_code],
  ];
  const quantityRows: Array<[string, string]> = [
    ["现存", `${batch.qty_on_hand} 件`],
    ["锁定", `${batch.qty_locked} 件`],
    ["可用", `${available} 件`],
  ];
  const qualityRows: Array<[string, React.ReactNode]> = [
    ["质量状态", <StatusBadge key="qs" status={qualityStatusKey(batch.quality_status, batch.recall_flag)} label={qualityStatusLabel(batch.quality_status)} size="sm" />],
    ["召回", batch.recall_flag ? <StatusBadge key="rf" status="isolated" label="已标记" size="sm" /> : "未标记"],
  ];
  const dateRows: Array<[string, React.ReactNode]> = [
    ["生产日期", batch.production_date || "-"],
    ["有效期", <ExpiryDateCell key="exp" expiryDate={batch.expiry_date} />],
    ["创建时间", formatDateTime(batch.created_at)],
    ["更新时间", formatDateTime(batch.updated_at)],
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>批号详情</DialogTitle>
          <DialogDescription>
            {batch.batch_no} · {batch.product_code} · {batch.location_code}
          </DialogDescription>
        </DialogHeader>

        <DetailSection title="批号 / 商品 / 库位">
          <DetailOverview rows={identityRows} />
        </DetailSection>

        <DetailSection title="数量">
          <DetailOverview rows={quantityRows} />
        </DetailSection>

        <DetailSection title="质量状态 / 召回">
          <DetailOverviewNode rows={qualityRows} />
        </DetailSection>

        <DetailSection title="效期与时间">
          <DetailOverviewNode rows={dateRows} />
        </DetailSection>
      </DialogContent>
    </Dialog>
  );
}

function DetailSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="grid gap-2">
      <div className="text-sm font-semibold">{title}</div>
      {children}
    </section>
  );
}

function DetailOverview({ rows }: { rows: Array<[string, string]> }) {
  return (
    <section className="rounded-md border bg-muted/20">
      <div className="grid divide-y sm:grid-cols-3 sm:divide-x sm:divide-y-0">
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

function DetailOverviewNode({ rows }: { rows: Array<[string, React.ReactNode]> }) {
  return (
    <section className="rounded-md border bg-muted/20">
      <div className="grid divide-y sm:grid-cols-2 lg:grid-cols-4 sm:divide-x sm:divide-y-0">
        {rows.map(([label, value]) => (
          <div key={label} className="px-4 py-3">
            <div className="text-xs text-muted-foreground">{label}</div>
            <div className="mt-1 text-sm font-semibold">{value}</div>
          </div>
        ))}
      </div>
    </section>
  );
}

function ExpiryDateCell({ expiryDate }: { expiryDate: string }) {
  const tone = expiryTone(expiryDate);
  if (tone === "normal") {
    return <span>{expiryDate || "-"}</span>;
  }
  if (tone === "expired") {
    return (
      <div className="text-sm">
        <div className="font-medium text-destructive">{expiryDate || "-"}</div>
        <div className="text-xs text-destructive">已过期</div>
      </div>
    );
  }
  return (
    <div className="text-sm">
      <div className="font-medium text-wms-warning">{expiryDate || "-"}</div>
      <div className="text-xs text-wms-warning">近效期</div>
    </div>
  );
}

function expiryTone(expiryDate: string): "expired" | "near" | "normal" {
  const days = daysUntilExpiry(expiryDate);
  if (days === null) return "normal";
  if (days < 0) return "expired";
  if (days <= NEAR_EXPIRY_DAYS) return "near";
  return "normal";
}

function daysUntilExpiry(expiryDate: string): number | null {
  const datePart = expiryDate.slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(datePart)) return null;
  const expiry = new Date(`${datePart}T00:00:00`);
  if (Number.isNaN(expiry.getTime())) return null;
  const today = new Date();
  const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  return Math.floor((expiry.getTime() - startOfToday.getTime()) / 86_400_000);
}

function expiryCopyValue(expiryDate: string) {
  const tone = expiryTone(expiryDate);
  if (tone === "expired") return `${expiryDate} 已过期`;
  if (tone === "near") return `${expiryDate} 近效期`;
  return expiryDate;
}

function defaultM3BatchQueryValue(): QueryPanelValue {
  return {
    keyword: "",
    qualityStatus: [],
    recallFlag: [],
    productionDate: { from: "", to: "" },
    expiryDate: { from: "", to: "" },
    createdAt: { from: "", to: "" },
  };
}

function normalizeM3BatchQueryValue(value: QueryPanelValue): QueryPanelValue {
  return {
    keyword: queryString(value.keyword),
    qualityStatus: queryStringArray(value.qualityStatus),
    recallFlag: queryStringArray(value.recallFlag),
    productionDate: queryRange(value.productionDate),
    expiryDate: queryRange(value.expiryDate),
    createdAt: queryRange(value.createdAt),
  };
}

function filterBatches(batches: InventoryBatch[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword).trim().toLowerCase();
  const qualityStatuses = new Set(queryStringArray(query.qualityStatus));
  const recallFlags = new Set(queryStringArray(query.recallFlag));
  return batches.filter((batch) => {
    const searchable = [batch.batch_no, batch.product_code, batch.location_code].join(" ").toLowerCase();
    return (
      (!keyword || searchable.includes(keyword)) &&
      (qualityStatuses.size === 0 || qualityStatuses.has(batch.quality_status)) &&
      (recallFlags.size === 0 || recallFlags.has(batch.recall_flag ? "true" : "false")) &&
      dateInRange(batch.production_date, queryRange(query.productionDate)) &&
      dateInRange(batch.expiry_date, queryRange(query.expiryDate)) &&
      dateInRange(batch.created_at, queryRange(query.createdAt))
    );
  });
}

function availableQty(batch: InventoryBatch) {
  if (batch.quality_status !== "qualified" || batch.recall_flag) return 0;
  return batch.qty_on_hand - batch.qty_locked;
}

function qualityStatusLabel(status: string) {
  return qualityStatusLabels[status] ?? status;
}

function qualityStatusKey(status: string, recalled: boolean): StatusKey {
  if (recalled) return "isolated";
  if (status === "qualified") return "qualified";
  if (status === "quarantined" || status === "quarantine") return "isolated";
  if (status === "unqualified" || status === "pending_destruction" || status === "loss_deducted") return "unqualified";
  return "pending";
}

function queryString(value: QueryPanelValue[string]) {
  return typeof value === "string" ? value : "";
}

function queryStringArray(value: QueryPanelValue[string]) {
  return Array.isArray(value) ? value.filter((item) => typeof item === "string") : [];
}

function queryRange(value: QueryPanelValue[string]): QueryPanelRangeValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) return { from: "", to: "" };
  return {
    from: typeof value.from === "string" ? value.from : "",
    to: typeof value.to === "string" ? value.to : "",
  };
}

function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as QueryPanelValue) : {};
}

function dateInRange(value: string, range: QueryPanelRangeValue) {
  const date = value.slice(0, 10);
  return (!range.from || date >= range.from) && (!range.to || date <= range.to);
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}
