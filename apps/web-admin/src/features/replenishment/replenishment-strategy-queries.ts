import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type ReplenishmentStrategy = components["schemas"]["ReplenishmentStrategy"];
export type UpsertReplenishmentStrategyRequest =
  components["schemas"]["UpsertReplenishmentStrategyRequest"];
export type ReplenishmentStrategyListResponse =
  components["schemas"]["ReplenishmentStrategyListResponse"];
export type ReplenishmentPreviewItem = components["schemas"]["ReplenishmentPreviewItem"];
export type ReplenishmentLocationGroup = components["schemas"]["ReplenishmentLocationGroup"];

export const replenishmentStrategiesQueryKey = ["replenishment", "strategies"] as const;
export const replenishmentLocationGroupsQueryKey = ["replenishment", "location-groups"] as const;

function idempotencyKey() {
  return `web-m3-replenishment-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}

function asData<T>(result: { data?: unknown; error?: unknown; response: { status: number } }, fallback: string): T {
  if (!result.data) {
    throw new ApiError(result.error, fallback, result.response.status);
  }
  return result.data as T;
}

export function useReplenishmentStrategiesQuery() {
  return useQuery<ReplenishmentStrategyListResponse, ApiError>({
    queryKey: replenishmentStrategiesQueryKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/replenishment/strategies");
      return asData<ReplenishmentStrategyListResponse>(result, "读取补货策略失败");
    },
    retry: false,
  });
}

export function useCreateReplenishmentStrategyMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (body: UpsertReplenishmentStrategyRequest) => {
      const result = await api.POST("/api/v1/replenishment/strategies", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      return asData<ReplenishmentStrategy>(result, "保存补货策略失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentStrategiesQueryKey }),
  });
}

export function useUpdateReplenishmentStrategyMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; body: UpsertReplenishmentStrategyRequest }) => {
      const result = await api.PUT("/api/v1/replenishment/strategies/{id}", {
        params: { path: { id: input.id }, header: { "Idempotency-Key": idempotencyKey() } },
        body: input.body,
      });
      return asData<ReplenishmentStrategy>(result, "更新补货策略失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentStrategiesQueryKey }),
  });
}

export function useDisableReplenishmentStrategyMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      const result = await api.POST("/api/v1/replenishment/strategies/{id}/disable", {
        params: { path: { id }, header: { "Idempotency-Key": idempotencyKey() } },
      });
      return asData<ReplenishmentStrategy>(result, "停用补货策略失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentStrategiesQueryKey }),
  });
}

export function usePreviewReplenishmentStrategyMutation() {
  return useMutation({
    mutationFn: async (id: string) => {
      const result = await api.GET("/api/v1/replenishment/strategies/{id}/preview", {
        params: { path: { id } },
      });
      return asData<{ data: ReplenishmentPreviewItem[] }>(result, "预览命中位失败");
    },
  });
}

export function useBindReplenishmentLocationsMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; location_ids: string[] }) => {
      const result = await api.PUT("/api/v1/replenishment/strategies/{id}/locations", {
        params: { path: { id: input.id }, header: { "Idempotency-Key": idempotencyKey() } },
        body: { location_ids: input.location_ids },
      });
      return asData<{ strategy_id: string; location_ids: string[] }>(result, "挂接拣选位失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentStrategiesQueryKey }),
  });
}

export function useReplenishmentLocationGroupsQuery() {
  return useQuery<ReplenishmentLocationGroup[], ApiError>({
    queryKey: replenishmentLocationGroupsQueryKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/replenishment/location-groups");
      if (!result.data) {
        return [];
      }
      const payload = result.data as { data?: ReplenishmentLocationGroup[] };
      return payload.data ?? [];
    },
    retry: false,
  });
}

export function useUpsertReplenishmentLocationGroupMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (body: {
      group_code: string;
      group_name: string;
      enabled: boolean;
      location_ids: string[];
    }) => {
      const result = await api.POST("/api/v1/replenishment/location-groups", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      return asData<ReplenishmentLocationGroup>(result, "保存库位组失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentLocationGroupsQueryKey }),
  });
}
