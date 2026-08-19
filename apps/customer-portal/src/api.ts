import { createApiClient, JsonApiError } from "@wms/api-client";

import type { components, paths } from "./schema";
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
type PortalError = components["schemas"]["Error"];

let onSessionExpired: (() => void) | null = null;

/** 注册会话过期回调；登录接口自身的 401（密码错误）不触发回调。 */
export function setSessionExpiredHandler(handler: (() => void) | null) {
  onSessionExpired = handler;
}

function portalApi(authToken?: string) {
  return createApiClient<paths>({
    baseUrl,
    authToken: () => authToken ?? null,
  });
}

function readData<T>(
  result: { data?: T; error?: PortalError; response: Response },
  fallback: string,
  authenticated = false,
): T {
  if (result.data !== undefined) return result.data;
  if (authenticated && result.response.status === 401) {
    onSessionExpired?.();
    throw new JsonApiError("登录已过期，请重新登录", result.response.status, result.error?.code);
  }
  throw new JsonApiError(result.error?.message ?? fallback, result.response.status, result.error?.code);
}

export function login(username: string, password: string) {
  return portalApi().POST("/api/v1/auth/login", { body: { username, password } }).then((result) =>
    readData<LoginResponse>(result, "登录失败"),
  );
}

export function listAddresses(token: string) {
  return portalApi(token).GET("/api/v1/addresses").then((result) =>
    readData<Address[]>(result, "读取地址失败", true),
  );
}

export function listOrders(
  token: string,
  query: { addressId?: string; status?: string; keyword?: string },
) {
  const status = query.status === "shipped" || query.status === "signed" ? query.status : undefined;
  return portalApi(token)
    .GET("/api/v1/orders", {
      params: {
        query: {
          address_id: query.addressId,
          status,
          keyword: query.keyword,
        },
      },
    })
    .then((result) => readData<OrderSummary[]>(result, "读取订单失败", true));
}

export function getOrder(token: string, orderId: string) {
  return portalApi(token)
    .GET("/api/v1/orders/{order_id}", { params: { path: { order_id: orderId } } })
    .then((result) => readData<OrderDetail>(result, "读取订单详情失败", true));
}

export function authorizeReportDownload(token: string, reportId: string) {
  return portalApi(token)
    .POST("/api/v1/report-versions/{report_version_id}/download", {
      params: { path: { report_version_id: reportId } },
    })
    .then((result) => readData<DownloadUrl>(result, "授权药检单下载失败", true));
}

export function createExport(token: string, orderIds: string[], includeHistory: boolean) {
  return portalApi(token)
    .POST("/api/v1/exports", { body: { order_ids: orderIds, include_history: includeHistory } })
    .then((result) => readData<ExportJob>(result, "创建导出任务失败", true));
}

export function listExports(token: string) {
  return portalApi(token).GET("/api/v1/exports").then((result) =>
    readData<ExportJob[]>(result, "读取导出任务失败", true),
  );
}

export function authorizeExportDownload(token: string, exportId: string) {
  return portalApi(token)
    .POST("/api/v1/exports/{export_id}/download", { params: { path: { export_id: exportId } } })
    .then((result) => readData<DownloadUrl>(result, "授权导出下载失败", true));
}

export function listUsers(token: string) {
  return portalApi(token).GET("/api/v1/users").then((result) =>
    readData<PortalUser[]>(result, "读取客户账号失败", true),
  );
}

export function createUser(
  token: string,
  request: components["schemas"]["CreateUserRequest"],
) {
  return portalApi(token).POST("/api/v1/users", { body: request }).then((result) =>
    readData<PortalUser>(result, "创建客户账号失败", true),
  );
}

export function updateUser(
  token: string,
  userId: string,
  request: components["schemas"]["UpdateUserRequest"],
) {
  return portalApi(token)
    .PUT("/api/v1/users/{user_id}", { params: { path: { user_id: userId } }, body: request })
    .then((result) => readData<PortalUser>(result, "更新客户账号失败", true));
}
