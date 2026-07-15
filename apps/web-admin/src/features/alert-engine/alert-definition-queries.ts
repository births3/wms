import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type AlertDefinition = components["schemas"]["AlertDefinition"];
export type AlertDefinitionDraft = components["schemas"]["AlertDefinitionDraft"];
export type AlertDefinitionChangeRequest = components["schemas"]["SubmitAlertDefinitionChangeRequest"];
export type QualityLiaisonOrder = components["schemas"]["QualityLiaisonOrder"];

export interface AlertDefinitionFilters {
  keyword?: string;
  severity?: string;
  enabled?: boolean;
}

export const alertDefinitionQueryKey = ["hal", "alert-definitions"] as const;

export function useAlertDefinitionsQuery(filters: AlertDefinitionFilters) {
  return useQuery<components["schemas"]["AlertDefinitionListResponse"], ApiError>({
    queryKey: [...alertDefinitionQueryKey, filters],
    queryFn: async () => {
      const result = await api.GET("/api/v1/alert-definitions", {
        params: { query: { ...filters, limit: 500 } },
      });
      if (!result.data) throw new ApiError(result.error, "读取告警定义失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useSubmitAlertDefinitionChangeMutation() {
  const queryClient = useQueryClient();
  return useMutation<QualityLiaisonOrder, ApiError, AlertDefinitionChangeRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/alert-definitions/change-requests", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "提交告警定义变更失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: alertDefinitionQueryKey }),
  });
}

function idempotencyKey() {
  return `web-hal-alert-definition-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}
