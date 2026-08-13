import { api } from "@/lib/api";

import { ApiError } from "@/features/auth/auth-queries";
import {
  type BatchCreateLocationsRequest,
  type CreateCustomerRequest,
  type CreateCustomerAddressRequest,
  type CustomerAddress,
  type CustomerProfile,
  type CreateLocationRequest,
  type CreateSupplierRequest,
  type CreateWarehouseRequest,
  type CreateWarehouseZoneRequest,
  type DisableSystemDictionaryItemRequest,
  type MasterDataRow,
  type SpecialDrugCategoryOption,
  type SystemDictionaryItem,
  type SystemDictionaryOption,
  type SystemDictionaryPaneGroup,
  type UpdateCustomerRequest,
  type UpdateCustomerAddressRequest,
  type UpsertCustomerProfileRequest,
  type UpdateLocationRequest,
  type UpdateSupplierRequest,
  type UpdateWarehouseRequest,
  type UpdateWarehouseZoneRequest,
  type UpsertSystemDictionaryItemRequest,
  type WarehouseRef,
  specialDrugCategoryDictCode,
} from "./types";
import {
  idempotencyKey,
  locationRow,
  customerRow,
  productRow,
  specialDrugCategoryOptionFromDictionaryItem,
  supplierRow,
  systemDictionaryPaneItem,
  systemDictionaryRow,
  warehouseRefFromWarehouse,
  warehouseRow,
  warehouseZoneRow,
} from "./mappers";

const systemDictionaryDefinitions = [
  { code: "document_type", name: "单据类型" },
  { code: "print_template_type", name: "打印模板类型" },
  { code: specialDrugCategoryDictCode, name: "特殊药品分类" },
  { code: "temperature_zone", name: "库区温区" },
  { code: "quality_color", name: "库区色标" },
  { code: "zone_type", name: "库区类型" },
  { code: "location_type", name: "库位类型" },
  { code: "inventory_policy", name: "库存管理参数" },
] as const;

export async function listProducts(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/products");
  if (!result.data) {
    throw new ApiError(result.error, "读取商品档案失败", result.response.status);
  }
  return result.data.data.map(productRow);
}

export async function listProductsPage(page: number, pageSize: number) {
  const result = await api.GET("/api/v1/master-data/products", {
    params: { query: { page, page_size: pageSize } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取商品档案失败", result.response.status);
  }
  return {
    rows: result.data.data.map(productRow),
    total: result.data.page.total ?? result.data.data.length,
  };
}

export async function listSuppliers(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/suppliers");
  if (!result.data) {
    throw new ApiError(result.error, "读取供应商档案失败", result.response.status);
  }
  return result.data.data.map(supplierRow);
}

export async function listCustomers(): Promise<MasterDataRow[]> {
  const result = await api.GET("/api/v1/master-data/customers");
  if (!result.data) {
    throw new ApiError(result.error, "读取客户档案失败", result.response.status);
  }
  return result.data.data.map(customerRow);
}

export async function listCustomerAddresses(customerId: string): Promise<CustomerAddress[]> {
  const result = await api.GET("/api/v1/master-data/customers/{customer_id}/addresses", {
    params: { path: { customer_id: customerId } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取客户地址失败", result.response.status);
  }
  return result.data.data;
}

export async function createCustomerAddress(input: {
  customerId: string;
  request: CreateCustomerAddressRequest;
}): Promise<CustomerAddress> {
  const result = await api.POST("/api/v1/master-data/customers/{customer_id}/addresses", {
    params: {
      path: { customer_id: input.customerId },
      header: { "Idempotency-Key": idempotencyKey(`web-m1-customer-address-create-${input.customerId}`) },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "新增客户地址失败", result.response.status);
  }
  return result.data;
}

export async function updateCustomerAddress(input: {
  customerId: string;
  addressId: string;
  request: UpdateCustomerAddressRequest;
}): Promise<CustomerAddress> {
  const result = await api.PATCH(
    "/api/v1/master-data/customers/{customer_id}/addresses/{address_id}",
    {
      params: {
        path: { customer_id: input.customerId, address_id: input.addressId },
        header: {
          "Idempotency-Key": idempotencyKey(
            `web-m1-customer-address-update-${input.addressId}`,
          ),
        },
      },
      body: input.request,
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "保存客户地址失败", result.response.status);
  }
  return result.data;
}

export async function getCustomerProfile(customerId: string): Promise<CustomerProfile> {
  const result = await api.GET("/api/v1/master-data/customers/{customer_id}/profile", {
    params: { path: { customer_id: customerId } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取客户档案扩展信息失败", result.response.status);
  }
  return result.data;
}

export async function upsertCustomerProfile(input: {
  customerId: string;
  request: UpsertCustomerProfileRequest;
}): Promise<CustomerProfile> {
  const result = await api.PATCH("/api/v1/master-data/customers/{customer_id}/profile", {
    params: {
      path: { customer_id: input.customerId },
      header: { "Idempotency-Key": idempotencyKey(`web-m1-customer-profile-${input.customerId}`) },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存客户档案扩展信息失败", result.response.status);
  }
  return result.data;
}

export async function listBusinessPartners(): Promise<MasterDataRow[]> {
  const [suppliers, customers] = await Promise.all([listSuppliers(), listCustomers()]);
  return [...suppliers, ...customers];
}

export async function listWarehouses(): Promise<MasterDataRow[]> {
  const warehouses = await fetchWarehouses();
  return warehouses.map(warehouseRow);
}

export async function listLocations(): Promise<MasterDataRow[]> {
  const [result, locationTypeOptions, warehouseRefs] = await Promise.all([
    api.GET("/api/v1/master-data/locations"),
    listSystemDictionaryItemOptions("location_type"),
    listWarehouseRefs(),
  ]);
  if (!result.data) {
    throw new ApiError(result.error, "读取库位档案失败", result.response.status);
  }
  const locationTypeLabels = new Map(locationTypeOptions);
  return result.data.data.map((location) =>
    locationRow(location, locationTypeLabels, warehouseRefs),
  );
}

export async function listWarehouseZones(): Promise<MasterDataRow[]> {
  const [result, warehouses] = await Promise.all([
    api.GET("/api/v1/master-data/warehouse-zones"),
    listWarehouseRefs(),
  ]);
  if (!result.data) throw new ApiError(result.error, "读取库区档案失败", result.response.status);
  return result.data.data.map((zone) => warehouseZoneRow(zone, warehouses));
}

export async function createWarehouseZone(request: CreateWarehouseZoneRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/warehouse-zones", {
    body: request,
    params: { header: { "Idempotency-Key": idempotencyKey("warehouse-zone-create") } },
  });
  if (!result.data) throw new ApiError(result.error, "新建库区失败", result.response.status);
  return warehouseZoneRow(result.data, await listWarehouseRefs());
}

export async function updateWarehouseZone(input: { id: string; request: UpdateWarehouseZoneRequest }): Promise<MasterDataRow> {
  const result = await api.PATCH("/api/v1/master-data/warehouse-zones/{id}", {
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey(`warehouse-zone-update-${input.id}`) },
    },
    body: input.request,
  });
  if (!result.data) throw new ApiError(result.error, "保存库区失败", result.response.status);
  return warehouseZoneRow(result.data, await listWarehouseRefs());
}

export async function batchCreateLocations(
  request: BatchCreateLocationsRequest,
): Promise<MasterDataRow[]> {
  const [locationTypeOptions, warehouseRefs] = await Promise.all([
    listSystemDictionaryItemOptions("location_type"),
    listWarehouseRefs(),
  ]);
  const result = await api.POST("/api/v1/master-data/locations/batch-create", {
    params: {
      header: { "Idempotency-Key": idempotencyKey("web-m1-location-batch") },
    },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "批量新增库位失败", result.response.status);
  }
  const locationTypeLabels = new Map(locationTypeOptions);
  return result.data.data.map((location) =>
    locationRow(location, locationTypeLabels, warehouseRefs),
  );
}

export async function createSupplier(request: CreateSupplierRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/suppliers", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-m1-supplier-create") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "新建供应商失败", result.response.status);
  }
  return supplierRow(result.data);
}

export async function batchCreateSuppliers(
  requests: CreateSupplierRequest[],
): Promise<MasterDataRow[]> {
  const result = await api.POST("/api/v1/master-data/suppliers/batch-sync", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-m1-supplier-batch") } },
    body: requests,
  });
  if (!result.data) {
    throw new ApiError(result.error, "批量导入供应商失败", result.response.status);
  }
  return result.data.data.map(supplierRow);
}

export async function createCustomer(request: CreateCustomerRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/customers", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-m1-customer-create") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "新建客户失败", result.response.status);
  }
  return customerRow(result.data);
}

export async function batchCreateCustomers(
  requests: CreateCustomerRequest[],
): Promise<MasterDataRow[]> {
  const result = await api.POST("/api/v1/master-data/customers/batch-sync", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-m1-customer-batch") } },
    body: requests,
  });
  if (!result.data) {
    throw new ApiError(result.error, "批量导入客户失败", result.response.status);
  }
  return result.data.data.map(customerRow);
}

export async function updateSupplier(input: {
  id: string;
  request: UpdateSupplierRequest;
}): Promise<MasterDataRow> {
  const result = await api.PATCH("/api/v1/master-data/suppliers/{id}", {
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey(`web-m1-supplier-update-${input.id}`) },
    },
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
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey(`web-m1-customer-update-${input.id}`) },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存客户失败", result.response.status);
  }
  return customerRow(result.data);
}

export async function createWarehouse(request: CreateWarehouseRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/warehouses", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-m1-warehouse-create") } },
    body: request,
  });
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
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey(`web-m1-warehouse-update-${input.id}`) },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存仓库失败", result.response.status);
  }
  return warehouseRow(result.data);
}

export async function createLocation(request: CreateLocationRequest): Promise<MasterDataRow> {
  const result = await api.POST("/api/v1/master-data/locations", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-m1-location-create") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "新建库位失败", result.response.status);
  }
  const [locationTypeOptions, warehouseRefs] = await Promise.all([
    listSystemDictionaryItemOptions("location_type").catch(() => []),
    listWarehouseRefs().catch(() => new Map<string, WarehouseRef>()),
  ]);
  return locationRow(result.data, new Map(locationTypeOptions), warehouseRefs);
}

export async function updateLocation(input: {
  id: string;
  request: UpdateLocationRequest;
}): Promise<MasterDataRow> {
  const result = await api.PATCH("/api/v1/master-data/locations/{id}", {
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey(`web-m1-location-update-${input.id}`) },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存库位失败", result.response.status);
  }
  const [locationTypeOptions, warehouseRefs] = await Promise.all([
    listSystemDictionaryItemOptions("location_type").catch(() => []),
    listWarehouseRefs().catch(() => new Map<string, WarehouseRef>()),
  ]);
  return locationRow(result.data, new Map(locationTypeOptions), warehouseRefs);
}

async function fetchWarehouses() {
  const result = await api.GET("/api/v1/master-data/warehouses");
  if (!result.data) {
    throw new ApiError(result.error, "读取仓库档案失败", result.response.status);
  }
  return result.data.data;
}

async function listWarehouseRefs(): Promise<ReadonlyMap<string, WarehouseRef>> {
  const warehouses = await fetchWarehouses();
  return new Map(warehouses.map((item) => [item.id, warehouseRefFromWarehouse(item)]));
}

export async function listSystemDictionaryItems(): Promise<MasterDataRow[]> {
  const groups = await fetchSystemDictionaryGroupItems();
  return groups.flatMap((group) => group.items.map(systemDictionaryRow));
}

export async function listSystemDictionaryGroups(): Promise<SystemDictionaryPaneGroup[]> {
  const groups = await fetchSystemDictionaryGroupItems();
  return groups.map((group) => ({
    code: group.code,
    name: group.name,
    items: group.items.map(systemDictionaryPaneItem),
  }));
}

export async function fetchSystemDictionaryGroupItems() {
  return Promise.all(
    systemDictionaryDefinitions.map(async (definition) => ({
      ...definition,
      items: await fetchSystemDictionaryItems(definition.code),
    })),
  );
}

export async function fetchSystemDictionaryItems(dictCode: string): Promise<SystemDictionaryItem[]> {
  const result = await api.GET("/api/v1/system-dictionaries/{dict_code}/items", {
    params: { path: { dict_code: dictCode } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取系统字典失败", result.response.status);
  }
  return result.data.data;
}

export async function listSpecialDrugCategoryOptions(): Promise<SpecialDrugCategoryOption[]> {
  const items = await fetchSystemDictionaryItems(specialDrugCategoryDictCode);
  return items.map(specialDrugCategoryOptionFromDictionaryItem);
}

export async function listSystemDictionaryItemOptions(dictCode: string): Promise<SystemDictionaryOption[]> {
  const items = await fetchSystemDictionaryItems(dictCode);
  return items
    .filter((item) => item.enabled)
    .map((item) => [item.item_code, item.item_name] as const);
}

export async function upsertSystemDictionaryItem(input: {
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

export async function disableSystemDictionaryItem(input: {
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
