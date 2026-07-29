/**
 * M3RelocationPage — 库内移库
 * 层级：Layer 3 页面
 * 关联故事：US-M3-006
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
  useInventoryRelocationsQuery,
  useRelocateInventoryMutation,
  type InventoryRelocation,
} from "@/features/inventory/m3-ops-queries";
import { formatDateTime } from "@/lib/format";
import { queryValueFromUnknown } from "@/lib/query-value";
import { usePageQueryState } from "@/lib/use-page-query-state";

export const m3RelocationQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "商品 / 批号 / 库位" },
];
export const m3RelocationCoreQueryFieldKeys = ["keyword"];

export function M3RelocationPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(() => ({ keyword: "" }));
  const [open, setOpen] = React.useState(false);
  const [form, setForm] = React.useState({
    batch_id: "",
    qty: "1",
    to_location_id: "",
    to_location_code: "",
    reason: "",
  });
  const query = useInventoryRelocationsQuery();
  const relocate = useRelocateInventoryMutation();
  const rows = React.useMemo(() => {
    const keyword = String(appliedQuery.keyword ?? "").toLowerCase();
    return (query.data ?? []).filter((row) => {
      const text = `${row.product_code} ${row.batch_no} ${row.from_location_code} ${row.to_location_code}`.toLowerCase();
      return !keyword || text.includes(keyword);
    });
  }, [appliedQuery, query.data]);

  const columns = React.useMemo<DataGridColumn<InventoryRelocation>[]>(
    () => [
      { key: "created_at", header: "创建时间", width: 180, render: (row) => formatDateTime(row.created_at) },
      { key: "product_code", header: "商品", width: 120, mono: true, render: (row) => row.product_code },
      { key: "batch_no", header: "批号", width: 120, mono: true, render: (row) => row.batch_no },
      { key: "qty", header: "数量", width: 80, render: (row) => row.qty },
      { key: "from_location_code", header: "源库位", width: 140, mono: true, render: (row) => row.from_location_code },
      { key: "to_location_code", header: "目标库位", width: 140, mono: true, render: (row) => row.to_location_code },
      { key: "status", header: "状态", width: 100, render: (row) => row.status },
    ],
    [],
  );

  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新移库记录",
    disabled: query.isFetching,
    onClick: () => void query.refetch(),
  };
  const createAction: DataGridCreateAction = {
    label: "发起移库",
    description: "直接移库",
    onClick: () => setOpen(true),
  };

  return (
    <div className="space-y-4">
      <PageHeader title="M3 库内移库" subtitle="合格库存源库位减、目标库位增（同事务）" />
      <QueryPanel
        fields={m3RelocationQueryFields}
        defaultVisibleFieldKeys={m3RelocationCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => applyQuery(draftQuery)}
        onReset={resetQuery}
      />
      <Card>
        <CardContent className="p-5">
          <DataGrid
            storageKey="m3.relocations"
            columns={columns}
            data={rows}
            rowKey={(row) => row.id}
            emptyTitle={query.isError ? "读取移库记录失败" : "暂无移库记录"}
            emptyDescription={query.isError ? query.error.message : "请发起移库"}
            exportFileBaseName="M3-库内移库"
            refreshAction={refreshAction}
            createAction={createAction}
            queryState={appliedQuery}
            querySummaryItems={buildQueryPanelSummaryItems(m3RelocationQueryFields, appliedQuery)}
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
              void relocate
                .mutateAsync({
                  batch_id: form.batch_id.trim(),
                  qty: Number(form.qty),
                  to_location_id: form.to_location_id.trim(),
                  to_location_code: form.to_location_code.trim(),
                  reason: form.reason.trim() || undefined,
                })
                .then(() => setOpen(false))
                // 错误已由 relocate.error 在弹窗内展示，这里仅吞掉 unhandled rejection。
                .catch(() => undefined);
            }}
          >
            <DialogHeader>
              <DialogTitle>发起直接移库</DialogTitle>
            </DialogHeader>
            <label className="grid gap-1 text-sm">批次 ID<Input aria-label="批次 ID" required value={form.batch_id} onChange={(e) => setForm({ ...form, batch_id: e.target.value })} /></label>
            <label className="grid gap-1 text-sm">数量<Input aria-label="数量" required type="number" min="1" value={form.qty} onChange={(e) => setForm({ ...form, qty: e.target.value })} /></label>
            <label className="grid gap-1 text-sm">目标库位 ID<Input aria-label="目标库位 ID" required value={form.to_location_id} onChange={(e) => setForm({ ...form, to_location_id: e.target.value })} /></label>
            <label className="grid gap-1 text-sm">目标库位编码<Input aria-label="目标库位编码" required value={form.to_location_code} onChange={(e) => setForm({ ...form, to_location_code: e.target.value })} /></label>
            <label className="grid gap-1 text-sm">原因<Input aria-label="原因" value={form.reason} onChange={(e) => setForm({ ...form, reason: e.target.value })} /></label>
            {relocate.error && <p className="text-sm text-destructive">{relocate.error.message}</p>}
            <DialogFooter>
              <DialogClose asChild><Button type="button" variant="outline">取消</Button></DialogClose>
              <Button type="submit" disabled={relocate.isPending}>{relocate.isPending ? "提交中..." : "提交"}</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
