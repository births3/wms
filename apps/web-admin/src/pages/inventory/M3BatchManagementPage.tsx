/**
 * M3BatchManagementPage — 库内业务批号列表切片
 *
 * 层级：Layer 3 页面
 * 关联故事：US-M3-001, US-M3-002, US-BA-004
 * Wave：Wave 6
 */

import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridDetailAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
} from "@wms/ui";

import {
  useChangeInventoryStatusMutation,
  useCancelInventoryRecallMutation,
  useMarkInventoryRecallMutation,
  useInventoryExpiryPolicyQuery,
  useInventoryBatchesQuery,
  type InventoryBatch,
  type InventoryBatchQuery,
} from "@/features/inventory/inventory-queries";
import type { CancelInventoryRecallRequest, MarkInventoryRecallRequest } from "@/features/inventory/inventory-queries";
import { useSystemDictionaryItemOptionsQuery } from "@/features/master-data/master-data-queries";
import { M3BatchDetailDialog } from "./M3BatchDetailDialog";
import { M3BatchRecallDialog } from "./M3BatchRecallDialog";
import { M3BatchRecallCancelDialog } from "./M3BatchRecallCancelDialog";
import {
  availableQty,
  expiryCopyValue,
  ExpiryDateCell,
  expiryTone,
  formatDateTime,
  qualityStatusKey,
  qualityStatusLabel,
  type QualityStatusOption,
} from "./M3BatchViewHelpers";

interface M3BatchManagementPageProps {
  onBack: () => void;
}

/** 近效期阈值：有效期在 90 天内（含）视为近效期 */
const DEFAULT_NEAR_EXPIRY_DAYS = 180;

const m3BatchCoreQueryFieldKeys = ["keyword", "qualityStatus"];

function buildM3BatchQueryFields(qualityStatusOptions: QualityStatusOption[]): QueryPanelField[] {
  return [
    {
      key: "keyword",
      label: "关键字",
      type: "text",
      placeholder: "批号 / 商品编码 / 库位",
    },
    {
      key: "productCode",
      label: "商品编码",
      type: "text",
      placeholder: "按商品编码模糊查询",
    },
    {
      key: "batchNo",
      label: "批号",
      type: "text",
      placeholder: "按批号模糊查询",
    },
    {
      key: "locationCode",
      label: "库位",
      type: "text",
      placeholder: "按库位编码模糊查询",
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
    {
      key: "expiryRisk",
      label: "效期风险",
      type: "multiSelect",
      options: [
        { label: "近效期", value: "near" },
        { label: "已过期", value: "expired" },
      ],
    },
    { key: "productionDate", label: "生产日期", type: "dateRange" },
    { key: "expiryDate", label: "有效期", type: "dateRange" },
    { key: "createdAt", label: "创建时间", type: "dateRange" },
  ];
}

type StatusForm = {
  targetStatus: string;
  approvalSource: string;
  approvalId: string;
  reason: string;
};

export function M3BatchManagementPage({}: M3BatchManagementPageProps) {
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultM3BatchQueryValue());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultM3BatchQueryValue());
  const normalizedAppliedQuery = React.useMemo(() => normalizeM3BatchQueryValue(appliedQuery), [appliedQuery]);
  const inventoryBatchQuery = React.useMemo(
    () => toInventoryBatchQuery(normalizedAppliedQuery),
    [normalizedAppliedQuery],
  );
  const batchesQuery = useInventoryBatchesQuery(inventoryBatchQuery);
  const expiryPolicyQuery = useInventoryExpiryPolicyQuery();
  const qualityStatusQuery = useSystemDictionaryItemOptionsQuery("inventory_quality_status");
  const qualityStatusOptions = React.useMemo<QualityStatusOption[]>(
    () => (qualityStatusQuery.data ?? []).map(([value, label]) => ({
      value,
      label: typeof label === "string" ? label.trim() || value : value,
    })),
    [qualityStatusQuery.data],
  );
  const m3BatchQueryFields: QueryPanelField[] = React.useMemo(
    () => buildM3BatchQueryFields(qualityStatusOptions),
    [qualityStatusOptions],
  );
  const expiryWarningDays = expiryPolicyQuery.data?.warningDays ?? DEFAULT_NEAR_EXPIRY_DAYS;
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [detailOpen, setDetailOpen] = React.useState(false);
  const [detailBatch, setDetailBatch] = React.useState<InventoryBatch | null>(null);
  const [statusOpen, setStatusOpen] = React.useState(false);
  const [statusBatch, setStatusBatch] = React.useState<InventoryBatch | null>(null);
  const [statusForm, setStatusForm] = React.useState<StatusForm>(emptyStatusForm);
  const [recallOpen, setRecallOpen] = React.useState(false);
  const [recallBatch, setRecallBatch] = React.useState<InventoryBatch | null>(null);
  const [cancelRecallOpen, setCancelRecallOpen] = React.useState(false);
  const [cancelRecallBatch, setCancelRecallBatch] = React.useState<InventoryBatch | null>(null);
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const statusMutation = useChangeInventoryStatusMutation();
  const recallMutation = useMarkInventoryRecallMutation();
  const cancelRecallMutation = useCancelInventoryRecallMutation();
  const batches = React.useMemo(
    () => filterBatches(batchesQuery.data ?? [], normalizedAppliedQuery, expiryWarningDays),
    [batchesQuery.data, normalizedAppliedQuery, expiryWarningDays],
  );
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(m3BatchQueryFields, appliedQuery),
    [appliedQuery, m3BatchQueryFields],
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

  const statusAction: DataGridToolbarAction = {
    key: "change-status",
    label: "状态",
    description: "变更选中库存批次的质量状态",
    disabled: ({ selectedRowKeys }) => {
      const batch = batches.find((item) => item.id === selectedRowKeys[0]);
      return (
        selectedRowKeys.length !== 1
        || statusMutation.isPending
        || qualityStatusQuery.isPending
        || Boolean(qualityStatusQuery.error)
        || statusOptionsFor(batch?.quality_status ?? "", qualityStatusOptions).length === 0
      );
    },
    onClick: ({ selectedRowKeys }) => {
      const batch = batches.find((item) => item.id === selectedRowKeys[0]);
      if (!batch) return;
      setStatusBatch(batch);
      setStatusForm(statusFormFor(batch, qualityStatusOptions));
      setStatusOpen(true);
    },
  };

  const recallAction: DataGridToolbarAction = {
    key: "mark-recall",
    label: "召回",
    description: "标记选中库存批次召回并隔离",
    disabled: ({ selectedRowKeys }) => {
      const batch = batches.find((item) => item.id === selectedRowKeys[0]);
      return selectedRowKeys.length !== 1 || !batch || batch.recall_flag || recallMutation.isPending;
    },
    onClick: ({ selectedRowKeys }) => {
      const batch = batches.find((item) => item.id === selectedRowKeys[0]);
      if (!batch) return;
      recallMutation.reset();
      setRecallBatch(batch);
      setRecallOpen(true);
    },
  };

  const cancelRecallAction: DataGridToolbarAction = {
    key: "cancel-recall",
    label: "撤回",
    description: "双人审批取消选中库存批次召回",
    disabled: ({ selectedRowKeys }) => {
      const batch = batches.find((item) => item.id === selectedRowKeys[0]);
      return selectedRowKeys.length !== 1 || !batch?.recall_flag || cancelRecallMutation.isPending;
    },
    onClick: ({ selectedRowKeys }) => {
      const batch = batches.find((item) => item.id === selectedRowKeys[0]);
      if (!batch) return;
      cancelRecallMutation.reset();
      setCancelRecallBatch(batch);
      setCancelRecallOpen(true);
    },
  };

  async function submitStatus(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!statusBatch) return;
    try {
      await statusMutation.mutateAsync({
        batch_id: statusBatch.id,
        target_status: statusForm.targetStatus,
        approval_source: statusForm.approvalSource.trim(),
        approval_id: statusForm.approvalId.trim(),
        reason: statusForm.reason.trim(),
      });
      await batchesQuery.refetch();
      setStatusOpen(false);
      setStatusBatch(null);
      setLastEvent(`${statusBatch.batch_no} 状态已更新`);
    } catch {
      // 错误由 mutation 状态统一展示，保留弹窗内容便于修正后重试。
    }
  }

  async function submitRecall(request: MarkInventoryRecallRequest) {
    try {
      await recallMutation.mutateAsync(request);
      await batchesQuery.refetch();
      setRecallOpen(false);
      setRecallBatch(null);
      setLastEvent(`${request.batch_id} 已标记召回`);
    } catch {
      // 错误由弹窗展示，保留表单便于修正后重试。
    }
  }

  async function submitCancelRecall(request: CancelInventoryRecallRequest) {
    try {
      await cancelRecallMutation.mutateAsync(request);
      await batchesQuery.refetch();
      setCancelRecallOpen(false);
      setCancelRecallBatch(null);
      setLastEvent(`${request.batch_id} 已取消召回`);
    } catch {
      // 错误由弹窗展示，保留表单便于修正后重试。
    }
  }

  const columns = buildBatchColumns(openBatchDetail, expiryWarningDays, qualityStatusOptions);
  const targetStatusOptions = statusOptionsFor(statusBatch?.quality_status ?? "", qualityStatusOptions);

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
      {expiryPolicyQuery.error && (
        <div className="rounded-md border border-wms-warning/30 bg-wms-warning/10 px-4 py-3 text-sm text-wms-warning">
          近效期配置读取失败，当前使用默认 {DEFAULT_NEAR_EXPIRY_DAYS} 天阈值
        </div>
      )}
      {qualityStatusQuery.isPending && (
        <div className="rounded-md border border-muted-foreground/30 bg-muted/20 px-4 py-3 text-sm text-muted-foreground">
          库存质量状态字典加载中，筛选和状态变更选项暂不可用
        </div>
      )}
      {qualityStatusQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          库存质量状态字典读取失败，筛选和状态变更选项不可用：{qualityStatusQuery.error.message}
        </div>
      )}
      {!qualityStatusQuery.isPending && !qualityStatusQuery.error && qualityStatusOptions.length === 0 && (
        <div className="rounded-md border border-wms-warning/30 bg-wms-warning/10 px-4 py-3 text-sm text-wms-warning">
          暂无启用的库存质量状态字典项
        </div>
      )}
      {statusMutation.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {statusMutation.error.message}
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
        toolbarActions={[statusAction, recallAction, cancelRecallAction]}
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
        expiryWarningDays={expiryWarningDays}
        qualityStatusOptions={qualityStatusOptions}
        open={detailOpen}
        onOpenChange={(open) => {
          setDetailOpen(open);
          if (!open) setDetailBatch(null);
        }}
      />

      <Dialog open={statusOpen} onOpenChange={(open) => !statusMutation.isPending && setStatusOpen(open)}>
        <DialogContent className="sm:max-w-lg">
          <form className="grid gap-4" onSubmit={submitStatus}>
            <DialogHeader>
              <DialogTitle>变更库存状态</DialogTitle>
              <DialogDescription>
                {statusBatch?.batch_no ?? "库存批次"} 当前为{qualityStatusLabel(statusBatch?.quality_status ?? "", qualityStatusOptions)}，状态变更必须提供审批来源。
              </DialogDescription>
            </DialogHeader>
            <label className="grid gap-1 text-sm">目标状态
              <select
                className="h-9 rounded-md border border-input bg-background px-3 text-sm"
                aria-label="目标状态"
                value={statusForm.targetStatus}
                disabled={qualityStatusQuery.isPending || Boolean(qualityStatusQuery.error) || targetStatusOptions.length === 0}
                onChange={(event) => setStatusForm((value) => ({ ...value, targetStatus: event.target.value }))}
              >
                {targetStatusOptions.map((option) => (
                  <option key={option.value} value={option.value}>{option.label}</option>
                ))}
              </select>
            </label>
            <label className="grid gap-1 text-sm">审批来源<Input required value={statusForm.approvalSource} onChange={(event) => setStatusForm((value) => ({ ...value, approvalSource: event.target.value }))} placeholder="例如 温度超标事件" /></label>
            <label className="grid gap-1 text-sm">审批编号<Input required value={statusForm.approvalId} onChange={(event) => setStatusForm((value) => ({ ...value, approvalId: event.target.value }))} placeholder="例如 TEMP-001" /></label>
            <label className="grid gap-1 text-sm">变更原因<Input required value={statusForm.reason} onChange={(event) => setStatusForm((value) => ({ ...value, reason: event.target.value }))} placeholder="说明状态变更原因" /></label>
            <DialogFooter>
              <DialogClose asChild><Button type="button" variant="outline" disabled={statusMutation.isPending}>取消</Button></DialogClose>
              <Button type="submit" disabled={statusMutation.isPending || !statusForm.targetStatus}>{statusMutation.isPending ? "提交中..." : "确认变更"}</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <M3BatchRecallDialog
        batch={recallBatch}
        open={recallOpen}
        pending={recallMutation.isPending}
        errorMessage={recallMutation.error?.message}
        onOpenChange={(open) => {
          setRecallOpen(open);
          if (!open) setRecallBatch(null);
        }}
        onSubmit={submitRecall}
      />

      <M3BatchRecallCancelDialog
        batch={cancelRecallBatch}
        open={cancelRecallOpen}
        pending={cancelRecallMutation.isPending}
        errorMessage={cancelRecallMutation.error?.message}
        onOpenChange={(open) => {
          setCancelRecallOpen(open);
          if (!open) setCancelRecallBatch(null);
        }}
        onSubmit={submitCancelRecall}
      />
    </section>
  );
}

function buildBatchColumns(
  onOpenDetail: (id: string) => void,
  expiryWarningDays: number,
  qualityStatusOptions: QualityStatusOption[],
): DataGridColumn<InventoryBatch>[] {
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
      sortValue: (row) => qualityStatusLabel(row.quality_status, qualityStatusOptions),
      filterValue: (row) => row.quality_status,
      copyValue: (row) => qualityStatusLabel(row.quality_status, qualityStatusOptions),
      filter: {
        type: "multiSelect",
        options: qualityStatusOptions,
      },
      render: (row) => (
        <StatusBadge status={qualityStatusKey(row.quality_status, row.recall_flag)} label={qualityStatusLabel(row.quality_status, qualityStatusOptions)} size="sm" />
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
      copyValue: (row) => expiryCopyValue(row.expiry_date, expiryWarningDays),
      filter: { type: "dateRange" },
      render: (row) => <ExpiryDateCell expiryDate={row.expiry_date} warningDays={expiryWarningDays} />,
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

function defaultM3BatchQueryValue(): QueryPanelValue {
  return {
    keyword: "",
    productCode: "",
    batchNo: "",
    locationCode: "",
    qualityStatus: [],
    recallFlag: [],
    expiryRisk: [],
    productionDate: { from: "", to: "" },
    expiryDate: { from: "", to: "" },
    createdAt: { from: "", to: "" },
  };
}

const emptyStatusForm: StatusForm = {
  targetStatus: "",
  approvalSource: "",
  approvalId: "",
  reason: "",
};

const qualityStatusTransitions: Record<string, readonly string[]> = {
  qualified: ["quarantined", "loss_deducted"],
  quarantined: ["qualified", "unqualified", "loss_deducted"],
  quarantine: ["qualified", "unqualified", "loss_deducted"],
  unqualified: ["pending_destruction", "loss_deducted"],
};

function statusFormFor(batch: InventoryBatch, qualityStatusOptions: QualityStatusOption[]): StatusForm {
  return {
    ...emptyStatusForm,
    targetStatus: statusOptionsFor(batch.quality_status, qualityStatusOptions)[0]?.value ?? "",
  };
}

function statusOptionsFor(status: string, qualityStatusOptions: readonly QualityStatusOption[]) {
  const allowedStatuses = qualityStatusTransitions[status] ?? [];
  return qualityStatusOptions.filter((option) => allowedStatuses.includes(option.value));
}

function normalizeM3BatchQueryValue(value: QueryPanelValue): QueryPanelValue {
  return {
    keyword: queryString(value.keyword),
    productCode: queryString(value.productCode),
    batchNo: queryString(value.batchNo),
    locationCode: queryString(value.locationCode),
    qualityStatus: queryStringArray(value.qualityStatus),
    recallFlag: queryStringArray(value.recallFlag),
    expiryRisk: queryStringArray(value.expiryRisk),
    productionDate: queryRange(value.productionDate),
    expiryDate: queryRange(value.expiryDate),
    createdAt: queryRange(value.createdAt),
  };
}

function filterBatches(batches: InventoryBatch[], query: QueryPanelValue, warningDays: number) {
  const keyword = queryString(query.keyword).trim().toLowerCase();
  const productCode = queryString(query.productCode).trim().toLowerCase();
  const batchNo = queryString(query.batchNo).trim().toLowerCase();
  const locationCode = queryString(query.locationCode).trim().toLowerCase();
  const qualityStatuses = new Set(queryStringArray(query.qualityStatus));
  const recallFlags = new Set(queryStringArray(query.recallFlag));
  const expiryRisks = new Set(queryStringArray(query.expiryRisk));
  return batches.filter((batch) => {
    const searchable = [batch.batch_no, batch.product_code, batch.location_code].join(" ").toLowerCase();
    return (
      (!keyword || searchable.includes(keyword)) &&
      (!productCode || batch.product_code.toLowerCase().includes(productCode)) &&
      (!batchNo || batch.batch_no.toLowerCase().includes(batchNo)) &&
      (!locationCode || batch.location_code.toLowerCase().includes(locationCode)) &&
      (qualityStatuses.size === 0 || qualityStatuses.has(batch.quality_status)) &&
      (recallFlags.size === 0 || recallFlags.has(batch.recall_flag ? "true" : "false")) &&
      (expiryRisks.size === 0 || expiryRisks.has(expiryTone(batch.expiry_date, warningDays))) &&
      dateInRange(batch.production_date, queryRange(query.productionDate)) &&
      dateInRange(batch.expiry_date, queryRange(query.expiryDate)) &&
      dateInRange(batch.created_at, queryRange(query.createdAt))
    );
  });
}

function toInventoryBatchQuery(query: QueryPanelValue): InventoryBatchQuery {
  const qualityStatuses = queryStringArray(query.qualityStatus);
  const productionDate = queryRange(query.productionDate);
  const expiryDate = queryRange(query.expiryDate);
  const createdAt = queryRange(query.createdAt);
  return {
    product_code: optionalQueryString(query.productCode),
    batch_no: optionalQueryString(query.batchNo),
    location_code: optionalQueryString(query.locationCode),
    quality_status: qualityStatuses.length === 1 ? qualityStatuses[0] : undefined,
    production_from: productionDate.from || undefined,
    production_to: productionDate.to || undefined,
    expiry_from: expiryDate.from || undefined,
    expiry_to: expiryDate.to || undefined,
    created_from: createdAt.from ? `${createdAt.from}T00:00:00Z` : undefined,
    created_to: createdAt.to ? `${createdAt.to}T23:59:59Z` : undefined,
  };
}

function optionalQueryString(value: QueryPanelValue[string]) {
  const normalized = queryString(value).trim();
  return normalized || undefined;
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
