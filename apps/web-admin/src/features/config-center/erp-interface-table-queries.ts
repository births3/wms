import { useQuery } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type H8ErpInterfaceTableRow = components["schemas"]["H8ErpInterfaceTableRow"];
export type H8ErpInterfaceTableDetail = components["schemas"]["H8ErpInterfaceTableDetail"];
export type H8ErpInterfaceTableConnectorOption = components["schemas"]["H8ErpInterfaceTableConnectorOption"];

export type H8ErpInterfaceTableListParams = {
  connector_id: string;
  table_key: string;
  sync_status?: string;
  time_from?: string;
  time_to?: string;
  warehouse_id?: string;
  external_doc_no?: string;
  source_outbox_id?: string;
  event_type?: string;
  external_ref?: string;
  wms_resource_id?: string;
  idempotency_key?: string;
  page?: number;
  page_size?: number;
};

const key = ["h8", "erp-interface-tables"] as const;

export function useH8ErpInterfaceTableConnectorsQuery() {
  return useQuery({
    queryKey: [...key, "connectors"],
    queryFn: async () => {
      const result = await api.GET("/api/v1/h8/erp-interface-tables/connectors");
      if (!result.data) {
        throw new ApiError(result.error, "读取接口表连接失败", result.response.status);
      }
      return result.data;
    },
  });
}

export function useH8ErpInterfaceTableRowsQuery(params: H8ErpInterfaceTableListParams) {
  return useQuery({
    queryKey: [...key, params],
    enabled: Boolean(params.connector_id && params.table_key),
    queryFn: async () => {
      const result = await api.GET("/api/v1/h8/erp-interface-tables/rows", {
        params: { query: params },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取接口表失败", result.response.status);
      }
      return result.data;
    },
  });
}

export function useH8ErpInterfaceTableDetailQuery(
  connectorId: string,
  tableKey: string,
  rowId: string | null,
) {
  return useQuery({
    queryKey: [...key, "detail", connectorId, tableKey, rowId],
    enabled: Boolean(connectorId && tableKey && rowId),
    queryFn: async () => {
      const result = await api.GET("/api/v1/h8/erp-interface-tables/rows/{row_id}", {
        params: {
          path: { row_id: rowId ?? "" },
          query: { connector_id: connectorId, table_key: tableKey },
        },
      });
      if (!result.data) {
        throw new ApiError(result.error, "读取接口表详情失败", result.response.status);
      }
      return result.data;
    },
  });
}
