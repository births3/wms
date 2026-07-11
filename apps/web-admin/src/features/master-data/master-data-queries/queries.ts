import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiError } from "@/features/auth/auth-queries";

import {
  type BatchCreateLocationsRequest,
  type CreateCustomerRequest,
  type CreateLocationRequest,
  type CreateProductRequest,
  type CreateSupplierRequest,
  type CreateWarehouseRequest,
  type CreateWarehouseZoneRequest,
  type MasterDataRow,
  type MasterDataViewId,
  type SpecialDrugCategoryOption,
  type SystemDictionaryOption,
  type SystemDictionaryPaneGroup,
  type UpdateCustomerRequest,
  type UpdateLocationRequest,
  type UpdateProductRequest,
  type UpdateSupplierRequest,
  type UpdateWarehouseRequest,
  type UpdateWarehouseZoneRequest,
    specialDrugCategoryDictCode,
  masterDataQueryKey,
} from "./types";
import {
  batchCreateLocations,
  createCustomer,
  createLocation,
  createProduct,
  createSupplier,
  createWarehouse,
  createWarehouseZone,
  disableSystemDictionaryItem,
  listBusinessPartners,
  listCustomers,
  listLocations,
  listProducts,
  listSpecialDrugCategoryOptions,
  listSystemDictionaryGroups,
  listSystemDictionaryItemOptions,
  listSystemDictionaryItems,
  listWarehouses,
  listWarehouseZones,
  upsertSystemDictionaryItem,
  updateCustomer,
  updateLocation,
  updateProduct,
  updateSupplier,
  updateWarehouse,
  updateWarehouseZone,
} from "./api";

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
const systemDictionaryOptionsQueryKey = [...masterDataQueryKey, "m1-system-dictionary", "options"] as const;

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

export function listMasterDataRows(viewId: MasterDataViewId): Promise<MasterDataRow[]> {
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

export function useBatchCreateLocationsMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: batchCreateLocations,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: [...masterDataQueryKey, "m1-locations"] });
      void queryClient.invalidateQueries({ queryKey: [...masterDataQueryKey, "m1-zones"] });
    },
  });
}

export {
  batchCreateLocations,
  createCustomer,
  createLocation,
  createProduct,
  createSupplier,
  createWarehouse,
  createWarehouseZone,
  listProducts,
  listCustomers,
  listSystemDictionaryItemOptions,
  listSystemDictionaryGroups,
  listSpecialDrugCategoryOptions,
  listSystemDictionaryItems,
  listWarehouses,
  listLocations,
  listBusinessPartners,
  listWarehouseZones,
  updateCustomer,
  updateLocation,
  updateProduct,
  updateSupplier,
  updateWarehouse,
  updateWarehouseZone,
  upsertSystemDictionaryItem,
  disableSystemDictionaryItem,
};

export type {
  BatchCreateLocationsRequest,
  CreateCustomerRequest,
  CreateLocationRequest,
  CreateProductRequest,
  CreateSupplierRequest,
  CreateWarehouseRequest,
  CreateWarehouseZoneRequest,
  UpdateCustomerRequest,
  UpdateLocationRequest,
  UpdateProductRequest,
  UpdateSupplierRequest,
  UpdateWarehouseRequest,
  UpdateWarehouseZoneRequest,
};


function useInvalidateSystemDictionary() {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: systemDictionaryGroupsQueryKey });
    void queryClient.invalidateQueries({ queryKey: systemDictionaryRowsQueryKey });
    void queryClient.invalidateQueries({ queryKey: specialDrugCategoriesQueryKey });
  };
}
