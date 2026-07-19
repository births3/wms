import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type H8ErpConnector = components["schemas"]["H8ErpConnector"];
export type CreateH8ErpConnectorRequest = components["schemas"]["CreateH8ErpConnectorRequest"];
export type UpdateH8ErpConnectorRequest = components["schemas"]["UpdateH8ErpConnectorRequest"];
export type H8ErpConnectorTestResult = components["schemas"]["H8ErpConnectorTestResult"];

const key = ["config-center", "erp-connectors"] as const;

function idempotencyKey(prefix: string): string {
  // HTTP 局域网 IP 非 secure context 时 crypto.randomUUID 不可用
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}

export function useErpConnectorsQuery() {
  return useQuery({
    queryKey: key,
    queryFn: async () => {
      const result = await api.GET("/api/v1/config/erp-connectors");
      if (!result.data) {
        throw new ApiError(result.error, "读取 ERP 连接失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function useCreateErpConnectorMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: CreateH8ErpConnectorRequest) => {
      const result = await api.POST("/api/v1/config/erp-connectors", {
        params: { header: { "Idempotency-Key": idempotencyKey("h8-create") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "新建 ERP 连接失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: key }),
  });
}

export function useUpdateErpConnectorMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({
      id,
      body,
    }: {
      id: string;
      body: UpdateH8ErpConnectorRequest;
    }) => {
      const result = await api.PATCH("/api/v1/config/erp-connectors/{id}", {
        params: {
          path: { id },
          header: { "Idempotency-Key": idempotencyKey("h8-update") },
        },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "更新 ERP 连接失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: key }),
  });
}

export function useTestErpConnectorMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      const result = await api.POST("/api/v1/config/erp-connectors/{id}/test", {
        params: {
          path: { id },
          header: { "Idempotency-Key": idempotencyKey("h8-test") },
        },
      });
      if (!result.data) {
        throw new ApiError(result.error, "测试 ERP 连接失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: key }),
  });
}

export function useActivateErpConnectorMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      const result = await api.POST("/api/v1/config/erp-connectors/{id}/activate", {
        params: {
          path: { id },
          header: { "Idempotency-Key": idempotencyKey("h8-act") },
        },
      });
      if (!result.data) {
        throw new ApiError(result.error, "启用 ERP 连接失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: key }),
  });
}

export function useDisableErpConnectorMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      const result = await api.POST("/api/v1/config/erp-connectors/{id}/disable", {
        params: {
          path: { id },
          header: { "Idempotency-Key": idempotencyKey("h8-dis") },
        },
      });
      if (!result.data) {
        throw new ApiError(result.error, "停用 ERP 连接失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: key }),
  });
}

export function useDeleteErpConnectorMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      const result = await api.DELETE("/api/v1/config/erp-connectors/{id}", {
        params: {
          path: { id },
          header: { "Idempotency-Key": idempotencyKey("h8-del") },
        },
      });
      if (result.response.status !== 204 && result.error) {
        throw new ApiError(result.error, "删除 ERP 连接失败", result.response.status);
      }
    },
    onSuccess: () => void qc.invalidateQueries({ queryKey: key }),
  });
}
