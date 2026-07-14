import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components, operations } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type InventoryBatch = components["schemas"]["InventoryBatch"];
export type InventoryBatchQuery = NonNullable<operations["list_inventory_batches"]["parameters"]["query"]>;
export type InventoryBatchTrace = components["schemas"]["InventoryBatchTrace"];
export type ChangeInventoryStatusRequest = components["schemas"]["ChangeInventoryStatusRequest"];
export type MarkInventoryRecallRequest = components["schemas"]["MarkInventoryRecallRequest"];
export type CancelInventoryRecallRequest = components["schemas"]["CancelInventoryRecallRequest"];

export interface InventoryExpiryPolicy {
  warningDays: number;
  source: "owner" | "global" | "default";
}

export const inventoryBatchesQueryKey = ["inventory", "batches"] as const;

async function listInventoryBatches(query: InventoryBatchQuery): Promise<InventoryBatch[]> {
  const result = await api.GET("/api/v1/inventory/batches", { params: { query } });
  if (!result.data) {
    throw new ApiError(result.error, "读取库存批次失败", result.response.status);
  }
  return result.data.data;
}

export function useInventoryBatchesQuery(query: InventoryBatchQuery = {}) {
  return useQuery<InventoryBatch[], ApiError>({
    queryKey: [...inventoryBatchesQueryKey, query],
    queryFn: () => listInventoryBatches(query),
  });
}

export function useInventoryExpiryPolicyQuery() {
  return useQuery<InventoryExpiryPolicy, ApiError>({
    queryKey: ["inventory", "expiry-policy"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/system-dictionaries/{dict_code}/items", {
        params: { path: { dict_code: "inventory_policy" } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取近效期配置失败", result.response.status);
      }
      const item = result.data.data.find(
        (candidate) => candidate.item_code === "expiry_warning_days" && candidate.enabled,
      );
      const warningDays = Number(item?.params?.warning_days);
      return {
        warningDays: Number.isInteger(warningDays) && warningDays >= 1 && warningDays <= 3650 ? warningDays : 180,
        source: item?.owner_id ? "owner" : item ? "global" : "default",
      };
    },
    staleTime: 60_000,
  });
}

export function useInventoryBatchTraceQuery(batchId: string, enabled: boolean) {
  return useQuery<InventoryBatchTrace, ApiError>({
    queryKey: ["inventory", "batch-trace", batchId],
    enabled: enabled && batchId !== "",
    queryFn: async () => {
      const result = await api.GET("/api/v1/inventory/batches/{id}/trace", {
        params: { path: { id: batchId } },
      });
      if (!result.data) throw new ApiError(result.error, "读取批次追溯失败", result.response.status);
      return result.data;
    },
  });
}

export function useChangeInventoryStatusMutation() {
  const queryClient = useQueryClient();
  return useMutation<InventoryBatch, ApiError, ChangeInventoryStatusRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/inventory/batches/status", {
        params: { header: { "Idempotency-Key": `web-m3-status-${globalThis.crypto?.randomUUID?.() ?? Date.now()}` } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "更新库存状态失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: inventoryBatchesQueryKey }),
  });
}

export function useMarkInventoryRecallMutation() {
  const queryClient = useQueryClient();
  return useMutation<InventoryBatch, ApiError, MarkInventoryRecallRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/inventory/batches/recall", {
        params: { header: { "Idempotency-Key": `web-m3-recall-${globalThis.crypto?.randomUUID?.() ?? Date.now()}` } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "标记库存召回失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: inventoryBatchesQueryKey }),
  });
}

export function useCancelInventoryRecallMutation() {
  const queryClient = useQueryClient();
  return useMutation<InventoryBatch, ApiError, CancelInventoryRecallRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/inventory/batches/recall/cancel", {
        params: { header: { "Idempotency-Key": `web-m3-recall-cancel-${globalThis.crypto?.randomUUID?.() ?? Date.now()}` } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "取消库存召回失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: inventoryBatchesQueryKey }),
  });
}
