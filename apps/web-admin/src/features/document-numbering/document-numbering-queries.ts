import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type DocumentNumberRule = components["schemas"]["DocumentNumberRule"];
export type DocumentNumberAllocation = components["schemas"]["DocumentNumberAllocation"];
export type UpsertDocumentNumberRuleRequest = components["schemas"]["UpsertDocumentNumberRuleRequest"];

export const documentNumberingQueryKey = ["mcg", "document-numbering"] as const;

export function useDocumentNumberRulesQuery(documentType: string) {
  return useQuery<DocumentNumberRule[], ApiError>({
    queryKey: [...documentNumberingQueryKey, "rules", documentType],
    queryFn: async () => {
      const result = await api.GET("/api/v1/code-generator/document-number-rules", {
        params: { query: { document_type: documentType || undefined } },
      });
      if (!result.data) throw new ApiError(result.error, "读取单据号规则失败", result.response.status);
      return result.data.data;
    },
    retry: false,
  });
}

export function useDocumentNumberAllocationsQuery(documentType: string) {
  return useQuery<DocumentNumberAllocation[], ApiError>({
    queryKey: [...documentNumberingQueryKey, "allocations", documentType],
    queryFn: async () => {
      const result = await api.GET("/api/v1/code-generator/document-number-allocations", {
        params: { query: { document_type: documentType || undefined, limit: 100 } },
      });
      if (!result.data) throw new ApiError(result.error, "读取单据号生成记录失败", result.response.status);
      return result.data.data;
    },
    retry: false,
  });
}

export function useUpsertDocumentNumberRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<DocumentNumberRule, ApiError, { ruleCode: string; body: UpsertDocumentNumberRuleRequest }>({
    mutationFn: async ({ ruleCode, body }) => {
      const result = await api.PUT("/api/v1/code-generator/document-number-rules/{rule_code}", {
        params: {
          path: { rule_code: ruleCode },
          header: { "Idempotency-Key": idempotencyKey("web-mcg-rule-upsert") },
        },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "保存单据号规则失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: documentNumberingQueryKey }),
  });
}

export function useSetDocumentNumberRuleEnabledMutation() {
  const queryClient = useQueryClient();
  return useMutation<DocumentNumberRule, ApiError, { ruleCode: string; enabled: boolean }>({
    mutationFn: async ({ ruleCode, enabled }) => {
      const result = await api.PATCH("/api/v1/code-generator/document-number-rules/{rule_code}/enabled", {
        params: {
          path: { rule_code: ruleCode },
          header: { "Idempotency-Key": idempotencyKey("web-mcg-rule-enabled") },
        },
        body: { enabled },
      });
      if (!result.data) throw new ApiError(result.error, "更新单据号规则状态失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: documentNumberingQueryKey }),
  });
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
