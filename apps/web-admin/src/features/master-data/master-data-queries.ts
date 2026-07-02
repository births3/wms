import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type MasterDataViewId =
  | "m1-products"
  | "m1-suppliers"
  | "m1-customers"
  | "m1-warehouses"
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
export type CreateCustomerRequest = components["schemas"]["CreateCustomerRequest"];
export type BatchCreateLocationsRequest = components["schemas"]["BatchCreateLocationsRequest"];
export type SystemDictionaryItem = components["schemas"]["SystemDictionaryItem"];
export type UpsertSystemDictionaryItemRequest =
  components["schemas"]["UpsertSystemDictionaryItemRequest"];
export type DisableSystemDictionaryItemRequest =
  components["schemas"]["DisableSystemDictionaryItemRequest"];

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
  locationFields?: LocationMasterDataFields;
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
  volume: string;
  maxSku: string;
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

export const masterDataQueryKey = ["master-data"] as const;
const systemDictionaryGroupsQueryKey = [
  ...masterDataQueryKey,
  "m1-system-dictionary",
  "two-pane",
] as const;
const systemDictionaryRowsQueryKey = [...masterDataQueryKey, "m1-system-dictionary"] as const;

export function useMasterDataRowsQuery(viewId: MasterDataViewId) {
  return useQuery<MasterDataRow[], ApiError>({
    queryKey: [...masterDataQueryKey, viewId],
    queryFn: () => listMasterDataRows(viewId),
  });
}

export function useSystemDictionaryGroupsQuery() {
  return useQuery<SystemDictionaryPaneGroup[], ApiError>({
    queryKey: systemDictionaryGroupsQueryKey,
    queryFn: listSystemDictionaryGroups,
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
    case "m1-suppliers":
      return listSuppliers();
    case "m1-customers":
      return listCustomers();
    case "m1-warehouses":
      return listWarehouses();
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

async function listWarehouses(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/warehouses");
  if (!result.data) {
    throw new ApiError(result.error, "读取仓库档案失败", result.response.status);
  }
  return result.data.data.map(warehouseRow);
}

async function listLocations(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/locations");
  if (!result.data) {
    throw new ApiError(result.error, "读取库位档案失败", result.response.status);
  }
  return result.data.data.map(locationRow);
}

export async function batchCreateLocations(
  request: BatchCreateLocationsRequest,
): Promise<MasterDataRow[]> {
  const result = await api.POST("/api/v1/master-data/locations/batch-create", {
    params: {
      header: { "Idempotency-Key": idempotencyKey("web-m1-location-batch") },
    },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "批量新增库位失败", result.response.status);
  }
  return result.data.data.map(locationRow);
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

async function listSystemDictionaryItems(): Promise<MasterDataRow[]> {
  return (await fetchDocumentTypeDictionaryItems()).map(systemDictionaryRow);
}

async function listSystemDictionaryGroups(): Promise<SystemDictionaryPaneGroup[]> {
  const items = await fetchDocumentTypeDictionaryItems();
  return [
    {
      code: "document_type",
      name: "单据类型",
      items: items.map(systemDictionaryPaneItem),
    },
  ];
}

async function fetchDocumentTypeDictionaryItems(): Promise<SystemDictionaryItem[]> {
  const result = await api.GET("/api/v1/system-dictionaries/{dict_code}/items", {
    params: { path: { dict_code: "document_type" } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取系统字典失败", result.response.status);
  }
  return result.data.data;
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
    },
  });
}

function supplierRow(item: Supplier): MasterDataRow {
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
  });
}

function customerRow(item: Customer): MasterDataRow {
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

function locationRow(item: Location): MasterDataRow {
  const locationType = locationTypeLabel(item.location_type);
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
      volume,
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
  };
}

function row(input: Omit<MasterDataRow, "searchText">): MasterDataRow {
  const locationSearchText = input.locationFields ? Object.values(input.locationFields) : [];
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
      ...locationSearchText,
    ]
      .join(" ")
      .toLowerCase(),
  };
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

function locationTypeLabel(type: string) {
  if (type === "storage") return "存储位";
  if (type === "case_pick" || type === "box_pick" || type === "carton_pick") return "箱拣位";
  if (type === "piece_pick" || type === "each_pick") return "零拣位";
  return text(type);
}

function locationAreaCode(locationCode: string) {
  return text(locationCode.split("-")[0]);
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
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
