import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type ApiKey = components["schemas"]["ApiKey"];
export type ApiKeyListResponse = components["schemas"]["ApiKeyListResponse"];
export type CreateApiKeyRequest = components["schemas"]["CreateApiKeyRequest"];
export type RotateApiKeyRequest = components["schemas"]["RotateApiKeyRequest"];
export type ApiKeyRotationResponse = components["schemas"]["ApiKeyRotationResponse"];

export interface ApiKeyListParams {
  keyword?: string;
  status?: string;
  /** 服务端页码（1 基）；提供时启用服务端分页，pageSize 缺省 20 */
  page?: number;
  pageSize?: number;
}

export const apiKeyQueryKey = ["h1", "api-keys"] as const;

export function useApiKeysQuery(params: ApiKeyListParams) {
  return useQuery<ApiKeyListResponse, ApiError>({
    queryKey: [...apiKeyQueryKey, params],
    queryFn: async () => {
      const result = await api.GET("/api/v1/auth/api-keys", {
        params: {
          query: {
            q: emptyToUndefined(params.keyword),
            status: emptyToUndefined(params.status),
            // 未显式分页的调用方（全量选项列表）请求上限 200 保持全量语义，避免被后端默认 20 截断
            page: params.page ?? undefined,
            page_size: params.page !== undefined ? (params.pageSize ?? 20) : 200,
          },
        },
      });
      if (!result.data) throw new ApiError(result.error, "读取 API Key 列表失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useCreateApiKeyMutation() {
  const queryClient = useQueryClient();
  return useMutation<ApiKey, ApiError, CreateApiKeyRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/auth/api-keys", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h1-api-key-create") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "创建 API Key 失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: apiKeyQueryKey }),
  });
}

export function useRotateApiKeyMutation() {
  const queryClient = useQueryClient();
  return useMutation<ApiKeyRotationResponse, ApiError, { id: string; body: RotateApiKeyRequest }>({
    mutationFn: async ({ id, body }) => {
      const result = await api.POST("/api/v1/auth/api-keys/{api_key_id}/rotate", {
        params: {
          path: { api_key_id: id },
          header: { "Idempotency-Key": idempotencyKey("web-h1-api-key-rotate") },
        },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "轮换 API Key 失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: apiKeyQueryKey }),
  });
}

export function useRevokeApiKeyMutation() {
  const queryClient = useQueryClient();
  return useMutation<ApiKey, ApiError, string>({
    mutationFn: async (id) => {
      const result = await api.POST("/api/v1/auth/api-keys/{api_key_id}/revoke", {
        params: {
          path: { api_key_id: id },
          header: { "Idempotency-Key": idempotencyKey("web-h1-api-key-revoke") },
        },
      });
      if (!result.data) throw new ApiError(result.error, "吊销 API Key 失败", result.response.status);
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: apiKeyQueryKey }),
  });
}

function emptyToUndefined(value?: string) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
