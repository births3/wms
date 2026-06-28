import type { IncomingMessage, ServerResponse } from "node:http";
import type { Plugin } from "vite";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

const devMockEnabled = process.env.WMS_WEB_ADMIN_DEV_MOCK === "1";
const devOwnerId = "00000000-0000-0000-0000-000000000001";
const devUserId = "00000000-0000-0000-0000-000000000101";
const devOrderId = "00000000-0000-0000-0000-000000002001";
const devSalesReturnOrderId = "00000000-0000-0000-0000-000000002002";
const devWarehouseId = "00000000-0000-0000-0000-000000003001";
const devLocationId = "00000000-0000-0000-0000-000000000201";
const devLoginPassword = ["Correct", "Horse1!"].join("");
const devLoginDefaults = devMockEnabled
  ? {
      enabled: true,
      ownerCode: "PY_OWNER",
      username: "admin",
      password: devLoginPassword,
    }
  : {
      enabled: false,
      ownerCode: "",
      username: "",
      password: "",
    };

let devOrderStatus = "receiving";
let devSalesReturnOrderStatus = "receiving";

interface DevOrderLine {
  line_no: number;
  product_code: string;
  product_id: string | null;
  batch_no: string | null;
  expected_qty: number;
  production_date: string | null;
  expiry_date: string | null;
}

interface DevOrder {
  id: string;
  owner_id: string;
  receipt_no: string;
  warehouse_id: string;
  status: string;
  expected_arrival_at: string | null;
  external_ref: string | null;
  supplier_id: string | null;
  created_at: string;
  updated_at: string;
  lines: DevOrderLine[];
}

const devCreatedOrders: DevOrder[] = [];

const devUser = {
  user_id: devUserId,
  owner_id: devOwnerId,
  owner_code: "PY_OWNER",
  username: "admin",
  display_name: "Test Admin",
  roles: ["admin", "receiving"],
  permissions: ["h1.auth.me", "m2.receive", "m2.inspect", "m2.sign", "m2.putaway"],
};

function webAdminDevMock(): Plugin {
  return {
    name: "wms-web-admin-dev-mock",
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (!devMockEnabled || !req.url) {
          next();
          return;
        }

        const pathname = new URL(req.url, "http://wms.local").pathname;
        if (!pathname.startsWith("/api/v1/")) {
          next();
          return;
        }

        try {
          const handled = await handleDevMockRequest(req, res, pathname);
          if (!handled) next();
        } catch (error) {
          sendJson(res, 500, {
            code: "DEV_MOCK_ERROR",
            message: error instanceof Error ? error.message : "Dev mock failed",
            trace_id: "dev-mock",
          });
        }
      });
    },
  };
}

async function handleDevMockRequest(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (req.method === "POST" && pathname === "/api/v1/auth/login") {
    const body = await readJsonBody(req);
    const valid =
      body.owner_code === "PY_OWNER" && body.username === "admin" && body.password === devLoginPassword;

    if (!valid) {
      sendJson(res, 401, {
        code: "AUTH_INVALID_CREDENTIALS",
        message: "Login failed",
        trace_id: "dev-mock",
      });
      return true;
    }

    sendJson(res, 200, {
      access_token: `local-dev-${Date.now()}`,
      token_type: "Bearer",
      expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
      user: devUser,
    });
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/auth/me") {
    sendJson(res, 200, devUser);
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/inbound/receiving-orders") {
    const data = allDevOrders();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/inbound/receiving-orders") {
    const body = await readJsonBody(req);
    const created = devOrderFromCreateRequest(body);
    devCreatedOrders.unshift(created);
    sendJson(res, 200, created);
    return true;
  }

  const action = pathname.match(/^\/api\/v1\/inbound\/receiving-orders\/([^/]+)\/([^/]+)$/);
  if (req.method === "POST" && action && findDevOrder(action[1])) {
    await handleInboundAction(req, res, action[2], action[1]);
    return true;
  }

  const detail = pathname.match(/^\/api\/v1\/inbound\/receiving-orders\/([^/]+)$/);
  if (req.method === "GET" && detail) {
    const order = findDevOrder(detail[1]);
    if (!order) {
      sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Receiving order not found", trace_id: "dev-mock" });
      return true;
    }
    sendJson(res, 200, order);
    return true;
  }

  return false;
}

async function handleInboundAction(req: IncomingMessage, res: ServerResponse, action: string | undefined, orderId: string) {
  const body = await readJsonBody(req);
  const occurredAt = new Date().toISOString();

  if (action === "receive") {
    setDevOrderStatus(orderId, "inspecting");
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004001",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      actual_qty: asNumber(body.actual_qty, 120),
      shortage_qty: asNumber(body.shortage_qty, 0),
      rejected_qty: asNumber(body.rejected_qty, 0),
      occurred_at: occurredAt,
    });
    return;
  }

  if (action === "reject") {
    setDevOrderStatus(orderId, "closed_rejected");
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004005",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      actual_qty: 0,
      shortage_qty: 0,
      rejected_qty: devOrderExpectedQty(orderId),
      occurred_at: occurredAt,
    });
    return;
  }

  if (action === "inspect") {
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004002",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      batch_no: asString(body.batch_no, "BATCH-202606"),
      accepted_qty: asNumber(body.accepted_qty, 120),
      rejected_qty: asNumber(body.rejected_qty, 0),
      quality_status: asString(body.quality_status, "qualified"),
      occurred_at: occurredAt,
    });
    return;
  }

  if (action === "sign") {
    setDevOrderStatus(orderId, "putaway");
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004003",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      first_signer_id: asString(body.first_signer_id, devUserId),
      second_signer_id: asNullableString(body.second_signer_id),
      signed_at: occurredAt,
    });
    return;
  }

  if (action === "putaway") {
    setDevOrderStatus(orderId, "completed");
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004004",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      batch_no: asString(body.batch_no, "BATCH-202606"),
      product_code: asString(body.product_code, "P-M2-001"),
      qty: asNumber(body.qty, 120),
      location_id: asString(body.location_id, devLocationId),
      location_code: asString(body.location_code, "A-01-01"),
      occurred_at: occurredAt,
    });
    return;
  }

  sendJson(res, 404, {
    code: "DEV_MOCK_NOT_FOUND",
    message: "Dev mock route not found",
    trace_id: "dev-mock",
  });
}

function devReceivingOrder() {
  const now = new Date().toISOString();
  return {
    id: devOrderId,
    owner_id: devOwnerId,
    receipt_no: "ASN-M2-PC-0001",
    warehouse_id: devWarehouseId,
    status: devOrderStatus,
    expected_arrival_at: "2026-06-27T10:00:00.000Z",
    external_ref: "ERP-ASN-0001",
    supplier_id: null,
    created_at: "2026-06-27T08:00:00.000Z",
    updated_at: now,
    lines: [
      {
        line_no: 1,
        product_code: "P-M2-001",
        product_id: null,
        batch_no: "BATCH-202606",
        expected_qty: 120,
        production_date: "2026-01-01",
        expiry_date: "2028-01-01",
      },
    ],
  };
}

function devSalesReturnOrder() {
  const now = new Date().toISOString();
  return {
    id: devSalesReturnOrderId,
    owner_id: devOwnerId,
    receipt_no: "SR-M2-PC-0001",
    warehouse_id: devWarehouseId,
    status: devSalesReturnOrderStatus,
    expected_arrival_at: "2026-06-27T11:00:00.000Z",
    external_ref: "ERP-SR-0001",
    supplier_id: null,
    created_at: "2026-06-27T09:00:00.000Z",
    updated_at: now,
    lines: [
      {
        line_no: 1,
        product_code: "P-M2-SR-001",
        product_id: null,
        batch_no: "SR-BATCH-202606",
        expected_qty: 8,
        production_date: "2026-01-01",
        expiry_date: "2028-01-01",
      },
    ],
  };
}

function allDevOrders() {
  return [devReceivingOrder(), devSalesReturnOrder(), ...devCreatedOrders];
}

function findDevOrder(id: string) {
  if (id === devOrderId) return devReceivingOrder();
  if (id === devSalesReturnOrderId) return devSalesReturnOrder();
  return devCreatedOrders.find((order) => order.id === id) ?? null;
}

function setDevOrderStatus(id: string, status: string) {
  if (id === devOrderId) {
    devOrderStatus = status;
    return;
  }
  if (id === devSalesReturnOrderId) {
    devSalesReturnOrderStatus = status;
    return;
  }
  const order = devCreatedOrders.find((item) => item.id === id);
  if (!order) return;
  order.status = status;
  order.updated_at = new Date().toISOString();
}

function devOrderExpectedQty(id: string) {
  const order = findDevOrder(id);
  return order?.lines.reduce((total, line) => total + line.expected_qty, 0) ?? 0;
}

function devOrderFromCreateRequest(body: Record<string, unknown>): DevOrder {
  const now = new Date().toISOString();
  const lines = Array.isArray(body.lines) ? body.lines : [];
  const line = asRecord(lines[0]);
  return {
    id: crypto.randomUUID(),
    owner_id: devOwnerId,
    receipt_no: asString(body.receipt_no, `ASN-M2-PC-${Date.now()}`),
    warehouse_id: asString(body.warehouse_id, devWarehouseId),
    status: "receiving",
    expected_arrival_at: asNullableString(body.expected_arrival_at),
    external_ref: asNullableString(body.external_ref),
    supplier_id: asNullableString(body.supplier_id),
    created_at: now,
    updated_at: now,
    lines: [
      {
        line_no: asNumber(line.line_no, 1),
        product_code: asString(line.product_code, "P-M2-NEW"),
        product_id: asNullableString(line.product_id),
        batch_no: asNullableString(line.batch_no),
        expected_qty: asNumber(line.expected_qty, 1),
        production_date: asNullableString(line.production_date),
        expiry_date: asNullableString(line.expiry_date),
      },
    ],
  };
}

async function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown>> {
  let raw = "";
  for await (const chunk of req) {
    raw += String(chunk);
  }
  if (!raw) return {};
  const parsed: unknown = JSON.parse(raw);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
  const record: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(parsed)) {
    record[key] = value;
  }
  return record;
}

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const record: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value)) {
    record[key] = item;
  }
  return record;
}

function sendJson(res: ServerResponse, statusCode: number, body: unknown) {
  res.statusCode = statusCode;
  res.setHeader("content-type", "application/json; charset=utf-8");
  res.setHeader("cache-control", "no-store");
  res.end(JSON.stringify(body));
}

function asNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function asString(value: unknown, fallback: string) {
  return typeof value === "string" && value.trim() ? value.trim() : fallback;
}

function asNullableString(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

export default defineConfig({
  define: {
    __WMS_WEB_ADMIN_DEV_LOGIN__: JSON.stringify(devLoginDefaults),
  },
  plugins: [react(), webAdminDevMock()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    host: "0.0.0.0",
    port: 9002,
  },
});
