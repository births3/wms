import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type OutboundOrder = components["schemas"]["OutboundOrder"];
export type CreateOutboundOrderRequest = components["schemas"]["CreateOutboundOrderRequest"];
export type ReviewOutboundOrderRequest = components["schemas"]["ReviewOutboundOrderRequest"];
export type ShipOutboundOrderRequest = components["schemas"]["ShipOutboundOrderRequest"];
export type OutboundWave = components["schemas"]["OutboundWave"];
export type CreateOutboundWaveRequest = components["schemas"]["CreateOutboundWaveRequest"];
export type PurchaseReturnOrder = components["schemas"]["PurchaseReturnOrder"];
export type CreatePurchaseReturnRequest = components["schemas"]["CreatePurchaseReturnRequest"];
export type RejectPurchaseReturnRequest = components["schemas"]["RejectPurchaseReturnRequest"];

export const outboundOrdersQueryKey = ["outbound", "orders"] as const;
export const outboundWavesQueryKey = ["outbound", "waves"] as const;
export const purchaseReturnsQueryKey = ["outbound", "purchase-returns"] as const;

export function useOutboundOrdersQuery(enabled = true) {
  return useQuery<OutboundOrder[], ApiError>({
    queryKey: outboundOrdersQueryKey,
    enabled,
    queryFn: async () => {
      const result = await api.GET("/api/v1/outbound/orders");
      if (!result.data) throw new ApiError(result.error, "读取出库订单失败", result.response.status);
      return result.data.data;
    },
  });
}

export function useOutboundOrderQuery(orderId: string | null) {
  return useQuery<OutboundOrder, ApiError>({
    queryKey: [...outboundOrdersQueryKey, "detail", orderId ?? "none"],
    enabled: orderId !== null,
    queryFn: async () => {
      const result = await api.GET("/api/v1/outbound/orders/{id}", {
        params: { path: { id: orderId ?? "" } },
      });
      if (!result.data) throw new ApiError(result.error, "读取出库订单详情失败", result.response.status);
      return result.data;
    },
  });
}

export function useOutboundReviewQuery(orderId: string | null) {
  return useQuery<OutboundOrder, ApiError>({
    queryKey: [...outboundOrdersQueryKey, "review", orderId ?? "none"],
    enabled: orderId !== null,
    queryFn: async () => {
      const result = await api.GET("/api/v1/outbound/orders/{id}/review", {
        params: { path: { id: orderId ?? "" } },
      });
      if (!result.data) throw new ApiError(result.error, "读取出库复核明细失败", result.response.status);
      return result.data;
    },
  });
}

export function useReviewOutboundOrderMutation() {
  const queryClient = useQueryClient();
  return useMutation<OutboundOrder, ApiError, { orderId: string; request: ReviewOutboundOrderRequest }>({
    mutationFn: async ({ orderId, request }) => {
      const result = await api.POST("/api/v1/outbound/orders/{id}/review", {
        params: {
          path: { id: orderId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-review") },
        },
        body: request,
      });
      if (!result.data) throw new ApiError(result.error, "出库复核失败", result.response.status);
      return result.data;
    },
    onSuccess: (order) => {
      void queryClient.invalidateQueries({ queryKey: outboundOrdersQueryKey });
      void queryClient.invalidateQueries({ queryKey: [...outboundOrdersQueryKey, "review", order.id] });
    },
  });
}

export function useRevalidateOutboundOrderMutation() {
  const queryClient = useQueryClient();
  return useMutation<OutboundOrder, ApiError, string>({
    mutationFn: async (orderId) => {
      const result = await api.POST("/api/v1/outbound/orders/{id}/revalidate", {
        params: {
          path: { id: orderId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-revalidate") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "重新校验出库订单失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: outboundOrdersQueryKey });
    },
  });
}

export function useVoidRequestOutboundOrderMutation() {
  const queryClient = useQueryClient();
  return useMutation<OutboundOrder, ApiError, string>({
    mutationFn: async (orderId) => {
      const result = await api.POST("/api/v1/outbound/orders/{id}/void-request", {
        params: {
          path: { id: orderId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-void-request") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "提交出库作废申请失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: outboundOrdersQueryKey });
    },
  });
}

export function useShipOutboundOrderMutation() {
  const queryClient = useQueryClient();
  return useMutation<OutboundOrder, ApiError, { orderId: string; request: ShipOutboundOrderRequest }>({
    mutationFn: async ({ orderId, request }) => {
      const result = await api.POST("/api/v1/outbound/orders/{id}/ship", {
        params: {
          path: { id: orderId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-ship") },
        },
        body: request,
      });
      if (!result.data) throw new ApiError(result.error, "出库发货交接失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: outboundOrdersQueryKey });
    },
  });
}

export function useCreateOutboundOrderMutation() {
  const queryClient = useQueryClient();
  return useMutation<OutboundOrder, ApiError, CreateOutboundOrderRequest>({
    mutationFn: async (request) => {
      const result = await api.POST("/api/v1/outbound/orders", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-m4-create") } },
        body: request,
      });
      if (!result.data) throw new ApiError(result.error, "创建出库单失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: outboundOrdersQueryKey });
    },
  });
}

export function useCreateOutboundWaveMutation() {
  const queryClient = useQueryClient();
  return useMutation<OutboundWave, ApiError, CreateOutboundWaveRequest>({
    mutationFn: async (request) => {
      const result = await api.POST("/api/v1/outbound/waves", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-m4-wave-create") } },
        body: request,
      });
      if (!result.data) throw new ApiError(result.error, "创建出库波次失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: outboundOrdersQueryKey });
      void queryClient.invalidateQueries({ queryKey: outboundWavesQueryKey });
    },
  });
}

export function useReleaseOutboundWaveMutation() {
  const queryClient = useQueryClient();
  return useMutation<OutboundWave, ApiError, string>({
    mutationFn: async (waveId) => {
      const result = await api.POST("/api/v1/outbound/waves/{wave_id}/release", {
        params: {
          path: { wave_id: waveId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-wave-release") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "下发出库波次失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: outboundOrdersQueryKey });
      void queryClient.invalidateQueries({ queryKey: outboundWavesQueryKey });
    },
  });
}

export function useCancelOutboundWaveMutation() {
  const queryClient = useQueryClient();
  return useMutation<OutboundWave, ApiError, string>({
    mutationFn: async (waveId) => {
      const result = await api.POST("/api/v1/outbound/waves/{wave_id}/cancel", {
        params: {
          path: { wave_id: waveId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-wave-cancel") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "取消出库波次失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: outboundOrdersQueryKey });
      void queryClient.invalidateQueries({ queryKey: outboundWavesQueryKey });
    },
  });
}

export function useOutboundWavesQuery(enabled = true) {
  return useQuery<OutboundWave[], ApiError>({
    queryKey: outboundWavesQueryKey,
    enabled,
    queryFn: async () => {
      const result = await api.GET("/api/v1/outbound/waves");
      if (!result.data) throw new ApiError(result.error, "读取出库波次失败", result.response.status);
      return result.data.data;
    },
  });
}

export function useOutboundWaveQuery(waveId: string | null) {
  return useQuery<OutboundWave, ApiError>({
    queryKey: [...outboundWavesQueryKey, "detail", waveId ?? "none"],
    enabled: waveId !== null,
    queryFn: async () => {
      const result = await api.GET("/api/v1/outbound/waves/{wave_id}", {
        params: { path: { wave_id: waveId ?? "" } },
      });
      if (!result.data) throw new ApiError(result.error, "读取出库波次详情失败", result.response.status);
      return result.data;
    },
  });
}

export function usePurchaseReturnsQuery(enabled = true) {
  return useQuery<PurchaseReturnOrder[], ApiError>({
    queryKey: purchaseReturnsQueryKey,
    enabled,
    queryFn: async () => {
      const result = await api.GET("/api/v1/outbound/purchase-returns");
      if (!result.data) throw new ApiError(result.error, "读取采购退货单失败", result.response.status);
      return result.data.data;
    },
  });
}

export function useCreatePurchaseReturnMutation() {
  const queryClient = useQueryClient();
  return useMutation<PurchaseReturnOrder, ApiError, CreatePurchaseReturnRequest>({
    mutationFn: async (request) => {
      const result = await api.POST("/api/v1/outbound/purchase-returns", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-m4-return-create") } },
        body: request,
      });
      if (!result.data) throw new ApiError(result.error, "创建采购退货单失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: purchaseReturnsQueryKey });
    },
  });
}

export function useApprovePurchaseReturnMutation() {
  const queryClient = useQueryClient();
  return useMutation<PurchaseReturnOrder, ApiError, string>({
    mutationFn: async (returnId) => {
      const result = await api.POST("/api/v1/outbound/purchase-returns/{id}/approve", {
        params: {
          path: { id: returnId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-return-approve") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "采购退货审批失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: purchaseReturnsQueryKey });
    },
  });
}

export function useRejectPurchaseReturnMutation() {
  const queryClient = useQueryClient();
  return useMutation<PurchaseReturnOrder, ApiError, { returnId: string; request: RejectPurchaseReturnRequest }>({
    mutationFn: async ({ returnId, request }) => {
      const result = await api.POST("/api/v1/outbound/purchase-returns/{id}/reject", {
        params: {
          path: { id: returnId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-return-reject") },
        },
        body: request,
      });
      if (!result.data) throw new ApiError(result.error, "采购退货驳回失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: purchaseReturnsQueryKey });
    },
  });
}

export function usePickPurchaseReturnMutation() {
  const queryClient = useQueryClient();
  return useMutation<PurchaseReturnOrder, ApiError, string>({
    mutationFn: async (returnId) => {
      const result = await api.POST("/api/v1/outbound/purchase-returns/{id}/pick", {
        params: {
          path: { id: returnId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-return-pick") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "采购退货拣货失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: purchaseReturnsQueryKey });
    },
  });
}

export function useReviewPurchaseReturnMutation() {
  const queryClient = useQueryClient();
  return useMutation<PurchaseReturnOrder, ApiError, string>({
    mutationFn: async (returnId) => {
      const result = await api.POST("/api/v1/outbound/purchase-returns/{id}/review", {
        params: {
          path: { id: returnId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-return-review") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "采购退货复核失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: purchaseReturnsQueryKey });
    },
  });
}

export function useShipPurchaseReturnMutation() {
  const queryClient = useQueryClient();
  return useMutation<PurchaseReturnOrder, ApiError, string>({
    mutationFn: async (returnId) => {
      const result = await api.POST("/api/v1/outbound/purchase-returns/{id}/ship", {
        params: {
          path: { id: returnId },
          header: { "Idempotency-Key": idempotencyKey("web-m4-return-ship") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "采购退货出库交接失败", result.response.status);
      return result.data;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: purchaseReturnsQueryKey });
    },
  });
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
