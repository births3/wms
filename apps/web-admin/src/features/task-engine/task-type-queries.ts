import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type TaskType = components["schemas"]["TaskType"];
export type UpsertTaskTypeRequest = components["schemas"]["UpsertTaskTypeRequest"];

export const taskTypeQueryKey = ["mte", "task-types"] as const;

export function useTaskTypesQuery() {
  return useQuery<components["schemas"]["TaskTypeListResponse"], ApiError>({
    queryKey: taskTypeQueryKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/task-engine/task-types");
      if (!result.data) throw new ApiError(result.error, "读取任务类型失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useUpsertTaskTypeMutation() {
  const queryClient = useQueryClient();
  return useMutation<TaskType, ApiError, { code: string; body: UpsertTaskTypeRequest }>({
    mutationFn: async ({ code, body }) => {
      const result = await api.PUT("/api/v1/task-engine/task-types/{task_type_code}", {
        params: { path: { task_type_code: code }, header: { "Idempotency-Key": idempotencyKey("web-mte-task-type-save") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "保存任务类型失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: taskTypeQueryKey }),
  });
}

export function useSetTaskTypeEnabledMutation() {
  const queryClient = useQueryClient();
  return useMutation<TaskType, ApiError, { code: string; enabled: boolean }>({
    mutationFn: async ({ code, enabled }) => {
      const result = await api.PATCH("/api/v1/task-engine/task-types/{task_type_code}/enabled", {
        params: { path: { task_type_code: code }, header: { "Idempotency-Key": idempotencyKey("web-mte-task-type-enabled") } },
        body: { enabled },
      });
      if (!result.data) throw new ApiError(result.error, "更新任务类型状态失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: taskTypeQueryKey }),
  });
}

function idempotencyKey(prefix: string) {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}
