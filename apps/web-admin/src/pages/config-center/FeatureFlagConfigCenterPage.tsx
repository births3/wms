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
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  cn,
  type DataGridColumn,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Archive, Database, Download, Shuffle, Upload } from "lucide-react";

import {
  parseFeatureFlagImportJson,
  useArchiveFeatureFlagSourceMutation,
  useFeatureFlagsQuery,
  useImportFeatureFlagsMutation,
  useMigrateFeatureFlagsMutation,
  useSwitchFeatureFlagSourceMutation,
  type FeatureFlagConfig,
} from "@/features/config-center/feature-flag-queries";

interface FeatureFlagConfigCenterPageProps {
  onBack: () => void;
}

type Notice = { type: "success" | "error"; text: string } | null;

const sourceOptions = [
  ["config_center", "配置中心"],
  ["file", "TOML 文件"],
  ["environment", "环境变量"],
] as const;

const featureFlagQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: "关键字",
    type: "text",
    placeholder: "开关标识 / 负责人",
    ariaLabel: "搜索功能开关",
  },
  {
    key: "status",
    label: "状态",
    type: "multiSelect",
    options: [
      { label: "启用", value: "enabled" },
      { label: "关闭", value: "disabled" },
    ],
  },
];
const featureFlagCoreQueryFieldKeys = ["keyword", "status"];

const columns: DataGridColumn<FeatureFlagConfig>[] = [
  textColumn("key", "开关标识", 280),
  {
    key: "enabled",
    header: "状态",
    width: 110,
    sortable: true,
    sortValue: (row) => String(row.enabled),
    filterValue: (row) => (row.enabled ? "enabled" : "disabled"),
    copyValue: (row) => (row.enabled ? "启用" : "关闭"),
    filter: { type: "multiSelect", options: [{ label: "启用", value: "enabled" }, { label: "关闭", value: "disabled" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "offline_cached"} label={row.enabled ? "启用" : "关闭"} size="sm" />,
  },
  { ...textColumn("source", "来源", 140), render: (row) => sourceLabel(row.source), sortValue: (row) => sourceLabel(row.source) },
  textColumn("owner", "负责人", 150),
  {
    key: "created_at",
    header: "创建时间",
    width: 140,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.created_at,
    filterValue: (row) => row.created_at,
    copyValue: (row) => row.created_at,
    filter: { type: "text" },
  },
  textColumn("cleanup_by", "清理期限", 140),
];

export function FeatureFlagConfigCenterPage({ onBack }: FeatureFlagConfigCenterPageProps) {
  const flagsQuery = useFeatureFlagsQuery();
  const migrateMutation = useMigrateFeatureFlagsMutation();
  const importMutation = useImportFeatureFlagsMutation();
  const switchMutation = useSwitchFeatureFlagSourceMutation();
  const archiveMutation = useArchiveFeatureFlagSourceMutation();
  const [notice, setNotice] = React.useState<Notice>(null);
  const [targetSource, setTargetSource] = React.useState("config_center");
  const [archiveRef, setArchiveRef] = React.useState("deploy/feature_flags.toml");
  const [importOpen, setImportOpen] = React.useState(false);
  const [importText, setImportText] = React.useState("");
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultFeatureFlagQuery());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultFeatureFlagQuery());
  const flags = flagsQuery.data?.flags ?? [];
  const filteredFlags = React.useMemo(() => filterFeatureFlags(flags, appliedQuery), [flags, appliedQuery]);
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(featureFlagQueryFields, appliedQuery),
    [appliedQuery],
  );
  const busy = migrateMutation.isPending || importMutation.isPending || switchMutation.isPending || archiveMutation.isPending;
  const gridRefreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新功能开关列表",
    disabled: flagsQuery.isFetching,
    onClick: () => {
      void refreshFlags();
    },
  };
  const gridToolbarActions: DataGridToolbarAction[] = [
    {
      key: "json-export",
      label: "JSON",
      description: "导出 JSON",
      icon: <Download className="size-4" aria-hidden />,
      disabled: !flagsQuery.data,
      onClick: () => {
        if (flagsQuery.data) downloadJson(flagsQuery.data);
      },
    },
    {
      key: "json-import",
      label: "导入",
      description: "导入 JSON",
      icon: <Upload className="size-4" aria-hidden />,
      disabled: busy,
      onClick: () => {
        setNotice(null);
        setImportOpen(true);
      },
    },
  ];

  async function act<T>(task: Promise<T>, success: (result: T) => string, fallback: string) {
    setNotice(null);
    try {
      setNotice({ type: "success", text: success(await task) });
    } catch (error) {
      setNotice({ type: "error", text: errorMessage(error, fallback) });
    }
  }

  async function refreshFlags() {
    setNotice(null);
    const result = await flagsQuery.refetch();
    setNotice(
      result.error
        ? { type: "error", text: errorMessage(result.error, "刷新功能开关失败") }
        : { type: "success", text: "功能开关列表已刷新" },
    );
  }

  async function submitImport(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    let nextFlags: FeatureFlagConfig[];
    try {
      nextFlags = parseFeatureFlagImportJson(importText);
    } catch (error) {
      setNotice({ type: "error", text: errorMessage(error, "导入功能开关失败") });
      return;
    }
    await act(
      importMutation.mutateAsync(nextFlags),
      (result) => {
        setImportOpen(false);
        setImportText("");
        return `已导入 ${result.imported_count} 条到 ${result.target}`;
      },
      "导入功能开关失败",
    );
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="配置中心"
        subtitle={`Feature Flag · 读取源：${sourceLabel(flagsQuery.data?.source ?? "unknown")} · ${filteredFlags.length}/${flags.length} 条`}
        actions={
          <Button variant="outline" onClick={onBack}>
            返回
          </Button>
        }
      />
      <NoticePanel notice={notice} />

      <QueryPanel
        fields={featureFlagQueryFields}
        defaultVisibleFieldKeys={featureFlagCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeFeatureFlagQuery(next))}
        onQuery={() => setAppliedQuery(normalizeFeatureFlagQuery(draftQuery))}
        onReset={() => {
          const next = defaultFeatureFlagQuery();
          setDraftQuery(next);
          setAppliedQuery(next);
        }}
      />

      <div className="grid gap-4 lg:grid-cols-[1fr_22rem]">
        <Card className="rounded-lg shadow-sm">
          <CardContent className="p-5">
            <DataGrid
              storageKey="m1.config-center.feature-flags"
              columns={columns}
              data={filteredFlags}
              rowKey={(row) => row.key}
              caption={flagsQuery.isPending ? "加载功能开关..." : undefined}
              emptyTitle={flagsQuery.isError ? "读取功能开关失败" : "暂无功能开关"}
              emptyDescription={flagsQuery.isError ? errorMessage(flagsQuery.error, "请检查后端接口") : "当前导出结果为空，或没有匹配查询条件"}
              exportFileBaseName="功能开关配置中心"
              refreshAction={gridRefreshAction}
              toolbarActions={gridToolbarActions}
              queryState={appliedQuery}
              querySummaryItems={querySummaryItems}
              onApplyQueryState={(queryState) => {
                const next = normalizeFeatureFlagQuery(queryValueFromUnknown(queryState));
                setDraftQuery(next);
                setAppliedQuery(next);
              }}
              onClearQueryState={() => {
                const next = defaultFeatureFlagQuery();
                setDraftQuery(next);
                setAppliedQuery(next);
              }}
            />
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card className="rounded-lg shadow-sm">
            <CardContent className="space-y-4 p-5">
              <h2 className="text-base font-semibold tracking-normal">迁移与读取源</h2>
              <Button type="button" className="w-full justify-start" disabled={busy} onClick={() => void act(migrateMutation.mutateAsync(), (r) => `迁移完成：${r.source} → ${r.target}，${r.migrated_count} 条`, "迁移功能开关失败")}>
                <Database className="size-4" aria-hidden />{migrateMutation.isPending ? "迁移中..." : "从文件源迁移"}
              </Button>
              <label className="grid gap-1 text-xs text-muted-foreground">
                目标读取源
                <select className="h-10 rounded-md border border-input bg-background px-3 text-sm text-foreground shadow-sm" value={targetSource} onChange={(event) => setTargetSource(event.target.value)}>
                  {sourceOptions.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
                </select>
              </label>
              <Button type="button" variant="outline" className="w-full justify-start" disabled={busy} onClick={() => void act(switchMutation.mutateAsync(targetSource), (r) => `读取源已切换为 ${sourceLabel(r.active_source)}`, "切换读取源失败")}>
                <Shuffle className="size-4" aria-hidden />{switchMutation.isPending ? "切换中..." : "切换读取源"}
              </Button>
            </CardContent>
          </Card>

          <Card className="rounded-lg shadow-sm">
            <CardContent className="space-y-4 p-5">
              <h2 className="text-base font-semibold tracking-normal">文件源归档</h2>
              <label className="grid gap-1 text-xs text-muted-foreground">归档引用<Input value={archiveRef} onChange={(event) => setArchiveRef(event.target.value)} /></label>
              <Button type="button" variant="outline" className="w-full justify-start" disabled={busy || !archiveRef.trim()} onClick={() => void act(archiveMutation.mutateAsync(archiveRef.trim()), (r) => `已归档 ${r.archived_source} 到 ${r.archive_ref}`, "归档文件源失败")}>
                <Archive className="size-4" aria-hidden />{archiveMutation.isPending ? "归档中..." : "归档 W1 文件源"}
              </Button>
            </CardContent>
          </Card>
        </div>
      </div>

      <Dialog open={importOpen} onOpenChange={(open) => !importMutation.isPending && setImportOpen(open)}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
          <form className="grid gap-4" onSubmit={submitImport}>
            <DialogHeader><DialogTitle>导入功能开关 JSON</DialogTitle><DialogDescription>支持数组，或包含 flags 数组的导出对象。</DialogDescription></DialogHeader>
            <NoticePanel notice={notice?.type === "error" ? notice : null} />
            <textarea className="min-h-72 rounded-md border border-input bg-background px-3 py-2 font-mono text-sm text-foreground shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring" value={importText} onChange={(event) => setImportText(event.target.value)} aria-label="功能开关 JSON" placeholder={importPlaceholder} />
            <DialogFooter>
              <DialogClose asChild><Button type="button" variant="outline" disabled={importMutation.isPending}>取消</Button></DialogClose>
              <Button type="submit" disabled={importMutation.isPending}><Upload className="size-4" aria-hidden />{importMutation.isPending ? "导入中..." : "确认导入"}</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </section>
  );
}

function textColumn(key: keyof FeatureFlagConfig, header: string, width: number): DataGridColumn<FeatureFlagConfig> {
  return { key, header, width, minWidth: Math.max(width - 40, 100), mono: key === "key", sortable: true, sortValue: (row) => row[key], filterValue: (row) => row[key], copyValue: (row) => String(row[key]), filter: { type: "text" } };
}

function NoticePanel({ notice }: { notice: Notice }) {
  if (!notice) return null;
  const success = notice.type === "success";
  return <div className={cn("rounded-md border px-3 py-2 text-sm", success ? "border-wms-success/30 bg-wms-success/10 text-wms-success" : "border-destructive/30 bg-destructive/10 text-destructive")} role={success ? "status" : "alert"}>{notice.text}</div>;
}

function sourceLabel(value: string) {
  return sourceOptions.find(([source]) => source === value)?.[1] ?? value;
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function downloadJson(payload: unknown) {
  if (!payload || typeof document === "undefined") return;
  const url = URL.createObjectURL(new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = "feature-flags.json";
  anchor.click();
  URL.revokeObjectURL(url);
}

function defaultFeatureFlagQuery(): QueryPanelValue {
  return { keyword: "", status: [] };
}

function normalizeFeatureFlagQuery(value: QueryPanelValue): QueryPanelValue {
  return {
    keyword: typeof value.keyword === "string" ? value.keyword : "",
    status: Array.isArray(value.status) ? value.status.filter((item): item is string => typeof item === "string") : [],
  };
}

function filterFeatureFlags(flags: FeatureFlagConfig[], query: QueryPanelValue): FeatureFlagConfig[] {
  const keyword = (typeof query.keyword === "string" ? query.keyword : "").trim().toLowerCase();
  const statuses = new Set(
    Array.isArray(query.status) ? query.status.filter((item): item is string => typeof item === "string") : [],
  );
  return flags.filter((flag) => {
    const statusValue = flag.enabled ? "enabled" : "disabled";
    const haystack = [flag.key, flag.owner, flag.source, sourceLabel(flag.source)].join(" ").toLowerCase();
    return (!keyword || haystack.includes(keyword)) && (!statuses.size || statuses.has(statusValue));
  });
}

function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as QueryPanelValue) : {};
}

const importPlaceholder = JSON.stringify({ flags: [{ key: "m4_outbound_v2_picker_stage1", owner: "platform", created_at: "2026-07-03", cleanup_by: "2026-10-01", enabled: false, source: "config_center" }] }, null, 2);
