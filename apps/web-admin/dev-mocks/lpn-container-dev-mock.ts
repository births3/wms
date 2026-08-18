import type { IncomingMessage, ServerResponse } from "node:http";

import { readJsonBody, sendError, sendJson } from "./web-admin-dev-mock-core-common";
import { devLpnContainers, devLpnTypePolicies, devOwnerId } from "./web-admin-dev-mock-model";

const LPN_BATCH_CREATE_MAX_COUNT = 100;

function matchId(pathname: string, prefix: string): string | null {
  if (!pathname.startsWith(prefix)) return null;
  const id = decodeURIComponent(pathname.slice(prefix.length));
  return id && !id.includes("/") ? id : null;
}

function newIdleContainer(containerType: string, capacityCm3: number | null, suffix?: string) {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    owner_id: devOwnerId,
    lpn_code: suffix ? `LPN-DEV-${Date.now()}-${suffix}` : `LPN-DEV-${Date.now()}`,
    container_type: containerType,
    capacity_cm3: capacityCm3,
    status: "idle",
    current_lock_category: "qualified",
    location_id: null,
    created_at: now,
    updated_at: now,
  };
}

export async function handleLpnContainerDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (
    pathname !== "/api/v1/master-data/lpn-containers"
    && pathname !== "/api/v1/master-data/lpn-container-type-policies"
    && !pathname.startsWith("/api/v1/master-data/lpn-containers/")
  ) {
    return false;
  }

  if (pathname === "/api/v1/master-data/lpn-containers" && req.method === "GET") {
    const status = new URL(req.url ?? pathname, "http://wms.local").searchParams.get("status");
    const data = devLpnContainers.filter((item) =>
      status ? item.status === status : item.status !== "disabled",
    );
    sendJson(res, 200, { data });
    return true;
  }
  if (pathname === "/api/v1/master-data/lpn-containers" && req.method === "POST") {
    const body = (await readJsonBody(req)) as { container_type?: string; capacity_cm3?: number | null };
    const next = newIdleContainer(body.container_type ?? "pallet", body.capacity_cm3 ?? null);
    devLpnContainers.unshift(next);
    sendJson(res, 200, next);
    return true;
  }
  if (pathname === "/api/v1/master-data/lpn-containers/batch-create" && req.method === "POST") {
    const body = (await readJsonBody(req)) as {
      container_type?: string;
      capacity_cm3?: number | null;
      count?: number;
    };
    const count = body.count ?? 0;
    if (!Number.isInteger(count) || count < 1 || count > LPN_BATCH_CREATE_MAX_COUNT) {
      sendError(res, 422, "M1_LPN_BATCH_COUNT_INVALID", "批量新增数量必须为 1-100");
      return true;
    }
    const created = Array.from({ length: count }, (_, index) =>
      newIdleContainer(body.container_type ?? "pallet", body.capacity_cm3 ?? null, String(index + 1)),
    );
    devLpnContainers.unshift(...created);
    sendJson(res, 200, { data: created });
    return true;
  }
  const qualityLock = pathname.match(/^\/api\/v1\/master-data\/lpn-containers\/([^/]+)\/quality-lock(\/release)?$/);
  if (qualityLock) {
    const found = devLpnContainers.find((item) => item.id === qualityLock[1]);
    if (!found) {
      sendError(res, 404, "M1_LPN_NOT_FOUND", "LPN 容器不存在");
      return true;
    }
    const body = (await readJsonBody(req)) as {
      lock_category?: string;
      witness_id?: string;
      reason_dict_item_code?: string;
    };
    if (!body.witness_id) {
      sendError(res, 422, "M1_QUALITY_LOCK_WITNESS_INVALID", "见证人缺失或与操作人相同");
      return true;
    }
    if (qualityLock[2] === "/release") {
      found.current_lock_category = "qualified";
    } else {
      found.current_lock_category = body.lock_category ?? found.current_lock_category ?? "quarantine";
    }
    found.updated_at = new Date().toISOString();
    sendJson(res, 200, found);
    return true;
  }
  const lpnId = matchId(pathname, "/api/v1/master-data/lpn-containers/");
  if (lpnId && req.method === "GET") {
    const found = devLpnContainers.find((item) => item.id === lpnId);
    if (!found) {
      sendError(res, 404, "M1_LPN_NOT_FOUND", "LPN 容器不存在");
      return true;
    }
    sendJson(res, 200, found);
    return true;
  }
  if (lpnId && req.method === "PATCH") {
    const found = devLpnContainers.find((item) => item.id === lpnId);
    if (!found || found.status === "disabled") {
      sendError(res, 404, "M1_LPN_NOT_FOUND", "LPN 容器不存在");
      return true;
    }
    const body = (await readJsonBody(req)) as { capacity_cm3?: number | null; status?: string };
    if (body.status === "disabled") {
      sendError(res, 422, "M1_LPN_STATUS_INVALID", "LPN 容器状态非法");
      return true;
    }
    if (body.capacity_cm3 !== undefined) found.capacity_cm3 = body.capacity_cm3;
    if (body.status) found.status = body.status;
    found.updated_at = new Date().toISOString();
    sendJson(res, 200, found);
    return true;
  }
  if (lpnId && req.method === "DELETE") {
    const found = devLpnContainers.find((item) => item.id === lpnId);
    if (!found) {
      sendError(res, 404, "M1_LPN_NOT_FOUND", "LPN 容器不存在");
      return true;
    }
    if (found.status !== "idle" && found.status !== "disabled") {
      sendError(res, 422, "M1_LPN_NOT_DELETABLE", "在用或作业中的容器不能删除，请先解绑");
      return true;
    }
    found.status = "disabled";
    found.updated_at = new Date().toISOString();
    sendJson(res, 200, found);
    return true;
  }
  if (pathname === "/api/v1/master-data/lpn-container-type-policies" && req.method === "GET") {
    sendJson(res, 200, devLpnTypePolicies);
    return true;
  }
  if (pathname === "/api/v1/master-data/lpn-container-type-policies" && req.method === "PUT") {
    const body = (await readJsonBody(req)) as {
      container_type: string;
      allow_mix_batch: boolean;
      allow_mix_sku: boolean;
    };
    const next = { owner_id: devOwnerId, ...body };
    const existing = devLpnTypePolicies.findIndex((item) => item.container_type === body.container_type);
    if (existing >= 0) devLpnTypePolicies[existing] = next;
    else devLpnTypePolicies.unshift(next);
    sendJson(res, 200, next);
    return true;
  }
  return false;
}
