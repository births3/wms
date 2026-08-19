import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type AlertInstance = components["schemas"]["AlertInstance"];
export type AlertInstanceFilters = components["schemas"]["AlertInstanceListQuery"];
export type AlertStatistics = components["schemas"]["AlertStatisticsResponse"];
export type GspAlertLifecycleReport = components["schemas"]["GspAlertLifecycleReport"];
export type AlertExportJob = components["schemas"]["AlertExportJob"];
export type AlertEscalationRule = components["schemas"]["AlertEscalationRule"];
export type AlertEscalationRuleDraft = components["schemas"]["UpsertAlertEscalationRuleRequest"];

const activeAlertKey = ["hal", "active-alerts"] as const;
const alertStatisticsKey = ["hal", "alert-statistics"] as const;
const gspAlertReportKey = ["hal", "gsp-alert-report"] as const;
const escalationRuleKey = ["hal", "alert-escalation-rules"] as const;

export function useActiveAlertsQuery(filters: AlertInstanceFilters) {
  return useQuery<components["schemas"]["AlertInstanceListResponse"], ApiError>({
    queryKey: [...activeAlertKey, filters],
    queryFn: async () => {
      const result = await api.GET("/api/v1/alerts/active", { params: { query: filters } });
      if (!result.data) throw apiError(result.error, "读取活动告警失败", result.response.status);
      return result.data;
    },
    refetchInterval: 5_000,
    retry: false,
  });
}

export function useAlertStatisticsQuery(filters: AlertInstanceFilters) {
  return useQuery<AlertStatistics, ApiError>({
    queryKey: [...alertStatisticsKey, filters],
    queryFn: async () => {
      const result = await api.GET("/api/v1/alerts/statistics", { params: { query: filters } });
      if (!result.data) throw apiError(result.error, "读取告警统计失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useGspAlertReportQuery(filters: AlertInstanceFilters) {
  return useQuery<GspAlertLifecycleReport, ApiError>({
    queryKey: [...gspAlertReportKey, filters],
    queryFn: async () => {
      const result = await api.GET("/api/v1/alerts/gsp-report", { params: { query: filters } });
      if (!result.data) throw apiError(result.error, "读取 GSP 告警报表失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

type AlertAction = { id: string; operation: "acknowledge" | "handling" | "close" | "ignore"; description?: string };

export function useAlertActionMutation() {
  const queryClient = useQueryClient();
  return useMutation<AlertInstance, ApiError, AlertAction>({
    mutationFn: async ({ id, operation, description = "" }) => {
      if (operation === "acknowledge") {
        const result = await api.POST("/api/v1/alerts/{id}/acknowledge", { params: { path: { id } } });
        if (!result.data) throw apiError(result.error, "确认接警失败", result.response.status);
        return result.data;
      }
      const body = { description };
      const result = operation === "handling"
        ? await api.POST("/api/v1/alerts/{id}/handling", { params: { path: { id } }, body })
        : operation === "close"
          ? await api.POST("/api/v1/alerts/{id}/close", { params: { path: { id } }, body })
          : await api.POST("/api/v1/alerts/{id}/ignore", { params: { path: { id } }, body });
      if (!result.data) throw apiError(result.error, "处理告警失败", result.response.status);
      return result.data;
    },
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: activeAlertKey }),
        queryClient.invalidateQueries({ queryKey: alertStatisticsKey }),
        queryClient.invalidateQueries({ queryKey: gspAlertReportKey }),
      ]);
    },
  });
}

export function useCreateAlertExportMutation() {
  return useMutation<AlertExportJob, ApiError, { format: "excel" | "pdf"; filters: AlertInstanceFilters; recipientEmail?: string }>({
    mutationFn: async ({ format, filters, recipientEmail }) => {
      const result = await api.POST("/api/v1/alerts/exports", {
        body: { format, filters, recipient_email: recipientEmail?.trim() || null },
      });
      if (!result.data) throw apiError(result.error, "创建告警报表失败", result.response.status);
      return result.data;
    },
  });
}

export async function downloadAlertExport(downloadUrl: string, format: "excel" | "pdf") {
  const token = downloadUrl.split("/").at(-2);
  if (!token) throw new Error("导出下载地址无效");
  const result = await api.GET("/api/v1/alerts/exports/{token}/download", {
    params: { path: { token } },
    parseAs: "arrayBuffer",
  });
  if (!result.data) throw apiError(result.error, "下载告警报表失败", result.response.status);
  const blob = new Blob([result.data], { type: format === "pdf" ? "application/pdf" : "application/vnd.ms-excel" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `H-AL-告警报表.${format === "pdf" ? "pdf" : "xls"}`;
  anchor.click();
  URL.revokeObjectURL(url);
}

export function useAlertEscalationRulesQuery() {
  return useQuery<components["schemas"]["AlertEscalationRuleListResponse"], ApiError>({
    queryKey: escalationRuleKey,
    queryFn: async () => {
      const result = await api.GET("/api/v1/alert-escalation-rules");
      if (!result.data) throw apiError(result.error, "读取升级规则失败", result.response.status);
      return result.data;
    },
    retry: false,
  });
}

export function useUpsertAlertEscalationRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<AlertEscalationRule, ApiError, AlertEscalationRuleDraft>({
    mutationFn: async (body) => {
      const result = await api.PUT("/api/v1/alert-escalation-rules/{rule_code}", {
        params: { path: { rule_code: body.rule_code } },
        body,
      });
      if (!result.data) throw apiError(result.error, "保存升级规则失败", result.response.status);
      return result.data;
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: escalationRuleKey }),
  });
}

function apiError(error: components["schemas"]["ErrorResponse"] | undefined, fallback: string, status: number) {
  return new ApiError(error, fallback, status);
}
