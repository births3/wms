import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type DeliveryNoteCandidate = components["schemas"]["DeliveryNoteCandidate"];
export type DeliveryNoteGroup = components["schemas"]["DeliveryNoteGroup"];
export type DeliveryNoteGroupListItem = components["schemas"]["DeliveryNoteGroupListItem"];
export type ManualDeliveryNoteCutoffRequest =
  components["schemas"]["ManualDeliveryNoteCutoffRequest"];
export type RouteBinding = components["schemas"]["RouteBinding"];
export type PublishRouteBindingRequest = components["schemas"]["PublishRouteBindingRequest"];
export type CutoffPlan = components["schemas"]["CutoffPlan"];
export type CreateCutoffPlanRequest = components["schemas"]["CreateCutoffPlanRequest"];
export type AggregationFieldDefinition = components["schemas"]["AggregationFieldDefinition"];
export type AggregationDimension = components["schemas"]["AggregationDimension"];
export type AggregationRuleVersion = components["schemas"]["AggregationRuleVersion"];
export type AggregationRuleTestResult = components["schemas"]["AggregationRuleTestResult"];
export type CreateAggregationRuleDraftRequest =
  components["schemas"]["CreateAggregationRuleDraftRequest"];
export type PrintDocumentCategory = components["schemas"]["PrintDocumentCategory"];
export type PrintSuiteVersion = components["schemas"]["PrintSuiteVersion"];
export type PrintSuiteItemInput = components["schemas"]["PrintSuiteItemInput"];
export type CreatePrintSuiteDraftRequest =
  components["schemas"]["CreatePrintSuiteDraftRequest"];
export type PrintSuiteTestResult = components["schemas"]["PrintSuiteTestResult"];
export type PrintSuiteInstance = components["schemas"]["PrintSuiteInstance"];

const printOrchestrationQueryKey = ["h9", "print-orchestration"] as const;

export function useDeliveryNoteCandidatesQuery(warehouseId: string) {
  return useQuery<DeliveryNoteCandidate[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "candidates", warehouseId],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/delivery-note-candidates", {
        params: { query: { warehouse_id: warehouseId || undefined } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取待截单订单失败", result.response.status);
      }
      return result.data.data;
    },
    enabled: Boolean(warehouseId),
  });
}

export function useDeliveryNoteGroupsQuery(warehouseId: string) {
  return useQuery<DeliveryNoteGroupListItem[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "groups", warehouseId],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/delivery-note-groups", {
        params: { query: { warehouse_id: warehouseId || undefined } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取随货同行单结果失败", result.response.status);
      }
      return result.data.data;
    },
    enabled: Boolean(warehouseId),
  });
}

export function useRouteBindingsQuery(warehouseId: string) {
  return useQuery<RouteBinding[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "route-bindings", warehouseId],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/route-bindings", {
        params: { query: { warehouse_id: warehouseId || undefined } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取线路绑定失败", result.response.status);
      }
      return result.data.data;
    },
    enabled: Boolean(warehouseId),
  });
}

export function useCutoffPlansQuery(warehouseId: string) {
  return useQuery<CutoffPlan[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "cutoff-plans", warehouseId],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/cutoff-plans", {
        params: { query: { warehouse_id: warehouseId || undefined } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取截单计划失败", result.response.status);
      }
      return result.data.data;
    },
    enabled: Boolean(warehouseId),
  });
}

export function useManualDeliveryNoteCutoffMutation() {
  const queryClient = useQueryClient();
  return useMutation<DeliveryNoteGroup, ApiError, ManualDeliveryNoteCutoffRequest>({
    mutationFn: async (body) => {
      const result = await api.POST(
        "/api/v1/print-orchestration/delivery-note-groups/manual-cutoff",
        {
          params: { header: { "Idempotency-Key": idempotencyKey("web-h9-manual-cutoff") } },
          body,
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "人工截单失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function usePublishRouteBindingMutation() {
  const queryClient = useQueryClient();
  return useMutation<RouteBinding, ApiError, PublishRouteBindingRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/print-orchestration/route-bindings", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h9-route-binding") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "发布线路绑定失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function useCreateCutoffPlanMutation() {
  const queryClient = useQueryClient();
  return useMutation<CutoffPlan, ApiError, CreateCutoffPlanRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/print-orchestration/cutoff-plans", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h9-cutoff-plan") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "新建截单计划失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function usePublishCutoffPlanMutation() {
  const queryClient = useQueryClient();
  return useMutation<CutoffPlan, ApiError, string>({
    mutationFn: async (planId) => {
      const result = await api.POST(
        "/api/v1/print-orchestration/cutoff-plans/{plan_id}/publish",
        {
          params: {
            path: { plan_id: planId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-cutoff-plan-publish") },
          },
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "发布截单计划失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function useAggregationFieldsQuery() {
  return useQuery<AggregationFieldDefinition[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "aggregation-fields"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/aggregation-fields");
      if (!result.data) {
        throw new ApiError(result.error, "读取归集字段目录失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function useAggregationRulesQuery() {
  return useQuery<AggregationRuleVersion[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "aggregation-rules"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/aggregation-rules/versions");
      if (!result.data) {
        throw new ApiError(result.error, "读取归集规则版本失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function useCreateAggregationRuleDraftMutation() {
  const queryClient = useQueryClient();
  return useMutation<AggregationRuleVersion, ApiError, CreateAggregationRuleDraftRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/print-orchestration/aggregation-rules/versions", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h9-agg-rule-draft") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "创建归集规则草稿失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function useTestAggregationRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<AggregationRuleTestResult, ApiError, { versionId: string; orderIds: string[] }>({
    mutationFn: async ({ versionId, orderIds }) => {
      const result = await api.POST(
        "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/test",
        {
          params: {
            path: { version_id: versionId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-agg-rule-test") },
          },
          body: { order_ids: orderIds },
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "测试归集规则失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function usePublishAggregationRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<AggregationRuleVersion, ApiError, string>({
    mutationFn: async (versionId) => {
      const result = await api.POST(
        "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/publish",
        {
          params: {
            path: { version_id: versionId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-agg-rule-publish") },
          },
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "发布归集规则失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function useDisableAggregationRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<AggregationRuleVersion, ApiError, string>({
    mutationFn: async (versionId) => {
      const result = await api.POST(
        "/api/v1/print-orchestration/aggregation-rules/versions/{version_id}/disable",
        {
          params: {
            path: { version_id: versionId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-agg-rule-disable") },
          },
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "停用归集规则失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function usePrintDocumentCategoriesQuery() {
  return useQuery<PrintDocumentCategory[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "print-document-categories"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/print-document-categories");
      if (!result.data) {
        throw new ApiError(result.error, "读取打印单据分类字典失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function usePrintSuitesQuery() {
  return useQuery<PrintSuiteVersion[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "print-suites"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/print-suites/versions");
      if (!result.data) {
        throw new ApiError(result.error, "读取打印组套版本失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function usePrintSuiteInstancesQuery(groupId: string | null) {
  return useQuery<PrintSuiteInstance[], ApiError>({
    queryKey: [...printOrchestrationQueryKey, "suite-instances", groupId ?? "all"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-orchestration/suite-instances", {
        params: { query: { group_id: groupId || undefined } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取组套实例失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function useCreatePrintSuiteDraftMutation() {
  const queryClient = useQueryClient();
  return useMutation<PrintSuiteVersion, ApiError, CreatePrintSuiteDraftRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/print-orchestration/print-suites/versions", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h9-print-suite-draft") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "创建打印组套草稿失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function useTestPrintSuiteMutation() {
  const queryClient = useQueryClient();
  return useMutation<PrintSuiteTestResult, ApiError, { versionId: string; groupIds: string[] }>({
    mutationFn: async ({ versionId, groupIds }) => {
      const result = await api.POST(
        "/api/v1/print-orchestration/print-suites/versions/{version_id}/test",
        {
          params: {
            path: { version_id: versionId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-print-suite-test") },
          },
          body: { group_ids: groupIds },
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "测试打印组套失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function usePublishPrintSuiteMutation() {
  const queryClient = useQueryClient();
  return useMutation<PrintSuiteVersion, ApiError, string>({
    mutationFn: async (versionId) => {
      const result = await api.POST(
        "/api/v1/print-orchestration/print-suites/versions/{version_id}/publish",
        {
          params: {
            path: { version_id: versionId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-print-suite-publish") },
          },
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "发布打印组套失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

export function useDisablePrintSuiteMutation() {
  const queryClient = useQueryClient();
  return useMutation<PrintSuiteVersion, ApiError, string>({
    mutationFn: async (versionId) => {
      const result = await api.POST(
        "/api/v1/print-orchestration/print-suites/versions/{version_id}/disable",
        {
          params: {
            path: { version_id: versionId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-print-suite-disable") },
          },
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "停用打印组套失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printOrchestrationQueryKey }),
  });
}

function idempotencyKey(prefix: string) {
  return `${prefix}-${globalThis.crypto.randomUUID()}`;
}
