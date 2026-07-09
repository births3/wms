import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type H4NotificationConfig = components["schemas"]["H4NotificationConfig"];
export type H4NotificationRecord = components["schemas"]["H4NotificationRecord"];
export type H4WechatSettings = components["schemas"]["H4WechatSettings"];
export type UpsertH4NotificationConfigRequest = components["schemas"]["UpsertH4NotificationConfigRequest"];
export type UpsertH4WechatSettingsRequest = components["schemas"]["UpsertH4WechatSettingsRequest"];
export type SendH4NotificationRequest = components["schemas"]["SendH4NotificationRequest"];

export interface H4RecordQueryParams {
  eventType?: string;
  recipient?: string;
  status?: string;
  from?: string;
  to?: string;
}

export const wechatNotifyQueryKey = ["wechat-notify"] as const;

export function useH4NotificationConfigsQuery(eventType?: string) {
  return useQuery<H4NotificationConfig[], ApiError>({
    queryKey: [...wechatNotifyQueryKey, "configs", eventType ?? ""],
    queryFn: () => listH4NotificationConfigs(eventType),
  });
}

export function useH4NotificationRecordsQuery(params: H4RecordQueryParams) {
  return useQuery<H4NotificationRecord[], ApiError>({
    queryKey: [...wechatNotifyQueryKey, "records", params],
    queryFn: () => listH4NotificationRecords(params),
  });
}

export function useH4WechatSettingsQuery() {
  return useQuery<H4WechatSettings | null, ApiError>({
    queryKey: [...wechatNotifyQueryKey, "settings"],
    queryFn: getH4WechatSettings,
  });
}

export function useUpsertH4NotificationConfigMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: upsertH4NotificationConfig,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: wechatNotifyQueryKey });
    },
  });
}

export function useUpsertH4WechatSettingsMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: upsertH4WechatSettings,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: [...wechatNotifyQueryKey, "settings"] });
    },
  });
}

export function useSendH4NotificationMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: sendH4Notification,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: [...wechatNotifyQueryKey, "records"] });
    },
  });
}

export function useResendH4NotificationRecordMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: resendH4NotificationRecord,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: [...wechatNotifyQueryKey, "records"] });
    },
  });
}

async function getH4WechatSettings(): Promise<H4WechatSettings | null> {
  const result = await api.GET("/api/v1/wechat-notify/settings");
  if (!result.data) {
    throw new ApiError(result.error, "读取企业微信参数设置失败", result.response.status);
  }
  return result.data.data ?? null;
}

async function listH4NotificationConfigs(eventType?: string): Promise<H4NotificationConfig[]> {
  const result = await api.GET("/api/v1/wechat-notify/configs", {
    params: { query: { event_type: eventType || undefined } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取企业微信通知配置失败", result.response.status);
  }
  return result.data.data;
}

async function listH4NotificationRecords(params: H4RecordQueryParams): Promise<H4NotificationRecord[]> {
  const result = await api.GET("/api/v1/wechat-notify/records", {
    params: {
      query: {
        event_type: params.eventType || undefined,
        recipient: params.recipient || undefined,
        status: params.status || undefined,
        from: params.from || undefined,
        to: params.to || undefined,
        limit: 100,
      },
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取企业微信通知记录失败", result.response.status);
  }
  return result.data.data;
}

async function upsertH4NotificationConfig(request: UpsertH4NotificationConfigRequest) {
  const result = await api.POST("/api/v1/wechat-notify/configs", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-h4-config") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存企业微信通知配置失败", result.response.status);
  }
  return result.data;
}

async function upsertH4WechatSettings(request: UpsertH4WechatSettingsRequest) {
  const result = await api.POST("/api/v1/wechat-notify/settings", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-h4-settings") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存企业微信参数设置失败", result.response.status);
  }
  return result.data;
}

async function sendH4Notification(request: SendH4NotificationRequest) {
  const result = await api.POST("/api/v1/wechat-notify/send", {
    params: { header: { "Idempotency-Key": idempotencyKey("web-h4-send") } },
    body: request,
  });
  if (!result.data) {
    throw new ApiError(result.error, "发送企业微信通知失败", result.response.status);
  }
  return result.data;
}

async function resendH4NotificationRecord(recordId: string) {
  const result = await api.POST("/api/v1/wechat-notify/records/{record_id}/resend", {
    params: {
      path: { record_id: recordId },
      header: { "Idempotency-Key": idempotencyKey("web-h4-resend") },
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "重发企业微信通知失败", result.response.status);
  }
  return result.data;
}

function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}
