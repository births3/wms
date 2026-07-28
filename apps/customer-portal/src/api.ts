import { JsonApiError, requestJson } from "@wms/api-client";

import type {
  Address,
  DownloadUrl,
  ExportJob,
  LoginResponse,
  OrderDetail,
  OrderSummary,
  PortalUser,
} from "./types";

const baseUrl = import.meta.env.VITE_PORTAL_API_BASE_URL ?? "";

let onSessionExpired: (() => void) | null = null;

/** 注册会话过期回调；仅带 token 的接口返回 401 时触发，登录接口自身的 401（密码错误）不受影响。 */
export function setSessionExpiredHandler(handler: (() => void) | null) {
  onSessionExpired = handler;
}

async function authorizedJson<T>(opts: {
  path: string;
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  authToken: string;
  body?: unknown;
}): Promise<T> {
  try {
    return await requestJson<T>({ baseUrl, ...opts });
  } catch (error) {
    if (error instanceof JsonApiError && error.status === 401) {
      onSessionExpired?.();
      throw new JsonApiError("登录已过期，请重新登录", error.status, error.code);
    }
    throw error;
  }
}

export function login(username: string, password: string) {
  return requestJson<LoginResponse>({
    baseUrl,
    path: "/api/v1/auth/login",
    method: "POST",
    body: { username, password },
  });
}

export function listAddresses(token: string) {
  return authorizedJson<Address[]>({
    path: "/api/v1/addresses",
    authToken: token,
  });
}

export function listOrders(
  token: string,
  query: { addressId?: string; status?: string; keyword?: string },
) {
  const params = new URLSearchParams();
  if (query.addressId) params.set("address_id", query.addressId);
  if (query.status) params.set("status", query.status);
  if (query.keyword) params.set("keyword", query.keyword);
  return authorizedJson<OrderSummary[]>({
    path: `/api/v1/orders?${params.toString()}`,
    authToken: token,
  });
}

export function getOrder(token: string, orderId: string) {
  return authorizedJson<OrderDetail>({
    path: `/api/v1/orders/${orderId}`,
    authToken: token,
  });
}

export function authorizeReportDownload(token: string, reportId: string) {
  return authorizedJson<DownloadUrl>({
    path: `/api/v1/report-versions/${reportId}/download`,
    method: "POST",
    authToken: token,
  });
}

export function createExport(
  token: string,
  orderIds: string[],
  includeHistory: boolean,
) {
  return authorizedJson<ExportJob>({
    path: "/api/v1/exports",
    method: "POST",
    authToken: token,
    body: { order_ids: orderIds, include_history: includeHistory },
  });
}

export function listExports(token: string) {
  return authorizedJson<ExportJob[]>({
    path: "/api/v1/exports",
    authToken: token,
  });
}

export function authorizeExportDownload(token: string, exportId: string) {
  return authorizedJson<DownloadUrl>({
    path: `/api/v1/exports/${exportId}/download`,
    method: "POST",
    authToken: token,
  });
}

export function listUsers(token: string) {
  return authorizedJson<PortalUser[]>({
    path: "/api/v1/users",
    authToken: token,
  });
}

export function createUser(
  token: string,
  request: {
    username: string;
    display_name: string;
    password: string;
    role: "customer_admin" | "customer_user";
    can_view_report_history: boolean;
    address_ids: string[];
  },
) {
  return authorizedJson<PortalUser>({
    path: "/api/v1/users",
    method: "POST",
    authToken: token,
    body: request,
  });
}

export function updateUser(
  token: string,
  userId: string,
  request: {
    display_name: string;
    role: "customer_admin" | "customer_user";
    status: "active" | "disabled";
    can_view_report_history: boolean;
    address_ids: string[];
  },
) {
  return authorizedJson<PortalUser>({
    path: `/api/v1/users/${userId}`,
    method: "PUT",
    authToken: token,
    body: request,
  });
}
