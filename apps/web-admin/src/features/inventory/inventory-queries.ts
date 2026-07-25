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

/** 库位历史查询（OpenAPI 生成前用显式类型，避免阻塞前端接线）。 */
export type LocationHistoryQuery = {
  location_code?: string;
  from?: string;
  to?: string;
  movement_type?: string;
  product_code?: string;
  batch_no?: string;
  days?: number;
};

export type LocationHistoryMovement = {
  id: string;
  owner_id: string;
  batch_id: string;
  movement_type: string;
  qty_delta: number;
  source_document_type: string;
  source_document_id: string;
  occurred_at: string;
  location_code?: string | null;
  from_location_code?: string | null;
  to_location_code?: string | null;
  lpn_code?: string | null;
  operator_user_id?: string | null;
  operator_name?: string | null;
  volume_delta_cm3?: number | null;
  product_code?: string | null;
  product_name?: string | null;
  batch_no?: string | null;
  expiry_date?: string | null;
};

export type LocationHistoryRisk = {
  risk_code: string;
  severity: string;
  message: string;
};

export type LocationHistoryProductShare = {
  product_code: string;
  product_name?: string | null;
  event_count: number;
  total_qty_delta: number;
};

export type LocationHistoryResponse = {
  location_code: string;
  data: LocationHistoryMovement[];
  risks: LocationHistoryRisk[];
  product_shares: LocationHistoryProductShare[];
  page: { count: number; next_cursor: string | null };
};

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

export function useInventoryBatchesQuery(
  query: InventoryBatchQuery = {},
  options: { enabled?: boolean; retry?: boolean } = {},
) {
  return useQuery<InventoryBatch[], ApiError>({
    queryKey: [...inventoryBatchesQueryKey, query],
    queryFn: () => listInventoryBatches(query),
    enabled: options.enabled ?? true,
    retry: options.retry,
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

export function useLocationHistoryQuery(query: LocationHistoryQuery, enabled: boolean) {
  return useQuery<LocationHistoryResponse, ApiError>({
    queryKey: ["inventory", "location-history", query],
    enabled: enabled && Boolean(query.location_code?.trim()),
    queryFn: async () => {
      const result = await api.GET("/api/v1/inventory/locations/history", {
        params: { query },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取库位历史失败", result.response.status);
      }
      return result.data as LocationHistoryResponse;
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
