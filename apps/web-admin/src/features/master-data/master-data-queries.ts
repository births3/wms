import { useQuery } from "@tanstack/react-query";
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
type SystemDictionaryItem = components["schemas"]["SystemDictionaryItem"];

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
  updatedAt: string;
  searchText: string;
}

export const masterDataQueryKey = ["master-data"] as const;

export function useMasterDataRowsQuery(viewId: MasterDataViewId) {
  return useQuery<MasterDataRow[], ApiError>({
    queryKey: [...masterDataQueryKey, viewId],
    queryFn: () => listMasterDataRows(viewId),
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

async function listSystemDictionaryItems(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/system-dictionaries/{dict_code}/items", {
    params: { path: { dict_code: "document_type" } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取系统字典失败", result.response.status);
  }
  return result.data.data.map(systemDictionaryRow);
}

function productRow(item: Product): MasterDataRow {
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
    extraValue: text(item.attrs.storage_condition ?? item.attrs.storage),
    updatedAt: item.updated_at,
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
    updatedAt: item.updated_at,
  });
}

function locationRow(item: Location): MasterDataRow {
  return row({
    id: item.id,
    code: item.location_code,
    name: `${item.row_no}-${item.column_no}-${item.layer_no}`,
    status: item.status,
    statusLabel: item.status === "available" ? "可用" : activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "库位类型",
    primaryValue: item.location_type,
    secondaryLabel: "容量",
    secondaryValue: `${item.used_volume_cm3}/${item.max_volume_cm3} cm³`,
    extraLabel: "最大 SKU",
    extraValue: String(item.max_sku_count),
    updatedAt: item.updated_at,
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
    updatedAt: item.updated_at,
  });
}

function row(input: Omit<MasterDataRow, "searchText">): MasterDataRow {
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
    ]
      .join(" ")
      .toLowerCase(),
  };
}

function activeStatusLabel(status: string) {
  if (status === "active") return "启用";
  if (status === "disabled" || status === "inactive") return "停用";
  return status || "未知";
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
