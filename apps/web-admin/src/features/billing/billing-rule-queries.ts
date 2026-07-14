import { useMutation } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type BillingRule = components["schemas"]["BillingRule"];
export type CreateBillingRuleRequest = components["schemas"]["CreateBillingRuleRequest"];

export function useCreateBillingRuleMutation() {
  return useMutation<BillingRule, ApiError, CreateBillingRuleRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/billing/rules", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-m9-billing-rule") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "保存计费规则失败", result.response.status);
      }
      return result.data;
    },
  });
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
