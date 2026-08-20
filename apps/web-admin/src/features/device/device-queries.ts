import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

type ErrorResponse = components["schemas"]["ErrorResponse"];

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type Device = components["schemas"]["DeviceResponse"];
export type WcsTask = components["schemas"]["WcsTaskResponse"];
export type DeviceDashboardSummary = components["schemas"]["DeviceDashboardSummary"];
export type RegisterDeviceRequest = components["schemas"]["RegisterDeviceRequest"];
export type BindDeviceRequest = components["schemas"]["BindDeviceRequest"];

export const devicesQueryKey = ["device", "devices"] as const;
export const wcsTasksQueryKey = ["device", "wcs-tasks"] as const;

function idempotencyKey(prefix: string) {
  return `web-${prefix}-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}

function requireData<T>(
  data: T | undefined,
  error: ErrorResponse | undefined,
  status: number,
  fallback: string,
): T {
  if (data === undefined) {
    throw new ApiError(error, fallback, status);
  }
  return data;
}

export type DeviceListQuery = {
  device_type?: string;
  online_status?: string;
  enabled?: boolean;
};

export function useDevicesQuery(filters?: DeviceListQuery) {
  return useQuery<Device[], ApiError>({
    queryKey: [...devicesQueryKey, filters ?? {}],
    queryFn: async () => {
      const result = await api.GET("/api/v1/iot-devices", {
        params: {
          query: {
            device_type: filters?.device_type || undefined,
            online_status: filters?.online_status || undefined,
            enabled: filters?.enabled,
          },
        },
      });
      return requireData(result.data, result.error, result.response.status, "设备列表加载失败");
    },
  });
}

export function useRegisterDeviceMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (req: RegisterDeviceRequest) => {
      const result = await api.POST("/api/v1/iot-devices", {
        body: req,
        params: { header: { "Idempotency-Key": idempotencyKey("device-register") } },
      });
      return requireData(result.data, result.error, result.response.status, "设备注册失败");
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: devicesQueryKey });
    },
  });
}

export function useToggleDeviceEnabledMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, enabled }: { id: string; enabled: boolean }) => {
      const result = await api.PATCH("/api/v1/iot-devices/{id}", {
        params: { path: { id } },
        body: { enabled },
      });
      return requireData(result.data, result.error, result.response.status, "设备启停失败");
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: devicesQueryKey });
    },
  });
}

export function useUnbindDeviceMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ deviceId, reason }: { deviceId: string; reason: string }) => {
      const detail = await api.GET("/api/v1/iot-devices/{id}", {
        params: { path: { id: deviceId } },
      });
      const device = requireData(detail.data, detail.error, detail.response.status, "设备详情加载失败");
      const bindings = device.bindings ?? [];
      if (bindings.length === 0) {
        throw new ApiError(undefined, "该设备没有生效绑定", 422);
      }
      for (const binding of bindings) {
        const result = await api.POST("/api/v1/location-device-bindings/{id}/unbind", {
          params: { path: { id: binding.id } },
          body: { reason },
        });
        if (!result.response.ok) {
          throw new ApiError(result.error, "解绑失败", result.response.status);
        }
      }
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: devicesQueryKey });
    },
  });
}

export function useBindDeviceMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (req: BindDeviceRequest) => {
      const result = await api.POST("/api/v1/location-device-bindings", {
        body: req,
        params: { header: { "Idempotency-Key": idempotencyKey("device-bind") } },
      });
      return requireData(result.data, result.error, result.response.status, "库位绑定失败");
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: devicesQueryKey });
    },
  });
}

export type WcsTaskListQuery = {
  status?: string;
  task_type?: string;
};

export function useWcsTasksQuery(filters?: WcsTaskListQuery) {
  return useQuery<WcsTask[], ApiError>({
    queryKey: [...wcsTasksQueryKey, filters ?? {}],
    queryFn: async () => {
      const result = await api.GET("/api/v1/wcs-tasks", {
        params: {
          query: {
            status: filters?.status || undefined,
            task_type: filters?.task_type || undefined,
          },
        },
      });
      return requireData(result.data, result.error, result.response.status, "指令任务列表加载失败");
    },
  });
}

export function useDeviceDashboardQuery() {
  return useQuery<DeviceDashboardSummary, ApiError>({
    queryKey: ["device", "dashboard"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/device-dashboard");
      return requireData(result.data, result.error, result.response.status, "设备大盘加载失败");
    },
  });
}

export function useResendWcsTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, reason }: { id: string; reason: string }) => {
      const result = await api.POST("/api/v1/wcs-tasks/{id}/resend", {
        params: { path: { id } },
        body: { reason },
      });
      return requireData(result.data, result.error, result.response.status, "任务重发失败");
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: wcsTasksQueryKey });
    },
  });
}

export function useConfirmSkipWcsTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, reason }: { id: string; reason: string }) => {
      const result = await api.POST("/api/v1/wcs-tasks/{id}/confirm-skip", {
        params: { path: { id } },
        body: { reason },
      });
      return requireData(result.data, result.error, result.response.status, "跳过确认失败");
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: wcsTasksQueryKey });
    },
  });
}

export function useVoidWcsTaskMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, reason }: { id: string; reason: string }) => {
      const result = await api.POST("/api/v1/wcs-tasks/{id}/void", {
        params: { path: { id } },
        body: { reason },
      });
      return requireData(result.data, result.error, result.response.status, "任务作废失败");
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: wcsTasksQueryKey });
    },
  });
}
