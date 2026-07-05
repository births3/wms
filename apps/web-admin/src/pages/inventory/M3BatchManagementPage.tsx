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
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
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

const columns: DataGridColumn<InventoryBatch>[] = [
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
    width: 150,
    minWidth: 130,
    sortable: true,
    sortValue: (row) => row.expiry_date,
    filterValue: (row) => row.expiry_date,
    copyValue: (row) => row.expiry_date,
    filter: { type: "dateRange" },
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

export function M3BatchManagementPage({}: M3BatchManagementPageProps) {
  const batchesQuery = useInventoryBatchesQuery();
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultM3BatchQueryValue());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultM3BatchQueryValue());
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const batches = React.useMemo(
    () => filterBatches(batchesQuery.data ?? [], normalizeM3BatchQueryValue(appliedQuery)),
    [batchesQuery.data, appliedQuery],
  );
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(m3BatchQueryFields, appliedQuery),
    [appliedQuery],
  );

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
        storageKey="m3-batches-datagrid"
        exportFileBaseName="M3 批号管理"
        refreshAction={gridRefreshAction}
        tableClassName="min-w-[1670px]"
        queryState={appliedQuery}
        querySummaryItems={querySummaryItems}
        onApplyQueryState={(queryState) => {
          const next = normalizeM3BatchQueryValue(queryValueFromUnknown(queryState));
          setDraftQuery(next);
          setAppliedQuery(next);
        }}
        onClearQueryState={() => {
          const next = defaultM3BatchQueryValue();
          setDraftQuery(next);
          setAppliedQuery(next);
        }}
      />
    </section>
  );
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
