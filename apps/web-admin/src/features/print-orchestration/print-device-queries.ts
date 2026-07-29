import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type PrintSite = components["schemas"]["PrintSite"];
export type CreatePrintSiteRequest = components["schemas"]["CreatePrintSiteRequest"];
export type PrintSiteOwnerMapping = components["schemas"]["PrintSiteOwnerMapping"];
export type CreateSiteOwnerMappingRequest =
  components["schemas"]["CreateSiteOwnerMappingRequest"];
export type Printer = components["schemas"]["Printer"];
export type CreatePrinterRequest = components["schemas"]["CreatePrinterRequest"];
export type UpdatePrinterRequest = components["schemas"]["UpdatePrinterRequest"];
export type PrinterTray = components["schemas"]["PrinterTray"];
export type CreatePrinterTrayRequest = components["schemas"]["CreatePrinterTrayRequest"];
export type UpdatePrinterTrayRequest = components["schemas"]["UpdatePrinterTrayRequest"];
export type PrinterTestPrint = components["schemas"]["PrinterTestPrint"];
export type DeviceLease = components["schemas"]["DeviceLease"];
export type ReleaseDeviceLeaseRequest = components["schemas"]["ReleaseDeviceLeaseRequest"];

const printDeviceQueryKey = ["h9", "print-devices"] as const;

export function usePrintSitesQuery() {
  return useQuery<PrintSite[], ApiError>({
    queryKey: [...printDeviceQueryKey, "sites"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-devices/sites");
      if (!result.data) {
        throw new ApiError(result.error, "读取物理打印站点失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function useSiteOwnerMappingsQuery(siteId: string) {
  return useQuery<PrintSiteOwnerMapping[], ApiError>({
    queryKey: [...printDeviceQueryKey, "owner-mappings", siteId],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-devices/sites/{site_id}/owner-mappings", {
        params: { path: { site_id: siteId } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取站点货主仓映射失败", result.response.status);
      }
      return result.data.data;
    },
    enabled: Boolean(siteId),
  });
}

export function usePrintersQuery() {
  return useQuery<Printer[], ApiError>({
    queryKey: [...printDeviceQueryKey, "printers"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-devices/printers");
      if (!result.data) {
        throw new ApiError(result.error, "读取打印机失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function usePrinterTraysQuery(printerId: string) {
  return useQuery<PrinterTray[], ApiError>({
    queryKey: [...printDeviceQueryKey, "trays", printerId],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-devices/printers/{printer_id}/trays", {
        params: { path: { printer_id: printerId } },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取纸盒失败", result.response.status);
      }
      return result.data.data;
    },
    enabled: Boolean(printerId),
  });
}

export function useDeviceLeasesQuery() {
  return useQuery<DeviceLease[], ApiError>({
    queryKey: [...printDeviceQueryKey, "leases"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/print-devices/leases");
      if (!result.data) {
        throw new ApiError(result.error, "读取设备租约失败", result.response.status);
      }
      return result.data.data;
    },
  });
}

export function useCreatePrintSiteMutation() {
  const queryClient = useQueryClient();
  return useMutation<PrintSite, ApiError, CreatePrintSiteRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/print-devices/sites", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h9-print-site") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "创建物理打印站点失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

export function useCreateSiteOwnerMappingMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    PrintSiteOwnerMapping,
    ApiError,
    { siteId: string; request: CreateSiteOwnerMappingRequest }
  >({
    mutationFn: async ({ siteId, request }) => {
      const result = await api.POST("/api/v1/print-devices/sites/{site_id}/owner-mappings", {
        params: {
          path: { site_id: siteId },
          header: { "Idempotency-Key": idempotencyKey("web-h9-site-mapping") },
        },
        body: request,
      });
      if (!result.data) {
        throw new ApiError(result.error, "映射货主仓失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

export function useDisableSiteOwnerMappingMutation() {
  const queryClient = useQueryClient();
  return useMutation<PrintSiteOwnerMapping, ApiError, { siteId: string; mappingId: string }>({
    mutationFn: async ({ siteId, mappingId }) => {
      const result = await api.POST(
        "/api/v1/print-devices/sites/{site_id}/owner-mappings/{mapping_id}/disable",
        {
          params: {
            path: { site_id: siteId, mapping_id: mappingId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-site-mapping-disable") },
          },
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "停用货主仓映射失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

export function useCreatePrinterMutation() {
  const queryClient = useQueryClient();
  return useMutation<Printer, ApiError, CreatePrinterRequest>({
    mutationFn: async (body) => {
      const result = await api.POST("/api/v1/print-devices/printers", {
        params: { header: { "Idempotency-Key": idempotencyKey("web-h9-printer") } },
        body,
      });
      if (!result.data) {
        throw new ApiError(result.error, "创建打印机失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

export function useUpdatePrinterMutation() {
  const queryClient = useQueryClient();
  return useMutation<Printer, ApiError, { printerId: string; request: UpdatePrinterRequest }>({
    mutationFn: async ({ printerId, request }) => {
      const result = await api.PATCH("/api/v1/print-devices/printers/{printer_id}", {
        params: {
          path: { printer_id: printerId },
          header: { "Idempotency-Key": idempotencyKey("web-h9-printer-update") },
        },
        body: request,
      });
      if (!result.data) {
        throw new ApiError(result.error, "维护打印机失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

export function useCreatePrinterTrayMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    PrinterTray,
    ApiError,
    { printerId: string; request: CreatePrinterTrayRequest }
  >({
    mutationFn: async ({ printerId, request }) => {
      const result = await api.POST("/api/v1/print-devices/printers/{printer_id}/trays", {
        params: {
          path: { printer_id: printerId },
          header: { "Idempotency-Key": idempotencyKey("web-h9-printer-tray") },
        },
        body: request,
      });
      if (!result.data) {
        throw new ApiError(result.error, "创建纸盒失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

export function useUpdatePrinterTrayMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    PrinterTray,
    ApiError,
    { printerId: string; trayId: string; request: UpdatePrinterTrayRequest }
  >({
    mutationFn: async ({ printerId, trayId, request }) => {
      const result = await api.PATCH(
        "/api/v1/print-devices/printers/{printer_id}/trays/{tray_id}",
        {
          params: {
            path: { printer_id: printerId, tray_id: trayId },
            header: { "Idempotency-Key": idempotencyKey("web-h9-printer-tray-update") },
          },
          body: request,
        },
      );
      if (!result.data) {
        throw new ApiError(result.error, "维护纸盒失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

export function useTestPrintMutation() {
  const queryClient = useQueryClient();
  return useMutation<PrinterTestPrint, ApiError, { printerId: string; trayId: string }>({
    mutationFn: async ({ printerId, trayId }) => {
      const result = await api.POST("/api/v1/print-devices/printers/{printer_id}/test-print", {
        params: {
          path: { printer_id: printerId },
          header: { "Idempotency-Key": idempotencyKey("web-h9-test-print") },
        },
        body: { tray_id: trayId },
      });
      if (!result.data) {
        throw new ApiError(result.error, "下发测试打印失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

export function useReleaseDeviceLeaseMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    DeviceLease,
    ApiError,
    { leaseId: string; request: ReleaseDeviceLeaseRequest }
  >({
    mutationFn: async ({ leaseId, request }) => {
      const result = await api.POST("/api/v1/print-devices/leases/{lease_id}/release", {
        params: {
          path: { lease_id: leaseId },
          header: { "Idempotency-Key": idempotencyKey("web-h9-lease-release") },
        },
        body: request,
      });
      if (!result.data) {
        throw new ApiError(result.error, "人工释放租约失败", result.response.status);
      }
      return result.data;
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: printDeviceQueryKey }),
  });
}

function idempotencyKey(prefix: string) {
  return `${prefix}-${globalThis.crypto.randomUUID()}`;
}
