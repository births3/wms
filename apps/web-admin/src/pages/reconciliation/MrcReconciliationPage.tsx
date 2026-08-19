/**
 * MrcReconciliationPage — 库存对账差异处理
 *
 * 层级：Layer 3 页面
 * 关联故事：US-RC-001, US-RC-002
 * Wave：Wave 6
 * 业务约束：差异不自动隔离；ERP 为准必须显式选择库存批次并生成 M-SA 审批单。
 * 页面契约：列表型；QueryPanel + DataGrid 承载主信息；标准查询/刷新/导出/字段/视图；
 * 私有动作是隔离、释放、三种处置和频率配置；动作表单只进 Dialog；
 * 禁止常驻审计、轨迹、明细和当前处理对象。
 *
 * @example
 *   <MrcReconciliationPage currentUser={user} />
 */

import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  ListPageTemplate,
  StatusBadge,
  cn,
  type DataGridColumn,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Archive, CheckCircle2, LockKeyhole, Settings2, UnlockKeyhole } from "lucide-react";

import type { CurrentUser } from "@/features/auth/auth-queries";
import { useInventoryBatchesQuery } from "@/features/inventory/inventory-queries";
import {
  useReconciliationItemsQuery,
  useReconciliationRuleQuery,
  useResolveReconciliationMutation,
  useSetReconciliationIsolationMutation,
  useUpdateReconciliationRuleMutation,
  type ReconciliationDisposition,
  type ReconciliationFilters,
  type ReconciliationItem,
} from "@/features/reconciliation/reconciliation-queries";
import { MrcReconciliationIsolationDialog } from "./MrcReconciliationIsolationDialog";
import { MrcReconciliationRuleDialog } from "./MrcReconciliationRuleDialog";
import { errorText } from "@/lib/error-text";
import { queryString, queryStringArray, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_REFRESH,
  COLUMN_BATCH_NO,
  COLUMN_CREATED_AT,
  COLUMN_PRODUCT_CODE,
  LOADING_PROCESSING,
  STATUS_PENDING,
} from "@/lib/ui-strings";
import { usePageQueryState } from "@/lib/use-page-query-state";
const differenceOptions = [
  { label: "WMS 多", value: "wms_more" },
  { label: "ERP 多", value: "erp_more" },
  { label: "一致", value: "matched" },
];
const resolutionOptions = [
  { label: STATUS_PENDING, value: "open" },
  { label: "等待库存调整", value: "adjustment_pending" },
  { label: "等待 ERP 回执", value: "erp_feedback_pending" },
  { label: "处理异常", value: "exception" },
  { label: "已处理", value: "resolved" },
  { label: "已知差异", value: "known_difference" },
  { label: "一致", value: "matched" },
];

const mRcReconciliationQueryFields: QueryPanelField[] = [
  { key: "productCode", label: COLUMN_PRODUCT_CODE, type: "text", placeholder: "按商品编码模糊查询" },
  { key: "differenceType", label: "差异类型", type: "multiSelect", options: differenceOptions },
  { key: "resolutionStatus", label: "处理状态", type: "multiSelect", options: resolutionOptions },
  { key: "batchNo", label: COLUMN_BATCH_NO, type: "text", placeholder: "按批号模糊查询" },
];
const mRcReconciliationCoreQueryFieldKeys = [
  "productCode",
  "differenceType",
  "resolutionStatus",
];

const columns: DataGridColumn<ReconciliationItem>[] = [
  textColumn("product_code", COLUMN_PRODUCT_CODE, 150),
  textColumn("batch_no", COLUMN_BATCH_NO, 150),
  numberColumn("wms_qty", "WMS 在库数量"),
  numberColumn("erp_qty", "ERP 账面数量"),
  numberColumn("difference_qty", "差异数量"),
  {
    key: "difference_type",
    header: "差异类型",
    width: 120,
    filterValue: (row) => row.difference_type,
    render: (row) => (
      <StatusBadge
        size="sm"
        status={row.difference_type === "matched" ? "completed" : "isolated"}
        label={differenceLabel(row.difference_type)}
      />
    ),
  },
  {
    key: "resolution_status",
    header: "处理状态",
    width: 130,
    filterValue: (row) => row.resolution_status,
    render: (row) => (
      <StatusBadge
        size="sm"
        status={row.resolution_status === "exception"
          ? "isolated"
          : ["open", "adjustment_pending", "erp_feedback_pending"].includes(row.resolution_status)
            ? "pending"
            : "completed"}
        label={resolutionLabel(row.resolution_status)}
      />
    ),
  },
  {
    key: "stock_adjustment_order_ids",
    header: "M-SA 调整单",
    width: 210,
    render: (row) => row.stock_adjustment_order_ids.length
      ? <span className="font-mono text-xs">{row.stock_adjustment_order_ids.join("、")}</span>
      : "—",
  },
  {
    key: "created_at",
    header: COLUMN_CREATED_AT,
    width: 180,
    render: (row) => new Date(row.created_at).toLocaleString("zh-CN", { hour12: false }),
  },
];

type Notice = { type: "success" | "error"; text: string } | null;
type ResolveDialog = { item: ReconciliationItem; disposition: ReconciliationDisposition } | null;

export function MrcReconciliationPage({ currentUser }: { currentUser: CurrentUser }) {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [notice, setNotice] = React.useState<Notice>(null);
  const [ruleOpen, setRuleOpen] = React.useState(false);
  const [ruleInterval, setRuleInterval] = React.useState("24");
  const [ruleEnabled, setRuleEnabled] = React.useState(true);
  const [isolationIntent, setIsolationIntent] = React.useState<boolean | null>(null);
  const [resolveDialog, setResolveDialog] = React.useState<ResolveDialog>(null);
  const [allocationQuantities, setAllocationQuantities] = React.useState<Record<string, string>>({});
  const filters = React.useMemo(() => toFilters(appliedQuery), [appliedQuery]);
  const itemsQuery = useReconciliationItemsQuery(filters);
  const ruleQuery = useReconciliationRuleQuery();
  const isolationMutation = useSetReconciliationIsolationMutation();
  const resolveMutation = useResolveReconciliationMutation();
  const ruleMutation = useUpdateReconciliationRuleMutation();
  const rows = itemsQuery.data?.pages.flatMap((page) => page.data) ?? [];
  const selected = rows.find((row) => row.id === selectedRowKeys[0]);
  const selectedRows = rows.filter((row) => selectedRowKeys.includes(row.id));
  const targetBatchesQuery = useInventoryBatchesQuery({
    product_code: resolveDialog?.disposition === "erp_truth" ? resolveDialog.item.product_code : "__none__",
    batch_no: resolveDialog?.disposition === "erp_truth" ? resolveDialog.item.batch_no : "__none__",
  }, {
    enabled: resolveDialog?.disposition === "erp_truth",
    retry: false,
  });
  const busy = isolationMutation.isPending || resolveMutation.isPending || ruleMutation.isPending;
  const canExecute = currentUser.permissions.includes("rc.reconciliation.execute");
  const canResolve = currentUser.permissions.includes("rc.reconciliation.resolve");
  const refreshAction: DataGridRefreshAction = {
    label: BUTTON_REFRESH,
    description: "刷新真实对账差异",
    disabled: itemsQuery.isFetching,
    onClick: () => void refresh(),
  };
  const toolbarActions: DataGridToolbarAction[] = canResolve ? [
    {
      key: "isolate",
      label: "隔离",
      description: "为勾选差异对应的合格库存批次建立对账隔离引用",
      icon: <LockKeyhole className="size-4" aria-hidden />,
      disabled: (context) =>
        context.selectedRowKeys.length === 0
        || selectedRows.some((row) => row.resolution_status !== "open")
        || busy,
      onClick: () => openIsolation(true),
    },
    {
      key: "release",
      label: "释放",
      description: "释放勾选差异由本次对账建立的隔离引用",
      icon: <UnlockKeyhole className="size-4" aria-hidden />,
      disabled: (context) =>
        context.selectedRowKeys.length === 0
        || selectedRows.some((row) => !["resolved", "known_difference"].includes(row.resolution_status))
        || busy,
      onClick: () => openIsolation(false),
    },
    {
      key: "wms-truth",
      label: "实物",
      description: "生成通知 ERP 修正账面的出站事件",
      icon: <CheckCircle2 className="size-4" aria-hidden />,
      disabled: (context) => context.selectedRowKeys.length !== 1 || selected?.resolution_status !== "open" || busy,
      onClick: () => selected && openResolve(selected, "wms_truth"),
    },
    {
      key: "erp-truth",
      label: "账面",
      description: "选择目标库存批次并生成 M-SA 审批单",
      icon: <CheckCircle2 className="size-4" aria-hidden />,
      disabled: (context) => context.selectedRowKeys.length !== 1 || selected?.resolution_status !== "open" || busy,
      onClick: () => selected && openResolve(selected, "erp_truth"),
    },
    {
      key: "known-difference",
      label: "归档",
      description: "标记在途或时间差，并释放已有对账隔离",
      icon: <Archive className="size-4" aria-hidden />,
      disabled: (context) => context.selectedRowKeys.length !== 1 || selected?.resolution_status !== "open" || busy,
      onClick: () => selected && openResolve(selected, "known_difference"),
    },
  ] : [];

  function applyGridQueryState(value: unknown) {
    applyQuery(queryValueFromUnknown(value));
  }

  function clearGridQueryState() {
    resetQuery();
  }

  async function refresh() {
    const result = await itemsQuery.refetch();
    setNotice(result.error
      ? { type: "error", text: errorText(result.error, "读取库存对账差异失败") }
      : { type: "success", text: "库存对账差异已刷新" });
  }

  function openIsolation(isolate: boolean) {
    isolationMutation.reset();
    setNotice(null);
    setIsolationIntent(isolate);
  }

  async function submitIsolation() {
    if (isolationIntent === null) return;
    const isolate = isolationIntent;
    try {
      const changed = await isolationMutation.mutateAsync({ item_ids: selectedRowKeys, isolate });
      setIsolationIntent(null);
      setNotice({ type: "success", text: `${isolate ? "隔离" : "释放"}完成，共处理 ${changed} 个库存批次` });
    } catch (error) {
      setNotice({ type: "error", text: errorText(error, "更新对账隔离状态失败") });
    }
  }

  function openResolve(item: ReconciliationItem, disposition: ReconciliationDisposition) {
    resolveMutation.reset();
    setNotice(null);
    setAllocationQuantities({});
    setResolveDialog({ item, disposition });
  }

  async function submitResolve() {
    if (!resolveDialog) return;
    const allocations = Object.entries(allocationQuantities)
      .filter(([, quantity]) => quantity.trim() !== "")
      .map(([inventory_batch_id, quantity]) => ({ inventory_batch_id, quantity }));
    if (resolveDialog.disposition === "erp_truth"
      && (allocations.length === 0
        || allocations.some((allocation) => !Number.isInteger(Number(allocation.quantity)) || Number(allocation.quantity) <= 0)
        || allocations.reduce((sum, allocation) => sum + Number(allocation.quantity), 0)
          !== Math.abs(Number(resolveDialog.item.difference_qty)))) {
      setNotice({ type: "error", text: `库存分配必须为正整数，且合计等于差异绝对值 ${Math.abs(Number(resolveDialog.item.difference_qty))}` });
      return;
    }
    try {
      await resolveMutation.mutateAsync({
        id: resolveDialog.item.id,
        disposition: resolveDialog.disposition,
        allocations: resolveDialog.disposition === "erp_truth" ? allocations : [],
      });
      setResolveDialog(null);
      setSelectedRowKeys([]);
      setNotice({ type: "success", text: dispositionSuccess(resolveDialog.disposition) });
    } catch (error) {
      setNotice({ type: "error", text: errorText(error, "处理库存对账差异失败") });
    }
  }

  function openRule() {
    if (!ruleQuery.data) return;
    ruleMutation.reset();
    setNotice(null);
    setRuleInterval(String(ruleQuery.data?.interval_hours ?? 24));
    setRuleEnabled(ruleQuery.data?.enabled ?? true);
    setRuleOpen(true);
  }

  async function submitRule(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const interval = Number(ruleInterval);
    if (!Number.isInteger(interval) || interval < 1 || interval > 168) {
      setNotice({ type: "error", text: "对账间隔必须是 1 到 168 小时的整数" });
      return;
    }
    try {
      await ruleMutation.mutateAsync({ interval_hours: interval, enabled: ruleEnabled });
      setRuleOpen(false);
      setNotice({ type: "success", text: "对账频率已保存，后续调度按新规则执行" });
    } catch (error) {
      setNotice({ type: "error", text: errorText(error, "保存对账频率失败") });
    }
  }

  return (
    <ListPageTemplate
      header={{
        actions: canExecute ? (
          <Button
            type="button"
            variant="outline"
            disabled={ruleQuery.isPending || ruleQuery.isError}
            onClick={openRule}
          >
            <Settings2 className="size-4" aria-hidden />
            对账频率
          </Button>
        ) : undefined,
      }}
      notice={notice ? { kind: notice.type === "success" ? "success" : "error", text: notice.text } : null}
      queryFields={mRcReconciliationQueryFields}
      coreQueryFieldKeys={mRcReconciliationCoreQueryFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={setDraftQuery}
      onQuery={() => applyQuery(draftQuery)}
      onReset={resetQuery}
      gridProps={{
        storageKey: "m-rc.reconciliation-items",
        columns,
        data: rows,
        rowKey: (row) => row.id,
        selectable: canResolve,
        selectedRowKeys,
        onSelectedRowKeysChange: setSelectedRowKeys,
        caption: itemsQuery.isPending ? "加载真实对账差异..." : "差异不会自动隔离；主管勾选后才执行对账隔离",
        emptyTitle: itemsQuery.isPending
          ? "正在加载对账差异"
          : itemsQuery.isError
            ? "读取对账差异失败"
            : "当前没有对账差异",
        emptyDescription: itemsQuery.isPending
          ? "正在读取真实 PostgreSQL 对账数据"
          : itemsQuery.isError
            ? errorText(itemsQuery.error, "请检查权限和真实 PostgreSQL 数据")
            : "定时任务下一次拉取 ERP 库存快照后会更新列表",
        tableClassName: "min-w-[1250px]",
        exportFileBaseName: "M-RC库存对账差异",
        refreshAction,
        toolbarActions,
        queryState: appliedQuery,
        onApplyQueryState: applyGridQueryState,
        onClearQueryState: clearGridQueryState,
      }}
      dialogs={
        <>
          <MrcReconciliationIsolationDialog
            intent={isolationIntent}
            selectedCount={selectedRowKeys.length}
            pending={isolationMutation.isPending}
            errorMessage={isolationMutation.isError
              ? `更新对账隔离状态失败：${errorText(isolationMutation.error, "请稍后重试")}`
              : null}
            onOpenChange={(open) => {
              if (!isolationMutation.isPending && !open) setIsolationIntent(null);
            }}
            onSubmit={() => void submitIsolation()}
          />

          <MrcReconciliationRuleDialog
            open={ruleOpen}
            interval={ruleInterval}
            enabled={ruleEnabled}
            pending={ruleMutation.isPending}
            errorMessage={ruleMutation.isError
              ? `保存对账频率失败：${errorText(ruleMutation.error, "请稍后重试")}`
              : null}
            onOpenChange={(open) => {
              if (!ruleMutation.isPending) setRuleOpen(open);
            }}
            onIntervalChange={setRuleInterval}
            onEnabledChange={setRuleEnabled}
            onSubmit={submitRule}
          />

          <Dialog open={Boolean(resolveDialog)} onOpenChange={(open) => !resolveMutation.isPending && !open && setResolveDialog(null)}>
            <DialogContent className="sm:max-w-lg">
              <DialogHeader>
                <DialogTitle>{resolveDialog ? dispositionTitle(resolveDialog.disposition) : "处理差异"}</DialogTitle>
                <DialogDescription>
                  {resolveDialog && `${resolveDialog.item.product_code} / ${resolveDialog.item.batch_no}，差异 ${resolveDialog.item.difference_qty}`}
                </DialogDescription>
              </DialogHeader>
              {resolveDialog?.disposition === "erp_truth" && (
                <div className="grid gap-2 text-sm">
                  <span className="font-medium">库存批次分配（合计 {Math.abs(Number(resolveDialog.item.difference_qty))}）</span>
                  {(targetBatchesQuery.data ?? []).map((batch) => (
                    <label key={batch.id} className="grid grid-cols-[1fr_7rem] items-center gap-2">
                      <span>{batch.location_code} · 在库 {batch.qty_on_hand} · {qualityStatusLabel(batch.status)}</span>
                      <input
                        className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                        type="number"
                        min={1}
                        step={1}
                        placeholder="分配数量"
                        value={allocationQuantities[batch.id] ?? ""}
                        disabled={targetBatchesQuery.isPending || targetBatchesQuery.isError}
                        onChange={(event) => setAllocationQuantities((current) => ({
                          ...current,
                          [batch.id]: event.target.value,
                        }))}
                      />
                    </label>
                  ))}
                  <span className="text-xs text-muted-foreground">
                    每条分配创建一张待审批 M-SA 单；全部完成后差异才关闭。ERP 独有批号需先走受控批次创建。
                  </span>
                  {targetBatchesQuery.isError && (
                    <span className="text-xs text-destructive" role="alert">
                      读取目标库存批次失败：{errorText(targetBatchesQuery.error, "请稍后重试")}
                    </span>
                  )}
                  {!targetBatchesQuery.isPending
                    && !targetBatchesQuery.isError
                    && (targetBatchesQuery.data?.length ?? 0) === 0 && (
                    <span className="text-xs text-destructive" role="alert">
                      未找到该商品同批号的可用库存批次。
                    </span>
                  )}
                </div>
              )}
              {resolveDialog?.disposition === "wms_truth" && (
                <p className="text-sm text-muted-foreground">
                  提交后将差异以 WMS 实物为准进行闭环，并写入审计追踪；后续由 ERP 账面拉平。
                </p>
              )}
              {resolveDialog?.disposition === "known_difference" && (
                <p className="text-sm text-muted-foreground">
                  提交后将差异标记并归档为已知差异，并写入审计追踪；不会调整实物库存。
                </p>
              )}
              {resolveMutation.isError && (
                <div
                  className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
                  role="alert"
                >
                  处理差异失败：{errorText(resolveMutation.error, "请稍后重试")}
                </div>
              )}
              <DialogFooter>
                <DialogClose asChild>
                  <Button type="button" variant="outline" disabled={resolveMutation.isPending}>
                    取消
                  </Button>
                </DialogClose>
                <Button
                  type="button"
                  disabled={
                    resolveMutation.isPending
                    || (resolveDialog?.disposition === "erp_truth"
                      && (
                        Object.values(allocationQuantities).every((quantity) => quantity.trim() === "")
                        || targetBatchesQuery.isPending
                        || targetBatchesQuery.isError
                        || (targetBatchesQuery.data?.length ?? 0) === 0
                      ))
                  }
                  onClick={() => void submitResolve()}
                >
                  {resolveMutation.isPending ? LOADING_PROCESSING : "确认处理"}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </>
      }
    >
      {itemsQuery.hasNextPage && (
        <div className="mt-4 flex justify-center">
          <Button
            type="button"
            variant="outline"
            disabled={itemsQuery.isFetchingNextPage}
            onClick={() => void itemsQuery.fetchNextPage()}
          >
            {itemsQuery.isFetchingNextPage ? "加载中..." : "加载更多"}
          </Button>
        </div>
      )}
    </ListPageTemplate>
  );
}

function defaultQuery(): QueryPanelValue {
  return {
    productCode: "",
    batchNo: "",
    differenceType: ["wms_more", "erp_more"],
    resolutionStatus: ["open"],
  };
}

function normalizeQuery(value: unknown): QueryPanelValue {
  const row = queryValueFromUnknown(value);
  return {
    productCode: queryString(row.productCode),
    batchNo: queryString(row.batchNo),
    differenceType: queryStringArray(row.differenceType),
    resolutionStatus: queryStringArray(row.resolutionStatus),
  };
}

function toFilters(value: QueryPanelValue): ReconciliationFilters {
  const differenceType = queryStringArray(value.differenceType);
  const resolutionStatus = queryStringArray(value.resolutionStatus);
  return {
    product_code: optional(queryString(value.productCode)),
    batch_no: optional(queryString(value.batchNo)),
    difference_type: differenceType.length ? differenceType.join(",") : undefined,
    resolution_status: resolutionStatus.length ? resolutionStatus.join(",") : undefined,
  };
}

function optional(value: string) {
  return value.trim() || undefined;
}

function textColumn(
  key: "product_code" | "batch_no",
  header: string,
  width: number,
): DataGridColumn<ReconciliationItem> {
  return { key, header, width, render: (row) => <span className="font-mono">{row[key]}</span> };
}

function numberColumn(
  key: "wms_qty" | "erp_qty" | "difference_qty",
  header: string,
): DataGridColumn<ReconciliationItem> {
  return { key, header, width: 130, render: (row) => Number(row[key]).toLocaleString("zh-CN") };
}

function differenceLabel(value: string) {
  if (value === "wms_more") return "WMS 多";
  if (value === "erp_more") return "ERP 多";
  return "一致";
}

function resolutionLabel(value: string) {
  if (value === "open") return STATUS_PENDING;
  if (value === "adjustment_pending") return "等待库存调整";
  if (value === "erp_feedback_pending") return "等待 ERP 回执";
  if (value === "exception") return "处理异常";
  if (value === "resolved") return "已处理";
  if (value === "known_difference") return "已知差异";
  return "一致";
}

function dispositionTitle(value: ReconciliationDisposition) {
  if (value === "wms_truth") return "以 WMS 实物为准";
  if (value === "erp_truth") return "以 ERP 账面为准";
  return "归档为已知差异";
}

function dispositionSuccess(value: ReconciliationDisposition) {
  if (value === "wms_truth") return "已生成 ERP 修正事件，收到业务成功回执后关闭差异";
  if (value === "erp_truth") return "已生成待审批的 M-SA 库存调整单，全部完成后关闭差异";
  return "已归档为已知差异，并释放已有对账隔离";
}

function qualityStatusLabel(value: string) {
  if (value === "qualified") return "合格";
  if (value === "quarantined") return "隔离";
  if (value === "unqualified") return "不合格";
  if (value === "pending_destruction") return "待销毁";
  if (value === "loss_deducted") return "报损扣减";
  return `未知状态（${value}）`;
}
