import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type H8ErpMessage = components["schemas"]["H8ErpMessage"];
export type H8ErpMessageDetail = components["schemas"]["H8ErpMessageDetail"];
export type H8ErpMessageStats = components["schemas"]["H8ErpMessageStats"];
export type ReplayH8ErpMessageRequest = components["schemas"]["ReplayH8ErpMessageRequest"];

const key = ["integration", "erp-messages"] as const;

export function useErpMessagesQuery(params: {
  direction?: string;
  message_type?: string;
  status?: string;
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

export function useErpMessageStatsQuery() {
  return useQuery({
    queryKey: [...key, "stats"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/integration/erp-messages/stats");
      if (!result.data) {
        throw new ApiError(result.error, "加载消息统计失败", result.response.status);
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
