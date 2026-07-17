import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";
import { readAccessToken } from "@/lib/auth-session";
import { apiBaseUrl } from "@/lib/api";

async function authFetch(path: string, init?: RequestInit) {
  const token = readAccessToken();
  const response = await fetch(`${apiBaseUrl}${path}`, {
    ...init,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(init?.headers ?? {}),
    },
    credentials: "include",
  });
  if (!response.ok) {
    let body: unknown = null;
    try {
      body = await response.json();
    } catch {
      body = null;
    }
    throw new ApiError(body as { code: string; details: Record<string, unknown>; message: string; severity: string; trace_id: string } | undefined, "库存操作失败", response.status);
  }
  return response.json();
}

export type InventoryCountSummary = {
  id: string;
  count_type: string;
  status: string;
  product_code?: string | null;
  started_at: string;
  created_at: string;
  lines: Array<{ id: string; product_code: string; batch_no: string; book_qty: number; physical_qty?: number | null; variance_qty?: number | null }>;
};

export type MaintenanceTask = {
  id: string;
  product_code: string;
  batch_no: string;
  location_code: string;
  planned_at: string;
  status: string;
  created_at: string;
};

export type InventoryRelocation = {
  id: string;
  product_code: string;
  batch_no: string;
  qty: number;
  from_location_code: string;
  to_location_code: string;
  status: string;
  created_at: string;
};

export function useInventoryCountsQuery() {
  return useQuery<InventoryCountSummary[], ApiError>({
    queryKey: ["inventory", "counts"],
    queryFn: async () => {
      const body = await authFetch("/api/v1/inventory/counts");
      return (body.data ?? []) as InventoryCountSummary[];
    },
  });
}

export function useCreateInventoryCountMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (body: { count_type: string; product_code?: string }) => {
      const token = readAccessToken();
      const response = await fetch(`${apiBaseUrl}/api/v1/inventory/counts`, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "Idempotency-Key": `web-m3-count-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`,
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        credentials: "include",
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        throw new ApiError(await response.json().catch(() => null), "创建盘点单失败", response.status);
      }
      return response.json();
    },
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventory", "counts"] }),
  });
}

export function useMaintenanceTasksQuery() {
  return useQuery<MaintenanceTask[], ApiError>({
    queryKey: ["inventory", "maintenance-tasks"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/inventory/maintenance/tasks", { params: { query: {} } });
      if (!result.data) throw new ApiError(result.error, "读取养护任务失败", result.response.status);
      return result.data.data as MaintenanceTask[];
    },
  });
}

export function useGenerateMaintenanceTasksMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async () => authFetch("/api/v1/inventory/maintenance/tasks/generate", { method: "POST", body: "{}" }),
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventory", "maintenance-tasks"] }),
  });
}

export function useInventoryRelocationsQuery() {
  return useQuery<InventoryRelocation[], ApiError>({
    queryKey: ["inventory", "relocations"],
    queryFn: async () => {
      const body = await authFetch("/api/v1/inventory/relocations");
      return (body.data ?? []) as InventoryRelocation[];
    },
  });
}

export function useRelocateInventoryMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (body: {
      batch_id: string;
      qty: number;
      to_location_id: string;
      to_location_code: string;
      reason?: string;
    }) => {
      const token = readAccessToken();
      const response = await fetch(`${apiBaseUrl}/api/v1/inventory/relocations`, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "Idempotency-Key": `web-m3-relocate-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`,
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        credentials: "include",
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        throw new ApiError(await response.json().catch(() => null), "移库失败", response.status);
      }
      return response.json();
    },
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["inventory", "relocations"] });
      void client.invalidateQueries({ queryKey: ["inventory", "batches"] });
    },
  });
}
