import * as React from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  DataGrid,
  ListPageTemplate,
  buildLocationBatchPreview,
  toLocationBatchGeneratePayload,
  type DataGridCreateAction,
  type DataGridDetailAction,
  type DataGridDisableAction,
  type DataGridEditAction,
  type DataGridPrintAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type LocationBatchRange,
  type QueryPanelField,
  type QueryPanelValue,
  validateLocationBatchRange,
} from "@wms/ui";
import { Plus, Printer, Upload } from "lucide-react";

import {
  batchCreateCustomers,
  batchCreateSuppliers,
  batchGenerateLocations,
  createCustomer,
  createSupplier,
  productsPageQueryOptions,
  useMasterDataRowsQuery,
  useProductsPageQuery,
  useSystemDictionaryItemOptionsQuery,
  masterDataQueryKey,
  type CreateCustomerRequest,
  type CreateSupplierRequest,
  type MasterDataRow,
  type MasterDataViewId,
  specialDrugCategoryDictCode,
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
import { ProductDetailDialog } from "./ProductDetailDialog";
import { productColumns } from "./ProductEditTable";
import {
  baseMasterDataColumns,
  locationMasterDataColumns,
  statusFilterValue,
} from "./M1MasterDataColumns";
import {
  masterDataColumns,
  productTableClassName,
} from "./m1-product-page-model";
import { M1SystemDictionaryPage } from "./SystemDictionaryPage";
import type { CurrentUser } from "@/features/auth/auth-queries";
import {
  H9BusinessPrintDialog,
  type H9BusinessPrintTarget,
} from "../print-template/H9BusinessPrintDialog";
import { BUTTON_ADD } from "@/lib/ui-strings";
import { usePageQueryState } from "@/lib/use-page-query-state";
export type { MasterDataViewId } from "@/features/master-data/master-data-queries";
// 治理声明：ERP 权威商品投影；本页只读 —— M1 商品由 H8 商品消息同步维护，本页不提供写入口
export const masterDataViewMeta: Record<
  MasterDataViewId,
  {
    title: string;
    emptyTitle: string;
    emptyDescription: string;
    storageKey: string;
  }
> = {
  "m1-products": {
    title: "M1 商品档案",
    emptyTitle: "暂无商品档案",
    emptyDescription: "当前筛选条件下没有商品，请调整关键字或清空筛选。",
    storageKey: "m1-products-datagrid",
  },
  "m1-business-partners": {
    title: "M1 客商档案",
    emptyTitle: "暂无客商档案",
    emptyDescription: "当前筛选条件下没有客商，请调整关键字或清空筛选。",
    storageKey: "m1-business-partners-datagrid",
  },
  "m1-warehouses": {
    title: "M1 仓库管理",
    emptyTitle: "暂无仓库档案",
    emptyDescription: "当前筛选条件下没有仓库，请调整关键字或清空筛选。",
    storageKey: "m1-warehouses-datagrid",
  },
  "m1-zones": {
    title: "M1 库区管理",
    emptyTitle: "暂无库区档案",
    emptyDescription: "当前筛选条件下没有库区，请调整关键字或清空筛选。",
    storageKey: "m1-zones-datagrid",
  },
  "m1-locations": {
    title: "M1 库位管理",
    emptyTitle: "暂无库位档案",
    emptyDescription: "当前筛选条件下没有库位，请调整关键字或清空筛选。",
    storageKey: "m1-locations-datagrid",
  },
  "m1-system-dictionary": {
    title: "M1 系统字典",
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
  {
    key: "statusFilter",
    label: "状态",
    type: "select",
    options: [
      { label: "全部", value: "" },
      { label: "启用/可用", value: "active" },
      { label: "停用/锁定", value: "disabled" },
    ],
  },
  {
    key: "createdAt",
    label: "创建时间",
    type: "dateRange",
  },
];
const m1CoreQueryFieldKeys = ["keyword", "statusFilter"];

interface M1MasterDataPageProps {
  viewId: MasterDataViewId;
  currentUser: CurrentUser;
  onBack: () => void;
}

export function M1MasterDataPage({ viewId, currentUser }: M1MasterDataPageProps) {
  if (viewId === "m1-system-dictionary") {
    return <M1SystemDictionaryPage currentUser={currentUser} meta={masterDataViewMeta[viewId]} />;
  }

  return <M1MasterDataGridPage currentUser={currentUser} viewId={viewId} />;
}

function M1MasterDataGridPage({ currentUser, viewId }: Pick<M1MasterDataPageProps, "currentUser" | "viewId">) {
  const meta = masterDataViewMeta[viewId];
  const canWrite = currentUser.permissions.includes("m1.master_data.write");
  const canPrint = currentUser.permissions.includes("h9.print_template.print");
  const queryClient = useQueryClient();
  const [productPageIndex, setProductPageIndex] = React.useState(0);
  const [productPageSize, setProductPageSize] = React.useState(20);
  const rowsQuery = useMasterDataRowsQuery(viewId, viewId !== "m1-products");
  const productsPageQuery = useProductsPageQuery(productPageIndex, productPageSize, viewId === "m1-products");
  const prefetchProductPage = React.useCallback(
    (pageIndex: number, pageSize: number) => queryClient.prefetchQuery(productsPageQueryOptions(pageIndex, pageSize)),
    [queryClient],
  );
  const warehouseRowsQuery = useMasterDataRowsQuery("m1-warehouses", viewId === "m1-zones");
  const zoneRowsQuery = useMasterDataRowsQuery("m1-zones", viewId === "m1-locations");
  const locationTypeOptionsQuery = useSystemDictionaryItemOptionsQuery(
    "location_type",
    viewId === "m1-locations",
  );
  const locationTypeOptions = locationTypeOptionsQuery.data ?? [];
  const specialDrugCategoryOptions = useSystemDictionaryItemOptionsQuery(
    specialDrugCategoryDictCode,
    viewId === "m1-products",
  ).data ?? [];
  const specialDrugCategoryLabels = React.useMemo(
    () => new Map(specialDrugCategoryOptions),
    [specialDrugCategoryOptions],
  );
  const temperatureZoneOptions = useSystemDictionaryItemOptionsQuery("temperature_zone", viewId === "m1-zones").data ?? [];
  const qualityColorOptions = useSystemDictionaryItemOptionsQuery("quality_color", viewId === "m1-zones").data ?? [];
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultM1QueryValue, normalizeM1QueryValue);
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const [rowActionError, setRowActionError] = React.useState<string | null>(null);
  const [disablingId, setDisablingId] = React.useState<string | null>(null);
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [printTarget, setPrintTarget] = React.useState<H9BusinessPrintTarget | null>(null);
  const [detailTarget, setDetailTarget] = React.useState<MasterDataRow | null>(null);
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
  const activeRows = viewId === "m1-products" ? productsPageQuery.data?.rows ?? [] : rowsQuery.data ?? [];
  const activeRowsError = viewId === "m1-products" ? productsPageQuery.error : rowsQuery.error;
  const activeRowsPending = viewId === "m1-products" ? productsPageQuery.isPending : rowsQuery.isPending;
  const rows = React.useMemo(() => {
    let data = activeRows;
    const statusFilter = String(appliedQuery.statusFilter ?? "").trim();
    if (statusFilter) {
      data = data.filter((row) => statusFilterValue(row.status) === statusFilter);
    }
    if (!normalizedKeyword) return data;
    return data.filter((row) => row.searchText.includes(normalizedKeyword));
  }, [activeRows, appliedQuery.statusFilter, normalizedKeyword]);
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
      ? productColumns(baseGridColumns, specialDrugCategoryLabels)
      : isMasterDataCrudView(viewId)
        ? masterDataCrudColumns(baseGridColumns, viewId, openCrudEdit, disableCrudRow, disablingId)
        : baseGridColumns;

  React.useEffect(() => {
    setSelectedRowKeys([]);
    setRowActionError(null);
    setProductPageIndex(0);
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
    if (viewId === "m1-products") {
      await queryClient.invalidateQueries({
        queryKey: [...masterDataQueryKey, "m1-products-page"],
      });
      await productsPageQuery.refetch();
    } else {
      await rowsQuery.refetch();
    }
    setRowActionError(null);
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
      const generate = toLocationBatchGeneratePayload(locationBatchRange);
      const createdRows = await batchGenerateLocations({
        warehouse_id: locationBatchScope.warehouseId,
        zone_id: locationBatchScope.zoneId,
        ...generate,
        max_volume_cm3: 5_000_000,
        max_sku_count: 1,
        location_type: locationBatchType,
        current_owner_id: locationBatchScope.ownerId,
        is_agv_managed: generate.rule_type === "agv",
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

  function applyGridQueryState(queryState: unknown) {
    applyQuery(queryValueFromUnknown(queryState));
    setSelectedRowKeys([]);
    setProductPageIndex(0);
  }

  function clearGridQueryState(key?: string) {
    if (key) {
      const next = { ...appliedQuery };
      delete (next as Record<string, unknown>)[key];
      applyQuery(next);
      setDraftQuery(next);
    } else {
      resetQuery();
    }
    setSelectedRowKeys([]);
    setProductPageIndex(0);
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
            label: BUTTON_ADD,
            description: "新建仓库",
            onClick: () => setCrudTarget({ kind: "warehouse", mode: "create" }),
          }
        : viewId === "m1-zones"
          ? {
              label: BUTTON_ADD,
              description: "新建库区",
              onClick: () => setCrudTarget({ kind: "zone", mode: "create" }),
            }
        : viewId === "m1-locations"
          ? {
              label: BUTTON_ADD,
              description: "新建库位",
              onClick: () => setCrudTarget({ kind: "location", mode: "create" }),
            }
          : undefined
    : undefined;
  const gridDetailAction: DataGridDetailAction | undefined = viewId === "m1-products"
    ? {
        label: "查看",
        description: "查看选中商品详情",
        disabled: ({ selectedRowKeys: keys }) => keys.length !== 1,
        onClick: ({ selectedRowKeys: keys }) => {
          setRowActionError(null);
          setDetailTarget(selectedRowFrom(keys));
        },
      }
    : isMasterDataCrudView(viewId)
      ? {
          label: "查看",
          description: "查看或修改选中档案",
          disabled: ({ selectedRowKeys: keys }) => keys.length !== 1,
          onClick: ({ selectedRowKeys: keys }) => {
            const row = selectedRowFrom(keys);
            if (!row) return;
            openCrudEdit(row);
          },
        }
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
  const gridPrintAction: DataGridPrintAction | undefined = canPrint && (viewId === "m1-products" || viewId === "m1-locations")
    ? {
        label: "打印",
        description: viewId === "m1-products" ? "打印商品标签" : "打印库位标签",
        disabled: ({ selectedRowKeys: keys }) => keys.length !== 1,
        onClick: ({ selectedRowKeys: keys }) => {
          const row = selectedRowFrom(keys);
          if (row) setPrintTarget(masterDataPrintTarget(viewId, row));
        },
      }
    : undefined;
  const gridToolbarActions: DataGridToolbarAction[] = [
    ...(canWrite && viewId === "m1-business-partners"
      ? [
          {
            key: "supplier-create",
            label: "新建供应商",
            description: "新建供应商",
            icon: <Plus className="size-4" aria-hidden />,
            onClick: () => supplierActionsRef.current?.openCreate(),
          },
          {
            key: "supplier-import",
            label: "导入供应商",
            description: "批量导入供应商",
            icon: <Upload className="size-4" aria-hidden />,
            onClick: () => supplierActionsRef.current?.openImport(),
          },
          {
            key: "customer-create",
            label: "新建客户",
            description: "新建客户",
            icon: <Plus className="size-4" aria-hidden />,
            onClick: () => customerActionsRef.current?.openCreate(),
          },
          {
            key: "customer-import",
            label: "导入客户",
            description: "批量导入客户",
            icon: <Upload className="size-4" aria-hidden />,
            onClick: () => customerActionsRef.current?.openImport(),
          },
        ]
      : []),
    ...(canWrite && viewId === "m1-locations"
      ? [
          {
            key: "location-batch-create",
            label: "批量新增",
            description: "批量新增库位",
            icon: <Plus className="size-4" aria-hidden />,
            onClick: () => setLocationBatchOpen(true),
          },
        ]
      : []),
  ];

  return (
    <ListPageTemplate
      header={{
        actions: (
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
        ),
      }}
      banner={
        (activeRowsError || rowActionError) ? (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {activeRowsError?.message ?? rowActionError}
          </div>
        ) : null
      }
      queryFields={m1QueryFields}
      coreQueryFieldKeys={m1CoreQueryFieldKeys}
      queryValue={draftQuery}
      loading={activeRowsPending}
      onQueryValueChange={(next) => setDraftQuery(normalizeM1QueryValue(next))}
      onQuery={() => {
        applyQuery(draftQuery);
        setSelectedRowKeys([]);
        setProductPageIndex(0);
      }}
      onReset={clearGridQueryState}
      gridProps={{
        columns: gridColumns,
        data: rows,
        rowKey: (row) => row.id,
        selectedRowKeys,
        onSelectedRowKeysChange: setSelectedRowKeys,
        serverPagination: viewId === "m1-products" ? {
          pageIndex: productPageIndex,
          pageSize: productPageSize,
          total: productsPageQuery.data?.total ?? 0,
          prefetchPageCount: 2,
          onPrefetchPage: prefetchProductPage,
          onPageChange: (next) => {
            setProductPageIndex(next);
            setSelectedRowKeys([]);
          },
          onPageSizeChange: (next) => {
            setProductPageSize(next);
            setProductPageIndex(0);
            setSelectedRowKeys([]);
          },
        } : undefined,
        caption: activeRowsPending ? "加载基础档案..." : undefined,
        emptyTitle: meta.emptyTitle,
        emptyDescription: activeRowsPending
          ? "正在读取基础档案。"
          : normalizedKeyword
            ? meta.emptyDescription
            : `${meta.emptyTitle}，可通过右上角操作新增或导入。`,
        storageKey: meta.storageKey,
        exportFileBaseName: meta.title,
        refreshAction: gridRefreshAction,
        createAction: gridCreateAction,
        detailAction: gridDetailAction,
        printAction: gridPrintAction,
        editAction: gridEditAction,
        disableAction: gridDisableAction,
        toolbarActions: gridToolbarActions,
        tableClassName: productTableClassName(viewId),
        queryState: appliedQuery,
        onApplyQueryState: applyGridQueryState,
        onClearQueryState: clearGridQueryState,
        onRowDoubleClick: (row) => {
          if (viewId === "m1-products") {
            setRowActionError(null);
            setDetailTarget(row);
          }
        },
        selectable: viewId !== "m1-products" || canPrint,
      }}
      dialogs={
        <>
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
          <H9BusinessPrintDialog
            open={Boolean(printTarget)}
            target={printTarget}
            onOpenChange={(next) => {
              if (!next) setPrintTarget(null);
            }}
            onPrinted={(target) => setLastEvent(`${target.description}已登记打印结果`)}
          />
          <ProductDetailDialog
            row={detailTarget}
            specialDrugCategoryLabels={specialDrugCategoryLabels}
            onOpenChange={(open) => {
              if (!open) setDetailTarget(null);
            }}
          />
        </>
      }
    />
  );
}

export function masterDataPrintTarget(
  viewId: MasterDataViewId,
  row: MasterDataRow,
): H9BusinessPrintTarget {
  if (viewId === "m1-locations" && row.locationFields) {
    const fields = row.locationFields;
    return {
      templateTypeCode: "location_label",
      businessModule: "M1",
      businessDocumentType: "location_label",
      businessDocumentId: row.id,
      description: `${row.code} · 库位标签`,
      data: {
        id: row.id,
        owner_id: row.ownerId,
        warehouse_id: fields.warehouseId,
        zone_id: fields.zoneId,
        location_code: row.code,
        row_no: integer(fields.rowNo),
        column_no: integer(fields.columnNo),
        layer_no: integer(fields.layerNo),
        max_volume_cm3: integer(fields.maxVolumeCm3),
        used_volume_cm3: integer(fields.usedVolumeCm3),
        max_sku_count: integer(fields.maxSku),
        location_type: fields.locationTypeCode,
        current_owner_id: fields.boundOwnerId,
        status: row.status,
        created_at: row.createdAt,
        updated_at: row.updatedAt,
      },
    };
  }
  if (viewId === "m1-products" && row.productFields) {
    const fields = row.productFields;
    return {
      templateTypeCode: "product_label",
      businessModule: "M1",
      businessDocumentType: "product_label",
      businessDocumentId: row.id,
      description: `${row.code} · 商品标签`,
      data: {
        id: row.id,
        owner_id: row.ownerId,
        product_code: row.code,
        product_name: row.name,
        approval_no: fields.approvalNo,
        spec: fields.spec,
        dosage_form: fields.dosageForm,
        manufacturer: fields.manufacturer,
        udi_code: fields.udiCode,
        electronic_regulatory_code: fields.electronicRegulatoryCode,
        length_mm: numberOrNull(fields.lengthMm),
        width_mm: numberOrNull(fields.widthMm),
        height_mm: numberOrNull(fields.heightMm),
        volume_cm3: numberOrNull(fields.volumeCm3),
        weight_g: numberOrNull(fields.weightG),
        packaging_levels: fields.packagingLevels.map((level) => ({
          id: level.id,
          unit_code: level.unitCode,
          unit_name: level.unitName,
          ratio_to_base: level.ratioToBase,
          is_base: level.isBase,
          is_default: level.isDefault,
          sort_order: level.sortOrder,
        })),
        mapping_traces: fields.mappingTraces,
        special_drug_category_code: fields.specialDrugCategoryCode,
        status: row.status,
        attrs: fields.attrs,
        created_at: row.createdAt,
        updated_at: row.updatedAt,
      },
    };
  }
  throw new Error("当前基础档案不支持标签打印");
}

function integer(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

function numberOrNull(value: string) {
  if (!value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
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
