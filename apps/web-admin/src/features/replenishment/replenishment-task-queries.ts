import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type ReplenishmentTask = components["schemas"]["ReplenishmentTask"];
export type ReplenishmentTaskListResponse =
  components["schemas"]["ReplenishmentTaskListResponse"];
export type CreateReplenishmentTaskRequest =
  components["schemas"]["CreateReplenishmentTaskRequest"];

export const replenishmentTasksQueryKey = ["replenishment", "tasks"] as const;

function idempotencyKey() {
  return `web-m3-replenishment-task-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}

function requireData<T>(
  data: T | undefined,
  error: unknown,
  status: number,
  fallback: string,
): T {
  if (data === undefined) {
    throw new ApiError(error, fallback, status);
  }
  return data;
}

export function useReplenishmentTasksQuery() {
  return useQuery<ReplenishmentTaskListResponse, ApiError>({
    queryKey: replenishmentTasksQueryKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/replenishment/tasks");
      return requireData(result.data, result.error, result.response.status, "读取补货任务失败");
    },
    retry: false,
  });
}

export function useCreateReplenishmentTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (body: CreateReplenishmentTaskRequest) => {
      const result = await api.POST("/api/v1/replenishment/tasks", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      return requireData(result.data, result.error, result.response.status, "手工发起补货任务失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentTasksQueryKey }),
  });
}

export function useCancelReplenishmentTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; version: number; reason: string }) => {
      const result = await api.POST("/api/v1/replenishment/tasks/{id}/cancel", {
        params: { path: { id: input.id }, header: { "Idempotency-Key": idempotencyKey() } },
        body: { version: input.version, reason: input.reason },
      });
      return requireData(result.data, result.error, result.response.status, "取消补货任务失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentTasksQueryKey }),
  });
}

export function useReassignReplenishmentTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; version: number }) => {
      const result = await api.POST("/api/v1/replenishment/tasks/{id}/reassign", {
        params: { path: { id: input.id }, header: { "Idempotency-Key": idempotencyKey() } },
        body: { version: input.version },
      });
      return requireData(result.data, result.error, result.response.status, "改派补货任务失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentTasksQueryKey }),
  });
}
