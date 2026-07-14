import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type InventoryStatusTransition = components["schemas"]["InventoryStatusTransition"];
export type UpsertInventoryStatusTransitionRequest = components["schemas"]["UpsertInventoryStatusTransitionRequest"];
export type UpsertInventoryStatusTransitionInput = { fromStatus: string; toStatus: string; body: UpsertInventoryStatusTransitionRequest };
export const inventoryStatusTransitionsQueryKey = ["inventory", "status-transitions"] as const;

export function useInventoryStatusTransitionsQuery() {
  return useQuery<components["schemas"]["InventoryStatusTransitionListResponse"], ApiError>({
    queryKey: inventoryStatusTransitionsQueryKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/inventory/status-transitions");
      if (!result.data) throw new ApiError(result.error, "读取库存状态转换规则失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useUpsertInventoryStatusTransitionMutation() {
  const queryClient = useQueryClient();
  return useMutation<InventoryStatusTransition, ApiError, UpsertInventoryStatusTransitionInput>({
    mutationFn: async ({ fromStatus, toStatus, body }) => {
      const result = await api.PUT("/api/v1/inventory/status-transitions/{from_status}/{to_status}", {
        params: { path: { from_status: fromStatus, to_status: toStatus }, header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "保存库存状态转换规则失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: inventoryStatusTransitionsQueryKey }),
  });
}

function idempotencyKey() { return `web-m3-status-config-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`; }
