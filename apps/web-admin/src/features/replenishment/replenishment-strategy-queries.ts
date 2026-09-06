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
export type ReplenishmentPreviewResponse = components["schemas"]["ReplenishmentPreviewResponse"];
export type ReplenishmentLocationGroup = components["schemas"]["ReplenishmentLocationGroup"];
export type ReplenishmentLocationGroupListResponse =
  components["schemas"]["ReplenishmentLocationGroupListResponse"];
export type BindReplenishmentLocationsRequest =
  components["schemas"]["BindReplenishmentLocationsRequest"];
export type BindReplenishmentLocationsResponse =
  components["schemas"]["BindReplenishmentLocationsResponse"];
export type UpsertReplenishmentLocationGroupRequest =
  components["schemas"]["UpsertReplenishmentLocationGroupRequest"];
type ErrorResponse = components["schemas"]["ErrorResponse"];

export const replenishmentStrategiesQueryKey = ["replenishment", "strategies"] as const;
export const replenishmentLocationGroupsQueryKey = ["replenishment", "location-groups"] as const;

function idempotencyKey() {
  return `web-m3-replenishment-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}

function asErrorResponse(error: unknown): ErrorResponse | undefined {
  if (typeof error !== "object" || error === null) return undefined;
  const candidate = error as Partial<ErrorResponse>;
  return typeof candidate.code === "string" && typeof candidate.message === "string"
    ? (error as ErrorResponse)
    : undefined;
}

function requireData<T>(
  data: T | undefined,
  error: unknown,
  status: number,
  fallback: string,
): T {
  if (data === undefined) {
    throw new ApiError(asErrorResponse(error), fallback, status);
  }
  return data;
}

export type ReplenishmentStrategyListQuery = {
  keyword?: string;
  enabled?: boolean;
  scope_type?: string;
  target_type?: string;
};

export function useReplenishmentStrategiesQuery(filters?: ReplenishmentStrategyListQuery) {
  return useQuery<ReplenishmentStrategyListResponse, ApiError>({
    queryKey: [...replenishmentStrategiesQueryKey, filters ?? {}],
    queryFn: async () => {
      const result = await api.GET("/api/v1/replenishment/strategies", {
        params: {
          query: {
            keyword: emptyToUndefined(filters?.keyword),
            enabled: filters?.enabled,
            scope_type: emptyToUndefined(filters?.scope_type),
            target_type: emptyToUndefined(filters?.target_type),
          },
        },
      });
      return requireData(result.data, result.error, result.response.status, "读取补货策略失败");
    },
    retry: false,
  });
}

function emptyToUndefined(value: string | undefined) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

export function useCreateReplenishmentStrategyMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (body: UpsertReplenishmentStrategyRequest) => {
      const result = await api.POST("/api/v1/replenishment/strategies", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      return requireData(result.data, result.error, result.response.status, "保存补货策略失败");
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
      return requireData(result.data, result.error, result.response.status, "更新补货策略失败");
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
      return requireData(result.data, result.error, result.response.status, "停用补货策略失败");
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
      return requireData(result.data, result.error, result.response.status, "预览命中位失败");
    },
  });
}

export function useBindReplenishmentLocationsMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string } & BindReplenishmentLocationsRequest) => {
      const result = await api.PUT("/api/v1/replenishment/strategies/{id}/locations", {
        params: { path: { id: input.id }, header: { "Idempotency-Key": idempotencyKey() } },
        body: { location_ids: input.location_ids },
      });
      return requireData(result.data, result.error, result.response.status, "挂接拣选位失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentStrategiesQueryKey }),
  });
}

export function useReplenishmentLocationGroupsQuery() {
  return useQuery<ReplenishmentLocationGroup[], ApiError>({
    queryKey: replenishmentLocationGroupsQueryKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/replenishment/location-groups");
      return requireData(
        result.data,
        result.error,
        result.response.status,
        "读取补货库位组失败",
      ).data;
    },
    retry: false,
  });
}

export function useUpsertReplenishmentLocationGroupMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (body: UpsertReplenishmentLocationGroupRequest) => {
      const result = await api.POST("/api/v1/replenishment/location-groups", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      return requireData(result.data, result.error, result.response.status, "保存库位组失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentLocationGroupsQueryKey }),
  });
}
