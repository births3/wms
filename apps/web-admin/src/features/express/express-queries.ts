import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type ExpressCarrier = components["schemas"]["ExpressCarrier"];
export type ExpressRoutingRule = components["schemas"]["ExpressRoutingRule"];
export type ExpressWaybill = components["schemas"]["ExpressWaybill"];
export type ExpressTrackingResponse = components["schemas"]["ExpressTrackingResponse"];
export type UpsertExpressCarrierRequest = components["schemas"]["UpsertExpressCarrierRequest"];
export type UpsertExpressRoutingRuleRequest = components["schemas"]["UpsertExpressRoutingRuleRequest"];
export type CreateExpressWaybillRequest = components["schemas"]["CreateExpressWaybillRequest"];
export type CancelExpressWaybillRequest = components["schemas"]["CancelExpressWaybillRequest"];

export interface ExpressListParams {
  q?: string;
  enabled?: boolean;
}

export interface ExpressRuleListParams extends ExpressListParams {
  deliveryProviderType?: string;
}

export const expressQueryKey = ["express"] as const;

export function useExpressCarriersQuery(params: ExpressListParams) {
  return useQuery<ExpressCarrier[], ApiError>({
    queryKey: [...expressQueryKey, "carriers", params],
    queryFn: () => listExpressCarriers(params),
  });
}

export function useExpressRoutingRulesQuery(params: ExpressRuleListParams) {
  return useQuery<ExpressRoutingRule[], ApiError>({
    queryKey: [...expressQueryKey, "routing-rules", params],
    queryFn: () => listExpressRoutingRules(params),
  });
}

export function useUpsertExpressCarrierMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: upsertExpressCarrier,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: expressQueryKey });
    },
  });
}

export function useUpsertExpressRoutingRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: upsertExpressRoutingRule,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: expressQueryKey });
    },
  });
}

export function useCreateExpressWaybillMutation() {
  return useMutation<ExpressWaybill, ApiError, CreateExpressWaybillRequest>({
    mutationFn: createExpressWaybill,
  });
}

export function useCancelExpressWaybillMutation() {
  return useMutation<ExpressWaybill, ApiError, { waybillNo: string; request: CancelExpressWaybillRequest }>({
    mutationFn: cancelExpressWaybill,
  });
}

export function useExpressTrackingMutation() {
  return useMutation<ExpressTrackingResponse, ApiError, string>({
    mutationFn: getExpressTracking,
  });
}

async function listExpressCarriers(params: ExpressListParams): Promise<ExpressCarrier[]> {
  const result = await api.GET("/api/v1/express/carriers", {
    params: { query: { q: params.q || undefined, enabled: params.enabled, limit: 100 } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取快递商配置失败", result.response.status);
  }
  return result.data.data;
}

async function listExpressRoutingRules(params: ExpressRuleListParams): Promise<ExpressRoutingRule[]> {
  const result = await api.GET("/api/v1/express/routing-rules", {
    params: {
      query: {
        q: params.q || undefined,
        delivery_provider_type: params.deliveryProviderType || undefined,
        enabled: params.enabled,
        limit: 100,
      },
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取快递选择规则失败", result.response.status);
  }
  return result.data.data;
}

async function upsertExpressCarrier(request: UpsertExpressCarrierRequest) {
  const result = await api.POST("/api/v1/express/carriers", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-h5-carrier") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存快递商配置失败", result.response.status);
  }
  return result.data;
}

async function upsertExpressRoutingRule(request: UpsertExpressRoutingRuleRequest) {
  const result = await api.POST("/api/v1/express/routing-rules", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-h5-rule") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存快递选择规则失败", result.response.status);
  }
  return result.data;
}

async function createExpressWaybill(request: CreateExpressWaybillRequest) {
  const result = await api.POST("/api/v1/express/waybills", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-h5-waybill") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "快递下单失败", result.response.status);
  }
  return result.data;
}

async function cancelExpressWaybill(params: { waybillNo: string; request: CancelExpressWaybillRequest }) {
  const result = await api.POST("/api/v1/express/waybills/{waybill_no}/cancel", {
    params: {
      path: { waybill_no: params.waybillNo },
      header: { "Idempotency-Key": idempotencyKey("web-h5-waybill-cancel") },
    },
    body: params.request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "取消快递单失败", result.response.status);
  }
  return result.data;
}

async function getExpressTracking(waybillNo: string) {
  const result = await api.GET("/api/v1/express/waybills/{waybill_no}/tracking", {
    params: { path: { waybill_no: waybillNo } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "查询快递轨迹失败", result.response.status);
  }
  return result.data;
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
