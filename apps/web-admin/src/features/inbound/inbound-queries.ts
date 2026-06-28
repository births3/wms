import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type ReceivingOrder = components["schemas"]["ReceivingOrder"];
export type CreateReceivingOrderRequest = components["schemas"]["CreateReceivingOrderRequest"];
export type ReceiveReceivingOrderRequest = components["schemas"]["ReceiveReceivingOrderRequest"];
export type InspectReceivingOrderRequest = components["schemas"]["InspectReceivingOrderRequest"];
export type SignInspectionRequest = components["schemas"]["SignInspectionRequest"];
export type PutawayRequest = components["schemas"]["PutawayRequest"];

export const receivingOrdersQueryKey = ["inbound", "receiving-orders"] as const;

function receivingOrderQueryKey(id: string) {
  return [...receivingOrdersQueryKey, id] as const;
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}

async function listReceivingOrders(): Promise<ReceivingOrder[]> {
  const result = await api.GET("/api/v1/inbound/receiving-orders");
  if (!result.data) {
    throw new ApiError(result.error, "读取入库单失败", result.response.status);
  }
  return result.data.data;
}

async function getReceivingOrder(id: string): Promise<ReceivingOrder> {
  const result = await api.GET("/api/v1/inbound/receiving-orders/{id}", {
    params: { path: { id } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取入库单详情失败", result.response.status);
  }
  return result.data;
}

async function createReceivingOrder(request: CreateReceivingOrderRequest): Promise<ReceivingOrder> {
  const result = await api.POST("/api/v1/inbound/receiving-orders", { body: request });
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
  };
}

export function useReceiveReceivingOrderMutation() {
  const invalidate = useInvalidateReceivingOrders();
  return useMutation({
    mutationFn: receiveReceivingOrder,
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
