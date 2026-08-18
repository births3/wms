import type { IncomingMessage, ServerResponse } from "node:http";

import { readJsonBody, sendError, sendJson } from "./web-admin-dev-mock-core-common";

const OWNER_ID = "00000000-0000-0000-0000-000000000001";

type DevStrategy = {
  id: string;
  owner_id: string;
  strategy_code: string;
  strategy_name: string;
  scope_type: string;
  scope_ref: string;
  location_type: string;
  source_type: string;
  target_type: string;
  min_safety_threshold: string;
  max_replenish_target: string;
  trigger_modes: string[];
  enabled: boolean;
};

type DevGroup = {
  id: string;
  owner_id: string;
  group_code: string;
  group_name: string;
  enabled: boolean;
  location_ids: string[];
};

const strategies: DevStrategy[] = [
  {
    id: "00000000-0000-0000-0000-00000000b001",
    owner_id: OWNER_ID,
    strategy_code: "MM-CASE-01",
    strategy_name: "整箱 Min-Max",
    scope_type: "product",
    scope_ref: "00000000-0000-0000-0000-00000000c001",
    location_type: "case_pick",
    source_type: "storage",
    target_type: "case_pick",
    min_safety_threshold: "10",
    max_replenish_target: "50",
    trigger_modes: ["min_max"],
    enabled: true,
  },
];
const groups: DevGroup[] = [];
const bindings = new Map<string, string[]>();

function page(data: unknown[]) {
  return { data, page: { count: data.length, next_cursor: null } };
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" ? (value as Record<string, unknown>) : {};
}

export async function handleReplenishmentDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (!pathname.startsWith("/api/v1/replenishment/")) {
    return false;
  }

  if (pathname === "/api/v1/replenishment/strategies" && req.method === "GET") {
    sendJson(res, 200, page(strategies));
    return true;
  }
  if (pathname === "/api/v1/replenishment/strategies" && req.method === "POST") {
    const body = asRecord(await readJsonBody(req));
    const created: DevStrategy = {
      id: crypto.randomUUID(),
      owner_id: OWNER_ID,
      strategy_code: String(body.strategy_code ?? ""),
      strategy_name: String(body.strategy_name ?? ""),
      scope_type: String(body.scope_type ?? "product"),
      scope_ref: String(body.scope_ref ?? ""),
      location_type: String(body.target_type ?? "case_pick"),
      source_type: String(body.source_type ?? "storage"),
      target_type: String(body.target_type ?? "case_pick"),
      min_safety_threshold: String(body.min_safety_threshold ?? "0"),
      max_replenish_target: String(body.max_replenish_target ?? "1"),
      trigger_modes: Array.isArray(body.trigger_modes) ? body.trigger_modes.map(String) : ["min_max"],
      enabled: body.enabled !== false,
    };
    strategies.unshift(created);
    sendJson(res, 200, created);
    return true;
  }

  const strategyMatch = pathname.match(/^\/api\/v1\/replenishment\/strategies\/([^/]+)(?:\/(preview|locations|disable))?$/);
  if (strategyMatch) {
    const strategy = strategies.find((item) => item.id === decodeURIComponent(strategyMatch[1]));
    if (!strategy) {
      sendError(res, 404, "M3_REPLENISH_TASK_NOT_FOUND", "补货策略不存在");
      return true;
    }
    const action = strategyMatch[2];
    if (!action && req.method === "GET") {
      sendJson(res, 200, strategy);
      return true;
    }
    if (!action && req.method === "PUT") {
      const body = asRecord(await readJsonBody(req));
      Object.assign(strategy, {
        strategy_name: String(body.strategy_name ?? strategy.strategy_name),
        scope_type: String(body.scope_type ?? strategy.scope_type),
        scope_ref: String(body.scope_ref ?? strategy.scope_ref),
        location_type: String(body.target_type ?? strategy.target_type),
        source_type: String(body.source_type ?? strategy.source_type),
        target_type: String(body.target_type ?? strategy.target_type),
        min_safety_threshold: String(body.min_safety_threshold ?? strategy.min_safety_threshold),
        max_replenish_target: String(body.max_replenish_target ?? strategy.max_replenish_target),
        trigger_modes: Array.isArray(body.trigger_modes) ? body.trigger_modes.map(String) : strategy.trigger_modes,
        enabled: body.enabled !== false,
      });
      sendJson(res, 200, strategy);
      return true;
    }
    if (action === "disable" && req.method === "POST") {
      strategy.enabled = false;
      sendJson(res, 200, strategy);
      return true;
    }
    if (action === "preview" && req.method === "GET") {
      const locationIds = bindings.get(strategy.id) ?? [];
      sendJson(res, 200, {
        data: locationIds.map((locationId, index) => ({
          location_id: locationId,
          location_code: `LOC-${index + 1}`,
          product_id: strategy.scope_type === "product" ? strategy.scope_ref : null,
          available_qty: "0",
          min_safety_threshold: strategy.min_safety_threshold,
          max_replenish_target: strategy.max_replenish_target,
          would_trigger: true,
        })),
      });
      return true;
    }
    if (action === "locations" && req.method === "PUT") {
      const body = asRecord(await readJsonBody(req));
      const locationIds = Array.isArray(body.location_ids) ? body.location_ids.map(String) : [];
      bindings.set(strategy.id, locationIds);
      sendJson(res, 200, { strategy_id: strategy.id, location_ids: locationIds });
      return true;
    }
  }

  if (pathname === "/api/v1/replenishment/location-groups" && req.method === "GET") {
    sendJson(res, 200, page(groups));
    return true;
  }
  if (pathname === "/api/v1/replenishment/location-groups" && req.method === "POST") {
    const body = asRecord(await readJsonBody(req));
    const existing = groups.find((item) => item.group_code === String(body.group_code ?? ""));
    const next: DevGroup = {
      id: existing?.id ?? crypto.randomUUID(),
      owner_id: OWNER_ID,
      group_code: String(body.group_code ?? ""),
      group_name: String(body.group_name ?? ""),
      enabled: body.enabled !== false,
      location_ids: Array.isArray(body.location_ids) ? body.location_ids.map(String) : [],
    };
    if (existing) {
      Object.assign(existing, next);
      sendJson(res, 200, existing);
    } else {
      groups.unshift(next);
      sendJson(res, 200, next);
    }
    return true;
  }

  return false;
}
