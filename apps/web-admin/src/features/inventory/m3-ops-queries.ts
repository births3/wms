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

export function useSubmitInventoryCountLineMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (input: { countId: string; lineId: string; physical_qty: number }) => {
      const token = readAccessToken();
      const response = await fetch(
        `${apiBaseUrl}/api/v1/inventory/counts/${input.countId}/lines/${input.lineId}/submit`,
        {
          method: "POST",
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "Idempotency-Key": `web-m3-count-line-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`,
            ...(token ? { Authorization: `Bearer ${token}` } : {}),
          },
          credentials: "include",
          body: JSON.stringify({ physical_qty: input.physical_qty }),
        },
      );
      if (!response.ok) {
        throw new ApiError(await response.json().catch(() => null), "提交实盘数量失败", response.status);
      }
      return response.json();
    },
    onSuccess: () => void client.invalidateQueries({ queryKey: ["inventory", "counts"] }),
  });
}

export function useApproveInventoryCountMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (input: { countId: string; approval_source?: string; approval_id?: string }) => {
      const token = readAccessToken();
      const response = await fetch(`${apiBaseUrl}/api/v1/inventory/counts/${input.countId}/approve`, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "Idempotency-Key": `web-m3-count-approve-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`,
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        credentials: "include",
        body: JSON.stringify({
          // 默认普通审批源；超阈值时服务端要求「盘点-高级」，由调用方覆盖。
          approval_source: input.approval_source ?? "盘点",
          approval_id: input.approval_id ?? input.countId,
        }),
      });
      if (!response.ok) {
        throw new ApiError(await response.json().catch(() => null), "审批盘点差异失败", response.status);
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

export function useCreateMaintenanceRecordMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (body: {
      task_id: string;
      temperature_celsius: number;
      humidity_percent: number;
      appearance: string;
      packaging: string;
      pest: string;
      rodent: string;
      mildew: string;
      conclusion: string;
      exception_type?: string | null;
      notes?: string | null;
    }) => {
      const token = readAccessToken();
      const response = await fetch(`${apiBaseUrl}/api/v1/inventory/maintenance/records`, {
        method: "POST",
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "Idempotency-Key": `web-m3-maint-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`,
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        credentials: "include",
        body: JSON.stringify(body),
      });
      if (!response.ok) {
        throw new ApiError(await response.json().catch(() => null), "提交养护结果失败", response.status);
      }
      return response.json();
    },
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
