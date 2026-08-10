import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiError } from "@/features/auth/auth-queries";

import {
  type BatchCreateLocationsRequest,
  type CreateCustomerRequest,
  type CreateCustomerAddressRequest,
  type CreateLocationRequest,
  type CreateSupplierRequest,
  type CreateWarehouseRequest,
  type CreateWarehouseZoneRequest,
  type MasterDataRow,
  type MasterDataViewId,
  type CustomerAddress,
  type CustomerProfile,
  type CustomerQualification,
  type SpecialDrugCategoryOption,
  type SystemDictionaryOption,
  type SystemDictionaryPaneGroup,
  type UpdateCustomerRequest,
  type UpdateCustomerAddressRequest,
  type UpsertCustomerProfileRequest,
  type UpdateLocationRequest,
  type UpdateSupplierRequest,
  type UpdateWarehouseRequest,
  type UpdateWarehouseZoneRequest,
    specialDrugCategoryDictCode,
  masterDataQueryKey,
} from "./types";
import {
  batchCreateLocations,
  createCustomer,
  createCustomerAddress,
  createLocation,
  createSupplier,
  createWarehouse,
  createWarehouseZone,
  disableSystemDictionaryItem,
  listBusinessPartners,
  listCustomers,
  listCustomerAddresses,
  getCustomerProfile,
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
  updateCustomerAddress,
  upsertCustomerProfile,
  updateLocation,
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
    // 列表接口当前为全量返回，缓存避免切换视图/重挂载重复拉取大表
    staleTime: 30_000,
  });
}

export function useCustomerAddressesQuery(customerId: string | null) {
  return useQuery<CustomerAddress[], ApiError>({
    queryKey: [...masterDataQueryKey, "customer-addresses", customerId],
    queryFn: () => listCustomerAddresses(customerId as string),
    enabled: Boolean(customerId),
  });
}

export function useCreateCustomerAddressMutation(customerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: CreateCustomerAddressRequest) =>
      createCustomerAddress({ customerId, request }),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [...masterDataQueryKey, "customer-addresses", customerId],
      });
    },
  });
}

export function useUpdateCustomerAddressMutation(customerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { addressId: string; request: UpdateCustomerAddressRequest }) =>
      updateCustomerAddress({ customerId, ...input }),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [...masterDataQueryKey, "customer-addresses", customerId],
      });
    },
  });
}

export function useCustomerProfileQuery(customerId: string | null) {
  return useQuery<CustomerProfile, ApiError>({
    queryKey: [...masterDataQueryKey, "customer-profile", customerId],
    queryFn: () => getCustomerProfile(customerId as string),
    enabled: Boolean(customerId),
  });
}

export function useUpsertCustomerProfileMutation(customerId: string) {
  const queryClient = useQueryClient();
  return useMutation<CustomerProfile, ApiError, UpsertCustomerProfileRequest>({
    mutationFn: (request) => upsertCustomerProfile({ customerId, request }),
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [...masterDataQueryKey, "customer-profile", customerId],
      });
    },
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

export {
  batchCreateLocations,
  createCustomer,
  createCustomerAddress,
  createLocation,
  createSupplier,
  createWarehouse,
  createWarehouseZone,
  listProducts,
  listCustomers,
  listCustomerAddresses,
  getCustomerProfile,
  listSystemDictionaryItemOptions,
  listSystemDictionaryGroups,
  listSpecialDrugCategoryOptions,
  listSystemDictionaryItems,
  listWarehouses,
  listLocations,
  listBusinessPartners,
  listWarehouseZones,
  updateCustomer,
  updateCustomerAddress,
  upsertCustomerProfile,
  updateLocation,
  updateSupplier,
  updateWarehouse,
  updateWarehouseZone,
  upsertSystemDictionaryItem,
  disableSystemDictionaryItem,
};

export type {
  BatchCreateLocationsRequest,
  CreateCustomerRequest,
  CreateCustomerAddressRequest,
  CustomerProfile,
  CreateLocationRequest,
  CreateSupplierRequest,
  CreateWarehouseRequest,
  CreateWarehouseZoneRequest,
  UpdateCustomerRequest,
  UpdateCustomerAddressRequest,
  UpsertCustomerProfileRequest,
  UpdateLocationRequest,
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
