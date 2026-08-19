/**
 * M3MaintenancePage — 在库养护任务
 * 层级：Layer 3 页面
 * 关联故事：US-M3-004
 */

import * as React from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  ListPageTemplate,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type ListPageNotice,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";

import {
  useCreateMaintenanceRecordMutation,
  useGenerateMaintenanceTasksMutation,
  useMaintenanceTasksQuery,
  type MaintenanceTask,
} from "@/features/inventory/m3-ops-queries";
import { formatDateTime } from "@/lib/format";
import { queryValueFromUnknown } from "@/lib/query-value";
import { BUTTON_REFRESH, COLUMN_BATCH_NO, COLUMN_CREATED_AT, COLUMN_STATUS, FIELD_KEYWORD, FILTER_ALL, LOADING_SUBMITTING, STATUS_COMPLETED } from "@/lib/ui-strings";
import { usePageQueryState } from "@/lib/use-page-query-state";

export const m3MaintenanceQueryFields: QueryPanelField[] = [
  { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "商品 / 批号 / 库位" },
  {
    key: "status",
    label: COLUMN_STATUS,
    type: "multiSelect",
    options: [
      { label: FILTER_ALL, value: "" },
      { label: "待执行", value: "pending" },
      { label: STATUS_COMPLETED, value: "completed" },
    ],
  },
];
export const m3MaintenanceCoreQueryFieldKeys = ["keyword", "status"];

export function M3MaintenancePage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(() => ({ keyword: "", status: "" }));
  const [selected, setSelected] = React.useState<MaintenanceTask | null>(null);
  const [temperature, setTemperature] = React.useState("22");
  const [humidity, setHumidity] = React.useState("45");
  const [conclusion, setConclusion] = React.useState("normal");
  const [exceptionType, setExceptionType] = React.useState("package_damage");
  const [notes, setNotes] = React.useState("");
  // 打开某个任务的提交弹窗时重置结果表单，避免上一个任务填写的内容串到下一个任务。
  function openResultDialog(task: MaintenanceTask) {
    setTemperature("22");
    setHumidity("45");
    setConclusion("normal");
    setExceptionType("package_damage");
    setNotes("");
    setSelected(task);
  }
  const query = useMaintenanceTasksQuery();
  const generate = useGenerateMaintenanceTasksMutation();
  const createRecord = useCreateMaintenanceRecordMutation();
  const rows = React.useMemo(() => {
    const keyword = String(appliedQuery.keyword ?? "").toLowerCase();
    const status = String(appliedQuery.status ?? "");
    return (query.data ?? []).filter((row) => {
      const text = `${row.product_code} ${row.batch_no} ${row.location_code}`.toLowerCase();
      return (!keyword || text.includes(keyword)) && (!status || row.status === status);
    });
  }, [appliedQuery, query.data]);

  const columns = React.useMemo<DataGridColumn<MaintenanceTask>[]>(
    () => [
      { key: "created_at", header: COLUMN_CREATED_AT, width: 180, render: (row) => formatDateTime(row.created_at) },
      { key: "product_code", header: "商品", width: 140, mono: true, render: (row) => row.product_code },
      { key: "batch_no", header: COLUMN_BATCH_NO, width: 140, mono: true, render: (row) => row.batch_no },
      { key: "location_code", header: "库位", width: 140, mono: true, render: (row) => row.location_code },
      { key: "planned_at", header: "计划时间", width: 180, render: (row) => formatDateTime(row.planned_at) },
      { key: "status", header: COLUMN_STATUS, width: 100, render: (row) => row.status },
      {
        key: "actions",
        header: "操作",
        width: 120,
        render: (row) =>
          row.status === "pending" ? (
            <Button type="button" size="sm" variant="outline" onClick={() => openResultDialog(row)}>
              提交结果
            </Button>
          ) : (
            <span className="text-sm text-muted-foreground">—</span>
          ),
      },
    ],
    [],
  );

  const refreshAction: DataGridRefreshAction = {
    label: BUTTON_REFRESH,
    description: "刷新养护任务",
    disabled: query.isFetching,
    onClick: () => void query.refetch(),
  };
  const generateAction: DataGridToolbarAction = {
    key: "generate",
    label: generate.isPending ? "生成中..." : "生成计划",
    description: "按重点/一般周期与近效期窗口生成养护任务",
    disabled: generate.isPending,
    onClick: () => {
      // 错误已由页面顶部 generate.error 展示，这里仅吞掉 unhandled rejection。
      void generate.mutateAsync().catch(() => undefined);
    },
  };

  const errorMessage =
    generate.error?.message ?? query.error?.message ?? createRecord.error?.message;
  const notice: ListPageNotice | null = errorMessage
    ? { kind: "error", text: errorMessage }
    : null;

  return (
    <ListPageTemplate
      notice={notice}
      queryFields={m3MaintenanceQueryFields}
      coreQueryFieldKeys={m3MaintenanceCoreQueryFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={setDraftQuery}
      onQuery={() => applyQuery(draftQuery)}
      onReset={resetQuery}
      gridProps={{
        storageKey: "m3.maintenance-tasks",
        columns,
        data: rows,
        rowKey: (row) => row.id,
        emptyTitle: query.isError ? "读取养护任务失败" : "暂无养护任务",
        emptyDescription: "可点击生成计划：重点品种约每月、一般品种约每季，并含近效期窗口",
        exportFileBaseName: "M3-在库养护",
        refreshAction,
        toolbarActions: [generateAction],
        queryState: appliedQuery,
        querySummaryItems: buildQueryPanelSummaryItems(m3MaintenanceQueryFields, appliedQuery),
        onApplyQueryState: (value) => applyQuery(queryValueFromUnknown(value)),
        onClearQueryState: resetQuery,
      }}
      dialogs={
        <Dialog
          open={selected != null}
          onOpenChange={(next) => {
            if (!next) setSelected(null);
          }}
        >
          <DialogContent>
            <form
              className="grid gap-3"
              onSubmit={(event) => {
                event.preventDefault();
                if (!selected) return;
                const temp = Number(temperature);
                const hum = Number(humidity);
                if (!Number.isFinite(temp) || !Number.isFinite(hum)) return;
                void createRecord
                  .mutateAsync({
                    task_id: selected.id,
                    temperature_celsius: temp,
                    humidity_percent: hum,
                    appearance: "intact",
                    packaging: "intact",
                    pest: "none",
                    rodent: "none",
                    mildew: "none",
                    conclusion,
                    exception_type: conclusion === "abnormal" ? exceptionType : null,
                    notes: notes.trim() || null,
                  })
                  .then(() => setSelected(null))
                  // 错误已由页面顶部 createRecord.error 展示，这里仅吞掉 unhandled rejection。
                  .catch(() => undefined);
              }}
            >
              <DialogHeader>
                <DialogTitle>
                  提交养护结果 · {selected?.product_code} / {selected?.batch_no}
                </DialogTitle>
              </DialogHeader>
              <label className="grid gap-1 text-sm">
                库区温度（℃）
                <Input aria-label="库区温度" value={temperature} onChange={(e) => setTemperature(e.target.value)} />
              </label>
              <label className="grid gap-1 text-sm">
                库区湿度（%）
                <Input aria-label="库区湿度" value={humidity} onChange={(e) => setHumidity(e.target.value)} />
              </label>
              <label className="grid gap-1 text-sm">
                养护结论
                <select
                  aria-label="养护结论"
                  className="h-10 rounded-md border px-3"
                  value={conclusion}
                  onChange={(e) => setConclusion(e.target.value)}
                >
                  <option value="normal">正常</option>
                  <option value="abnormal">异常</option>
                </select>
              </label>
              {conclusion === "abnormal" && (
                <label className="grid gap-1 text-sm">
                  异常类型
                  <select
                    aria-label="异常类型"
                    className="h-10 rounded-md border px-3"
                    value={exceptionType}
                    onChange={(e) => setExceptionType(e.target.value)}
                  >
                    <option value="quality_change">质量变化</option>
                    <option value="package_damage">包装破损</option>
                    <option value="temperature_excursion">温湿度超标</option>
                    <option value="pest_rodent_mildew">虫鼠霉害</option>
                    <option value="other">其他</option>
                  </select>
                </label>
              )}
              <label className="grid gap-1 text-sm">
                备注
                <Input aria-label="养护备注" value={notes} onChange={(e) => setNotes(e.target.value)} />
              </label>
              <DialogFooter>
                <DialogClose asChild>
                  <Button type="button" variant="outline">
                    取消
                  </Button>
                </DialogClose>
                <Button type="submit" disabled={createRecord.isPending}>
                  {createRecord.isPending ? LOADING_SUBMITTING : "提交养护结果"}
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
      }
    />
  );
}
