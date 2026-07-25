import * as React from "react";
import {
  DataGrid,
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
  buildLocationBatchPreview,
  type DataGridCreateAction,
  type DataGridDisableAction,
  type DataGridEditAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type LocationBatchRange,
  type QueryPanelField,
  type QueryPanelValue,
  validateLocationBatchRange,
} from "@wms/ui";
import { Plus, Upload } from "lucide-react";

import {
  batchCreateCustomers,
  batchCreateSuppliers,
  batchCreateLocations,
  createCustomer,
  createSupplier,
  useMasterDataRowsQuery,
  useSystemDictionaryItemOptionsQuery,
  type CreateCustomerRequest,
  type CreateSupplierRequest,
  type MasterDataRow,
  type MasterDataViewId,
} from "@/features/master-data/master-data-queries";
import {
  LocationBatchDialog,
  defaultLocationBatchType,
  initialLocationBatchRange,
} from "./LocationBatchDialog";
import {
  MasterDataCrudDialog,
  crudTargetForRow,
  disableMasterDataCrudRow,
  isMasterDataCrudView,
  masterDataCrudColumns,
  saveMasterDataCrudForm,
  type LocationScopeOption,
  type MasterDataCrudForm,
  type MasterDataCrudTarget,
} from "./MasterDataCrudDialog";
import {
  MasterDataSourceActions,
  type MasterDataSourceActionsHandle,
} from "./MasterDataSourceActions";
import { productColumns } from "./ProductEditTable";
import {
  baseMasterDataColumns,
  locationMasterDataColumns,
} from "./M1MasterDataColumns";
import {
  masterDataColumns,
  productTableClassName,
} from "./m1-product-page-model";
import { M1SystemDictionaryPage } from "./SystemDictionaryPage";
import type { CurrentUser } from "@/features/auth/auth-queries";
export type { MasterDataViewId } from "@/features/master-data/master-data-queries";
export const masterDataViewMeta: Record<
  MasterDataViewId,
  {
    title: string;
    subtitle: string;
    emptyTitle: string;
    emptyDescription: string;
    storageKey: string;
  }
> = {
  "m1-products": {
    title: "M1 商品档案",
    subtitle: "ERP 权威商品投影；本页只读，变更由 H8 商品消息同步",
    emptyTitle: "暂无商品档案",
    emptyDescription: "当前筛选条件下没有商品，请调整关键字或清空筛选。",
    storageKey: "m1-products-datagrid",
  },
  "m1-business-partners": {
    title: "M1 客商档案",
    subtitle: "供应商、客户/门店与资质来源",
    emptyTitle: "暂无客商档案",
    emptyDescription: "当前筛选条件下没有客商，请调整关键字或清空筛选。",
    storageKey: "m1-business-partners-datagrid",
  },
  "m1-warehouses": {
    title: "M1 仓库管理",
    subtitle: "仓库编码、名称与启停状态",
    emptyTitle: "暂无仓库档案",
    emptyDescription: "当前筛选条件下没有仓库，请调整关键字或清空筛选。",
    storageKey: "m1-warehouses-datagrid",
  },
  "m1-zones": {
    title: "M1 库区管理",
    subtitle: "库区编码、温区、色标与启停状态",
    emptyTitle: "暂无库区档案",
    emptyDescription: "当前筛选条件下没有库区，请调整关键字或清空筛选。",
    storageKey: "m1-zones-datagrid",
  },
  "m1-locations": {
    title: "M1 库位管理",
    subtitle: "库位编码、容量、类型与状态",
    emptyTitle: "暂无库位档案",
    emptyDescription: "当前筛选条件下没有库位，请调整关键字或清空筛选。",
    storageKey: "m1-locations-datagrid",
  },
  "m1-system-dictionary": {
    title: "M1 系统字典",
    subtitle: "单据类型、特殊药品分类等系统字典项",
    emptyTitle: "暂无系统字典项",
    emptyDescription: "当前分类下没有可展示字典项。",
    storageKey: "m1-system-dictionary-datagrid",
  },
};

const m1QueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: "关键字",
    type: "text",
    placeholder: "搜索编码、名称、状态或字段",
    ariaLabel: "搜索基础档案",
  },
];
const m1CoreQueryFieldKeys = ["keyword"];

interface M1MasterDataPageProps {
  viewId: MasterDataViewId;
  currentUser: CurrentUser;
  onBack: () => void;
}

export function M1MasterDataPage({ viewId, currentUser }: M1MasterDataPageProps) {
  if (viewId === "m1-system-dictionary") {
    return <M1SystemDictionaryPage meta={masterDataViewMeta[viewId]} />;
  }

  return <M1MasterDataGridPage currentUser={currentUser} viewId={viewId} />;
}

function M1MasterDataGridPage({ currentUser, viewId }: Pick<M1MasterDataPageProps, "currentUser" | "viewId">) {
  const meta = masterDataViewMeta[viewId];
  const canWrite = currentUser.permissions.includes("m1.master_data.write");
  const rowsQuery = useMasterDataRowsQuery(viewId);
  const warehouseRowsQuery = useMasterDataRowsQuery("m1-warehouses", viewId === "m1-zones");
  const zoneRowsQuery = useMasterDataRowsQuery("m1-zones", viewId === "m1-locations");
  const locationTypeOptionsQuery = useSystemDictionaryItemOptionsQuery(
    "location_type",
    viewId === "m1-locations",
  );
  const locationTypeOptions = locationTypeOptionsQuery.data ?? [];
  const temperatureZoneOptions = useSystemDictionaryItemOptionsQuery("temperature_zone", viewId === "m1-zones").data ?? [];
  const qualityColorOptions = useSystemDictionaryItemOptionsQuery("quality_color", viewId === "m1-zones").data ?? [];
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultM1QueryValue());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultM1QueryValue());
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const [rowActionError, setRowActionError] = React.useState<string | null>(null);
  const [disablingId, setDisablingId] = React.useState<string | null>(null);
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [crudTarget, setCrudTarget] = React.useState<MasterDataCrudTarget | null>(null);
  const supplierActionsRef = React.useRef<MasterDataSourceActionsHandle>(null);
  const customerActionsRef = React.useRef<MasterDataSourceActionsHandle>(null);
  const [locationBatchOpen, setLocationBatchOpen] = React.useState(false);
  const [locationBatchRange, setLocationBatchRange] =
    React.useState<LocationBatchRange>(initialLocationBatchRange);
  const [locationBatchType, setLocationBatchType] = React.useState(defaultLocationBatchType);
  const [locationBatchMessage, setLocationBatchMessage] = React.useState<string | null>(null);
  const [locationBatchSubmitting, setLocationBatchSubmitting] = React.useState(false);
  const [locationBatchScopeKey, setLocationBatchScopeKey] = React.useState("");
  const normalizedKeyword = queryString(appliedQuery.keyword).trim().toLowerCase();
  const rows = React.useMemo(() => {
    const data = rowsQuery.data ?? [];
    if (!normalizedKeyword) return data;
    return data.filter((row) => row.searchText.includes(normalizedKeyword));
  }, [normalizedKeyword, rowsQuery.data]);
  const m1QuerySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(m1QueryFields, appliedQuery),
    [appliedQuery],
  );
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
    const scopes = new Map<string, LocationScopeOption>();
    for (const row of zoneRowsQuery.data ?? []) {
      const fields = row.zoneFields;
      if (!fields || fields.warehouseId === "-" || fields.zoneId === "-") continue;
      const ownerId = fields.owner === "-" ? null : fields.owner;
      const key = `${fields.warehouseId}:${fields.zoneId}:${ownerId ?? "none"}`;
      if (scopes.has(key)) continue;
      scopes.set(key, {
        key,
        label: `仓库 ${fields.warehouse} / 库区 ${row.code} · ${row.name}`,
        warehouseId: fields.warehouseId,
        zoneId: fields.zoneId,
        ownerId,
      });
    }
    return Array.from(scopes.values());
  }, [zoneRowsQuery.data, viewId]);
  const locationBatchScope =
    locationBatchScopes.find((scope) => scope.key === locationBatchScopeKey) ??
    locationBatchScopes[0] ??
    null;
  const baseGridColumns = masterDataColumns(viewId, baseMasterDataColumns, locationMasterDataColumns);
  const gridColumns =
    viewId === "m1-products"
      ? productColumns(baseGridColumns)
      : isMasterDataCrudView(viewId)
        ? masterDataCrudColumns(baseGridColumns, viewId, openCrudEdit, disableCrudRow, disablingId)
        : baseGridColumns;

  React.useEffect(() => {
    setSelectedRowKeys([]);
    setRowActionError(null);
  }, [viewId]);

  React.useEffect(() => {
    if (viewId !== "m1-locations") return;
    if (locationBatchScopes.some((scope) => scope.key === locationBatchScopeKey)) return;
    setLocationBatchScopeKey(locationBatchScopes[0]?.key ?? "");
  }, [locationBatchScopeKey, locationBatchScopes, viewId]);

  React.useEffect(() => {
    if (viewId !== "m1-locations" || locationTypeOptions.length === 0) return;
    if (locationTypeOptions.some(([value]) => value === locationBatchType)) return;
    setLocationBatchType(locationTypeOptions[0][0]);
  }, [locationBatchType, locationTypeOptions, viewId]);

  async function refreshRows() {
    await rowsQuery.refetch();
    setRowActionError(null);
    setLastEvent(`${meta.title} 已刷新`);
  }

  async function createSupplierFromDialog(request: CreateSupplierRequest) {
    const created = await createSupplier(request);
    await rowsQuery.refetch();
    setLastEvent(`${created.code} 已新建供应商`);
  }

  async function importSuppliersFromDialog(requests: CreateSupplierRequest[]) {
    const createdRows = await batchCreateSuppliers(requests);
    await rowsQuery.refetch();
    setLastEvent(`已批量导入 ${createdRows.length} 个供应商`);
  }

  async function createCustomerFromDialog(request: CreateCustomerRequest) {
    const created = await createCustomer(request);
    await rowsQuery.refetch();
    setLastEvent(`${created.code} 已新建客户`);
  }

  async function importCustomersFromDialog(requests: CreateCustomerRequest[]) {
    const createdRows = await batchCreateCustomers(requests);
    await rowsQuery.refetch();
    setLastEvent(`已批量导入 ${createdRows.length} 个客户`);
  }

  function openCrudEdit(row: MasterDataRow) {
    if (!isMasterDataCrudView(viewId)) return;
    setRowActionError(null);
    setCrudTarget(crudTargetForRow(viewId, row));
  }

  async function disableCrudRow(row: MasterDataRow) {
    if (!isMasterDataCrudView(viewId)) return;
    setDisablingId(row.id);
    setRowActionError(null);
    try {
      const saved = await disableMasterDataCrudRow(viewId, row);
      await rowsQuery.refetch();
      setLastEvent(`${saved.code} 已停用`);
    } catch (error) {
      setRowActionError(error instanceof Error ? error.message : "停用基础档案失败");
    } finally {
      setDisablingId(null);
    }
  }

  async function submitCrudForm(form: MasterDataCrudForm) {
    const saved = await saveMasterDataCrudForm(form, locationBatchScopes);
    await rowsQuery.refetch();
    setRowActionError(null);
    setLastEvent(`${saved.code} ${form.mode === "create" ? "已新建" : "已保存"}`);
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

  function selectedRowFrom(keys: string[]) {
    if (keys.length !== 1) return null;
    return rows.find((row) => row.id === keys[0]) ?? null;
  }

  function selectedCrudRowFrom(keys: string[]) {
    const row = selectedRowFrom(keys);
    return row && isMasterDataCrudView(viewId) ? row : null;
  }

  const gridRefreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: `刷新${meta.title}列表`,
    onClick: () => {
      void refreshRows();
    },
  };
  const gridCreateAction: DataGridCreateAction | undefined = canWrite
    ? viewId === "m1-warehouses"
        ? {
            label: "新增",
            description: "新建仓库",
            onClick: () => setCrudTarget({ kind: "warehouse", mode: "create" }),
          }
        : viewId === "m1-zones"
          ? {
              label: "新增",
              description: "新建库区",
              onClick: () => setCrudTarget({ kind: "zone", mode: "create" }),
            }
        : viewId === "m1-locations"
          ? {
              label: "新增",
              description: "新建库位",
              onClick: () => setCrudTarget({ kind: "location", mode: "create" }),
            }
          : undefined
    : undefined;
  const gridEditAction: DataGridEditAction | undefined = canWrite &&
    isMasterDataCrudView(viewId)
      ? {
          label: "修改",
          description: "修改选中档案",
          disabled: ({ selectedRowKeys: keys }) => keys.length !== 1,
          onClick: ({ selectedRowKeys: keys }) => {
            const row = selectedRowFrom(keys);
            if (!row) return;
            openCrudEdit(row);
          },
        }
      : undefined;
  const gridDisableAction: DataGridDisableAction | undefined = canWrite && isMasterDataCrudView(viewId)
    ? {
        label: "停用",
        description: "停用选中档案",
        disabled: ({ selectedRowKeys: keys }) => {
          const row = selectedCrudRowFrom(keys);
          return !row || disablingId === row.id || ["disabled", "inactive"].includes(row.status);
        },
        onClick: ({ selectedRowKeys: keys }) => {
          const row = selectedCrudRowFrom(keys);
          if (row && window.confirm(`确认停用「${row.name}」？`)) void disableCrudRow(row);
        },
      }
    : undefined;
  const gridToolbarActions: DataGridToolbarAction[] = canWrite ? [
    ...(viewId === "m1-business-partners"
      ? [
          {
            key: "supplier-create",
            label: "供应",
            description: "新建供应商",
            icon: <Plus className="size-4" aria-hidden />,
            onClick: () => supplierActionsRef.current?.openCreate(),
          },
          {
            key: "supplier-import",
            label: "供入",
            description: "批量导入供应商",
            icon: <Upload className="size-4" aria-hidden />,
            onClick: () => supplierActionsRef.current?.openImport(),
          },
          {
            key: "customer-create",
            label: "客户",
            description: "新建客户",
            icon: <Plus className="size-4" aria-hidden />,
            onClick: () => customerActionsRef.current?.openCreate(),
          },
          {
            key: "customer-import",
            label: "客入",
            description: "批量导入客户",
            icon: <Upload className="size-4" aria-hidden />,
            onClick: () => customerActionsRef.current?.openImport(),
          },
        ]
      : []),
    ...(viewId === "m1-locations"
      ? [
          {
            key: "location-batch-create",
            label: "批量",
            description: "批量新增库位",
            icon: <Plus className="size-4" aria-hidden />,
            onClick: () => setLocationBatchOpen(true),
          },
        ]
      : []),
  ] : [];

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
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
            {canWrite && viewId === "m1-business-partners" && (
              <MasterDataSourceActions
                ref={supplierActionsRef}
                kind="supplier"
                showTriggers={false}
                onCreate={createSupplierFromDialog}
                onImport={importSuppliersFromDialog}
              />
            )}
            {canWrite && viewId === "m1-business-partners" && (
              <MasterDataSourceActions
                ref={customerActionsRef}
                kind="customer"
                showTriggers={false}
                onCreate={createCustomerFromDialog}
                onImport={importCustomersFromDialog}
              />
            )}
          </div>
        }
      />

      <QueryPanel
        fields={m1QueryFields}
        defaultVisibleFieldKeys={m1CoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeM1QueryValue(next))}
        onQuery={() => {
          setAppliedQuery(normalizeM1QueryValue(draftQuery));
          setSelectedRowKeys([]);
        }}
        onReset={() => {
          const defaults = defaultM1QueryValue();
          setDraftQuery(defaults);
          setAppliedQuery(defaults);
          setSelectedRowKeys([]);
        }}
        resetLabel="重置"
      />

      {(rowsQuery.error || rowActionError) && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {rowsQuery.error?.message ?? rowActionError}
        </div>
      )}

      <DataGrid
        columns={gridColumns}
        data={rows}
        rowKey={(row) => row.id}
        selectedRowKeys={selectedRowKeys}
        onSelectedRowKeysChange={setSelectedRowKeys}
        caption={rowsQuery.isPending ? "加载基础档案..." : undefined}
        emptyTitle={meta.emptyTitle}
        emptyDescription={
          rowsQuery.isPending
            ? "正在读取基础档案。"
            : normalizedKeyword
              ? meta.emptyDescription
              : `${meta.emptyTitle}，可通过右上角操作新增或导入。`
        }
        storageKey={meta.storageKey}
        exportFileBaseName={meta.title}
        refreshAction={gridRefreshAction}
        createAction={gridCreateAction}
        editAction={gridEditAction}
        disableAction={gridDisableAction}
        toolbarActions={gridToolbarActions}
        tableClassName={productTableClassName(viewId)}
        queryState={appliedQuery}
        querySummaryItems={m1QuerySummaryItems}
        onApplyQueryState={(queryState) => {
          const next = normalizeM1QueryValue(queryValueFromUnknown(queryState));
          setDraftQuery(next);
          setAppliedQuery(next);
          setSelectedRowKeys([]);
        }}
        onClearQueryState={() => {
          const defaults = defaultM1QueryValue();
          setDraftQuery(defaults);
          setAppliedQuery(defaults);
          setSelectedRowKeys([]);
        }}
        selectable={viewId !== "m1-products"}
      />

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
          locationTypeOptions={locationTypeOptions}
          onLocationTypeChange={setLocationBatchType}
          errors={locationBatchErrors}
          preview={locationBatchPreview}
          message={locationBatchMessage}
          confirmDisabled={!locationBatchScope || locationBatchSubmitting || locationTypeOptions.length === 0}
          confirmLabel={locationBatchSubmitting ? "提交中..." : "确认新增"}
          onConfirm={confirmLocationBatchPreview}
        />
      )}

        <MasterDataCrudDialog
        target={crudTarget}
        locationScopes={locationBatchScopes}
          locationTypeOptions={locationTypeOptions}
          warehouseOptions={warehouseRowsQuery.data ?? []}
          temperatureZoneOptions={temperatureZoneOptions}
          qualityColorOptions={qualityColorOptions}
        onOpenChange={(open) => !open && setCrudTarget(null)}
        onSubmit={submitCrudForm}
      />
    </section>
  );
}

function defaultM1QueryValue(): QueryPanelValue {
  return { keyword: "" };
}

function normalizeM1QueryValue(value: QueryPanelValue): QueryPanelValue {
  return { keyword: queryString(value.keyword) };
}

function queryString(value: QueryPanelValue[string]) {
  return typeof value === "string" ? value : "";
}

function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as QueryPanelValue) : {};
}
