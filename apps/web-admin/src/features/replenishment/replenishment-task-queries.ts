import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type ReplenishmentTask = {
  id: string;
  owner_id: string;
  task_no: string;
  trigger_mode: string;
  priority: string;
  strategy_id: string | null;
  source_location_id: string;
  source_batch_id: string;
  source_lpn_id: string | null;
  target_location_id: string;
  product_id: string;
  batch_no: string;
  qty: string;
  picked_qty: string;
  done_qty: string;
  status: string;
  operator_id: string | null;
  created_by: string;
  version: number;
};

export type ReplenishmentTaskListResponse = {
  data: ReplenishmentTask[];
  page: { count: number; next_cursor: string | null };
};

export type CreateReplenishmentTaskRequest = {
  source_location_id: string;
  source_batch_id: string;
  target_location_id: string;
  qty: string;
  source_lpn_id?: string | null;
};

export const replenishmentTasksQueryKey = ["replenishment", "tasks"] as const;

function idempotencyKey() {
  return `web-m3-replenishment-task-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}

function asData<T>(result: { data?: unknown; error?: unknown; response: { status: number } }, fallback: string): T {
  if (!result.data) {
    throw new ApiError(result.error, fallback, result.response.status);
  }
  return result.data as T;
}

export function useReplenishmentTasksQuery() {
  return useQuery<ReplenishmentTaskListResponse, ApiError>({
    queryKey: replenishmentTasksQueryKey,
    queryFn: async () => {
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
      const result = await api.GET("/api/v1/replenishment/tasks");
      return asData<ReplenishmentTaskListResponse>(result, "读取补货任务失败");
    },
    retry: false,
  });
}

export function useCreateReplenishmentTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (body: CreateReplenishmentTaskRequest) => {
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
      const result = await api.POST("/api/v1/replenishment/tasks", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      return asData<ReplenishmentTask>(result, "手工发起补货任务失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentTasksQueryKey }),
  });
}

export function useCancelReplenishmentTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; version: number; reason: string }) => {
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
      const result = await api.POST("/api/v1/replenishment/tasks/{id}/cancel", {
        params: { path: { id: input.id }, header: { "Idempotency-Key": idempotencyKey() } },
        body: { version: input.version, reason: input.reason },
      });
      return asData<ReplenishmentTask>(result, "取消补货任务失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentTasksQueryKey }),
  });
}

export function useReassignReplenishmentTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (input: { id: string; version: number }) => {
      // @ts-expect-error 补货路径由 T14 openapi-sync 收口
      const result = await api.POST("/api/v1/replenishment/tasks/{id}/reassign", {
        params: { path: { id: input.id }, header: { "Idempotency-Key": idempotencyKey() } },
        body: { version: input.version },
      });
      return asData<ReplenishmentTask>(result, "改派补货任务失败");
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: replenishmentTasksQueryKey }),
  });
}
