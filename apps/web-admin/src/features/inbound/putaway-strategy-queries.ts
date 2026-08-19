import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type PutawayStrategyProfile = components["schemas"]["PutawayStrategyProfile"];
export type UpsertPutawayStrategyProfileRequest = components["schemas"]["UpsertPutawayStrategyProfileRequest"];

export const putawayStrategyProfilesQueryKey = ["inbound", "putaway-strategy-profiles"] as const;

function isDevMockNotFound(error: unknown): boolean {
  if (!error || typeof error !== "object") return false;
  const code = "code" in error ? String(error.code ?? "") : "";
  const message = "message" in error ? String(error.message ?? "") : "";
  return code === "DEV_MOCK_NOT_FOUND" || /Dev mock route not found/i.test(message);
}

function idempotencyKey() {
  return `web-m2-putaway-strategy-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}

async function listPutawayStrategyProfiles(): Promise<PutawayStrategyProfile[]> {
  const result = await api.GET("/api/v1/inbound/putaway-strategy-profiles");
  if (!result.data) {
    if (isDevMockNotFound(result.error) || result.response.status === 404) {
      return [];
    }
    throw new ApiError(result.error, "读取上架策略方案失败", result.response.status);
  }
  return result.data.data;
}

async function upsertPutawayStrategyProfile(
  body: UpsertPutawayStrategyProfileRequest,
): Promise<PutawayStrategyProfile> {
  const result = await api.PUT("/api/v1/inbound/putaway-strategy-profiles", {
    params: { header: { "Idempotency-Key": idempotencyKey() } },
    body,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存上架策略方案失败", result.response.status);
  }
  return result.data;
}

export function usePutawayStrategyProfilesQuery() {
  return useQuery<PutawayStrategyProfile[], ApiError>({
    queryKey: putawayStrategyProfilesQueryKey,
    queryFn: listPutawayStrategyProfiles,
  });
}

export function useUpsertPutawayStrategyProfileMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: upsertPutawayStrategyProfile,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: putawayStrategyProfilesQueryKey });
    },
  });
}
