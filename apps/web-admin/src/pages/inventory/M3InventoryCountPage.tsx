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
  useCreateInventoryCountMutation,
  useInventoryCountsQuery,
  type InventoryCountSummary,
} from "@/features/inventory/m3-ops-queries";

export const m3CountQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "状态 / 类型 / 商品" },
  {
    key: "status",
    label: "状态",
    type: "select",
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
  const [draft, setDraft] = React.useState<QueryPanelValue>({ keyword: "", status: "" });
  const [applied, setApplied] = React.useState<QueryPanelValue>({ keyword: "", status: "" });
  const [open, setOpen] = React.useState(false);
  const [countType, setCountType] = React.useState("cycle");
  const [productCode, setProductCode] = React.useState("");
  const query = useInventoryCountsQuery();
  const create = useCreateInventoryCountMutation();
  const rows = React.useMemo(() => {
    const keyword = String(applied.keyword ?? "").toLowerCase();
    const status = String(applied.status ?? "");
    return (query.data ?? []).filter((row) => {
      const hitKeyword =
        !keyword ||
        row.status.includes(keyword) ||
        row.count_type.includes(keyword) ||
        (row.product_code ?? "").toLowerCase().includes(keyword);
      const hitStatus = !status || row.status === status;
      return hitKeyword && hitStatus;
    });
  }, [applied, query.data]);

  const columns = React.useMemo<DataGridColumn<InventoryCountSummary>[]>(
    () => [
      { key: "created_at", header: "创建时间", width: 180, render: (row) => formatTime(row.created_at) },
      { key: "count_type", header: "类型", width: 100, render: (row) => row.count_type },
      { key: "status", header: "状态", width: 120, render: (row) => row.status },
      { key: "product_code", header: "商品范围", width: 140, render: (row) => row.product_code ?? "全部" },
      { key: "lines", header: "明细数", width: 90, render: (row) => row.lines?.length ?? 0 },
      { key: "started_at", header: "开始时间", width: 180, render: (row) => formatTime(row.started_at) },
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

  return (
    <div className="space-y-4">
      <PageHeader title="M3 库存盘点" subtitle="创建盘点单、提交实盘并审批调整" />
      <QueryPanel
        fields={m3CountQueryFields}
        defaultVisibleFieldKeys={m3CountCoreQueryFieldKeys}
        value={draft}
        onValueChange={setDraft}
        onQuery={() => setApplied(draft)}
        onReset={() => {
          const next = { keyword: "", status: "" };
          setDraft(next);
          setApplied(next);
        }}
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
            queryState={applied}
            querySummaryItems={buildQueryPanelSummaryItems(m3CountQueryFields, applied)}
            onApplyQueryState={(value) => {
              const next = (value && typeof value === "object" ? value : {}) as QueryPanelValue;
              setDraft(next);
              setApplied(next);
            }}
            onClearQueryState={() => {
              const next = { keyword: "", status: "" };
              setDraft(next);
              setApplied(next);
            }}
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
                .then(() => setOpen(false));
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
    </div>
  );
}

function formatTime(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false });
}
