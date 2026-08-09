import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type FeatureFlagConfig = components["schemas"]["FeatureFlagConfig"];
type FeatureFlagExportResponse = components["schemas"]["FeatureFlagExportResponse"];

export const featureFlagQueryKey = ["config-center", "feature-flags"] as const;

export function useFeatureFlagsQuery() {
  return useQuery<FeatureFlagExportResponse, ApiError>({
    queryKey: [...featureFlagQueryKey, "export"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/config-center/feature-flags/export");
      if (!result.data) throw new ApiError(result.error, "读取 Feature Flag 列表失败", result.response.status);
      return result.data;
    },
  });
}

export function useMigrateFeatureFlagsMutation() {
  const invalidate = useInvalidateFeatureFlags();
  return useMutation({
    mutationFn: async () => {
      const result = await api.POST("/api/v1/config-center/feature-flags/migrate");
      if (!result.data) throw new ApiError(result.error, "迁移 Feature Flag 失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useImportFeatureFlagsMutation() {
  const invalidate = useInvalidateFeatureFlags();
  return useMutation({
    mutationFn: async (flags: FeatureFlagConfig[]) => {
      const result = await api.POST("/api/v1/config-center/feature-flags/import", { body: { flags } });
      if (!result.data) throw new ApiError(result.error, "导入 Feature Flag 失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useSwitchFeatureFlagSourceMutation() {
  const invalidate = useInvalidateFeatureFlags();
  return useMutation({
    mutationFn: async (source: string) => {
      const result = await api.POST("/api/v1/config-center/feature-flags/source", { body: { source } });
      if (!result.data) throw new ApiError(result.error, "切换 Feature Flag 读取源失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function useArchiveFeatureFlagSourceMutation() {
  const invalidate = useInvalidateFeatureFlags();
  return useMutation({
    mutationFn: async (archiveRef: string) => {
      const result = await api.POST("/api/v1/config-center/feature-flags/archive-file-source", {
        body: { archive_ref: archiveRef },
      });
      if (!result.data) throw new ApiError(result.error, "归档 Feature Flag 文件源失败", result.response.status);
      return result.data;
    },
    onSuccess: invalidate,
  });
}

export function parseFeatureFlagImportJson(text: string): FeatureFlagConfig[] {
  const payload = text.trim();
  if (!payload) throw new Error("请粘贴 Feature Flag JSON");
  let parsed: unknown;
  try {
    parsed = JSON.parse(payload);
  } catch {
    throw new Error("JSON 格式不正确");
  }
  const flags = Array.isArray(parsed) ? parsed : isRecord(parsed) && Array.isArray(parsed.flags) ? parsed.flags : null;
  if (!flags) throw new Error("JSON 必须是数组，或包含 flags 数组的对象");
  if (flags.length === 0) throw new Error("请至少导入一个 Feature Flag");
  return flags.map((flag, index) => featureFlagConfig(flag, index + 1));
}

function featureFlagConfig(input: unknown, index: number): FeatureFlagConfig {
  if (!isRecord(input)) throw new Error(`第 ${index} 条 Feature Flag 必须是对象`);
  if (typeof input.enabled !== "boolean") throw new Error(`第 ${index} 条 enabled 必须是布尔值`);
  return {
    key: requiredText(input.key, `第 ${index} 条 key`),
    owner: requiredText(input.owner, `第 ${index} 条 owner`),
    created_at: requiredText(input.created_at, `第 ${index} 条 created_at`),
    cleanup_by: requiredText(input.cleanup_by, `第 ${index} 条 cleanup_by`),
    enabled: input.enabled,
    source: requiredText(input.source, `第 ${index} 条 source`),
  };
}

function requiredText(value: unknown, label: string) {
  if (typeof value !== "string" || !value.trim()) throw new Error(`${label} 不能为空`);
  return value.trim();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function useInvalidateFeatureFlags() {
  const queryClient = useQueryClient();
  return () => void queryClient.invalidateQueries({ queryKey: featureFlagQueryKey });
}
