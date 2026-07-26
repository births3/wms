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

function idempotencyKey(prefix: string) {
  return `${prefix}-${globalThis.crypto.randomUUID()}`;
}
