import type { IncomingMessage, ServerResponse } from "node:http";
import { randomUUID } from "node:crypto";

import { asString, readJsonBody, sendError, sendJson } from "./web-admin-dev-mock-core-common";
import { devOwnerId } from "./web-admin-dev-mock-model";

interface DevH8Message {
  id: string;
  owner_id: string;
  warehouse_id: string | null;
  connector_id: string | null;
  connector_code: string | null;
  config_version: number | null;
  direction: string;
  message_type: string;
  channel: string;
  external_ref: string;
  wms_resource_id: string | null;
  idempotency_key: string;
  correlation_id: string;
  sync_status: string;
  retry_count: number;
  next_retry_at: string | null;
  last_error_summary: string | null;
  payload_digest: string;
  claimed_by: string | null;
  lease_expires_at: string | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  acked_at: string | null;
  schema_version: string;
}

interface DevAttempt {
  id: string;
  message_id: string;
  attempt_no: number;
  channel: string;
  started_at: string;
  finished_at: string | null;
  result: string;
  error_summary: string | null;
  actor: string;
}

const now = "2026-07-19T08:00:00.000Z";

const messages: DevH8Message[] = [
  {
    id: "00000000-0000-0000-0000-00000000m001",
    owner_id: devOwnerId,
    warehouse_id: "00000000-0000-0000-0000-000000000801",
    connector_id: "00000000-0000-0000-0000-00000000e801",
    connector_code: "demo-rest-erp",
    config_version: 1,
    direction: "inbound",
    message_type: "asn",
    channel: "rest",
    external_ref: "ERP-ASN-FAIL-1",
    wms_resource_id: null,
    idempotency_key: "idem-fail-1",
    correlation_id: "corr-fail-1",
    sync_status: "failed",
    retry_count: 3,
    next_retry_at: null,
    last_error_summary: "mapping: unit not found",
    payload_digest: "digest-fail",
    claimed_by: null,
    lease_expires_at: null,
    created_at: now,
    updated_at: now,
    completed_at: null,
    acked_at: null,
    schema_version: "1",
  },
  {
    id: "00000000-0000-0000-0000-00000000m002",
    owner_id: devOwnerId,
    warehouse_id: "00000000-0000-0000-0000-000000000801",
    connector_id: "00000000-0000-0000-0000-00000000e801",
    connector_code: "demo-rest-erp",
    config_version: 1,
    direction: "inbound",
    message_type: "asn",
    channel: "rest",
    external_ref: "ERP-ASN-DEAD-1",
    wms_resource_id: null,
    idempotency_key: "idem-dead-1",
    correlation_id: "corr-dead-1",
    sync_status: "dead",
    retry_count: 5,
    next_retry_at: null,
    last_error_summary: "auth: invalid api key",
    payload_digest: "digest-dead",
    claimed_by: null,
    lease_expires_at: null,
    created_at: now,
    updated_at: now,
    completed_at: null,
    acked_at: null,
    schema_version: "1",
  },
];

const attempts: DevAttempt[] = [
  {
    id: randomUUID(),
    message_id: "00000000-0000-0000-0000-00000000m002",
    attempt_no: 1,
    channel: "rest",
    started_at: now,
    finished_at: now,
    result: "failed",
    error_summary: "auth: invalid api key",
    actor: "worker",
  },
];

const worker = {
  worker_id: "h8-worker-demo-01",
  worker_version: "1.0.0",
  connector_id: "00000000-0000-0000-0000-00000000e801",
  directions: ["inbound", "outbound"],
  current_claims: 1,
  created_at: now,
  last_heartbeat_at: now,
  heartbeat_expires_at: "2099-07-19T08:00:15.000Z",
  health: "healthy",
};

let workerControls: Array<{
  connector_id: string;
  direction: string;
  paused: boolean;
  reason: string;
  paused_until: string | null;
  updated_by: string;
  updated_at: string;
}> = [];

let payloadPolicy = {
  connector_id: worker.connector_id,
  enabled: false,
  retention_days: 7,
  updated_at: now,
};

function parsePath(pathname: string): { id: string; action: string } | null {
  const match = pathname.match(/^\/api\/v1\/integration\/erp-messages\/([^/]+)(?:\/(replay))?$/);
  if (!match) return null;
  return { id: match[1] ?? "", action: match[2] ?? "" };
}

export async function handleH8ErpMessageDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (
    pathname === "/api/v1/integration/erp-messages/payload-retention" &&
    req.method === "GET"
  ) {
    sendJson(res, 200, [payloadPolicy]);
    return true;
  }

  if (
    pathname === "/api/v1/integration/erp-messages/payload-retention" &&
    req.method === "POST"
  ) {
    const body = (await readJsonBody(req)) as {
      connector_id?: string;
      enabled?: boolean;
      retention_days?: number;
      confirmed?: boolean;
    };
    if (!body.confirmed || (body.retention_days ?? 7) < 1 || (body.retention_days ?? 7) > 30) {
      sendError(res, 400, "H8-400", "invalid payload retention policy");
      return true;
    }
    payloadPolicy = {
      connector_id: asString(body.connector_id, worker.connector_id),
      enabled: Boolean(body.enabled),
      retention_days: body.retention_days ?? 7,
      updated_at: new Date().toISOString(),
    };
    sendJson(res, 200, payloadPolicy);
    return true;
  }

  if (pathname === "/api/v1/integration/erp-messages/worker-runtime" && req.method === "GET") {
    sendJson(res, 200, { workers: [worker], controls: workerControls });
    return true;
  }

  if (
    pathname === "/api/v1/integration/erp-messages/worker-runtime/control" &&
    req.method === "POST"
  ) {
    const body = (await readJsonBody(req)) as {
      connector_id?: string;
      direction?: string;
      paused?: boolean;
      reason?: string;
      paused_until?: string | null;
      confirmed?: boolean;
    };
    if (!body.confirmed || !asString(body.reason, "").trim()) {
      sendError(res, 400, "H8-400", "reason and confirmation required");
      return true;
    }
    const control = {
      connector_id: asString(body.connector_id, ""),
      direction: asString(body.direction, ""),
      paused: Boolean(body.paused),
      reason: asString(body.reason, "").trim(),
      paused_until: body.paused_until ?? null,
      updated_by: "admin",
      updated_at: new Date().toISOString(),
    };
    workerControls = workerControls.filter(
      (item) =>
        item.connector_id !== control.connector_id || item.direction !== control.direction,
    );
    workerControls.push(control);
    sendJson(res, 200, control);
    return true;
  }

  if (
    pathname === "/api/v1/integration/erp-messages/worker-runtime/claim-decision" &&
    req.method === "GET"
  ) {
    const url = new URL(req.url ?? "", "http://localhost");
    const control = workerControls.find(
      (item) =>
        item.connector_id === url.searchParams.get("connector_id") &&
        item.direction === url.searchParams.get("direction"),
    );
    sendJson(res, 200, {
      allowed: !control?.paused,
      reason: control?.paused ? control.reason : null,
      paused_until: control?.paused_until ?? null,
    });
    return true;
  }

  if (
    pathname === "/api/v1/integration/erp-messages/worker-runtime/heartbeat" &&
    req.method === "POST"
  ) {
    sendJson(res, 200, worker);
    return true;
  }

  if (pathname === "/api/v1/integration/erp-messages/stats" && req.method === "GET") {
    const url = new URL(req.url ?? "", "http://localhost");
    const connectorCode = url.searchParams.get("connector_code");
    const channel = url.searchParams.get("channel");
    const messageType = url.searchParams.get("message_type");
    const rows = messages.filter(
      (message) =>
        (!connectorCode || message.connector_code === connectorCode) &&
        (!channel || message.channel === channel) &&
        (!messageType || message.message_type === messageType),
    );
    sendJson(res, 200, {
      owner_id: devOwnerId,
      total: rows.length,
      succeeded: rows.filter((m) => m.sync_status === "succeeded" || m.sync_status === "acked")
        .length,
      failed: rows.filter((m) => m.sync_status === "failed").length,
      dead: rows.filter((m) => m.sync_status === "dead").length,
      processing: rows.filter((m) => m.sync_status === "processing").length,
      pending: rows.filter((m) => m.sync_status === "pending").length,
      retry_total: rows.reduce((sum, m) => sum + m.retry_count, 0),
      p95_latency_ms: 120,
    });
    return true;
  }

  if (pathname === "/api/v1/integration/erp-messages" && req.method === "GET") {
    const url = new URL(req.url ?? "", "http://localhost");
    let rows = [...messages];
    const direction = url.searchParams.get("direction");
    const messageType = url.searchParams.get("message_type");
    const status = url.searchParams.get("status");
    const connectorCode = url.searchParams.get("connector_code");
    const channel = url.searchParams.get("channel");
    const warehouseId = url.searchParams.get("warehouse_id");
    const externalRef = url.searchParams.get("external_ref");
    const idempotencyKey = url.searchParams.get("idempotency_key");
    const correlationId = url.searchParams.get("correlation_id");
    const createdFrom = url.searchParams.get("created_from");
    const createdTo = url.searchParams.get("created_to");
    if (direction) rows = rows.filter((m) => m.direction === direction);
    if (messageType) rows = rows.filter((m) => m.message_type === messageType);
    if (status) rows = rows.filter((m) => m.sync_status === status);
    if (connectorCode) rows = rows.filter((m) => m.connector_code === connectorCode);
    if (channel) rows = rows.filter((m) => m.channel === channel);
    if (warehouseId) rows = rows.filter((m) => m.warehouse_id === warehouseId);
    if (externalRef) rows = rows.filter((m) => m.external_ref === externalRef);
    if (idempotencyKey) rows = rows.filter((m) => m.idempotency_key === idempotencyKey);
    if (correlationId) rows = rows.filter((m) => m.correlation_id === correlationId);
    if (createdFrom) rows = rows.filter((m) => m.created_at >= createdFrom);
    if (createdTo) rows = rows.filter((m) => m.created_at <= createdTo);
    sendJson(res, 200, {
      data: rows,
      page: { next_cursor: null, count: rows.length },
    });
    return true;
  }

  const payloadMatch = pathname.match(
    /^\/api\/v1\/integration\/erp-messages\/([^/]+)\/payload$/,
  );
  if (payloadMatch && req.method === "GET") {
    if (!payloadPolicy.enabled) {
      sendError(res, 404, "H8-404", "retained payload unavailable");
      return true;
    }
    sendJson(res, 200, {
      message_id: payloadMatch[1],
      payload: JSON.stringify({ external_ref: "ERP-ASN-DEAD-1", qty: 1 }),
      expires_at: "2099-07-26T08:00:00.000Z",
    });
    return true;
  }

  const parsed = parsePath(pathname);
  if (!parsed || parsed.id === "stats") {
    return false;
  }

  const msg = messages.find((m) => m.id === parsed.id);
  if (!msg) {
    sendError(res, 404, "H8-404", "message not found");
    return true;
  }

  if (req.method === "GET" && !parsed.action) {
    sendJson(res, 200, {
      message: msg,
      attempts: attempts.filter((a) => a.message_id === msg.id),
      payload_retained: payloadPolicy.enabled,
      payload_expires_at: payloadPolicy.enabled ? "2099-07-26T08:00:00.000Z" : null,
    });
    return true;
  }

  if (req.method === "POST" && parsed.action === "replay") {
    const body = (await readJsonBody(req)) as { reason?: string; confirmed?: boolean };
    if (!body.confirmed) {
      sendError(res, 400, "H8-400", "confirmed must be true");
      return true;
    }
    if (!asString(body.reason, "").trim()) {
      sendError(res, 400, "H8-400", "reason required");
      return true;
    }
    if (msg.sync_status !== "failed" && msg.sync_status !== "dead") {
      sendError(res, 409, "H8-409", "only failed/dead messages may be replayed");
      return true;
    }
    const prev = msg.sync_status;
    msg.sync_status = "processing";
    msg.updated_at = new Date().toISOString();
    msg.last_error_summary = `replay: ${String(body.reason).trim()}`;
    attempts.push({
      id: randomUUID(),
      message_id: msg.id,
      attempt_no: attempts.filter((a) => a.message_id === msg.id).length + 1,
      channel: msg.channel,
      started_at: msg.updated_at,
      finished_at: msg.updated_at,
      result: "replayed",
      error_summary: `from ${prev}`,
      actor: "admin",
    });
    sendJson(res, 200, msg);
    return true;
  }

  return false;
}
