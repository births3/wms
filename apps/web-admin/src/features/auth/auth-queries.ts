import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { api } from "@/lib/api";
import { clearAuthSession, saveAuthSession } from "@/lib/auth-session";

export type CurrentUser = components["schemas"]["CurrentUser"];
export type LoginRequest = components["schemas"]["LoginRequest"];
export type LoginResponse = components["schemas"]["LoginResponse"];
export type AuthSession = components["schemas"]["AuthSession"];
export type AuthRevocationResponse = components["schemas"]["AuthRevocationResponse"];
export type AuthSessionRevokeResponse = components["schemas"]["AuthSessionRevokeResponse"];
type ErrorResponse = components["schemas"]["ErrorResponse"];

export const currentUserQueryKey = ["auth", "current-user"] as const;
export const authSessionsQueryKey = ["auth", "sessions"] as const;

export class ApiError extends Error {
  readonly code: string;
  readonly status: number;

  constructor(error: ErrorResponse | undefined, fallbackMessage: string, status: number) {
    super(error?.message ?? fallbackMessage);
    this.name = "ApiError";
    this.code = error?.code ?? "UNKNOWN";
    this.status = status;
  }
}

async function fetchCurrentUser(): Promise<CurrentUser> {
  const result = await api.GET("/api/v1/auth/me");
  if (!result.data) {
    throw new ApiError(result.error, "读取当前用户失败", result.response.status);
  }
  return result.data;
}

async function login(request: LoginRequest): Promise<LoginResponse> {
  const result = await api.POST("/api/v1/auth/login", { body: request });
  if (!result.data) {
    throw new ApiError(result.error, "登录失败", result.response.status);
  }
  saveAuthSession({
    accessToken: result.data.access_token,
    expiresAt: result.data.expires_at,
  });
  return result.data;
}

async function listAuthSessions(userId?: string): Promise<AuthSession[]> {
  const result = await api.GET("/api/v1/auth/sessions", {
    params: { query: { user_id: userId || undefined } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取登录会话失败", result.response.status);
  }
  return result.data.data;
}

async function revokeAuthSession(sessionId: string): Promise<AuthRevocationResponse> {
  const result = await api.POST("/api/v1/auth/sessions/{session_id}/revoke", {
    params: {
      path: { session_id: sessionId },
      header: { "Idempotency-Key": idempotencyKey("web-h1-session-revoke") },
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "撤销登录会话失败", result.response.status);
  }
  return result.data;
}

async function revokeOtherAuthSessions(): Promise<AuthSessionRevokeResponse> {
  const result = await api.POST("/api/v1/auth/sessions/revoke-others", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-h1-session-revoke-others") } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "撤销其他登录会话失败", result.response.status);
  }
  return result.data;
}

async function kickAuthUser(userId: string): Promise<AuthSessionRevokeResponse> {
  const result = await api.POST("/api/v1/auth/users/{user_id}/kick", {
    params: {
      path: { user_id: userId },
      header: { "Idempotency-Key": idempotencyKey("web-h1-session-kick") },
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "强制踢出用户失败", result.response.status);
  }
  return result.data;
}

export function useCurrentUserQuery(enabled: boolean) {
  return useQuery<CurrentUser, ApiError>({
    queryKey: currentUserQueryKey,
    queryFn: fetchCurrentUser,
    enabled,
    retry: false,
  });
}

export function useLoginMutation() {
  const queryClient = useQueryClient();
  return useMutation<LoginResponse, ApiError, LoginRequest>({
    mutationFn: login,
    onSuccess: (data) => {
      queryClient.setQueryData(currentUserQueryKey, data.user);
    },
  });
}

export function useAuthSessionsQuery(userId?: string) {
  return useQuery<AuthSession[], ApiError>({
    queryKey: [...authSessionsQueryKey, userId ?? "self"],
    queryFn: () => listAuthSessions(userId),
    retry: false,
  });
}

export function useRevokeAuthSessionMutation() {
  const queryClient = useQueryClient();
  return useMutation<AuthRevocationResponse, ApiError, string>({
    mutationFn: revokeAuthSession,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: authSessionsQueryKey });
    },
  });
}

export function useRevokeOtherAuthSessionsMutation() {
  const queryClient = useQueryClient();
  return useMutation<AuthSessionRevokeResponse, ApiError>({
    mutationFn: revokeOtherAuthSessions,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: authSessionsQueryKey });
    },
  });
}

export function useKickAuthUserMutation() {
  const queryClient = useQueryClient();
  return useMutation<AuthSessionRevokeResponse, ApiError, string>({
    mutationFn: kickAuthUser,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: authSessionsQueryKey });
    },
  });
}

export function useLogout() {
  const queryClient = useQueryClient();
  return async () => {
    try {
      await api.POST("/api/v1/auth/logout", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h1-logout") } },
      });
    } catch {
      // 登出必须完成本地收口；网络故障由后端 TTL 降级策略兜底。
    } finally {
      clearAuthSession();
      queryClient.removeQueries({ queryKey: currentUserQueryKey });
      queryClient.removeQueries({ queryKey: authSessionsQueryKey });
    }
  };
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
