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
    warehouse_id: null,
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
  },
  {
    id: "00000000-0000-0000-0000-00000000m002",
    owner_id: devOwnerId,
    warehouse_id: null,
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
  if (pathname === "/api/v1/integration/erp-messages/stats" && req.method === "GET") {
    sendJson(res, 200, {
      owner_id: devOwnerId,
      total: messages.length,
      succeeded: messages.filter((m) => m.sync_status === "succeeded" || m.sync_status === "acked")
        .length,
      failed: messages.filter((m) => m.sync_status === "failed").length,
      dead: messages.filter((m) => m.sync_status === "dead").length,
      processing: messages.filter((m) => m.sync_status === "processing").length,
      pending: messages.filter((m) => m.sync_status === "pending").length,
      retry_total: messages.reduce((sum, m) => sum + m.retry_count, 0),
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
    if (direction) rows = rows.filter((m) => m.direction === direction);
    if (messageType) rows = rows.filter((m) => m.message_type === messageType);
    if (status) rows = rows.filter((m) => m.sync_status === status);
    sendJson(res, 200, {
      data: rows,
      page: { next_cursor: null, count: rows.length },
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
