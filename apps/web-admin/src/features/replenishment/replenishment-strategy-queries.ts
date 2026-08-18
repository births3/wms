import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type ReplenishmentStrategy = {
  id: string;
  owner_id: string;
  strategy_code: string;
  strategy_name: string;
  scope_type: string;
  scope_ref: string;
  location_type: string;
  source_type: string;
  target_type: string;
  min_safety_threshold: string;
  max_replenish_target: string;
  trigger_modes: string[];
  enabled: boolean;
};

export type UpsertReplenishmentStrategyRequest = {
  strategy_code: string;
  strategy_name: string;
  scope_type: string;
  scope_ref: string;
  source_type: string;
  target_type: string;
  min_safety_threshold: string;
  max_replenish_target: string;
  trigger_modes: string[];
  enabled: boolean;
};

export type ReplenishmentStrategyListResponse = {
  data: ReplenishmentStrategy[];
  page: { count: number; next_cursor: string | null };
};

export type ReplenishmentPreviewItem = {
  location_id: string;
  location_code: string;
  product_id: string | null;
  available_qty: string;
  min_safety_threshold: string;
  max_replenish_target: string;
  would_trigger: boolean;
};

export type ReplenishmentLocationGroup = {
  id: string;
  owner_id: string;
  group_code: string;
  group_name: string;
  enabled: boolean;
  location_ids: string[];
};

export const replenishmentStrategiesQueryKey = ["replenishment", "strategies"] as const;
export const replenishmentLocationGroupsQueryKey = ["replenishment", "location-groups"] as const;

type UntypedCall = (
  url: string,
  init?: object,
) => Promise<{ data?: unknown; error?: unknown; response: { status: number } }>;

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
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
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
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
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
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
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
      const post = api.POST as UntypedCall;
      const result = await post(`/api/v1/replenishment/strategies/${id}/disable`, {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
      });
      return asData<ReplenishmentStrategy>(result, "停用补货策略失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentStrategiesQueryKey }),
  });
}

export function usePreviewReplenishmentStrategyMutation() {
  return useMutation({
    mutationFn: async (id: string) => {
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
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
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
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
      const get = api.GET as UntypedCall;
      const result = await get("/api/v1/replenishment/location-groups");
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
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
      const result = await api.POST("/api/v1/replenishment/location-groups", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      return asData<ReplenishmentLocationGroup>(result, "保存库位组失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentLocationGroupsQueryKey }),
  });
}
