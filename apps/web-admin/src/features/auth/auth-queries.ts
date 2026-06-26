import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { api } from "@/lib/api";
import { clearAuthSession, saveAuthSession } from "@/lib/auth-session";

export type CurrentUser = components["schemas"]["CurrentUser"];
export type LoginRequest = components["schemas"]["LoginRequest"];
export type LoginResponse = components["schemas"]["LoginResponse"];
type ErrorResponse = components["schemas"]["ErrorResponse"];

export const currentUserQueryKey = ["auth", "current-user"] as const;

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

export function useLogout() {
  const queryClient = useQueryClient();
  return () => {
    clearAuthSession();
    queryClient.removeQueries({ queryKey: currentUserQueryKey });
  };
}
