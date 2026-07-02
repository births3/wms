import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  Input,
  PageHeader,
  StatusBadge,
  buildLocationBatchPreview,
  type DataGridColumn,
  type LocationBatchRange,
  type StatusKey,
  validateLocationBatchRange,
} from "@wms/ui";
import { ArrowLeft, Plus, RefreshCw, Search, Upload } from "lucide-react";

import {
  batchCreateLocations,
  createProduct,
  useMasterDataRowsQuery,
  type CreateProductRequest,
  type LocationMasterDataFields,
  type MasterDataRow,
  type MasterDataViewId,
} from "@/features/master-data/master-data-queries";
import {
  LocationBatchDialog,
  defaultLocationBatchType,
  initialLocationBatchRange,
} from "./LocationBatchDialog";
import { ProductCreateDialog } from "./ProductCreateDialog";
import { ProductEditDialog } from "./ProductEditDialog";
import { productColumns } from "./ProductEditTable";
import {
  masterDataColumns,
  productActionLabels,
  productTableClassName,
} from "./m1-product-page-model";
import { M1SystemDictionaryPage } from "./SystemDictionaryPage";
import { useProductEditDialog } from "./use-product-edit-dialog";
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
    key: "createdAt",
    header: "创建时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.createdAt,
    filterValue: (row) => row.createdAt,
    copyValue: (row) => formatDateTime(row.createdAt),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.createdAt),
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

const locationColumns: DataGridColumn<MasterDataRow>[] = [
  {
    key: "owner",
    header: "货主",
    width: 230,
    minWidth: 200,
    sortable: true,
    sortValue: (row) => locationValue(row, "owner"),
    filterValue: (row) => locationValue(row, "owner"),
    copyValue: (row) => locationValue(row, "owner"),
    filter: { type: "text" },
  },
  {
    key: "warehouse",
    header: "仓库 / 库区",
    width: 280,
    minWidth: 240,
    filterValue: (row) => `${locationValue(row, "warehouse")} ${locationValue(row, "zone")}`,
    copyValue: (row) => `仓库 ${locationValue(row, "warehouse")} / 库区 ${locationValue(row, "zone")}`,
    filter: { type: "text" },
    render: (row) => (
      <FieldText label={`库区 ${locationValue(row, "zone")}`} value={`仓库 ${locationValue(row, "warehouse")}`} />
    ),
  },
  {
    key: "code",
    header: "库位编码",
    mono: true,
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.code,
    filterValue: (row) => row.code,
    copyValue: (row) => row.code,
    filter: { type: "text" },
    render: (row) => <span className="text-primary">{row.code}</span>,
  },
  {
    key: "coordinate",
    header: "区域 / 排列层",
    width: 210,
    minWidth: 190,
    filterValue: (row) =>
      `${locationValue(row, "area")} ${locationValue(row, "rowNo")} ${locationValue(row, "columnNo")} ${locationValue(row, "layerNo")}`,
    copyValue: (row) =>
      `区域 ${locationValue(row, "area")} / 排 ${locationValue(row, "rowNo")} / 列 ${locationValue(row, "columnNo")} / 层 ${locationValue(row, "layerNo")}`,
    filter: { type: "text" },
    render: (row) => (
      <FieldText
        label={`排 ${locationValue(row, "rowNo")} / 列 ${locationValue(row, "columnNo")} / 层 ${locationValue(row, "layerNo")}`}
        value={`区域 ${locationValue(row, "area")}`}
      />
    ),
  },
  {
    key: "locationType",
    header: "库位类型",
    width: 140,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => locationValue(row, "locationType"),
    filterValue: (row) => locationValue(row, "locationType"),
    copyValue: (row) => locationValue(row, "locationType"),
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
        { label: "可用", value: "active" },
        { label: "停用/锁定", value: "disabled" },
        { label: "其他", value: "other" },
      ],
    },
    render: (row) => (
      <StatusBadge status={statusKey(row.status)} label={row.statusLabel} size="sm" />
    ),
  },
  {
    key: "volume",
    header: "已用 / 最大体积",
    width: 180,
    minWidth: 160,
    filterValue: (row) => locationValue(row, "volume"),
    copyValue: (row) => locationValue(row, "volume"),
    filter: { type: "text" },
  },
  {
    key: "maxSku",
    header: "最大 SKU",
    width: 130,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => Number.parseInt(locationValue(row, "maxSku"), 10) || 0,
    filterValue: (row) => locationValue(row, "maxSku"),
    copyValue: (row) => locationValue(row, "maxSku"),
    filter: { type: "text" },
  },
  {
    key: "createdAt",
    header: "创建时间",
    width: 190,
    minWidth: 170,
    sortable: true,
    sortValue: (row) => row.createdAt,
    filterValue: (row) => row.createdAt,
    copyValue: (row) => formatDateTime(row.createdAt),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.createdAt),
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
  if (viewId === "m1-system-dictionary") {
    return <M1SystemDictionaryPage meta={masterDataViewMeta[viewId]} onBack={onBack} />;
  }

  return <M1MasterDataGridPage viewId={viewId} onBack={onBack} />;
}

function M1MasterDataGridPage({ viewId, onBack }: M1MasterDataPageProps) {
  const meta = masterDataViewMeta[viewId];
  const rowsQuery = useMasterDataRowsQuery(viewId);
  const [keyword, setKeyword] = React.useState("");
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const productEdit = useProductEditDialog({
    refetchRows: rowsQuery.refetch,
    onSaved: (productCode) => setLastEvent(`${productCode} 已保存`),
  });
  const [productCreateOpen, setProductCreateOpen] = React.useState(false);
  const [locationBatchOpen, setLocationBatchOpen] = React.useState(false);
  const [locationBatchRange, setLocationBatchRange] =
    React.useState<LocationBatchRange>(initialLocationBatchRange);
  const [locationBatchType, setLocationBatchType] = React.useState(defaultLocationBatchType);
  const [locationBatchMessage, setLocationBatchMessage] = React.useState<string | null>(null);
  const [locationBatchSubmitting, setLocationBatchSubmitting] = React.useState(false);
  const [locationBatchScopeKey, setLocationBatchScopeKey] = React.useState("");
  const normalizedKeyword = keyword.trim().toLowerCase();
  const rows = React.useMemo(() => {
    const data = rowsQuery.data ?? [];
    if (!normalizedKeyword) return data;
    return data.filter((row) => row.searchText.includes(normalizedKeyword));
  }, [normalizedKeyword, rowsQuery.data]);
  const activeCount = rows.filter((row) => statusKey(row.status) === "completed").length;
  const locationAreas = React.useMemo(() => {
    if (viewId !== "m1-locations") return [];
    const areas = new Set<string>();
    for (const row of rowsQuery.data ?? []) {
      if (row.locationFields?.area && row.locationFields.area !== "-") areas.add(row.locationFields.area);
    }
    return Array.from(areas).sort((left, right) => left.localeCompare(right, "zh-CN"));
  }, [rowsQuery.data, viewId]);
  const locationBatchErrors = React.useMemo(
    () => validateLocationBatchRange(locationBatchRange),
    [locationBatchRange],
  );
  const locationBatchPreview = React.useMemo(
    () => buildLocationBatchPreview(locationBatchRange),
    [locationBatchRange],
  );
  const locationBatchScopes = React.useMemo(() => {
    if (viewId !== "m1-locations") return [];
    const scopes = new Map<
      string,
      { key: string; label: string; warehouseId: string; zoneId: string; ownerId: string | null }
    >();
    for (const row of rowsQuery.data ?? []) {
      const fields = row.locationFields;
      if (!fields || fields.warehouse === "-" || fields.zone === "-") continue;
      const ownerId = fields.owner === "-" ? null : fields.owner;
      const key = `${fields.warehouse}:${fields.zone}:${ownerId ?? "none"}`;
      if (scopes.has(key)) continue;
      scopes.set(key, {
        key,
        label: `仓库 ${shortId(fields.warehouse)} / 库区 ${shortId(fields.zone)}`,
        warehouseId: fields.warehouse,
        zoneId: fields.zone,
        ownerId,
      });
    }
    return Array.from(scopes.values());
  }, [rowsQuery.data, viewId]);
  const locationBatchScope =
    locationBatchScopes.find((scope) => scope.key === locationBatchScopeKey) ??
    locationBatchScopes[0] ??
    null;
  const productActions = productActionLabels(viewId);
  const baseGridColumns = masterDataColumns(viewId, columns, locationColumns);
  const gridColumns =
    viewId === "m1-products" ? productColumns(baseGridColumns, productEdit.openDialog) : baseGridColumns;

  React.useEffect(() => {
    if (viewId !== "m1-locations") return;
    if (locationBatchScopes.some((scope) => scope.key === locationBatchScopeKey)) return;
    setLocationBatchScopeKey(locationBatchScopes[0]?.key ?? "");
  }, [locationBatchScopeKey, locationBatchScopes, viewId]);

  async function refreshRows() {
    await rowsQuery.refetch();
    setLastEvent(`${meta.title} 已刷新`);
  }

  async function submitProductCreate(request: CreateProductRequest) {
    const created = await createProduct(request);
    await rowsQuery.refetch();
    setLastEvent(`${created.code} 已新建`);
  }

  function triggerProductBatchImport() {
    setLastEvent("批量导入入口已记录，导入记录来源将显示为批量导入");
  }

  function updateLocationBatchRange(patch: Partial<LocationBatchRange>) {
    setLocationBatchRange((value) => ({ ...value, ...patch }));
    setLocationBatchMessage(null);
  }

  async function confirmLocationBatchPreview() {
    if (locationBatchErrors.length > 0) return;
    if (!locationBatchScope) {
      setLocationBatchMessage("缺少可用仓库和库区上下文，请先确认后端已有库位基础数据。");
      return;
    }
    setLocationBatchSubmitting(true);
    setLocationBatchMessage(null);
    try {
      const createdRows = await batchCreateLocations({
        warehouse_id: locationBatchScope.warehouseId,
        zone_id: locationBatchScope.zoneId,
        area_code: locationBatchRange.areaCode.trim().toUpperCase(),
        row_start: locationBatchRange.rowStart,
        row_end: locationBatchRange.rowEnd,
        column_start: locationBatchRange.columnStart,
        column_end: locationBatchRange.columnEnd,
        layer_start: locationBatchRange.layerStart,
        layer_end: locationBatchRange.layerEnd,
        max_volume_cm3: 5_000_000,
        max_sku_count: 1,
        location_type: locationBatchType,
        bound_owner_id: locationBatchScope.ownerId,
      });
      await rowsQuery.refetch();
      setLocationBatchOpen(false);
      setLastEvent(`已新增 ${createdRows.length} 个库位`);
    } catch (error) {
      setLocationBatchMessage(error instanceof Error ? error.message : "批量新增库位失败");
    } finally {
      setLocationBatchSubmitting(false);
    }
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
            {productActions.includes("新建商品") && (
              <Button type="button" onClick={() => setProductCreateOpen(true)}>
                <Plus className="size-4" aria-hidden />
                新建商品
              </Button>
            )}
            {productActions.includes("批量导入") && (
              <Button type="button" variant="outline" onClick={triggerProductBatchImport}>
                <Upload className="size-4" aria-hidden />
                批量导入
              </Button>
            )}
            {viewId === "m1-locations" && (
              <Button type="button" onClick={() => setLocationBatchOpen(true)}>
                <Plus className="size-4" aria-hidden />
                批量新增库位
              </Button>
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
        columns={gridColumns}
        data={rows}
        rowKey={(row) => row.id}
        caption={rowsQuery.isPending ? "加载基础档案..." : undefined}
        emptyTitle={meta.emptyTitle}
        storageKey={meta.storageKey}
        tableClassName={productTableClassName(viewId)}
      />

      <ProductEditDialog
        form={productEdit.form}
        pending={productEdit.pending}
        error={productEdit.error}
        onFormChange={productEdit.updateForm}
        onOpenChange={productEdit.closeDialog}
        onSubmit={productEdit.submit}
      />

      {viewId === "m1-products" && (
        <ProductCreateDialog
          open={productCreateOpen}
          onOpenChange={setProductCreateOpen}
          onCreate={submitProductCreate}
        />
      )}

      {viewId === "m1-locations" && (
        <LocationBatchDialog
          open={locationBatchOpen}
          onOpenChange={setLocationBatchOpen}
          scopeOptions={locationBatchScopes.map((scope) => ({
            value: scope.key,
            label: scope.label,
          }))}
          scopeValue={locationBatchScope?.key ?? ""}
          onScopeValueChange={setLocationBatchScopeKey}
          areaOptions={locationAreas}
          range={locationBatchRange}
          onRangeChange={updateLocationBatchRange}
          locationType={locationBatchType}
          onLocationTypeChange={setLocationBatchType}
          errors={locationBatchErrors}
          preview={locationBatchPreview}
          message={locationBatchMessage}
          confirmDisabled={!locationBatchScope || locationBatchSubmitting}
          confirmLabel={locationBatchSubmitting ? "提交中..." : "确认新增"}
          onConfirm={confirmLocationBatchPreview}
        />
      )}
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

function locationValue(row: MasterDataRow, key: keyof LocationMasterDataFields) {
  return row.locationFields?.[key] ?? "-";
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

function shortId(value: string) {
  return value.length > 8 ? value.slice(0, 8) : value;
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value || "-";
  return date.toLocaleString("zh-CN", { hour12: false });
}
