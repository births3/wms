import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type InventoryCountSummary = components["schemas"]["InventoryCount"];
export type MaintenanceTask = components["schemas"]["MaintenanceTask"];
export type InventoryRelocation = components["schemas"]["InventoryRelocation"];
type CreateInventoryCountRequest = components["schemas"]["CreateInventoryCountRequest"];
type SubmitInventoryCountLineRequest = components["schemas"]["SubmitInventoryCountLineRequest"];
type ApproveInventoryCountRequest = components["schemas"]["ApproveInventoryCountRequest"];
type CreateMaintenanceRecordRequest = components["schemas"]["CreateMaintenanceRecordRequest"];
type RelocateInventoryRequest = components["schemas"]["RelocateInventoryRequest"];

const idempotencyKey = (prefix: string) => `${prefix}-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`;

export function useInventoryCountsQuery() {
  return useQuery<InventoryCountSummary[], ApiError>({
    queryKey: ["inventory", "counts"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/inventory/counts");
      if (!result.data) throw new ApiError(result.error, "读取盘点单失败", result.response.status);
      return result.data.data;
    },
  });
}

export function useCreateInventoryCountMutation() {
  const client = useQueryClient();
  return useMutation<InventoryCountSummary, ApiError, CreateInventoryCountRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/inventory/counts", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-m3-count") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "创建盘点单失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventory", "counts"] }),
  });
}

export function useSubmitInventoryCountLineMutation() {
  const client = useQueryClient();
  return useMutation<
    components["schemas"]["InventoryCountLine"],
    ApiError,
    { countId: string; lineId: string; physical_qty: number }
  >({
    mutationFn: async ({ countId, lineId, physical_qty }) => {
      const result = await api.POST("/api/v1/inventory/counts/{id}/lines/{line_id}/submit", {
        params: {
          path: { id: countId, line_id: lineId },
          header: { "Idempotency-Key": idempotencyKey("web-m3-count-line") },
        },
        body: { physical_qty: String(physical_qty) } satisfies SubmitInventoryCountLineRequest,
      });
      if (!result.data) throw new ApiError(result.error, "提交实盘数量失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventory", "counts"] }),
  });
}

export function useApproveInventoryCountMutation() {
  const client = useQueryClient();
  return useMutation<
    InventoryCountSummary,
    ApiError,
    { countId: string; approval_source?: string; approval_id?: string }
  >({
    mutationFn: async ({ countId, approval_source, approval_id }) => {
      const result = await api.POST("/api/v1/inventory/counts/{id}/approve", {
        params: {
          path: { id: countId },
          header: { "Idempotency-Key": idempotencyKey("web-m3-count-approve") },
        },
        body: {
          approval_source: approval_source ?? "盘点",
          approval_id: approval_id ?? countId,
        } satisfies ApproveInventoryCountRequest,
      });
      if (!result.data) throw new ApiError(result.error, "审批盘点差异失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventory", "counts"] }),
  });
}

export function useMaintenanceTasksQuery() {
  return useQuery<MaintenanceTask[], ApiError>({
    queryKey: ["inventory", "maintenance-tasks"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/inventory/maintenance/tasks", { params: { query: { page: 1, page_size: 200 } } });
      if (!result.data) throw new ApiError(result.error, "读取养护任务失败", result.response.status);
      return result.data.data;
    },
  });
}

export function useGenerateMaintenanceTasksMutation() {
  const client = useQueryClient();
  return useMutation<void, ApiError>({
    mutationFn: async () => {
      const result = await api.POST("/api/v1/inventory/maintenance/tasks/generate");
      if (result.error) throw new ApiError(result.error, "生成养护计划失败", result.response.status);
    },
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventory", "maintenance-tasks"] }),
  });
}

export function useCreateMaintenanceRecordMutation() {
  const client = useQueryClient();
  return useMutation<components["schemas"]["MaintenanceRecord"], ApiError, CreateMaintenanceRecordRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/inventory/maintenance/records", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-m3-maint") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "提交养护结果失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventory", "maintenance-tasks"] }),
  });
}

export function useInventoryRelocationsQuery() {
  return useQuery<InventoryRelocation[], ApiError>({
    queryKey: ["inventory", "relocations"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/inventory/relocations");
      if (!result.data) throw new ApiError(result.error, "读取移库记录失败", result.response.status);
      return result.data.data;
    },
  });
}

export function useRelocateInventoryMutation() {
  const client = useQueryClient();
  return useMutation<InventoryRelocation, ApiError, RelocateInventoryRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/inventory/relocations", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-m3-relocate") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "移库失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["inventory", "relocations"] });
      void client.invalidateQueries({ queryKey: ["inventory", "batches"] });
    },
  });
}
