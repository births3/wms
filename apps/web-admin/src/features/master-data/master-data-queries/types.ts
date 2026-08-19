import type { components } from "@wms/api-client";

export type MasterDataViewId =
  | "m1-products"
  | "m1-business-partners"
  | "m1-warehouses"
  | "m1-zones"
  | "m1-locations"
  | "m1-system-dictionary";

export type Product = components["schemas"]["Product"];
export type ProductMappingTrace = components["schemas"]["ProductMappingTrace"];
export type ProductPackagingLevel = components["schemas"]["ProductPackagingLevel"];
export type ProductPackagingLevelInput = components["schemas"]["ProductPackagingLevelInput"];
export type Supplier = components["schemas"]["Supplier"];
export type Customer = components["schemas"]["Customer"];
export type CustomerAddress = components["schemas"]["CustomerAddress"];
export type CustomerAddressListResponse = components["schemas"]["CustomerAddressListResponse"];
export type CustomerProfile = components["schemas"]["CustomerProfile"];
export type CustomerQualification = components["schemas"]["CustomerQualification"];
export type Warehouse = components["schemas"]["Warehouse"];
export type WarehouseZone = components["schemas"]["WarehouseZone"];
export type Location = components["schemas"]["Location"];
export type CreateSupplierRequest = components["schemas"]["CreateSupplierRequest"];
export type UpdateSupplierRequest = components["schemas"]["UpdateSupplierRequest"];
export type CreateCustomerRequest = components["schemas"]["CreateCustomerRequest"];
export type UpdateCustomerRequest = components["schemas"]["UpdateCustomerRequest"];
export type CreateCustomerAddressRequest = components["schemas"]["CreateCustomerAddressRequest"];
export type UpdateCustomerAddressRequest = components["schemas"]["UpdateCustomerAddressRequest"];
export type UpsertCustomerProfileRequest = components["schemas"]["UpsertCustomerProfileRequest"];
export type CreateWarehouseRequest = components["schemas"]["CreateWarehouseRequest"];
export type UpdateWarehouseRequest = components["schemas"]["UpdateWarehouseRequest"];
export type CreateWarehouseZoneRequest = components["schemas"]["CreateWarehouseZoneRequest"];
export type UpdateWarehouseZoneRequest = components["schemas"]["UpdateWarehouseZoneRequest"];
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
  warehouseFields?: WarehouseMasterDataFields;
  zoneFields?: WarehouseZoneMasterDataFields;
  locationFields?: LocationMasterDataFields;
  partnerKind?: BusinessPartnerKind;
  partnerTypeLabel?: string;
  searchText: string;
}

export interface ProductMasterDataFields {
  approvalNo?: string | null;
  attrs: Record<string, unknown>;
  barcode69?: string | null;
  dosageForm?: string | null;
  electronicRegulatoryCode?: string | null;
  heightMm: string;
  lengthMm: string;
  manufacturer?: string | null;
  mappingTraces: ProductMappingTrace[];
  packagingLevels: ProductPackagingDisplayLevel[];
  packagingText: string;
  specialDrugCategoryCode?: string | null;
  spec: string;
  storageCondition?: string | null;
  udiCode?: string | null;
  volumeCm3: string;
  weightG: string;
  widthMm: string;
}

export interface ProductPackagingDisplayLevel {
  id: string;
  unitCode: string;
  unitName: string;
  ratioToBase: number;
  isBase: boolean;
  isDefault: boolean;
  sortOrder: number;
}

export interface WarehouseMasterDataFields {
  warehouseType: string;
}

export interface LocationMasterDataFields {
  owner: string;
  boundOwnerId: string | null;
  /** 仓库业务展示（编码 · 名称），非 UUID */
  warehouse: string;
  warehouseId: string;
  /** 库区业务展示（区域码或可读别名），非 UUID */
  zone: string;
  zoneId: string;
  area: string;
  rowNo: string;
  columnNo: string;
  layerNo: string;
  locationType: string;
  locationTypeCode: string;
  volume: string;
  maxVolumeCm3: string;
  usedVolumeCm3: string;
  remainingVolumeCm3: string;
  maxSku: string;
}

export interface WarehouseZoneMasterDataFields {
  owner: string;
  /** 仓库业务展示（编码 · 名称），非 UUID */
  warehouse: string;
  warehouseId: string;
  /** 库区业务展示，非 UUID */
  zone: string;
  zoneId: string;
  locationCount: string;
  availableLocationCount: string;
}

/** 仓库 ID → 业务可读码/名，供库位 / 库区列表解析 */
export interface WarehouseRef {
  id: string;
  code: string;
  name: string;
}

export interface SystemDictionaryPaneItem {
  id: string;
  code: string;
  name: string;
  source: string;
  enabled: boolean;
  sortOrder: number;
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
