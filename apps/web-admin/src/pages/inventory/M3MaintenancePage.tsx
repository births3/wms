/**
 * M3MaintenancePage — 在库养护任务
 * 层级：Layer 3 页面
 * 关联故事：US-M3-004
 */

import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";

import {
  useGenerateMaintenanceTasksMutation,
  useMaintenanceTasksQuery,
  type MaintenanceTask,
} from "@/features/inventory/m3-ops-queries";

export const m3MaintenanceQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "商品 / 批号 / 库位" },
  {
    key: "status",
    label: "状态",
    type: "select",
    options: [
      { label: "全部", value: "" },
      { label: "待执行", value: "pending" },
      { label: "已完成", value: "completed" },
    ],
  },
];
export const m3MaintenanceCoreQueryFieldKeys = ["keyword", "status"];

export function M3MaintenancePage() {
  const [draft, setDraft] = React.useState<QueryPanelValue>({ keyword: "", status: "" });
  const [applied, setApplied] = React.useState<QueryPanelValue>({ keyword: "", status: "" });
  const query = useMaintenanceTasksQuery();
  const generate = useGenerateMaintenanceTasksMutation();
  const rows = React.useMemo(() => {
    const keyword = String(applied.keyword ?? "").toLowerCase();
    const status = String(applied.status ?? "");
    return (query.data ?? []).filter((row) => {
      const text = `${row.product_code} ${row.batch_no} ${row.location_code}`.toLowerCase();
      return (!keyword || text.includes(keyword)) && (!status || row.status === status);
    });
  }, [applied, query.data]);

  const columns = React.useMemo<DataGridColumn<MaintenanceTask>[]>(
    () => [
      { key: "created_at", header: "创建时间", width: 180, render: (row) => formatTime(row.created_at) },
      { key: "product_code", header: "商品", width: 140, mono: true, render: (row) => row.product_code },
      { key: "batch_no", header: "批号", width: 140, mono: true, render: (row) => row.batch_no },
      { key: "location_code", header: "库位", width: 140, mono: true, render: (row) => row.location_code },
      { key: "planned_at", header: "计划时间", width: 180, render: (row) => formatTime(row.planned_at) },
      { key: "status", header: "状态", width: 100, render: (row) => row.status },
    ],
    [],
  );

  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新养护任务",
    disabled: query.isFetching,
    onClick: () => void query.refetch(),
  };
  const generateAction: DataGridToolbarAction = {
    key: "generate",
    label: generate.isPending ? "生成中..." : "生成计划",
    description: "按近效期批次生成养护任务",
    disabled: generate.isPending,
    onClick: () => {
      void generate.mutateAsync();
    },
  };

  return (
    <div className="space-y-4">
      <PageHeader title="M3 在库养护" subtitle="近效期计划生成与养护任务执行" />
      <QueryPanel
        fields={m3MaintenanceQueryFields}
        defaultVisibleFieldKeys={m3MaintenanceCoreQueryFieldKeys}
        value={draft}
        onValueChange={setDraft}
        onQuery={() => setApplied(draft)}
        onReset={() => {
          const next = { keyword: "", status: "" };
          setDraft(next);
          setApplied(next);
        }}
      />
      {(generate.error || query.error) && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {generate.error?.message ?? query.error?.message}
        </div>
      )}
      <Card>
        <CardContent className="p-5">
          <DataGrid
            storageKey="m3.maintenance-tasks"
            columns={columns}
            data={rows}
            rowKey={(row) => row.id}
            emptyTitle={query.isError ? "读取养护任务失败" : "暂无养护任务"}
            emptyDescription="可点击生成计划，基于近效期库存创建待执行任务"
            exportFileBaseName="M3-在库养护"
            refreshAction={refreshAction}
            toolbarActions={[generateAction]}
            queryState={applied}
            querySummaryItems={buildQueryPanelSummaryItems(m3MaintenanceQueryFields, applied)}
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
    </div>
  );
}

function formatTime(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false });
}
