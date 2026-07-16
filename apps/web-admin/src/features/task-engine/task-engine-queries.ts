import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type TaskGroup = components["schemas"]["TaskGroup"];
export type UpsertTaskGroupRequest = components["schemas"]["UpsertTaskGroupRequest"];
export type WarehouseTask = components["schemas"]["WarehouseTask"];
export type TaskTransitionAction = components["schemas"]["TaskTransitionAction"];
export type TransitionWarehouseTaskRequest = components["schemas"]["TransitionWarehouseTaskRequest"];
export type Warehouse = components["schemas"]["Warehouse"];
export type WarehouseZone = components["schemas"]["WarehouseZone"];
export type TaskWorker = components["schemas"]["TaskWorker"];

export interface WarehouseTaskFilters {
  status?: string;
  taskTypeCode?: string;
  warehouseId?: string;
  mineOnly?: boolean;
}

export const taskEngineQueryKey = ["mte", "task-engine"] as const;

export function useTaskEngineWarehousesQuery() {
  return useQuery<Warehouse[], ApiError>({
    queryKey: [...taskEngineQueryKey, "warehouses"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/master-data/warehouses");
      if (!result.data) throw new ApiError(result.error, "读取仓库档案失败", result.response.status);
      return result.data.data;
    },
    retry: false,
  });
}

export function useTaskEngineWarehouseZonesQuery() {
  return useQuery<WarehouseZone[], ApiError>({
    queryKey: [...taskEngineQueryKey, "warehouse-zones"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/master-data/warehouse-zones");
      if (!result.data) throw new ApiError(result.error, "读取库区档案失败", result.response.status);
      return result.data.data;
    },
    retry: false,
  });
}

export function useTaskGroupsQuery() {
  return useQuery<components["schemas"]["TaskGroupListResponse"], ApiError>({
    queryKey: [...taskEngineQueryKey, "task-groups"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/task-engine/task-groups");
      if (!result.data) throw new ApiError(result.error, "读取任务组失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useTaskWorkersQuery() {
  return useQuery<components["schemas"]["TaskWorkerListResponse"], ApiError>({
    queryKey: [...taskEngineQueryKey, "workers"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/task-engine/workers");
      if (!result.data) throw new ApiError(result.error, "读取任务组人员失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useUpsertTaskGroupMutation() {
  const queryClient = useQueryClient();
  return useMutation<TaskGroup, ApiError, { code: string; body: UpsertTaskGroupRequest }>({
    mutationFn: async ({ code, body }) => {
      const result = await api.PUT("/api/v1/task-engine/task-groups/{task_group_code}", {
        params: {
          path: { task_group_code: code },
          header: { "Idempotency-Key": idempotencyKey("web-mte-task-group") },
        },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "保存任务组失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: taskEngineQueryKey }),
  });
}

export function useWarehouseTasksQuery(filters: WarehouseTaskFilters, refetchInterval: number) {
  return useQuery<components["schemas"]["WarehouseTaskListResponse"], ApiError>({
    queryKey: [...taskEngineQueryKey, "tasks", filters],
    queryFn: async () => {
      const result = await api.GET("/api/v1/task-engine/tasks", {
        params: {
          query: {
            mine_only: filters.mineOnly ?? false,
            status: filters.status || undefined,
            task_type_code: filters.taskTypeCode || undefined,
            warehouse_id: filters.warehouseId || undefined,
            limit: 500,
          },
        },
      });
      if (!result.data) throw new ApiError(result.error, "读取任务队列失败", result.response.status);
      return result.data;
    },
    retry: false,
    refetchInterval: refetchInterval || false,
  });
}

export function useTransitionWarehouseTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation<WarehouseTask, ApiError, { taskId: string; body: TransitionWarehouseTaskRequest }>({
    mutationFn: async ({ taskId, body }) => {
      const result = await api.POST("/api/v1/task-engine/tasks/{task_id}/transitions", {
        params: {
          path: { task_id: taskId },
          header: { "Idempotency-Key": idempotencyKey(`web-mte-${body.action}`) },
        },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "更新任务状态失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: taskEngineQueryKey }),
  });
}

function idempotencyKey(prefix: string) {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}
