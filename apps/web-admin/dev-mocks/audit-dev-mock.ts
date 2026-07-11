import type { IncomingMessage, ServerResponse } from "node:http";

import { sendJson } from "./web-admin-dev-mock-core-common";

const devOwnerId = "00000000-0000-0000-0000-000000000001";
const actors = {
  zhang: {
    actor_id: "00000000-0000-0000-0000-000000000101",
    actor_name: "张三",
    jti: "jti-dev-zhang",
    owner_id: devOwnerId,
  },
  li: {
    actor_id: "00000000-0000-0000-0000-000000000102",
    actor_name: "李四",
    jti: "jti-dev-li",
    owner_id: devOwnerId,
  },
  wang: {
    actor_id: "00000000-0000-0000-0000-000000000103",
    actor_name: "王五",
    jti: "jti-dev-wang",
    owner_id: devOwnerId,
  },
  zhao: {
    actor_id: "00000000-0000-0000-0000-000000000104",
    actor_name: "赵六",
    jti: "jti-dev-zhao",
    owner_id: devOwnerId,
  },
} as const;

const mockAuditEvents = [
  {
    id: 10001,
    occurred_at: "2026-05-22T01:14:23.000Z",
    actor: actors.zhang,
    action: "验收提交",
    resource_type: "receiving_order",
    resource_id: "PO-2026-0001",
    owner_id: devOwnerId,
    trace_id: "tr-h2-0001",
    diff: { status: { before: "验收中", after: "已验收" } },
  },
  {
    id: 10002,
    occurred_at: "2026-05-22T01:18:11.000Z",
    actor: actors.li,
    action: "双人复核签字",
    resource_type: "receiving_order",
    resource_id: "PO-2026-0001",
    owner_id: devOwnerId,
    trace_id: "tr-h2-0002",
    diff: { reviewer: { before: null, after: "u002" } },
  },
  {
    id: 10003,
    occurred_at: "2026-05-22T02:02:55.000Z",
    actor: actors.wang,
    action: "库位移动",
    resource_type: "pallet",
    resource_id: "LPN-001234",
    owner_id: devOwnerId,
    trace_id: "tr-h2-0003",
    diff: { location: { before: "A-01-01", after: "B-02-03" } },
  },
  {
    id: 10004,
    occurred_at: "2026-05-22T02:15:42.000Z",
    actor: actors.zhao,
    action: "批号调整驳回",
    resource_type: "batch_adjustment",
    resource_id: "BA-2026-0008",
    owner_id: devOwnerId,
    trace_id: "tr-h2-0004",
    diff: { status: { before: "待审批", after: "驳回" }, reason: "未提供调整原因" },
  },
  {
    id: 10005,
    occurred_at: "2026-05-22T03:30:08.000Z",
    actor: actors.zhang,
    action: "出库复核",
    resource_type: "shipping_order",
    resource_id: "SO-2026-0042",
    owner_id: devOwnerId,
    trace_id: "tr-h2-0005",
    diff: { status: { before: "复核中", after: "已复核" } },
  },
  {
    id: 10006,
    occurred_at: "2026-05-22T04:05:19.000Z",
    actor: actors.li,
    action: "登录成功",
    resource_type: "auth_session",
    resource_id: "sess-dev-li",
    owner_id: devOwnerId,
    trace_id: "tr-h2-0006",
    diff: { channel: "web-admin" },
  },
];

export async function handleAuditDevMock(req: IncomingMessage, res: ServerResponse, pathname: string) {
  if (req.method === "GET" && pathname === "/api/v1/audit/events") {
    const url = new URL(req.url ?? "/", "http://wms.local");
    const resourceType = url.searchParams.get("resource_type")?.trim() ?? "";
    const actorId = url.searchParams.get("actor_id")?.trim() ?? "";
    const from = url.searchParams.get("from")?.trim() ?? "";
    const to = url.searchParams.get("to")?.trim() ?? "";
    const limitRaw = Number(url.searchParams.get("limit") ?? "100");
    const limit = Number.isFinite(limitRaw) && limitRaw > 0 ? Math.min(limitRaw, 200) : 100;

    const data = mockAuditEvents
      .filter((event) => {
        if (resourceType && event.resource_type !== resourceType) return false;
        if (actorId && event.actor.actor_id !== actorId) return false;
        if (from && event.occurred_at < from) return false;
        if (to && event.occurred_at > to) return false;
        return true;
      })
      .slice(0, limit);

    sendJson(res, 200, { data, next_cursor: null });
    return true;
  }

  return false;
}
