/**
 * M3InventoryCountPage — 库存盘点列表
 * 层级：Layer 3 页面
 * 关联故事：US-M3-005
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
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";

import {
  useApproveInventoryCountMutation,
  useCreateInventoryCountMutation,
  useInventoryCountsQuery,
  useSubmitInventoryCountLineMutation,
  type InventoryCountSummary,
} from "@/features/inventory/m3-ops-queries";
import { formatDateTime } from "@/lib/format";
import { queryValueFromUnknown } from "@/lib/query-value";
import { usePageQueryState } from "@/lib/use-page-query-state";

export const m3CountQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "状态 / 类型 / 商品" },
  {
    key: "status",
    label: "状态",
    type: "multiSelect",
    options: [
      { label: "全部", value: "" },
      { label: "盘点中", value: "in_progress" },
      { label: "待审批", value: "pending_approval" },
      { label: "已审批", value: "approved" },
    ],
  },
];
export const m3CountCoreQueryFieldKeys = ["keyword", "status"];

export function M3InventoryCountPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(() => ({ keyword: "", status: "" }));
  const [open, setOpen] = React.useState(false);
  const [countType, setCountType] = React.useState("cycle");
  const [productCode, setProductCode] = React.useState("");
  const [selected, setSelected] = React.useState<InventoryCountSummary | null>(null);
  const [physicalQtyByLine, setPhysicalQtyByLine] = React.useState<Record<string, string>>({});
  const [qtyError, setQtyError] = React.useState<string | null>(null);
  const query = useInventoryCountsQuery();
  const create = useCreateInventoryCountMutation();
  const submitLine = useSubmitInventoryCountLineMutation();
  const approve = useApproveInventoryCountMutation();
  const rows = React.useMemo(() => {
    const keyword = String(appliedQuery.keyword ?? "").toLowerCase();
    const status = String(appliedQuery.status ?? "");
    return (query.data ?? []).filter((row) => {
      const hitKeyword =
        !keyword ||
        row.status.includes(keyword) ||
        row.count_type.includes(keyword) ||
        (row.product_code ?? "").toLowerCase().includes(keyword);
      const hitStatus = !status || row.status === status;
      return hitKeyword && hitStatus;
    });
  }, [appliedQuery, query.data]);

  const columns = React.useMemo<DataGridColumn<InventoryCountSummary>[]>(
    () => [
      { key: "created_at", header: "创建时间", width: 180, render: (row) => formatDateTime(row.created_at) },
      { key: "count_type", header: "类型", width: 100, render: (row) => row.count_type },
      { key: "status", header: "状态", width: 120, render: (row) => row.status },
      { key: "product_code", header: "商品范围", width: 140, render: (row) => row.product_code ?? "全部" },
      { key: "lines", header: "明细数", width: 90, render: (row) => row.lines?.length ?? 0 },
      { key: "started_at", header: "开始时间", width: 180, render: (row) => formatDateTime(row.started_at) },
      {
        key: "actions",
        header: "操作",
        width: 120,
        render: (row) => (
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() => {
              setSelected(row);
              setQtyError(null);
              const next: Record<string, string> = {};
              for (const line of row.lines ?? []) {
                next[line.id] =
                  line.physical_qty != null && line.physical_qty !== undefined
                    ? String(line.physical_qty)
                    : "";
              }
              setPhysicalQtyByLine(next);
            }}
          >
            明细
          </Button>
        ),
      },
    ],
    [],
  );

  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新盘点单",
    disabled: query.isFetching,
    onClick: () => void query.refetch(),
  };
  const createAction: DataGridCreateAction = {
    label: "新建盘点",
    description: "创建盘点单",
    onClick: () => setOpen(true),
  };

  const canSubmitLines = selected?.status === "in_progress";
  const canApprove =
    selected?.status === "pending_approval" ||
    (selected?.status === "in_progress" &&
      (selected.lines ?? []).every((line) => line.physical_qty != null && line.physical_qty !== undefined));

  return (
    <div className="space-y-4">
      <PageHeader title="M3 库存盘点" />
      <QueryPanel
        fields={m3CountQueryFields}
        defaultVisibleFieldKeys={m3CountCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => applyQuery(draftQuery)}
        onReset={resetQuery}
      />
      <Card>
        <CardContent className="p-5">
          <DataGrid
            storageKey="m3-counts-datagrid"
            columns={columns}
            data={rows}
            rowKey={(row) => row.id}
            emptyTitle={query.isError ? "读取盘点单失败" : "暂无盘点单"}
            emptyDescription={query.isError ? query.error.message : "请新建盘点单"}
            exportFileBaseName="M3-库存盘点"
            refreshAction={refreshAction}
            createAction={createAction}
            queryState={appliedQuery}
            querySummaryItems={buildQueryPanelSummaryItems(m3CountQueryFields, appliedQuery)}
            onApplyQueryState={(value) => applyQuery(queryValueFromUnknown(value))}
            onClearQueryState={resetQuery}
          />
        </CardContent>
      </Card>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <form
            className="grid gap-3"
            onSubmit={(event) => {
              event.preventDefault();
              void create
                .mutateAsync({
                  count_type: countType,
                  product_code: productCode.trim() || undefined,
                })
                .then(() => setOpen(false))
                // 错误已由 create.error 在弹窗内展示，这里仅吞掉 unhandled rejection。
                .catch(() => undefined);
            }}
          >
            <DialogHeader>
              <DialogTitle>新建盘点单</DialogTitle>
            </DialogHeader>
            <label className="grid gap-1 text-sm">
              盘点类型
              <select aria-label="盘点类型" className="h-10 rounded-md border px-3" value={countType} onChange={(e) => setCountType(e.target.value)}>
                <option value="cycle">循环盘点</option>
                <option value="full">全面盘点</option>
                <option value="blind">盲盘</option>
              </select>
            </label>
            <label className="grid gap-1 text-sm">
              商品编码（可选）
              <Input aria-label="商品编码" value={productCode} onChange={(e) => setProductCode(e.target.value)} placeholder="留空=范围内全部" />
            </label>
            {create.error && <p className="text-sm text-destructive">{create.error.message}</p>}
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">取消</Button>
              </DialogClose>
              <Button type="submit" disabled={create.isPending}>{create.isPending ? "创建中..." : "创建"}</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog
        open={selected != null}
        onOpenChange={(next) => {
          if (!next) setSelected(null);
        }}
      >
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>盘点明细 · {selected?.id.slice(0, 8)}</DialogTitle>
          </DialogHeader>
          <div className="max-h-[50vh] space-y-3 overflow-y-auto">
            {(selected?.lines ?? []).length === 0 ? (
              <p className="text-sm text-muted-foreground">暂无明细行</p>
            ) : (
              (selected?.lines ?? []).map((line) => (
                <div key={line.id} className="grid gap-2 rounded-md border p-3 sm:grid-cols-[1fr_1fr_1fr_auto] sm:items-end">
                  <div className="text-sm">
                    <div className="font-medium">{line.product_code}</div>
                    <div className="text-muted-foreground">批号 {line.batch_no}</div>
                  </div>
                  <div className="text-sm">
                    账面
                    <div className="font-medium">
                      {selected?.count_type === "blind" && line.physical_qty == null ? "盲盘隐藏" : line.book_qty}
                    </div>
                  </div>
                  <label className="grid gap-1 text-sm">
                    实盘数量
                    <Input
                      aria-label={`实盘数量-${line.batch_no}`}
                      type="number"
                      min={0}
                      value={physicalQtyByLine[line.id] ?? ""}
                      disabled={!canSubmitLines || submitLine.isPending}
                      onChange={(event) =>
                        setPhysicalQtyByLine((prev) => ({ ...prev, [line.id]: event.target.value }))
                      }
                    />
                  </label>
                  <Button
                    type="button"
                    size="sm"
                    disabled={!canSubmitLines || submitLine.isPending || physicalQtyByLine[line.id] === ""}
                    onClick={() => {
                      if (!selected) return;
                      const qty = Number(physicalQtyByLine[line.id]);
                      if (!Number.isFinite(qty) || qty < 0) return;
                      if (!Number.isInteger(qty)) {
                        setQtyError(`批号 ${line.batch_no} 的实盘数量必须是整数`);
                        return;
                      }
                      setQtyError(null);
                      void submitLine.mutateAsync({
                        countId: selected.id,
                        lineId: line.id,
                        physical_qty: qty,
                      }).then(() => query.refetch().then((result) => {
                        const next = (result.data ?? []).find((item) => item.id === selected.id) ?? null;
                        setSelected(next);
                      }))
                        // 错误已由 submitLine.error 在弹窗内展示，这里仅吞掉 unhandled rejection。
                        .catch(() => undefined);
                    }}
                  >
                    提交实盘
                  </Button>
                </div>
              ))
            )}
          </div>
          {(qtyError || submitLine.error || approve.error) && (
            <p className="text-sm text-destructive">
              {qtyError ?? submitLine.error?.message ?? approve.error?.message}
            </p>
          )}
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">关闭</Button>
            </DialogClose>
            <Button
              type="button"
              disabled={!canApprove || approve.isPending}
              onClick={() => {
                if (!selected) return;
                const elevated = (selected.lines ?? []).some((line) => {
                  const book = Number(line.book_qty ?? 0);
                  const variance = Number(line.variance_qty ?? 0);
                  if (variance === 0) return false;
                  if (book <= 0) return true;
                  return Math.abs(variance) * 100 > book * 10;
                });
                void approve
                  .mutateAsync({
                    countId: selected.id,
                    approval_source: elevated ? "盘点-高级" : "盘点",
                  })
                  .then(() => {
                    setSelected(null);
                  })
                  // 错误已由 approve.error 在弹窗内展示，这里仅吞掉 unhandled rejection。
                  .catch(() => undefined);
              }}
            >
              {approve.isPending ? "审批中..." : "审批差异并调账"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
