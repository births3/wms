import { createApiClient, type paths } from "@wms/api-client";

type AuthTokenProvider = () => string | null;

let authTokenProvider: AuthTokenProvider = () => null;

export const apiBaseUrl = import.meta.env.VITE_API_BASE_URL ?? "";

export const wave1ContractPaths = [
  "/api/v1/healthz",
  "/api/v1/auth/login",
  "/api/v1/auth/me",
  "/api/v1/audit/events",
] as const satisfies readonly (keyof paths)[];

export const api = createApiClient({
  baseUrl: apiBaseUrl,
  authToken: () => authTokenProvider(),
});

export function setAuthTokenProvider(provider: AuthTokenProvider) {
  authTokenProvider = provider;
}
