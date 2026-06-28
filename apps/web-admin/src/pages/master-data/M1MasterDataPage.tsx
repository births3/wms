import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  Input,
  PageHeader,
  StatusBadge,
  type DataGridColumn,
  type StatusKey,
} from "@wms/ui";
import { ArrowLeft, RefreshCw, Search } from "lucide-react";

import {
  useMasterDataRowsQuery,
  type MasterDataRow,
  type MasterDataViewId,
} from "@/features/master-data/master-data-queries";

export type { MasterDataViewId } from "@/features/master-data/master-data-queries";

export const masterDataViewMeta: Record<
  MasterDataViewId,
  { title: string; subtitle: string; emptyTitle: string; storageKey: string }
> = {
  "m1-products": {
    title: "M1 商品档案",
    subtitle: "商品编码、规格、批准文号与储存条件",
    emptyTitle: "暂无商品档案",
    storageKey: "m1-products-datagrid",
  },
  "m1-suppliers": {
    title: "M1 供应商档案",
    subtitle: "供应商编码、资质证号与联系人",
    emptyTitle: "暂无供应商档案",
    storageKey: "m1-suppliers-datagrid",
  },
  "m1-customers": {
    title: "M1 客户档案",
    subtitle: "客户/门店编码、名称与资质信息",
    emptyTitle: "暂无客户档案",
    storageKey: "m1-customers-datagrid",
  },
  "m1-warehouses": {
    title: "M1 仓库管理",
    subtitle: "仓库编码、名称与启停状态",
    emptyTitle: "暂无仓库档案",
    storageKey: "m1-warehouses-datagrid",
  },
  "m1-locations": {
    title: "M1 库位管理",
    subtitle: "库位编码、容量、类型与状态",
    emptyTitle: "暂无库位档案",
    storageKey: "m1-locations-datagrid",
  },
  "m1-system-dictionary": {
    title: "M1 系统字典",
    subtitle: "document_type 字典项与运行参数",
    emptyTitle: "暂无 document_type 字典项",
    storageKey: "m1-system-dictionary-datagrid",
  },
};

interface M1MasterDataPageProps {
  viewId: MasterDataViewId;
  onBack: () => void;
}

const columns: DataGridColumn<MasterDataRow>[] = [
  {
    key: "code",
    header: "编码",
    mono: true,
    width: 220,
    minWidth: 180,
    sortable: true,
    sortValue: (row) => row.code,
    filterValue: (row) => row.code,
    copyValue: (row) => row.code,
    filter: { type: "text" },
    render: (row) => <span className="text-primary">{row.code}</span>,
  },
  {
    key: "name",
    header: "名称",
    width: 240,
    minWidth: 200,
    sortable: true,
    sortValue: (row) => row.name,
    filterValue: (row) => row.name,
    copyValue: (row) => row.name,
    filter: { type: "text" },
  },
  {
    key: "status",
    header: "状态",
    width: 130,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => row.statusLabel,
    filterValue: (row) => statusFilterValue(row.status),
    copyValue: (row) => row.statusLabel,
    filter: {
      type: "multiSelect",
      options: [
        { label: "启用/可用", value: "active" },
        { label: "停用", value: "disabled" },
        { label: "其他", value: "other" },
      ],
    },
    render: (row) => (
      <StatusBadge status={statusKey(row.status)} label={row.statusLabel} size="sm" />
    ),
  },
  {
    key: "primary",
    header: "关键字段",
    width: 230,
    minWidth: 190,
    filterValue: (row) => `${row.primaryLabel} ${row.primaryValue}`,
    copyValue: (row) => `${row.primaryLabel}: ${row.primaryValue}`,
    filter: { type: "text" },
    render: (row) => <FieldText label={row.primaryLabel} value={row.primaryValue} />,
  },
  {
    key: "secondary",
    header: "扩展字段",
    width: 240,
    minWidth: 200,
    filterValue: (row) => `${row.secondaryLabel} ${row.secondaryValue}`,
    copyValue: (row) => `${row.secondaryLabel}: ${row.secondaryValue}`,
    filter: { type: "text" },
    render: (row) => <FieldText label={row.secondaryLabel} value={row.secondaryValue} />,
  },
  {
    key: "extra",
    header: "运行字段",
    width: 260,
    minWidth: 220,
    filterValue: (row) => `${row.extraLabel} ${row.extraValue}`,
    copyValue: (row) => `${row.extraLabel}: ${row.extraValue}`,
    filter: { type: "text" },
    render: (row) => <FieldText label={row.extraLabel} value={row.extraValue} />,
  },
  {
    key: "updatedAt",
    header: "更新时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.updatedAt,
    filterValue: (row) => row.updatedAt,
    copyValue: (row) => formatDateTime(row.updatedAt),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.updatedAt),
  },
];

export function M1MasterDataPage({ viewId, onBack }: M1MasterDataPageProps) {
  const meta = masterDataViewMeta[viewId];
  const rowsQuery = useMasterDataRowsQuery(viewId);
  const [keyword, setKeyword] = React.useState("");
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const normalizedKeyword = keyword.trim().toLowerCase();
  const rows = React.useMemo(() => {
    const data = rowsQuery.data ?? [];
    if (!normalizedKeyword) return data;
    return data.filter((row) => row.searchText.includes(normalizedKeyword));
  }, [normalizedKeyword, rowsQuery.data]);
  const activeCount = rows.filter((row) => statusKey(row.status) === "completed").length;

  async function refreshRows() {
    await rowsQuery.refetch();
    setLastEvent(`${meta.title} 已刷新`);
  }

  return (
    <section className="mx-auto flex w-full max-w-[1680px] flex-col gap-5 px-4 py-8 xl:px-6">
      <PageHeader
        title={meta.title}
        subtitle={meta.subtitle}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            {lastEvent && (
              <span className="text-sm text-muted-foreground" role="status">
                {lastEvent}
              </span>
            )}
            <Button type="button" variant="outline" onClick={refreshRows}>
              <RefreshCw className="size-4" aria-hidden />
              刷新
            </Button>
            <Button type="button" variant="outline" onClick={onBack}>
              <ArrowLeft className="size-4" aria-hidden />
              返回工作台
            </Button>
          </div>
        }
      />

      <div className="grid gap-3 md:grid-cols-3">
        <Metric label="当前列表" value={rows.length} />
        <Metric label="启用/可用" value={activeCount} />
        <Metric label="API 返回" value={rowsQuery.data?.length ?? 0} />
      </div>

      <Card className="rounded-lg shadow-sm">
        <CardContent className="grid gap-3 p-4 md:grid-cols-[minmax(16rem,24rem)_auto] md:items-center">
          <label className="relative block">
            <Search className="pointer-events-none absolute left-2.5 top-2.5 size-4 text-muted-foreground" aria-hidden />
            <Input
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              placeholder="搜索编码、名称、状态或字段"
              aria-label="搜索基础档案"
              className="h-9 pl-9 text-sm"
            />
          </label>
          <Button
            type="button"
            variant="outline"
            className="justify-self-start"
            onClick={() => setKeyword("")}
          >
            清空
          </Button>
        </CardContent>
      </Card>

      {rowsQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {rowsQuery.error.message}
        </div>
      )}

      <DataGrid
        columns={columns}
        data={rows}
        rowKey={(row) => row.id}
        caption={rowsQuery.isPending ? "加载基础档案..." : undefined}
        emptyTitle={meta.emptyTitle}
        storageKey={meta.storageKey}
        tableClassName="min-w-[1460px]"
      />
    </section>
  );
}

function FieldText({ label, value }: { label: string; value: string }) {
  return (
    <div className="text-sm">
      <div className="font-medium">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <Card className="rounded-lg shadow-sm">
      <CardContent className="p-4">
        <p className="text-xs font-medium text-muted-foreground">{label}</p>
        <p className="mt-2 text-2xl font-semibold tracking-normal text-foreground">{value}</p>
      </CardContent>
    </Card>
  );
}

function statusKey(status: string): StatusKey {
  if (status === "active" || status === "available") return "completed";
  if (status === "disabled" || status === "inactive" || status === "locked") return "isolated";
  return "pending";
}

function statusFilterValue(status: string) {
  if (status === "active" || status === "available") return "active";
  if (status === "disabled" || status === "inactive" || status === "locked") return "disabled";
  return "other";
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}
