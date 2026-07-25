import { requestJson } from "@wms/api-client";

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

export function login(username: string, password: string) {
  return requestJson<LoginResponse>({
    baseUrl,
    path: "/api/v1/auth/login",
    method: "POST",
    body: { username, password },
  });
}

export function listAddresses(token: string) {
  return requestJson<Address[]>({
    baseUrl,
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
  return requestJson<OrderSummary[]>({
    baseUrl,
    path: `/api/v1/orders?${params.toString()}`,
    authToken: token,
  });
}

export function getOrder(token: string, orderId: string) {
  return requestJson<OrderDetail>({
    baseUrl,
    path: `/api/v1/orders/${orderId}`,
    authToken: token,
  });
}

export function authorizeReportDownload(token: string, reportId: string) {
  return requestJson<DownloadUrl>({
    baseUrl,
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
  return requestJson<ExportJob>({
    baseUrl,
    path: "/api/v1/exports",
    method: "POST",
    authToken: token,
    body: { order_ids: orderIds, include_history: includeHistory },
  });
}

export function listExports(token: string) {
  return requestJson<ExportJob[]>({
    baseUrl,
    path: "/api/v1/exports",
    authToken: token,
  });
}

export function authorizeExportDownload(token: string, exportId: string) {
  return requestJson<DownloadUrl>({
    baseUrl,
    path: `/api/v1/exports/${exportId}/download`,
    method: "POST",
    authToken: token,
  });
}

export function listUsers(token: string) {
  return requestJson<PortalUser[]>({
    baseUrl,
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
  return requestJson<PortalUser>({
    baseUrl,
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
  return requestJson<PortalUser>({
    baseUrl,
    path: `/api/v1/users/${userId}`,
    method: "PUT",
    authToken: token,
    body: request,
  });
}
