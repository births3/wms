import { useMutation } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type ReceiveTmsRoutePlanRequest = components["schemas"]["ReceiveTmsRoutePlanRequest"];
export type TmsRoutePlan = components["schemas"]["TmsRoutePlan"];

function idempotencyKey() {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `web-m10-route-plan-${random}`;
}

export function useReceiveTmsRoutePlanMutation() {
  return useMutation<TmsRoutePlan, ApiError, ReceiveTmsRoutePlanRequest>({
    mutationFn: async (request) => {
      const result = await api.POST("/api/v1/tms/route-plans", {
        params: { header: { "Idempotency-Key": idempotencyKey() } },
        body: request,
      });
      if (!result.data) throw new ApiError(result.error, "接收 TMS 路径规划结果失败", result.response.status);
      return result.data;
    },
  });
}
