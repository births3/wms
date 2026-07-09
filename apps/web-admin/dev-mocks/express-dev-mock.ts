import type { IncomingMessage, ServerResponse } from "node:http";

const devOwnerId = "00000000-0000-0000-0000-000000000001";

interface DevExpressCarrier {
  id: string;
  owner_id: string;
  carrier_code: string;
  carrier_name: string;
  api_url: string;
  api_key_alias: string | null;
  api_secret_alias: string | null;
  account_no: string | null;
  enabled: boolean;
  priority: number;
  conditions: Record<string, unknown>;
  status: string;
  created_at: string;
  updated_at: string;
}

interface DevExpressRule {
  id: string;
  owner_id: string;
  rule_code: string;
  rule_name: string;
  delivery_provider_type: string;
  carrier_code: string | null;
  priority: number;
  conditions: Record<string, unknown>;
  fallback_strategy: string | null;
  enabled: boolean;
  effective_from: string | null;
  effective_to: string | null;
  created_at: string;
  updated_at: string;
}

const devCreatedCarriers: DevExpressCarrier[] = [];
const devCreatedRules: DevExpressRule[] = [];
const devWaybills: Record<string, Record<string, unknown>> = {};

export async function handleH5ExpressDevMock(req: IncomingMessage, res: ServerResponse, pathname: string) {
  const searchParams = new URL(req.url ?? "", "http://wms.local").searchParams;
  if (req.method === "GET" && pathname === "/api/v1/express/carriers") {
    const q = searchParams.get("q")?.trim().toLowerCase() ?? "";
    const enabled = searchParams.get("enabled");
    const data = devExpressCarriers().filter((item) => {
      const matchesText = !q || item.carrier_code.toLowerCase().includes(q) || item.carrier_name.toLowerCase().includes(q);
      const matchesEnabled = !enabled || String(item.enabled) === enabled;
      return matchesText && matchesEnabled;
    });
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/express/carriers") {
    const body = await readJsonBody(req);
    const saved = devSaveCarrier(body);
    sendJson(res, 200, saved);
    return;
  }

  if (req.method === "GET" && pathname === "/api/v1/express/routing-rules") {
    const q = searchParams.get("q")?.trim().toLowerCase() ?? "";
    const enabled = searchParams.get("enabled");
    const data = devExpressRules().filter((item) => {
      const matchesText = !q || item.rule_code.toLowerCase().includes(q) || item.rule_name.toLowerCase().includes(q);
      const matchesEnabled = !enabled || String(item.enabled) === enabled;
      return matchesText && matchesEnabled;
    });
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/express/routing-rules") {
    const body = await readJsonBody(req);
    const saved = devSaveRule(body);
    sendJson(res, 200, saved);
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/express/waybills") {
    const body = await readJsonBody(req);
    const now = new Date().toISOString();
    const packageNo = asString(body.package_no, `PKG-${Date.now()}`);
    const existing = Object.values(devWaybills).find((item) => item.package_no === packageNo);
    if (existing) {
      sendJson(res, 200, existing);
      return;
    }
    const waybillNo = `${asString(body.carrier_code, "SF")}-${Date.now()}`;
    const waybill = {
      id: crypto.randomUUID(),
      owner_id: devOwnerId,
      outbound_order_id: null,
      package_no: packageNo,
      carrier_code: asString(body.carrier_code, "SF"),
      waybill_no: waybillNo,
      status: "pushed",
      sender_name: asString(body.sender_name, "平宇仓库"),
      sender_mobile: asString(body.sender_mobile, "13800000000"),
      sender_address: asString(body.sender_address, "上海市浦东新区 WMS 一号仓"),
      receiver_name: asString(body.receiver_name, "张三"),
      receiver_mobile: asString(body.receiver_mobile, "13900000000"),
      receiver_address: asString(body.receiver_address, "上海市黄浦区客户门店"),
      weight_grams: asNumber(body.weight_grams, 1200),
      volume_cm3: asNumber(body.volume_cm3, 8000),
      package_count: asNumber(body.package_count, 1),
      eta_at: new Date(Date.now() + 2 * 86400 * 1000).toISOString(),
      created_at: now,
      updated_at: now,
    };
    devWaybills[waybillNo] = waybill;
    sendJson(res, 200, waybill);
    return;
  }

  const cancel = pathname.match(/^\/api\/v1\/express\/waybills\/([^/]+)\/cancel$/);
  if (req.method === "POST" && cancel) {
    const waybillNo = decodeURIComponent(cancel[1]);
    const waybill = devWaybills[waybillNo];
    if (!waybill) {
      sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Express waybill not found", trace_id: "dev-mock" });
      return;
    }
    waybill.status = "cancelled";
    waybill.updated_at = new Date().toISOString();
    sendJson(res, 200, waybill);
    return;
  }

  const tracking = pathname.match(/^\/api\/v1\/express\/waybills\/([^/]+)\/tracking$/);
  if (req.method === "GET" && tracking) {
    const waybillNo = decodeURIComponent(tracking[1]);
    const waybill = devWaybills[waybillNo];
    if (!waybill) {
      sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Express waybill not found", trace_id: "dev-mock" });
      return;
    }
    sendJson(res, 200, {
      waybill,
      events: [{
        id: crypto.randomUUID(),
        waybill_no: waybillNo,
        event_time: new Date().toISOString(),
        status: "pushed",
        location: "WMS",
        description: "快递下单成功，等待承运商揽收",
        source: "dev_mock",
        cached_at: new Date().toISOString(),
      }],
    });
    return;
  }

  sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Express dev mock route not found", trace_id: "dev-mock" });
}

function devExpressCarriers(): DevExpressCarrier[] {
  return [
    ...devCreatedCarriers,
    {
      id: "00000000-0000-0000-0000-000000005501",
      owner_id: devOwnerId,
      carrier_code: "SF",
      carrier_name: "顺丰速运",
      api_url: "https://carrier.example.test/api",
      api_key_alias: "sf_api_key",
      api_secret_alias: "sf_api_secret",
      account_no: "WMS-001",
      enabled: true,
      priority: 10,
      conditions: { cold_chain: true },
      status: "testing",
      created_at: "2026-07-09T09:00:00.000Z",
      updated_at: "2026-07-09T09:00:00.000Z",
    },
  ];
}

function devExpressRules(): DevExpressRule[] {
  return [
    ...devCreatedRules,
    {
      id: "00000000-0000-0000-0000-000000005511",
      owner_id: devOwnerId,
      rule_code: "DEFAULT_THIRD_PARTY",
      rule_name: "默认三方快递",
      delivery_provider_type: "third_party_express",
      carrier_code: "SF",
      priority: 10,
      conditions: { province: ["上海", "江苏", "浙江"] },
      fallback_strategy: "manual_review",
      enabled: true,
      effective_from: null,
      effective_to: null,
      created_at: "2026-07-09T09:00:00.000Z",
      updated_at: "2026-07-09T09:00:00.000Z",
    },
  ];
}

function devSaveCarrier(body: Record<string, unknown>): DevExpressCarrier {
  const now = new Date().toISOString();
  const carrierCode = asString(body.carrier_code, "SF");
  const existing = devCreatedCarriers.findIndex((item) => item.carrier_code === carrierCode);
  const saved: DevExpressCarrier = {
    id: existing >= 0 ? devCreatedCarriers[existing].id : crypto.randomUUID(),
    owner_id: devOwnerId,
    carrier_code: carrierCode,
    carrier_name: asString(body.carrier_name, "顺丰速运"),
    api_url: asString(body.api_url, "https://carrier.example.test/api"),
    api_key_alias: asNullableString(body.api_key_alias),
    api_secret_alias: asNullableString(body.api_secret_alias),
    account_no: asNullableString(body.account_no),
    enabled: body.enabled !== false,
    priority: asNumber(body.priority, 100),
    conditions: asRecord(body.conditions),
    status: body.enabled === false ? "disabled" : "testing",
    created_at: existing >= 0 ? devCreatedCarriers[existing].created_at : now,
    updated_at: now,
  };
  if (existing >= 0) devCreatedCarriers[existing] = saved;
  else devCreatedCarriers.unshift(saved);
  return saved;
}

function devSaveRule(body: Record<string, unknown>): DevExpressRule {
  const now = new Date().toISOString();
  const ruleCode = asString(body.rule_code, "DEFAULT_THIRD_PARTY");
  const existing = devCreatedRules.findIndex((item) => item.rule_code === ruleCode);
  const saved: DevExpressRule = {
    id: existing >= 0 ? devCreatedRules[existing].id : crypto.randomUUID(),
    owner_id: devOwnerId,
    rule_code: ruleCode,
    rule_name: asString(body.rule_name, "默认三方快递"),
    delivery_provider_type: asString(body.delivery_provider_type, "third_party_express"),
    carrier_code: asNullableString(body.carrier_code),
    priority: asNumber(body.priority, 100),
    conditions: asRecord(body.conditions),
    fallback_strategy: asNullableString(body.fallback_strategy),
    enabled: body.enabled !== false,
    effective_from: null,
    effective_to: null,
    created_at: existing >= 0 ? devCreatedRules[existing].created_at : now,
    updated_at: now,
  };
  if (existing >= 0) devCreatedRules[existing] = saved;
  else devCreatedRules.unshift(saved);
  return saved;
}

async function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown>> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  const raw = Buffer.concat(chunks).toString("utf8").trim();
  return raw ? JSON.parse(raw) as Record<string, unknown> : {};
}

function sendJson(res: ServerResponse, statusCode: number, body: unknown) {
  res.statusCode = statusCode;
  res.setHeader("content-type", "application/json; charset=utf-8");
  res.end(JSON.stringify(body));
}

function asString(value: unknown, fallback: string) {
  return typeof value === "string" && value.trim() ? value.trim() : fallback;
}

function asNullableString(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function asNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function asRecord(value: unknown) {
  return value && !Array.isArray(value) && typeof value === "object" ? value as Record<string, unknown> : {};
}
