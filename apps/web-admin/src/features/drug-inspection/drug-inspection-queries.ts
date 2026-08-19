import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type DrugInspectionPlatform = components["schemas"]["DrugInspectionPlatform"];
export type DrugInspectionPlatformListResponse = components["schemas"]["DrugInspectionPlatformListResponse"];
export type UpsertDrugInspectionPlatformRequest = components["schemas"]["UpsertDrugInspectionPlatformRequest"];
export type ChangeDrugInspectionPlatformStatusRequest = components["schemas"]["ChangeDrugInspectionPlatformStatusRequest"];

export const drugInspectionQueryKey = ["m-di", "platforms"] as const;

export function useDrugInspectionPlatformsQuery(status?: string) {
  return useQuery<DrugInspectionPlatformListResponse, ApiError>({
    queryKey: [...drugInspectionQueryKey, status ?? ""],
    queryFn: async () => {
      const result = await api.GET("/api/v1/drug-inspection/platforms", {
        params: { query: { status: status?.trim() || undefined } },
      });
      if (!result.data) throw new ApiError(result.error, "读取药检平台列表失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useUpsertDrugInspectionPlatformMutation() {
  const queryClient = useQueryClient();
  return useMutation<DrugInspectionPlatform, ApiError, UpsertDrugInspectionPlatformRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/drug-inspection/platforms", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-m-di-platform-upsert") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "保存药检平台配置失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: drugInspectionQueryKey }),
  });
}

export function useChangeDrugInspectionPlatformStatusMutation() {
  const queryClient = useQueryClient();
  return useMutation<DrugInspectionPlatform, ApiError, { id: string; body: ChangeDrugInspectionPlatformStatusRequest }>({
    mutationFn: async ({ id, body }) => {
      const result = await api.PATCH("/api/v1/drug-inspection/platforms/{platform_id}/status", {
        params: {
          path: { platform_id: id },
          header: { "Idempotency-Key": idempotencyKey("web-m-di-platform-status") },
        },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "更新药检平台状态失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: drugInspectionQueryKey }),
  });
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
