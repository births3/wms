import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components, operations } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { inventoryBatchesQueryKey } from "@/features/inventory/inventory-queries";
import { api } from "@/lib/api";

export type ReconciliationItem = components["schemas"]["ReconciliationItem"];
export type ReconciliationRule = components["schemas"]["ReconciliationRule"];
export type ReconciliationFilters =
  NonNullable<operations["list_reconciliation_items"]["parameters"]["query"]>;
export type ReconciliationDisposition = components["schemas"]["ReconciliationDisposition"];

export const reconciliationItemsQueryKey = ["m-rc", "items"] as const;
export const reconciliationRuleQueryKey = ["m-rc", "rule"] as const;

export function useReconciliationItemsQuery(filters: ReconciliationFilters) {
  return useInfiniteQuery({
    queryKey: [...reconciliationItemsQueryKey, filters],
    initialPageParam: undefined as string | undefined,
    queryFn: async ({ pageParam }) => {
      const result = await api.GET("/api/v1/reconciliation/items", {
        params: { query: { ...filters, cursor: pageParam, limit: 50 } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取库存对账差异失败", result.response.status);
      }
      return result.data;
    },
    getNextPageParam: (lastPage) => lastPage.page.next_cursor ?? undefined,
    retry: false,
  });
}

export function useReconciliationRuleQuery() {
  return useQuery({
    queryKey: reconciliationRuleQueryKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/reconciliation/rule");
      if (!result.data) {
        throw new ApiError(result.error, "读取对账频率失败", result.response.status);
      }
      return result.data;
    },
    retry: false,
  });
}

export function useUpdateReconciliationRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    ReconciliationRule,
    ApiError,
    components["schemas"]["UpsertReconciliationRuleRequest"]
  >({
    mutationFn: async (body) => {
      const result = await api.PUT("/api/v1/reconciliation/rule", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-rc-rule") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "保存对账频率失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: reconciliationRuleQueryKey }),
  });
}

export function useSetReconciliationIsolationMutation() {
  const queryClient = useQueryClient();
  return useMutation<number, ApiError, { item_ids: string[]; isolate: boolean }>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/reconciliation/items/isolation", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-rc-isolation") } },
        body,
      });
      if (result.data === undefined) {
        throw new ApiError(result.error, "更新对账隔离状态失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: reconciliationItemsQueryKey });
      void queryClient.invalidateQueries({ queryKey: inventoryBatchesQueryKey });
    },
  });
}

export function useResolveReconciliationMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    ReconciliationItem,
    ApiError,
    {
      id: string;
      disposition: ReconciliationDisposition;
      allocations: components["schemas"]["ReconciliationInventoryAllocation"][];
    }
  >({
    mutationFn: async ({ id, ...body }) => {
      const result = await api.POST("/api/v1/reconciliation/items/{id}/resolve", {
        params: {
          path: { id },
          header: { "Idempotency-Key": idempotencyKey("web-rc-resolve") },
        },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "处理库存对账差异失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: reconciliationItemsQueryKey }),
  });
}

function idempotencyKey(prefix: string) {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`;
}
