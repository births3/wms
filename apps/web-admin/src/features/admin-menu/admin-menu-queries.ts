import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type AdminMenuNode = components["schemas"]["AdminMenuNode"];
export type AdminMenuButtonPermission = components["schemas"]["AdminMenuButtonPermission"];
export type AdminMenuTreeResponse = components["schemas"]["AdminMenuTreeResponse"];
export type AdminMenuVersion = components["schemas"]["AdminMenuVersion"];
export type CreateAdminMenuNodeRequest = components["schemas"]["CreateAdminMenuNodeRequest"];
export type UpdateAdminMenuNodeRequest = components["schemas"]["UpdateAdminMenuNodeRequest"];
export type BatchEnableAdminMenuRequest = components["schemas"]["BatchEnableAdminMenuRequest"];
export type PublishAdminMenuRequest = components["schemas"]["PublishAdminMenuRequest"];
export type RollbackAdminMenuRequest = components["schemas"]["RollbackAdminMenuRequest"];

export const adminMenuQueryKey = ["admin-menu"] as const;

export function usePublishedAdminMenuQuery(enabled = true) {
  return useQuery<AdminMenuTreeResponse, ApiError>({
    queryKey: [...adminMenuQueryKey, "published"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/admin/menus/published");
      if (!result.data) throw new ApiError(result.error, "读取已发布菜单失败", result.response.status);
      return result.data;
    },
    enabled,
    retry: false,
  });
}

export function useDraftAdminMenuQuery() {
  return useQuery<AdminMenuTreeResponse, ApiError>({
    queryKey: [...adminMenuQueryKey, "draft"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/admin/menus/draft");
      if (!result.data) throw new ApiError(result.error, "读取菜单草稿失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useCreateAdminMenuNodeMutation() {
  const invalidate = useInvalidateAdminMenu();
  return useMutation<AdminMenuNode, ApiError, CreateAdminMenuNodeRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/admin/menus/draft/nodes", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h1-menu-create") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "新增菜单节点失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useUpdateAdminMenuNodeMutation() {
  const invalidate = useInvalidateAdminMenu();
  return useMutation<AdminMenuNode, ApiError, { id: string; body: UpdateAdminMenuNodeRequest }>({
    mutationFn: async ({ id, body }) => {
      const result = await api.PATCH("/api/v1/admin/menus/draft/nodes/{id}", {
        params: {
          path: { id },
          header: { "Idempotency-Key": idempotencyKey("web-h1-menu-update") },
        },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "更新菜单节点失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useBatchEnableAdminMenuMutation() {
  const invalidate = useInvalidateAdminMenu();
  return useMutation<AdminMenuNode[], ApiError, BatchEnableAdminMenuRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/admin/menus/draft/batch-enable", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h1-menu-batch-enable") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "批量启停菜单失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function usePublishAdminMenuMutation() {
  const invalidate = useInvalidateAdminMenu();
  return useMutation<AdminMenuVersion, ApiError, PublishAdminMenuRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/admin/menus/publish", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h1-menu-publish") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "发布菜单失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useRollbackAdminMenuMutation() {
  const invalidate = useInvalidateAdminMenu();
  return useMutation<AdminMenuVersion, ApiError, RollbackAdminMenuRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/admin/menus/rollback", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h1-menu-rollback") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "回滚菜单失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

function useInvalidateAdminMenu() {
  const queryClient = useQueryClient();
  return () => void queryClient.invalidateQueries({ queryKey: adminMenuQueryKey });
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
