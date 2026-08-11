import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type Dock = components["schemas"]["Dock"];
export type CreateDockRequest = components["schemas"]["CreateDockRequest"];
export type CreateDockImportRequest = components["schemas"]["CreateDockImportRequest"];
export type UpdateDockRequest = components["schemas"]["UpdateDockRequest"];
export type DockAppointment = components["schemas"]["DockAppointment"];
export type CreateDockAppointmentRequest = components["schemas"]["CreateDockAppointmentRequest"];
export type UpdateDockAppointmentRequest = components["schemas"]["UpdateDockAppointmentRequest"];

export const dockQueryKey = ["m1", "docks"] as const;
export const dockAppointmentQueryKey = ["m1", "dock-appointments"] as const;

const DAY_MS = 24 * 60 * 60 * 1000;

function dockAppointmentWindow(now: Date) {
  return { from: now.toISOString(), to: new Date(now.getTime() + DAY_MS).toISOString() };
}

export function useDocksQuery(warehouseId: string | null) {
  return useQuery<Dock[], ApiError>({
    queryKey: [...dockQueryKey, warehouseId],
    queryFn: async () => {
      if (!warehouseId) return [];
      const result = await api.GET("/api/v1/docks", {
        params: { query: { warehouse_id: warehouseId } },
      });
      if (!result.data) throw new ApiError(result.error, "读取月台列表失败", result.response.status);
      return result.data;
    },
    enabled: Boolean(warehouseId),
    retry: false,
  });
}

export function useDockAppointmentsQuery(warehouseId: string | null) {
  return useQuery<DockAppointment[], ApiError>({
    queryKey: [...dockAppointmentQueryKey, warehouseId],
    queryFn: async () => {
      if (!warehouseId) return [];
      const window = dockAppointmentWindow(new Date());
      const result = await api.GET("/api/v1/dock-appointments", {
        params: { query: { warehouse_id: warehouseId, from: window.from, to: window.to } },
      });
      if (!result.data) throw new ApiError(result.error, "读取月台预约失败", result.response.status);
      return result.data.data;
    },
    enabled: Boolean(warehouseId),
    retry: false,
  });
}

export function useCreateDockMutation() {
  const queryClient = useQueryClient();
  return useMutation<Dock, ApiError, CreateDockRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/docks", {
        params: { header: { "Idempotency-Key": `dock-create-${crypto.randomUUID()}` } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "新增月台失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: dockQueryKey }),
  });
}

export function useUpdateDockMutation() {
  const queryClient = useQueryClient();
  return useMutation<Dock, ApiError, { id: string; body: UpdateDockRequest }>({
    mutationFn: async ({ id, body }) => {
      const result = await api.PATCH("/api/v1/docks/{id}", {
        params: { path: { id }, header: { "Idempotency-Key": `dock-update-${id}-${crypto.randomUUID()}` } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "保存月台状态失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: dockQueryKey }),
  });
}

export function useImportDocksMutation() {
  const queryClient = useQueryClient();
  return useMutation<Dock[], ApiError, CreateDockImportRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/docks/import", {
        params: { header: { "Idempotency-Key": `dock-import-${crypto.randomUUID()}` } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "导入月台失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: dockQueryKey }),
  });
}

export function useDeleteDockMutation() {
  const queryClient = useQueryClient();
  return useMutation<void, ApiError, string>({
    mutationFn: async (id) => {
      const result = await api.DELETE("/api/v1/docks/{id}", {
        params: { path: { id }, header: { "Idempotency-Key": `dock-delete-${id}-${crypto.randomUUID()}` } },
      });
      if (!result.response.ok) throw new ApiError(result.error, "删除月台失败", result.response.status);
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: dockQueryKey }),
  });
}

export function useCreateDockAppointmentMutation() {
  const queryClient = useQueryClient();
  return useMutation<DockAppointment, ApiError, CreateDockAppointmentRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/dock-appointments", {
        params: { header: { "Idempotency-Key": `dock-appointment-create-${crypto.randomUUID()}` } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "创建月台预约失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: dockAppointmentQueryKey }),
  });
}

export function useUpdateDockAppointmentMutation() {
  const queryClient = useQueryClient();
  return useMutation<DockAppointment, ApiError, { id: string; body: UpdateDockAppointmentRequest }>({
    mutationFn: async ({ id, body }) => {
      const result = await api.PATCH("/api/v1/dock-appointments/{id}", {
        params: { path: { id }, header: { "Idempotency-Key": `dock-appointment-update-${id}-${crypto.randomUUID()}` } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "变更月台预约失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: dockAppointmentQueryKey }),
  });
}

export function useCancelDockAppointmentMutation() {
  const queryClient = useQueryClient();
  return useMutation<DockAppointment, ApiError, string>({
    mutationFn: async (id) => {
      const result = await api.POST("/api/v1/dock-appointments/{id}/cancel", {
        params: { path: { id }, header: { "Idempotency-Key": `dock-appointment-cancel-${id}` } },
        body: {},
      });
      if (!result.data) throw new ApiError(result.error, "取消月台预约失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: dockAppointmentQueryKey }),
  });
}
