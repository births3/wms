import { useQuery } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type InventoryBatch = components["schemas"]["InventoryBatch"];

export const inventoryBatchesQueryKey = ["inventory", "batches"] as const;

async function listInventoryBatches(): Promise<InventoryBatch[]> {
  const result = await api.GET("/api/v1/inventory/batches");
  if (!result.data) {
    throw new ApiError(result.error, "读取库存批次失败", result.response.status);
  }
  return result.data.data;
}

export function useInventoryBatchesQuery() {
  return useQuery<InventoryBatch[], ApiError>({
    queryKey: inventoryBatchesQueryKey,
    queryFn: listInventoryBatches,
  });
}
