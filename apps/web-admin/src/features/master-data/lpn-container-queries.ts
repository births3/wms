import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type LpnContainer = components["schemas"]["LpnContainer"];
export type LpnContainerTypePolicy = components["schemas"]["LpnContainerTypePolicy"];
export type CreateLpnContainerRequest = components["schemas"]["CreateLpnContainerRequest"];
export type UpsertLpnContainerTypePolicyRequest = components["schemas"]["UpsertLpnContainerTypePolicyRequest"];

const listKey = ["master-data", "lpn-containers"] as const;
const policyKey = ["master-data", "lpn-container-type-policies"] as const;

function idempotencyKey() {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
}

export function useLpnContainersQuery(filters?: {
  keyword?: string;
  containerType?: string;
  status?: string;
}) {
  const keyword = filters?.keyword?.trim() || undefined;
  const containerType = filters?.containerType?.trim() || undefined;
  const status = filters?.status?.trim() || undefined;
  return useQuery<LpnContainer[], ApiError>({
    queryKey: [...listKey, keyword ?? "", containerType ?? "", status ?? ""],
    queryFn: async () => {
      const result = await api.GET("/api/v1/master-data/lpn-containers", {
        params: {
          query: {
            keyword,
            type: containerType,
            status,
          },
        },
      });
      if (!result.data) throw new ApiError(result.error, "读取容器失败", result.response.status);
      return result.data.data;
    },
  });
}

export function useCreateLpnContainerMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (body: CreateLpnContainerRequest) => {
      const result = await api.POST("/api/v1/master-data/lpn-containers", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "创建容器失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: listKey });
    },
  });
}

export function useLpnTypePoliciesQuery() {
  return useQuery<LpnContainerTypePolicy[], ApiError>({
    queryKey: policyKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/master-data/lpn-container-type-policies");
      if (!result.data) throw new ApiError(result.error, "读取类型策略失败", result.response.status);
      return result.data;
    },
  });
}

export function useUpsertLpnTypePolicyMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (body: UpsertLpnContainerTypePolicyRequest) => {
      const result = await api.PUT("/api/v1/master-data/lpn-container-type-policies", { body });
      if (!result.data) throw new ApiError(result.error, "保存类型策略失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: policyKey });
    },
  });
}
