import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type Role = components["schemas"]["RoleResponse"];
export type Permission = components["schemas"]["PermissionResponse"];
export type RoleUser = components["schemas"]["RoleUserResponse"];
export type RoleUserListResponse = components["schemas"]["RoleUserListResponse"];
export type CreateRoleRequest = components["schemas"]["CreateRoleRequest"];
export type UpdateRoleRequest = components["schemas"]["UpdateRoleRequest"];
export type BatchAssignRolesRequest = components["schemas"]["BatchAssignRolesRequest"];
export type CreateUserRequest = components["schemas"]["CreateUserRequest"];

export const rolePermissionQueryKey = ["auth", "role-permissions"] as const;

export function useRolesQuery(enabled = true) {
  return useQuery<Role[], ApiError>({
    queryKey: [...rolePermissionQueryKey, "roles"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/auth/roles");
      if (!result.data) throw new ApiError(result.error, "读取角色列表失败", result.response.status);
      return result.data.items;
    },
    enabled,
    retry: false,
  });
}

export function usePermissionsQuery(enabled = true) {
  return useQuery<Permission[], ApiError>({
    queryKey: [...rolePermissionQueryKey, "permissions"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/auth/permissions");
      if (!result.data) throw new ApiError(result.error, "读取权限目录失败", result.response.status);
      return result.data.items;
    },
    enabled,
    retry: false,
  });
}

export interface RoleUsersParams {
  /** 服务端页码（1 基）；提供时启用服务端分页，pageSize 缺省 20 */
  page?: number;
  pageSize?: number;
}

export function useRoleUsersQuery(enabled = true, params: RoleUsersParams = {}) {
  return useQuery<RoleUserListResponse, ApiError>({
    queryKey: [...rolePermissionQueryKey, "users", params],
    queryFn: async () => {
      const result = await api.GET("/api/v1/auth/users", {
        params: {
          query: {
            // 未显式分页的调用方（如双人矩阵确认人选项）请求上限 200 保持全量语义，避免被后端默认 20 截断
            page: params.page ?? undefined,
            page_size: params.page !== undefined ? (params.pageSize ?? 20) : 200,
          },
        },
      });
      if (!result.data) throw new ApiError(result.error, "读取用户列表失败", result.response.status);
      return result.data;
    },
    enabled,
    retry: false,
  });
}

export function useCreateRoleMutation() {
  const invalidate = useInvalidateRolePermissionQueries();
  return useMutation<Role, ApiError, CreateRoleRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/auth/roles", {
        params: { header: { "Idempotency-Key": idempotencyKey("h1-role-create") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "新增角色失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useUpdateRoleMutation() {
  const invalidate = useInvalidateRolePermissionQueries();
  return useMutation<Role, ApiError, { id: string; body: UpdateRoleRequest }>({
    mutationFn: async ({ id, body }) => {
      const result = await api.PUT("/api/v1/auth/roles/{role_id}", {
        params: { path: { role_id: id }, header: { "Idempotency-Key": idempotencyKey("h1-role-update") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "修改角色失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useDeleteRoleMutation() {
  const invalidate = useInvalidateRolePermissionQueries();
  return useMutation<{ id: string }, ApiError, string>({
    mutationFn: async (id) => {
      const result = await api.DELETE("/api/v1/auth/roles/{role_id}", {
        params: { path: { role_id: id }, header: { "Idempotency-Key": idempotencyKey("h1-role-delete") } },
      });
      if (!result.data) throw new ApiError(result.error, "删除角色失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useReplaceRolePermissionsMutation() {
  const invalidate = useInvalidateRolePermissionQueries();
  return useMutation<Role, ApiError, { id: string; permissionCodes: string[] }>({
    mutationFn: async ({ id, permissionCodes }) => {
      const result = await api.PUT("/api/v1/auth/roles/{role_id}/permissions", {
        params: { path: { role_id: id }, header: { "Idempotency-Key": idempotencyKey("h1-role-permissions") } },
        body: { permission_codes: permissionCodes },
      });
      if (!result.data) throw new ApiError(result.error, "保存权限矩阵失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useBatchAssignRolesMutation() {
  const invalidate = useInvalidateRolePermissionQueries();
  return useMutation<{ user_ids: string[]; role_ids: string[] }, ApiError, BatchAssignRolesRequest>({
    mutationFn: async (body) => {
      const result = await api.PUT("/api/v1/auth/user-roles/batch", {
        params: { header: { "Idempotency-Key": idempotencyKey("h1-user-roles-batch") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "批量分配角色失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useCreateUserMutation() {
  const invalidate = useInvalidateRolePermissionQueries();
  return useMutation<RoleUser, ApiError, CreateUserRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/auth/users", {
        params: { header: { "Idempotency-Key": idempotencyKey("h1-user-create") } },
        body,
      });
      if (!result.data) throw new ApiError(result.error, "新增用户失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

function useInvalidateRolePermissionQueries() {
  const queryClient = useQueryClient();
  return () => void queryClient.invalidateQueries({ queryKey: rolePermissionQueryKey });
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
