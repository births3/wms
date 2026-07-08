import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type MasterDataViewId =
  | "m1-products"
  | "m1-business-partners"
  | "m1-warehouses"
  | "m1-zones"
  | "m1-locations"
  | "m1-system-dictionary";

type Product = components["schemas"]["Product"];
type Supplier = components["schemas"]["Supplier"];
type Customer = components["schemas"]["Customer"];
type Warehouse = components["schemas"]["Warehouse"];
type Location = components["schemas"]["Location"];
export type UpdateProductRequest = components["schemas"]["UpdateProductRequest"];
export type CreateProductRequest = components["schemas"]["CreateProductRequest"];
export type CreateSupplierRequest = components["schemas"]["CreateSupplierRequest"];
export type UpdateSupplierRequest = components["schemas"]["UpdateSupplierRequest"];
export type CreateCustomerRequest = components["schemas"]["CreateCustomerRequest"];
export type UpdateCustomerRequest = components["schemas"]["UpdateCustomerRequest"];
export type CreateWarehouseRequest = components["schemas"]["CreateWarehouseRequest"];
export type UpdateWarehouseRequest = components["schemas"]["UpdateWarehouseRequest"];
export type CreateLocationRequest = components["schemas"]["CreateLocationRequest"];
export type UpdateLocationRequest = components["schemas"]["UpdateLocationRequest"];
export type BatchCreateLocationsRequest = components["schemas"]["BatchCreateLocationsRequest"];
export type SystemDictionaryItem = components["schemas"]["SystemDictionaryItem"];
export type UpsertSystemDictionaryItemRequest =
  components["schemas"]["UpsertSystemDictionaryItemRequest"];
export type DisableSystemDictionaryItemRequest =
  components["schemas"]["DisableSystemDictionaryItemRequest"];
export type BusinessPartnerKind = "supplier" | "customer";

export interface MasterDataRow {
  id: string;
  code: string;
  name: string;
  status: string;
  statusLabel: string;
  ownerId: string;
  primaryLabel: string;
  primaryValue: string;
  secondaryLabel: string;
  secondaryValue: string;
  extraLabel: string;
  extraValue: string;
  createdAt: string;
  sourceValue?: string;
  updatedAt: string;
  productFields?: ProductMasterDataFields;
  zoneFields?: WarehouseZoneMasterDataFields;
  locationFields?: LocationMasterDataFields;
  partnerKind?: BusinessPartnerKind;
  partnerTypeLabel?: string;
  searchText: string;
}

export interface ProductMasterDataFields {
  approvalNo?: string | null;
  attrs: Record<string, unknown>;
  dosageForm?: string | null;
  manufacturer?: string | null;
  specialDrugCategoryCode?: string | null;
  spec?: string | null;
  storageCondition?: string | null;
  middlePackage: string;
  largePackage: string;
  unitLengthMm: string;
  unitWidthMm: string;
  unitHeightMm: string;
  unitWeightG: string;
  unitVolumeCm3: string;
}

export interface LocationMasterDataFields {
  owner: string;
  warehouse: string;
  zone: string;
  area: string;
  rowNo: string;
  columnNo: string;
  layerNo: string;
  locationType: string;
  locationTypeCode: string;
  volume: string;
  maxVolumeCm3: string;
  usedVolumeCm3: string;
  maxSku: string;
}

export interface WarehouseZoneMasterDataFields {
  owner: string;
  warehouse: string;
  zone: string;
  locationCount: string;
  availableLocationCount: string;
}

export interface SystemDictionaryPaneItem {
  id: string;
  code: string;
  name: string;
  source: string;
  enabled: boolean;
  ownerId?: string | null;
  params: Record<string, unknown>;
  effectiveFrom?: string | null;
  effectiveTo?: string | null;
  disabledReason?: string | null;
  updatedAt: string;
}

export interface SystemDictionaryPaneGroup {
  code: string;
  name: string;
  items: SystemDictionaryPaneItem[];
}

export interface SpecialDrugCategoryOption {
  value: string;
  label: string;
  status: string;
  requiresDualSign: boolean;
}

export type SystemDictionaryOption = readonly [string, string];

export const masterDataQueryKey = ["master-data"] as const;
export const specialDrugCategoryDictCode = "special_drug_category";
const systemDictionaryDefinitions = [
  { code: "document_type", name: "单据类型" },
  { code: "print_template_type", name: "打印模板类型" },
  { code: specialDrugCategoryDictCode, name: "特殊药品分类" },
  { code: "temperature_zone", name: "库区温区" },
  { code: "quality_color", name: "库区色标" },
  { code: "zone_type", name: "库区类型" },
  { code: "location_type", name: "库位类型" },
] as const;
const systemDictionaryGroupsQueryKey = [
  ...masterDataQueryKey,
  "m1-system-dictionary",
  "two-pane",
] as const;
const systemDictionaryRowsQueryKey = [...masterDataQueryKey, "m1-system-dictionary"] as const;
const specialDrugCategoriesQueryKey = [
  ...masterDataQueryKey,
  "m1-system-dictionary",
  specialDrugCategoryDictCode,
  "options",
] as const;
const systemDictionaryOptionsQueryKey = [
  ...masterDataQueryKey,
  "m1-system-dictionary",
  "options",
] as const;

export function useMasterDataRowsQuery(viewId: MasterDataViewId, enabled = true) {
  return useQuery<MasterDataRow[], ApiError>({
    queryKey: [...masterDataQueryKey, viewId],
    queryFn: () => listMasterDataRows(viewId),
    enabled,
  });
}

export function useSystemDictionaryGroupsQuery() {
  return useQuery<SystemDictionaryPaneGroup[], ApiError>({
    queryKey: systemDictionaryGroupsQueryKey,
    queryFn: listSystemDictionaryGroups,
  });
}

export function useSpecialDrugCategoriesQuery(enabled = true) {
  return useQuery<SpecialDrugCategoryOption[], ApiError>({
    queryKey: specialDrugCategoriesQueryKey,
    queryFn: listSpecialDrugCategoryOptions,
    enabled,
  });
}

export function useSystemDictionaryItemOptionsQuery(dictCode: string, enabled = true) {
  return useQuery<SystemDictionaryOption[], ApiError>({
    queryKey: [...systemDictionaryOptionsQueryKey, dictCode],
    queryFn: () => listSystemDictionaryItemOptions(dictCode),
    enabled,
  });
}

export function useUpsertSystemDictionaryItemMutation() {
  const invalidate = useInvalidateSystemDictionary();
  return useMutation({
    mutationFn: upsertSystemDictionaryItem,
    onSuccess: () => invalidate(),
  });
}

export function useDisableSystemDictionaryItemMutation() {
  const invalidate = useInvalidateSystemDictionary();
  return useMutation({
    mutationFn: disableSystemDictionaryItem,
    onSuccess: () => invalidate(),
  });
}

export function useUpdateProductMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: updateProduct,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: [...masterDataQueryKey, "m1-products"] });
    },
  });
}

async function listMasterDataRows(viewId: MasterDataViewId): Promise<MasterDataRow[]> {
  switch (viewId) {
    case "m1-products":
      return listProducts();
    case "m1-business-partners":
      return listBusinessPartners();
    case "m1-warehouses":
      return listWarehouses();
    case "m1-zones":
      return listWarehouseZones();
    case "m1-locations":
      return listLocations();
    case "m1-system-dictionary":
      return listSystemDictionaryItems();
  }
}

async function listProducts(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/products");
  if (!result.data) {
    throw new ApiError(result.error, "读取商品档案失败", result.response.status);
  }
  return result.data.data.map(productRow);
}

async function updateProduct(input: {
  id: string;
  request: UpdateProductRequest;
}): Promise<Product> {
  const result = await api.PATCH("/api/v1/master-data/products/{id}", {
    params: { path: { id: input.id } },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存商品档案失败", result.response.status);
  }
  return result.data;
}

async function listSuppliers(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/suppliers");
  if (!result.data) {
    throw new ApiError(result.error, "读取供应商档案失败", result.response.status);
  }
  return result.data.data.map(supplierRow);
}

async function listCustomers(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/customers");
  if (!result.data) {
    throw new ApiError(result.error, "读取客户档案失败", result.response.status);
  }
  return result.data.data.map(customerRow);
}

async function listBusinessPartners(): Promise<MasterDataRow[]> {
  const [suppliers, customers] = await Promise.all([listSuppliers(), listCustomers()]);
  return [...suppliers, ...customers];
}

async function listWarehouses(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/warehouses");
  if (!result.data) {
    throw new ApiError(result.error, "读取仓库档案失败", result.response.status);
  }
  return result.data.data.map(warehouseRow);
}

async function listLocations(): Promise<MasterDataRow[]> {
  const [result, locationTypeOptions] = await Promise.all([
    api.GET("/api/v1/master-data/locations"),
    listSystemDictionaryItemOptions("location_type"),
  ]);
  if (!result.data) {
    throw new ApiError(result.error, "读取库位档案失败", result.response.status);
  }
  const locationTypeLabels = new Map(locationTypeOptions);
  return result.data.data.map((location) => locationRow(location, locationTypeLabels));
}

async function listWarehouseZones(): Promise<MasterDataRow[]> {
  return warehouseZoneRowsFromLocations(await listLocations());
}

export function warehouseZoneRowsFromLocations(
  locations: readonly MasterDataRow[],
): MasterDataRow[] {
  const zones = new Map<
    string,
    {
      owner: string;
      warehouse: string;
      zone: string;
      locationCount: number;
      availableLocationCount: number;
      createdAt: string;
      updatedAt: string;
    }
  >();

  for (const location of locations) {
    const fields = location.locationFields;
    if (!fields || fields.warehouse === "-" || fields.zone === "-") continue;
    const owner = fields.owner === "-" ? location.ownerId : fields.owner;
    const key = `${owner}:${fields.warehouse}:${fields.zone}`;
    const current = zones.get(key);
    zones.set(key, {
      owner,
      warehouse: fields.warehouse,
      zone: fields.zone,
      locationCount: (current?.locationCount ?? 0) + 1,
      availableLocationCount:
        (current?.availableLocationCount ?? 0) + (location.status === "available" ? 1 : 0),
      createdAt: current ? minText(current.createdAt, location.createdAt) : location.createdAt,
      updatedAt: current ? maxText(current.updatedAt, location.updatedAt) : location.updatedAt,
    });
  }

  return Array.from(zones.values())
    .sort((left, right) =>
      `${left.warehouse}:${left.zone}`.localeCompare(`${right.warehouse}:${right.zone}`, "zh-CN"),
    )
    .map((zone) =>
      row({
        id: `${zone.owner}:${zone.warehouse}:${zone.zone}`,
        code: zone.zone,
        name: `库区 ${shortId(zone.zone)}`,
        status: "derived_readonly",
        statusLabel: "只读派生",
        ownerId: zone.owner,
        primaryLabel: "仓库 ID",
        primaryValue: zone.warehouse,
        secondaryLabel: "库区 ID",
        secondaryValue: zone.zone,
        extraLabel: "库位数",
        extraValue: `${zone.locationCount} 个 / 可用 ${zone.availableLocationCount} 个`,
        createdAt: zone.createdAt,
        updatedAt: zone.updatedAt,
        zoneFields: {
          owner: zone.owner,
          warehouse: zone.warehouse,
          zone: zone.zone,
          locationCount: String(zone.locationCount),
          availableLocationCount: String(zone.availableLocationCount),
        },
      }),
    );
}

export async function batchCreateLocations(
  request: BatchCreateLocationsRequest,
): Promise<MasterDataRow[]> {
  const [result, locationTypeOptions] = await Promise.all([
    api.POST("/api/v1/master-data/locations/batch-create", {
      params: {
        header: { "Idempotency-Key": idempotencyKey("web-m1-location-batch") },
      },
      body: request,
    }),
    listSystemDictionaryItemOptions("location_type"),
  ]);
  if (!result.data) {
    throw new ApiError(result.error, "批量新增库位失败", result.response.status);
  }
  const locationTypeLabels = new Map(locationTypeOptions);
  return result.data.data.map((location) => locationRow(location, locationTypeLabels));
}

export async function createProduct(request: CreateProductRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/products", { body: request });
  if (!result.data) {
    throw new ApiError(result.error, "新建商品失败", result.response.status);
  }
  return productRow(result.data);
}

export async function createSupplier(request: CreateSupplierRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/suppliers", { body: request });
  if (!result.data) {
    throw new ApiError(result.error, "新建供应商失败", result.response.status);
  }
  return supplierRow(result.data);
}

export async function createCustomer(request: CreateCustomerRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/customers", { body: request });
  if (!result.data) {
    throw new ApiError(result.error, "新建客户失败", result.response.status);
  }
  return customerRow(result.data);
}

export async function updateSupplier(input: {
  id: string;
  request: UpdateSupplierRequest;
}): Promise<MasterDataRow> {
  const result = await api.PATCH("/api/v1/master-data/suppliers/{id}", {
    params: { path: { id: input.id } },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存供应商失败", result.response.status);
  }
  return supplierRow(result.data);
}

export async function updateCustomer(input: {
  id: string;
  request: UpdateCustomerRequest;
}): Promise<MasterDataRow> {
  const result = await api.PATCH("/api/v1/master-data/customers/{id}", {
    params: { path: { id: input.id } },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存客户失败", result.response.status);
  }
  return customerRow(result.data);
}

export async function createWarehouse(request: CreateWarehouseRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/warehouses", { body: request });
  if (!result.data) {
    throw new ApiError(result.error, "新建仓库失败", result.response.status);
  }
  return warehouseRow(result.data);
}

export async function updateWarehouse(input: {
  id: string;
  request: UpdateWarehouseRequest;
}): Promise<MasterDataRow> {
  const result = await api.PATCH("/api/v1/master-data/warehouses/{id}", {
    params: { path: { id: input.id } },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存仓库失败", result.response.status);
  }
  return warehouseRow(result.data);
}

export async function createLocation(request: CreateLocationRequest): Promise<MasterDataRow> {
  const [result, locationTypeOptions] = await Promise.all([
    api.POST("/api/v1/master-data/locations", { body: request }),
    listSystemDictionaryItemOptions("location_type"),
  ]);
  if (!result.data) {
    throw new ApiError(result.error, "新建库位失败", result.response.status);
  }
  return locationRow(result.data, new Map(locationTypeOptions));
}

export async function updateLocation(input: {
  id: string;
  request: UpdateLocationRequest;
}): Promise<MasterDataRow> {
  const [result, locationTypeOptions] = await Promise.all([
    api.PATCH("/api/v1/master-data/locations/{id}", {
      params: { path: { id: input.id } },
      body: input.request,
    }),
    listSystemDictionaryItemOptions("location_type"),
  ]);
  if (!result.data) {
    throw new ApiError(result.error, "保存库位失败", result.response.status);
  }
  return locationRow(result.data, new Map(locationTypeOptions));
}

async function listSystemDictionaryItems(): Promise<MasterDataRow[]> {
  const groups = await fetchSystemDictionaryGroupItems();
  return groups.flatMap((group) => group.items.map(systemDictionaryRow));
}

async function listSystemDictionaryGroups(): Promise<SystemDictionaryPaneGroup[]> {
  const groups = await fetchSystemDictionaryGroupItems();
  return groups.map((group) => ({
    code: group.code,
    name: group.name,
    items: group.items.map(systemDictionaryPaneItem),
  }));
}

async function fetchSystemDictionaryGroupItems() {
  return Promise.all(
    systemDictionaryDefinitions.map(async (definition) => ({
      ...definition,
      items: await fetchSystemDictionaryItems(definition.code),
    })),
  );
}

async function fetchSystemDictionaryItems(dictCode: string): Promise<SystemDictionaryItem[]> {
  const result = await api.GET("/api/v1/system-dictionaries/{dict_code}/items", {
    params: { path: { dict_code: dictCode } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取系统字典失败", result.response.status);
  }
  return result.data.data;
}

async function listSpecialDrugCategoryOptions(): Promise<SpecialDrugCategoryOption[]> {
  const items = await fetchSystemDictionaryItems(specialDrugCategoryDictCode);
  return items.map(specialDrugCategoryOptionFromDictionaryItem);
}

async function listSystemDictionaryItemOptions(dictCode: string): Promise<SystemDictionaryOption[]> {
  const items = await fetchSystemDictionaryItems(dictCode);
  return items
    .filter((item) => item.enabled)
    .map((item) => [item.item_code, item.item_name] as const);
}

async function upsertSystemDictionaryItem(input: {
  dictCode: string;
  itemCode: string;
  request: UpsertSystemDictionaryItemRequest;
}): Promise<SystemDictionaryItem> {
  const result = await api.PUT("/api/v1/system-dictionaries/{dict_code}/items/{item_code}", {
    params: {
      path: { dict_code: input.dictCode, item_code: input.itemCode },
      header: { "Idempotency-Key": idempotencyKey("web-m1-dict-upsert") },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存系统字典项失败", result.response.status);
  }
  return result.data;
}

async function disableSystemDictionaryItem(input: {
  dictCode: string;
  itemCode: string;
  request: DisableSystemDictionaryItemRequest;
}): Promise<SystemDictionaryItem> {
  const result = await api.PATCH(
    "/api/v1/system-dictionaries/{dict_code}/items/{item_code}/disable",
    {
      params: {
        path: { dict_code: input.dictCode, item_code: input.itemCode },
        header: { "Idempotency-Key": idempotencyKey("web-m1-dict-disable") },
      },
      body: input.request,
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "停用系统字典项失败", result.response.status);
  }
  return result.data;
}

export function productRow(item: Product): MasterDataRow {
  const storageCondition = text(item.attrs.storage_condition ?? item.attrs.storage);
  const sourceValue = productSourceLabel(item.attrs.source);
  const middlePackage = productAttrText(item.attrs, "middle_package");
  const largePackage = productAttrText(item.attrs, "large_package");
  const unitLengthMm = productAttrText(item.attrs, "unit_length_mm");
  const unitWidthMm = productAttrText(item.attrs, "unit_width_mm");
  const unitHeightMm = productAttrText(item.attrs, "unit_height_mm");
  const unitWeightG = productAttrText(item.attrs, "unit_weight_g");
  const unitVolumeCm3 = productAttrText(item.attrs, "unit_volume_cm3");
  return row({
    id: item.id,
    code: item.product_code,
    name: item.product_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "规格",
    primaryValue: text(item.spec),
    secondaryLabel: "批准文号",
    secondaryValue: text(item.approval_no),
    extraLabel: "储存条件",
    extraValue: storageCondition,
    createdAt: item.created_at,
    sourceValue,
    updatedAt: item.updated_at,
    productFields: {
      approvalNo: item.approval_no,
      attrs: item.attrs,
      dosageForm: item.dosage_form,
      manufacturer: item.manufacturer,
      specialDrugCategoryCode: item.special_drug_category_code,
      spec: item.spec,
      storageCondition,
      middlePackage,
      largePackage,
      unitLengthMm,
      unitWidthMm,
      unitHeightMm,
      unitWeightG,
      unitVolumeCm3,
    },
  });
}

export function supplierRow(item: Supplier): MasterDataRow {
  return row({
    id: item.id,
    code: item.supplier_code,
    name: item.supplier_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "资质证号",
    primaryValue: text(item.license_no),
    secondaryLabel: "联系人",
    secondaryValue: text(item.contact_name),
    extraLabel: "档案类型",
    extraValue: "供应商",
    createdAt: item.created_at,
    sourceValue: supplierSource(item),
    updatedAt: item.updated_at,
    partnerKind: "supplier",
    partnerTypeLabel: "供应商",
  });
}

export function customerRow(item: Customer): MasterDataRow {
  return row({
    id: item.id,
    code: item.customer_code,
    name: item.customer_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "资质证号",
    primaryValue: text(item.license_no),
    secondaryLabel: "档案类型",
    secondaryValue: "客户/门店",
    extraLabel: "货主",
    extraValue: item.owner_id,
    createdAt: item.created_at,
    sourceValue: customerSource(item),
    updatedAt: item.updated_at,
    partnerKind: "customer",
    partnerTypeLabel: "客户/门店",
  });
}

function warehouseRow(item: Warehouse): MasterDataRow {
  return row({
    id: item.id,
    code: item.warehouse_code,
    name: item.warehouse_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "仓库 ID",
    primaryValue: item.id,
    secondaryLabel: "货主",
    secondaryValue: item.owner_id,
    extraLabel: "档案类型",
    extraValue: "仓库",
    createdAt: item.created_at,
    updatedAt: item.updated_at,
  });
}

function locationRow(item: Location, locationTypeLabels: ReadonlyMap<string, string>): MasterDataRow {
  const locationType = locationTypeLabels.get(item.location_type) ?? text(item.location_type);
  const volume = `${item.used_volume_cm3}/${item.max_volume_cm3} cm³`;
  return row({
    id: item.id,
    code: item.location_code,
    name: `${locationAreaCode(item.location_code)}-${item.row_no}-${item.column_no}-${item.layer_no}`,
    status: item.status,
    statusLabel: locationStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "库位类型",
    primaryValue: locationType,
    secondaryLabel: "容量",
    secondaryValue: volume,
    extraLabel: "最大 SKU",
    extraValue: String(item.max_sku_count),
    createdAt: item.created_at,
    updatedAt: item.updated_at,
    locationFields: {
      owner: item.owner_id,
      warehouse: item.warehouse_id,
      zone: item.zone_id,
      area: locationAreaCode(item.location_code),
      rowNo: String(item.row_no),
      columnNo: String(item.column_no),
      layerNo: String(item.layer_no),
      locationType,
      locationTypeCode: item.location_type,
      volume,
      maxVolumeCm3: String(item.max_volume_cm3),
      usedVolumeCm3: String(item.used_volume_cm3),
      maxSku: String(item.max_sku_count),
    },
  });
}

function systemDictionaryRow(item: SystemDictionaryItem): MasterDataRow {
  return row({
    id: item.id,
    code: item.item_code,
    name: item.item_name,
    status: item.enabled ? "active" : "disabled",
    statusLabel: item.enabled ? "启用" : "停用",
    ownerId: item.owner_id ?? "global",
    primaryLabel: "字典分类",
    primaryValue: item.dict_code,
    secondaryLabel: "来源",
    secondaryValue: item.source,
    extraLabel: "参数",
    extraValue: paramsText(item.params),
    createdAt: item.created_at,
    updatedAt: item.updated_at,
  });
}

function systemDictionaryPaneItem(item: SystemDictionaryItem): SystemDictionaryPaneItem {
  return {
    id: item.id,
    code: item.item_code,
    name: item.item_name,
    source: item.source,
    enabled: item.enabled,
    ownerId: item.owner_id,
    params: item.params,
    effectiveFrom: item.effective_from,
    effectiveTo: item.effective_to,
    disabledReason: item.disabled_reason,
    updatedAt: item.updated_at,
  };
}

function useInvalidateSystemDictionary() {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: systemDictionaryGroupsQueryKey });
    void queryClient.invalidateQueries({ queryKey: systemDictionaryRowsQueryKey });
    void queryClient.invalidateQueries({ queryKey: specialDrugCategoriesQueryKey });
  };
}

export function specialDrugCategoryOptions(
  categories: readonly SpecialDrugCategoryOption[],
  currentValue = "none",
  activeOnly = true,
): SpecialDrugCategoryOption[] {
  const options = categories
    .filter((category) => !activeOnly || category.status === "active" || category.value === currentValue)
    .map((category) => ({ ...category }));
  if (currentValue && !options.some((option) => option.value === currentValue)) {
    options.unshift({
      value: currentValue,
      label: currentValue === "none" ? "普通药品（none）" : currentValue,
      status: "unknown",
      requiresDualSign: false,
    });
  }
  return options;
}

function row(input: Omit<MasterDataRow, "searchText">): MasterDataRow {
  const locationSearchText = input.locationFields ? Object.values(input.locationFields) : [];
  const zoneSearchText = input.zoneFields ? Object.values(input.zoneFields) : [];
  const productSearchText = input.productFields ? Object.values(input.productFields).filter(isSearchTextValue) : [];
  return {
    ...input,
    searchText: [
      input.code,
      input.name,
      input.status,
      input.statusLabel,
      input.ownerId,
      input.primaryValue,
      input.secondaryValue,
      input.extraValue,
      input.createdAt,
      input.sourceValue ?? "",
      input.partnerTypeLabel ?? "",
      ...zoneSearchText,
      ...productSearchText,
      ...locationSearchText,
    ]
      .join(" ")
      .toLowerCase(),
  };
}

function productAttrText(attrs: Record<string, unknown>, key: string) {
  return text(attrs[key]);
}

function isSearchTextValue(value: unknown): value is string | number {
  return typeof value === "string" || typeof value === "number";
}

export function productSourceLabel(value: unknown) {
  if (typeof value !== "string") return "-";
  const normalized = value.trim().toLowerCase();
  if (!normalized) return "-";
  if (["manual", "manual_create", "manual_created", "hand_created", "手工新建"].includes(normalized)) {
    return "手工新建";
  }
  if (["batch_import", "batch", "excel_import", "import", "批量导入"].includes(normalized)) {
    return "批量导入";
  }
  if (["api_import", "api", "erp", "external_api", "接口导入", "api接口导入"].includes(normalized)) {
    return "API接口导入";
  }
  return value.trim();
}

function supplierSource(item: Supplier) {
  return productSourceLabel(item.source);
}

function customerSource(item: Customer) {
  return productSourceLabel(item.source);
}

function activeStatusLabel(status: string) {
  if (status === "active") return "启用";
  if (status === "disabled" || status === "inactive") return "停用";
  return status || "未知";
}

function locationStatusLabel(status: string) {
  if (status === "available") return "可用";
  if (status === "occupied") return "占用";
  if (status === "locked") return "锁定";
  return activeStatusLabel(status);
}

function locationAreaCode(locationCode: string) {
  return text(locationCode.split("-")[0]);
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}

function minText(left: string, right: string) {
  return left <= right ? left : right;
}

function maxText(left: string, right: string) {
  return left >= right ? left : right;
}

function shortId(value: string) {
  return value.length > 8 ? value.slice(0, 8) : value;
}

function text(value: unknown) {
  if (typeof value === "string" && value.trim()) return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return "-";
}

function paramsText(params: Record<string, unknown>) {
  const entries = Object.entries(params);
  if (entries.length === 0) return "-";
  return entries
    .slice(0, 3)
    .map(([key, value]) => `${key}=${text(value)}`)
    .join(" / ");
}

function specialDrugCategoryOptionFromDictionaryItem(
  item: SystemDictionaryItem,
): SpecialDrugCategoryOption {
  const status = item.enabled ? "active" : "disabled";
  const disabledSuffix = item.enabled ? "" : "，已停用";
  return {
    value: item.item_code,
    label: `${item.item_name}（${item.item_code}${disabledSuffix}）`,
    status,
    requiresDualSign: item.params.requires_dual_sign === true,
  };
}
