import type { IncomingMessage, ServerResponse } from "node:http";
import { randomUUID } from "node:crypto";

import { asString, readJsonBody, sendError, sendJson } from "./web-admin-dev-mock-core-common";
import { devOwnerId } from "./web-admin-dev-mock-model";

interface DevH8ErpConnector {
  id: string;
  owner_id: string;
  connector_code: string;
  connector_name: string;
  warehouse_ids: string[];
  directions: string[];
  message_types: string[];
  channel_mode: string;
  api_base_url: string | null;
  api_key_id: string | null;
  bearer_secret_alias: string | null;
  interface_db_host: string | null;
  interface_db_port: number | null;
  interface_db_name: string | null;
  interface_db_username: string | null;
  interface_db_password_alias: string | null;
  status: string;
  config_version: number;
  last_tested_at: string | null;
  last_tested_succeeded: boolean | null;
  last_tested_error_summary: string | null;
  last_tested_version: number | null;
  first_activated_at: string | null;
  created_at: string;
  updated_at: string;
}

const connectors: DevH8ErpConnector[] = [
  {
    id: "00000000-0000-0000-0000-00000000e801",
    owner_id: devOwnerId,
    connector_code: "demo-rest-erp",
    connector_name: "示例 REST ERP",
    warehouse_ids: [],
    directions: ["inbound", "outbound"],
    message_types: ["asn", "so"],
    channel_mode: "rest",
    api_base_url: "https://erp.example.test/api",
    api_key_id: null,
    bearer_secret_alias: "vault://wms/dev/h8/bearer",
    interface_db_host: null,
    interface_db_port: null,
    interface_db_name: null,
    interface_db_username: null,
    interface_db_password_alias: null,
    status: "testing",
    config_version: 1,
    last_tested_at: null,
    last_tested_succeeded: null,
    last_tested_error_summary: null,
    last_tested_version: null,
    first_activated_at: null,
    created_at: "2026-07-19T00:00:00.000Z",
    updated_at: "2026-07-19T00:00:00.000Z",
  },
];

function nowIso(): string {
  return new Date().toISOString();
}

function findById(id: string): DevH8ErpConnector | undefined {
  return connectors.find((row) => row.id === id);
}

function parseActionPath(pathname: string): { id: string; action: string } | null {
  const match = pathname.match(
    /^\/api\/v1\/config\/erp-connectors\/([^/]+)(?:\/(test|activate|disable))?$/,
  );
  if (!match) return null;
  return { id: match[1] ?? "", action: match[2] ?? "" };
}

export async function handleH8ErpConnectorDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (!pathname.startsWith("/api/v1/config/erp-connectors")) {
    return false;
  }

  if (req.method === "GET" && pathname === "/api/v1/config/erp-connectors") {
    sendJson(res, 200, {
      data: connectors,
      page: { count: connectors.length, next_cursor: null },
    });
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/config/erp-connectors") {
    const body = await readJsonBody(req);
    const code = asString(body.connector_code, "").trim();
    const name = asString(body.connector_name, "").trim();
    if (!code || !name) {
      sendError(res, 422, "DEV_MOCK_REQUEST_INVALID", "连接编码与名称必填");
      return true;
    }
    if (connectors.some((row) => row.connector_code === code)) {
      sendError(res, 409, "H8_ERP_CONNECTOR_CODE_EXISTS", "连接编码已存在");
      return true;
    }
    const ts = nowIso();
    const row: DevH8ErpConnector = {
      id: randomUUID(),
      owner_id: devOwnerId,
      connector_code: code,
      connector_name: name,
      warehouse_ids: Array.isArray(body.warehouse_ids)
        ? body.warehouse_ids.map((item) => String(item))
        : [],
      directions: Array.isArray(body.directions) ? body.directions.map((item) => String(item)) : ["inbound"],
      message_types: Array.isArray(body.message_types)
        ? body.message_types.map((item) => String(item))
        : ["asn"],
      channel_mode: asString(body.channel_mode, "rest") || "rest",
      api_base_url: body.api_base_url == null ? null : asString(body.api_base_url, ""),
      api_key_id: body.api_key_id == null ? null : asString(body.api_key_id, ""),
      bearer_secret_alias:
        body.bearer_secret_alias == null ? null : asString(body.bearer_secret_alias, ""),
      interface_db_host: body.interface_db_host == null ? null : asString(body.interface_db_host, ""),
      interface_db_port:
        typeof body.interface_db_port === "number" ? body.interface_db_port : null,
      interface_db_name: body.interface_db_name == null ? null : asString(body.interface_db_name, ""),
      interface_db_username:
        body.interface_db_username == null ? null : asString(body.interface_db_username, ""),
      interface_db_password_alias:
        body.interface_db_password_alias == null
          ? null
          : asString(body.interface_db_password_alias, ""),
      status: "testing",
      config_version: 1,
      last_tested_at: null,
      last_tested_succeeded: null,
      last_tested_error_summary: null,
      last_tested_version: null,
      first_activated_at: null,
      created_at: ts,
      updated_at: ts,
    };
    connectors.unshift(row);
    sendJson(res, 201, row);
    return true;
  }

  const parsed = parseActionPath(pathname);
  if (!parsed) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "H8 ERP connector mock route not found");
    return true;
  }

  const row = findById(parsed.id);
  if (!row) {
    sendError(res, 404, "H8_ERP_CONNECTOR_NOT_FOUND", "连接不存在");
    return true;
  }

  if (req.method === "GET" && !parsed.action) {
    sendJson(res, 200, row);
    return true;
  }

  if (req.method === "PATCH" && !parsed.action) {
    const body = await readJsonBody(req);
    const expected = Number(body.expected_config_version);
    if (!Number.isFinite(expected) || expected !== row.config_version) {
      sendError(res, 409, "H8_ERP_CONNECTOR_VERSION_CONFLICT", "config_version conflict");
      return true;
    }
    if (typeof body.connector_name === "string" && body.connector_name.trim()) {
      row.connector_name = body.connector_name.trim();
    }
    const runtimeAffecting =
      body.channel_mode != null ||
      body.api_base_url !== undefined ||
      body.bearer_secret_alias !== undefined ||
      body.warehouse_ids != null ||
      body.directions != null ||
      body.message_types != null;
    if (typeof body.channel_mode === "string" && body.channel_mode.trim()) {
      row.channel_mode = body.channel_mode.trim();
    }
    if (body.api_base_url !== undefined) {
      row.api_base_url = body.api_base_url == null ? null : asString(body.api_base_url, "");
    }
    if (body.bearer_secret_alias !== undefined) {
      row.bearer_secret_alias =
        body.bearer_secret_alias == null ? null : asString(body.bearer_secret_alias, "");
    }
    if (runtimeAffecting) {
      row.config_version += 1;
      row.last_tested_at = null;
      row.last_tested_succeeded = null;
      row.last_tested_error_summary = null;
      row.last_tested_version = null;
      if (row.status === "active") row.status = "testing";
    }
    row.updated_at = nowIso();
    sendJson(res, 200, row);
    return true;
  }

  if (req.method === "DELETE" && !parsed.action) {
    const idx = connectors.findIndex((item) => item.id === row.id);
    if (idx >= 0) connectors.splice(idx, 1);
    res.statusCode = 204;
    res.end();
    return true;
  }

  if (req.method === "POST" && parsed.action === "test") {
    const testedAt = nowIso();
    row.last_tested_at = testedAt;
    row.last_tested_succeeded = true;
    row.last_tested_error_summary = null;
    row.last_tested_version = row.config_version;
    row.updated_at = testedAt;
    sendJson(res, 200, {
      succeeded: true,
      tested_at: testedAt,
      tested_version: row.config_version,
      error_summary: null,
    });
    return true;
  }

  if (req.method === "POST" && parsed.action === "activate") {
    if (row.last_tested_succeeded !== true) {
      sendError(res, 422, "H8_ERP_CONNECTOR_NOT_TESTED", "请先测试通过再启用");
      return true;
    }
    const activeSameRoute = connectors.find(
      (item) =>
        item.id !== row.id &&
        item.status === "active" &&
        item.channel_mode === row.channel_mode &&
        JSON.stringify(item.warehouse_ids) === JSON.stringify(row.warehouse_ids) &&
        JSON.stringify(item.directions) === JSON.stringify(row.directions) &&
        JSON.stringify(item.message_types) === JSON.stringify(row.message_types),
    );
    if (activeSameRoute) {
      sendError(res, 409, "H8_ERP_CONNECTOR_ROUTE_OVERLAP", "route overlap：同路由已有启用连接");
      return true;
    }
    const ts = nowIso();
    row.status = "active";
    row.first_activated_at = row.first_activated_at ?? ts;
    row.updated_at = ts;
    sendJson(res, 200, row);
    return true;
  }

  if (req.method === "POST" && parsed.action === "disable") {
    row.status = "disabled";
    row.updated_at = nowIso();
    sendJson(res, 200, row);
    return true;
  }

  sendError(res, 404, "DEV_MOCK_NOT_FOUND", "H8 ERP connector mock route not found");
  return true;
}
