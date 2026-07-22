import type { IncomingMessage, ServerResponse } from "node:http";

import { devOwnerId } from "./web-admin-dev-mock-model";
import { sendError, sendJson } from "./web-admin-dev-mock-core-common";

const connectorId = "00000000-0000-0000-0000-00000000e801";
const warehouseId = "00000000-0000-0000-0000-00000000w001";

type DevInterfaceRow = {
  row_id: string;
  connector_id: string;
  table_key: string;
  owner_id: string;
  warehouse_id: string | null;
  business_key: string | null;
  event_type: string | null;
  external_ref: string | null;
  wms_resource_id: string | null;
  sync_status: string;
  retry_count: number;
  last_error: string | null;
  idempotency_key: string | null;
  created_at: string;
  updated_at: string;
  payload_summary: string;
};

const rows: DevInterfaceRow[] = [
  {
    row_id: "00000000-0000-0000-0000-00000000f001",
    connector_id: connectorId,
    table_key: "if_in_asn",
    owner_id: devOwnerId,
    warehouse_id: warehouseId,
    business_key: "ASN-20260719-001",
    event_type: "asn.received",
    external_ref: "ERP-ASN-001",
    wms_resource_id: "00000000-0000-0000-0000-00000000a001",
    sync_status: "success",
    retry_count: 0,
    last_error: null,
    idempotency_key: "h8-asn-001",
    created_at: "2026-07-19T08:00:00.000Z",
    updated_at: "2026-07-19T08:03:00.000Z",
    payload_summary: '{"asn_no":"ASN-20260719-001","line_count":2}',
  },
  {
    row_id: "00000000-0000-0000-0000-00000000f002",
    connector_id: connectorId,
    table_key: "if_in_asn",
    owner_id: devOwnerId,
    warehouse_id: warehouseId,
    business_key: "ASN-20260719-002",
    event_type: "asn.received",
    external_ref: "ERP-ASN-002",
    wms_resource_id: null,
    sync_status: "failed",
    retry_count: 2,
    last_error: "商品编码不存在（已脱敏）",
    idempotency_key: "h8-asn-002",
    created_at: "2026-07-19T09:00:00.000Z",
    updated_at: "2026-07-19T09:05:00.000Z",
    payload_summary: '{"asn_no":"ASN-20260719-002","line_count":1}',
  },
  {
    row_id: "00000000-0000-0000-0000-00000000f003",
    connector_id: connectorId,
    table_key: "if_out_message",
    owner_id: devOwnerId,
    warehouse_id: null,
    business_key: "OUT-20260719-001",
    event_type: "shipment.confirmed",
    external_ref: "ERP-OUT-001",
    wms_resource_id: null,
    sync_status: "acked",
    retry_count: 0,
    last_error: null,
    idempotency_key: "h8-out-001",
    created_at: "2026-07-19T10:00:00.000Z",
    updated_at: "2026-07-19T10:01:00.000Z",
    payload_summary: '{"message_type":"shipment.confirmed","outbox_id":"OUT-20260719-001"}',
  },
];

function filteredRows(url: URL): DevInterfaceRow[] {
  const connector = url.searchParams.get("connector_id");
  const table = url.searchParams.get("table_key");
  const statuses = new Set(
    url.searchParams.get("sync_status")?.split(",").map((value) => value.trim()).filter(Boolean) ?? [],
  );
  const externalDoc = url.searchParams.get("external_doc_no");
  const externalRef = url.searchParams.get("external_ref");
  const warehouse = url.searchParams.get("warehouse_id");
  const sourceOutbox = url.searchParams.get("source_outbox_id");
  const eventType = url.searchParams.get("event_type");
  const wmsResource = url.searchParams.get("wms_resource_id");
  const idempotency = url.searchParams.get("idempotency_key");
  return rows.filter((row) =>
    (!connector || row.connector_id === connector) &&
    (!table || row.table_key === table) &&
    (statuses.size === 0 || statuses.has(row.sync_status)) &&
    (!externalDoc || row.business_key === externalDoc) &&
    (!externalRef || row.external_ref === externalRef) &&
    (!warehouse || row.warehouse_id === warehouse) &&
    (!sourceOutbox || row.business_key === sourceOutbox) &&
    (!eventType || row.event_type === eventType) &&
    (!wmsResource || row.wms_resource_id === wmsResource) &&
    (!idempotency || row.idempotency_key === idempotency),
  );
}

export async function handleH8ErpInterfaceTableDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (pathname === "/api/v1/h8/erp-interface-tables/connectors") {
    if (req.method !== "GET") {
      sendError(res, 405, "METHOD_NOT_ALLOWED", "接口表连接选择仅支持 GET");
      return true;
    }
    sendJson(res, 200, [{
      id: connectorId,
      connector_code: "demo-rest-erp",
      connector_name: "示例 REST ERP",
      channel_mode: "interface_table",
      status: "testing",
      warehouse_ids: [warehouseId],
      probe_credentials_configured: true,
    }]);
    return true;
  }
  if (!pathname.startsWith("/api/v1/h8/erp-interface-tables/rows")) return false;

  const detail = pathname.match(/^\/api\/v1\/h8\/erp-interface-tables\/rows\/([^/]+)$/);
  if (req.method === "GET" && detail) {
    const url = new URL(req.url ?? pathname, "http://wms.local");
    const row = rows.find(
      (item) =>
        item.row_id === decodeURIComponent(detail[1]) &&
        item.connector_id === url.searchParams.get("connector_id") &&
        item.table_key === url.searchParams.get("table_key"),
    );
    if (!row) {
      sendError(res, 404, "H8_INTERFACE_TABLE_ROW_NOT_FOUND", "接口表行不存在");
      return true;
    }
    sendJson(res, 200, {
      row,
      fields: Object.entries(row).map(([key, value]) => ({ key, value: value == null ? null : String(value) })),
    });
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/h8/erp-interface-tables/rows") {
    const url = new URL(req.url ?? pathname, "http://wms.local");
    const filtered = filteredRows(url);
    const page = Math.max(Number(url.searchParams.get("page") ?? "1"), 1);
    const pageSize = Math.min(Math.max(Number(url.searchParams.get("page_size") ?? "50"), 1), 100);
    const start = (page - 1) * pageSize;
    sendJson(res, 200, {
      items: filtered.slice(start, start + pageSize),
      total: filtered.length,
      page,
      page_size: pageSize,
    });
    return true;
  }

  sendError(res, 405, "METHOD_NOT_ALLOWED", "接口表探查仅支持 GET");
  return true;
}
