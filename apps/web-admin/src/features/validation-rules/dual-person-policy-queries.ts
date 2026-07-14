import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type DualPersonPolicy = components["schemas"]["DualPersonPolicy"];
export type DualPersonPolicyRule = components["schemas"]["DualPersonPolicyRule"];
export type UpsertDualPersonPolicyRuleRequest =
  components["schemas"]["UpsertDualPersonPolicyRuleRequest"];

export const dualPersonPolicyQueryKey = ["m-vr", "dual-person-policy"] as const;
export const dualPersonPolicyRulesQueryKey = [...dualPersonPolicyQueryKey, "rules"] as const;

export interface DualPersonPolicyQueryInput {
  productId: string;
  process: string;
  node: string;
  ownerId: string;
  warehouseId?: string;
}

async function resolveDualPersonPolicy(input: DualPersonPolicyQueryInput) {
  const result = await api.GET("/api/v1/m-vr/dual-person-policy", {
    params: {
      query: {
        product_id: input.productId,
        process: input.process,
        node: input.node,
        owner_id: input.ownerId,
        warehouse_id: input.warehouseId,
      },
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取双人策略失败", result.response.status);
  }
  return result.data;
}

export function useDualPersonPolicyQuery(input: DualPersonPolicyQueryInput | null) {
  return useQuery<components["schemas"]["DualPersonPolicyResponse"], ApiError>({
    queryKey: ["m-vr", "dual-person-policy", "resolve", input],
    queryFn: () => input
      ? resolveDualPersonPolicy(input)
      : Promise.reject(new ApiError(undefined, "缺少双人策略查询条件", 400)),
    enabled: Boolean(input),
    retry: false,
  });
}

export function useDualPersonPolicyQueries(inputs: DualPersonPolicyQueryInput[]) {
  return useQueries({
    queries: inputs.map((input) => ({
      queryKey: ["m-vr", "dual-person-policy", "resolve", input],
      queryFn: () => resolveDualPersonPolicy(input),
      retry: false,
    })),
  });
}

export function useDualPersonPolicyRulesQuery(warehouseId?: string) {
  return useQuery<components["schemas"]["DualPersonPolicyRuleListResponse"], ApiError>({
    queryKey: [...dualPersonPolicyRulesQueryKey, warehouseId ?? "owner"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/m-vr/dual-person-policy/rules", {
        params: { query: { warehouse_id: warehouseId || undefined } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取双人策略矩阵失败", result.response.status);
      }
      return result.data;
    },
    retry: false,
  });
}

export function useUpsertDualPersonPolicyRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<DualPersonPolicyRule, ApiError, UpsertDualPersonPolicyRuleRequest>({
    mutationFn: async (body) => {
      const result = await api.PUT("/api/v1/m-vr/dual-person-policy/rules", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "保存双人策略矩阵失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: dualPersonPolicyQueryKey }),
  });
}

function idempotencyKey() {
  return `web-mvr-dual-person-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}
