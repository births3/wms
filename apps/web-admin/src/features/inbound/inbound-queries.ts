import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";
import { ERROR_INBOUND_ORDER_NOT_FOUND } from "@/lib/ui-strings";

export type ReceivingOrder = components["schemas"]["ReceivingOrder"];
export type ReceivingOrderPrintData = components["schemas"]["ReceivingOrderPrintData"];
export type ReceivingOrderReceipt = components["schemas"]["ReceivingOrderReceipt"];
export type CreateReceivingOrderRequest = components["schemas"]["CreateReceivingOrderRequest"];
export type ReceiveReceivingOrderRequest = components["schemas"]["ReceiveReceivingOrderRequest"];
export type RejectReceivingOrderRequest = components["schemas"]["RejectReceivingOrderRequest"];
export type InspectReceivingOrderRequest = components["schemas"]["InspectReceivingOrderRequest"];
export type SignInspectionRequest = components["schemas"]["SignInspectionRequest"];
export type PutawayRequest = components["schemas"]["PutawayRequest"];
export type PutawayRecommendation = components["schemas"]["PutawayLocationRecommendation"];
export type PutawayRecommendationResponse = components["schemas"]["PutawayRecommendationResponse"];

export const receivingOrdersQueryKey = ["inbound", "receiving-orders"] as const;

function receivingOrderQueryKey(id: string) {
  return [...receivingOrdersQueryKey, id] as const;
}

function receivingOrderPrintDataQueryKey(id: string) {
  return [...receivingOrdersQueryKey, id, "print-data"] as const;
}

function putawayRecommendationsQueryKey(id: string, input: components["schemas"]["PutawayRecommendationQuery"]) {
  return [...receivingOrdersQueryKey, id, "putaway-recommendations", input] as const;
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}

function isDevMockNotFound(error: unknown): boolean {
  if (!error || typeof error !== "object") return false;
  const code = "code" in error ? String(error.code ?? "") : "";
  const message = "message" in error ? String(error.message ?? "") : "";
  return code === "DEV_MOCK_NOT_FOUND" || /Dev mock route not found/i.test(message);
}

async function listReceivingOrders(): Promise<ReceivingOrder[]> {
  const result = await api.GET("/api/v1/inbound/receiving-orders");
  if (!result.data) {
    // 空关键字 / 未实现查询路由时，dev mock 可能返回 DEV_MOCK_NOT_FOUND；业务上按空列表处理
    if (isDevMockNotFound(result.error) || result.response.status === 404) {
      return [];
    }
    throw new ApiError(result.error, "读取入库单失败", result.response.status);
  }
  return result.data.data;
}

async function getReceivingOrder(id: string): Promise<ReceivingOrder> {
  const result = await api.GET("/api/v1/inbound/receiving-orders/{id}", {
    params: { path: { id } },
  });
  if (!result.data) {
    if (isDevMockNotFound(result.error) || result.response.status === 404) {
      throw new ApiError(
        { code: "INBOUND_ORDER_NOT_FOUND", message: ERROR_INBOUND_ORDER_NOT_FOUND, severity: "error", details: {}, trace_id: "web-admin" },
        ERROR_INBOUND_ORDER_NOT_FOUND,
        result.response.status,
      );
    }
    throw new ApiError(result.error, "读取入库单详情失败", result.response.status);
  }
  return result.data;
}

async function getReceivingOrderPrintData(id: string): Promise<ReceivingOrderPrintData> {
  const result = await api.GET("/api/v1/inbound/receiving-orders/{id}/print-data", {
    params: { path: { id } },
  });
  if (!result.data) {
    if (isDevMockNotFound(result.error) || result.response.status === 404) {
      throw new ApiError(
        { code: "INBOUND_ORDER_NOT_FOUND", message: ERROR_INBOUND_ORDER_NOT_FOUND, severity: "error", details: {}, trace_id: "web-admin" },
        ERROR_INBOUND_ORDER_NOT_FOUND,
        result.response.status,
      );
    }
    throw new ApiError(result.error, "读取入库打印数据失败", result.response.status);
  }
  return result.data;
}

async function getPutawayRecommendations(
  id: string,
  input: components["schemas"]["PutawayRecommendationQuery"],
): Promise<PutawayRecommendationResponse> {
  const result = await api.GET("/api/v1/inbound/receiving-orders/{id}/putaway-recommendations", {
    params: { path: { id }, query: { ...input, qty: Number(input.qty) } },
  });
  if (!result.data) throw new ApiError(result.error, "读取推荐库位失败", result.response.status);
  return result.data;
}

async function createReceivingOrder(request: CreateReceivingOrderRequest): Promise<ReceivingOrder> {
  const result = await api.POST("/api/v1/inbound/receiving-orders", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-m2-create") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "创建 ASN 失败", result.response.status);
  }
  return result.data;
}

async function receiveReceivingOrder(input: {
  id: string;
  request: ReceiveReceivingOrderRequest;
}) {
  const result = await api.POST("/api/v1/inbound/receiving-orders/{id}/receive", {
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey("web-m2-receive") },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "提交收货失败", result.response.status);
  }
  return result.data;
}

async function releaseReceivingOrder(id: string) {
  const result = await api.POST("/api/v1/inbound/receiving-orders/{id}/release", {
    params: { path: { id }, header: { "Idempotency-Key": idempotencyKey("web-m2-release") } },
  });
  if (!result.data) throw new ApiError(result.error, "放行 ASN 失败", result.response.status);
  return result.data;
}

async function rejectReceivingOrder(input: {
  id: string;
  request: RejectReceivingOrderRequest;
}) {
  const result = await api.POST("/api/v1/inbound/receiving-orders/{id}/reject", {
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey("web-m2-reject") },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "提交整单拒收失败", result.response.status);
  }
  return result.data;
}

async function inspectReceivingOrder(input: {
  id: string;
  request: InspectReceivingOrderRequest;
}) {
  const result = await api.POST("/api/v1/inbound/receiving-orders/{id}/inspect", {
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey("web-m2-inspect") },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "提交验收失败", result.response.status);
  }
  return result.data;
}

async function signReceivingOrder(input: { id: string; request: SignInspectionRequest }) {
  const result = await api.POST("/api/v1/inbound/receiving-orders/{id}/sign", {
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey("web-m2-sign") },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "提交双人签字失败", result.response.status);
  }
  return result.data;
}

async function putawayReceivingOrder(input: { id: string; request: PutawayRequest }) {
  const result = await api.POST("/api/v1/inbound/receiving-orders/{id}/putaway", {
    params: {
      path: { id: input.id },
      header: { "Idempotency-Key": idempotencyKey("web-m2-putaway") },
    },
    body: input.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "提交上架失败", result.response.status);
  }
  return result.data;
}

export function useReceivingOrdersQuery() {
  return useQuery<ReceivingOrder[], ApiError>({
    queryKey: receivingOrdersQueryKey,
    queryFn: listReceivingOrders,
  });
}

export function useReceivingOrderQuery(id: string | null) {
  return useQuery<ReceivingOrder, ApiError>({
    queryKey: id ? receivingOrderQueryKey(id) : receivingOrderQueryKey("none"),
    queryFn: () => getReceivingOrder(id ?? ""),
    enabled: id !== null,
  });
}

export function useReceivingOrderPrintDataQuery(id: string | null) {
  return useQuery<ReceivingOrderPrintData, ApiError>({
    queryKey: id ? receivingOrderPrintDataQueryKey(id) : receivingOrderPrintDataQueryKey("none"),
    queryFn: () => getReceivingOrderPrintData(id ?? ""),
    enabled: id !== null,
  });
}

export function usePutawayRecommendationsQuery(
  id: string | null,
  input: components["schemas"]["PutawayRecommendationQuery"],
  enabled = true,
) {
  const inputReady = Boolean(input.product_code && input.batch_no && input.quality_status && Number(input.qty) > 0);
  return useQuery<PutawayRecommendationResponse, ApiError>({
    queryKey: id && inputReady ? putawayRecommendationsQueryKey(id, input) : [...receivingOrdersQueryKey, "putaway-recommendations", "none"],
    queryFn: () => getPutawayRecommendations(id ?? "", input),
    enabled: Boolean(id) && enabled && inputReady,
    retry: false,
  });
}

export function useCreateReceivingOrderMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: createReceivingOrder,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: receivingOrdersQueryKey });
    },
  });
}

function useInvalidateReceivingOrders() {
  const queryClient = useQueryClient();
  return (id: string) => {
    void queryClient.invalidateQueries({ queryKey: receivingOrdersQueryKey });
    void queryClient.invalidateQueries({ queryKey: receivingOrderQueryKey(id) });
    void queryClient.invalidateQueries({ queryKey: receivingOrderPrintDataQueryKey(id) });
  };
}

export function useReceiveReceivingOrderMutation() {
  const invalidate = useInvalidateReceivingOrders();
  return useMutation({
    mutationFn: receiveReceivingOrder,
    onSuccess: (_data, input) => invalidate(input.id),
  });
}

export function useReleaseReceivingOrderMutation() {
  const invalidate = useInvalidateReceivingOrders();
  return useMutation({
    mutationFn: releaseReceivingOrder,
    onSuccess: (_data, id) => invalidate(id),
  });
}

export function useRejectReceivingOrderMutation() {
  const invalidate = useInvalidateReceivingOrders();
  return useMutation({
    mutationFn: rejectReceivingOrder,
    onSuccess: (_data, input) => invalidate(input.id),
  });
}

export function useInspectReceivingOrderMutation() {
  const invalidate = useInvalidateReceivingOrders();
  return useMutation({
    mutationFn: inspectReceivingOrder,
    onSuccess: (_data, input) => invalidate(input.id),
  });
}

export function useSignReceivingOrderMutation() {
  const invalidate = useInvalidateReceivingOrders();
  return useMutation({
    mutationFn: signReceivingOrder,
    onSuccess: (_data, input) => invalidate(input.id),
  });
}

export function usePutawayReceivingOrderMutation() {
  const invalidate = useInvalidateReceivingOrders();
  return useMutation({
    mutationFn: putawayReceivingOrder,
    onSuccess: (_data, input) => invalidate(input.id),
  });
}
