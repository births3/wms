import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type H8ErpMessage = components["schemas"]["H8ErpMessage"];
export type H8ErpMessageDetail = components["schemas"]["H8ErpMessageDetail"];
export type H8ErpMessageStats = components["schemas"]["H8ErpMessageStats"];
export type ReplayH8ErpMessageRequest = components["schemas"]["ReplayH8ErpMessageRequest"];
export type H8WorkerRuntimeResponse = components["schemas"]["H8WorkerRuntimeResponse"];
export type SetH8WorkerClaimControlRequest =
  components["schemas"]["SetH8WorkerClaimControlRequest"];
export type H8PayloadRetentionPolicy = components["schemas"]["H8PayloadRetentionPolicy"];
export type UpdateH8PayloadRetentionPolicyRequest =
  components["schemas"]["UpdateH8PayloadRetentionPolicyRequest"];

const key = ["integration", "erp-messages"] as const;

export function useErpMessagesQuery(params: {
  direction?: string;
  message_type?: string;
  status?: string;
  connector_code?: string;
  channel?: string;
  warehouse_id?: string;
  external_ref?: string;
  idempotency_key?: string;
  correlation_id?: string;
  created_from?: string;
  created_to?: string;
}) {
  return useQuery({
    queryKey: [...key, params],
    queryFn: async () => {
      const result = await api.GET("/api/v1/integration/erp-messages", {
        params: {
          query: {
            direction: params.direction || undefined,
            message_type: params.message_type || undefined,
            status: params.status || undefined,
            connector_code: params.connector_code || undefined,
            channel: params.channel || undefined,
            warehouse_id: params.warehouse_id || undefined,
            external_ref: params.external_ref || undefined,
            idempotency_key: params.idempotency_key || undefined,
            correlation_id: params.correlation_id || undefined,
            created_from: params.created_from || undefined,
            created_to: params.created_to || undefined,
          },
        },
      });
      if (!result.data) {
        throw new ApiError(result.error, "加载 ERP 消息失败", result.response.status);
      }
      return result.data;
    },
  });
}

export function useErpMessageStatsQuery(params: {
  connector_code?: string;
  channel?: string;
  message_type?: string;
}) {
  return useQuery({
    queryKey: [...key, "stats", params],
    queryFn: async () => {
      const result = await api.GET("/api/v1/integration/erp-messages/stats", {
        params: {
          query: {
            connector_code: params.connector_code || undefined,
            channel: params.channel || undefined,
            message_type: params.message_type || undefined,
          },
        },
      });
      if (!result.data) {
        throw new ApiError(result.error, "加载消息统计失败", result.response.status);
      }
      return result.data;
    },
  });
}

export function useH8WorkerRuntimeQuery() {
  return useQuery({
    queryKey: [...key, "worker-runtime"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/integration/erp-messages/worker-runtime");
      if (!result.data) {
        throw new ApiError(result.error, "加载 Worker 状态失败", result.response.status);
      }
      return result.data;
    },
  });
}

export function useSetH8WorkerClaimControlMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: SetH8WorkerClaimControlRequest) => {
      const result = await api.POST(
        "/api/v1/integration/erp-messages/worker-runtime/control",
        { body },
      );
      if (!result.data) {
        throw new ApiError(result.error, "更新 Worker 认领控制失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: [...key, "worker-runtime"] }),
  });
}

export function useH8PayloadRetentionPoliciesQuery() {
  return useQuery({
    queryKey: [...key, "payload-retention"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/integration/erp-messages/payload-retention");
      if (!result.data) {
        throw new ApiError(result.error, "加载报文保留策略失败", result.response.status);
      }
      return result.data;
    },
  });
}

export function useUpdateH8PayloadRetentionPolicyMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: UpdateH8PayloadRetentionPolicyRequest) => {
      const result = await api.POST("/api/v1/integration/erp-messages/payload-retention", {
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "更新报文保留策略失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: [...key, "payload-retention"] }),
  });
}

export function useDecryptH8PayloadMutation() {
  return useMutation({
    mutationFn: async (id: string) => {
      const result = await api.GET("/api/v1/integration/erp-messages/{id}/payload", {
        params: { path: { id } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取完整报文失败", result.response.status);
      }
      return result.data;
    },
  });
}

export function useErpMessageDetailQuery(id: string | null) {
  return useQuery({
    queryKey: [...key, "detail", id],
    enabled: Boolean(id),
    queryFn: async () => {
      if (!id) throw new Error("missing id");
      const result = await api.GET("/api/v1/integration/erp-messages/{id}", {
        params: { path: { id } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "加载消息详情失败", result.response.status);
      }
      return result.data;
    },
  });
}

export function useReplayErpMessageMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({
      id,
      body,
    }: {
      id: string;
      body: ReplayH8ErpMessageRequest;
    }) => {
      const result = await api.POST("/api/v1/integration/erp-messages/{id}/replay", {
        params: { path: { id } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "重放失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: key }),
  });
}
