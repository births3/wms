import type { IncomingMessage, ServerResponse } from "node:http";

const devOwnerId = "00000000-0000-0000-0000-000000000001";
const devUserId = "00000000-0000-0000-0000-000000000101";

interface DevH4NotificationConfig {
  id: string;
  owner_id: string;
  event_type: string;
  enabled: boolean;
  template: string;
  recipient_rule: Record<string, unknown>;
  channels: string[];
  created_at: string;
  updated_at: string;
  version: number;
}

interface DevH4NotificationRecord {
  id: string;
  owner_id: string;
  config_id: string | null;
  event_type: string;
  dedupe_key: string;
  recipient: string;
  channel: string;
  content_summary: string;
  status: string;
  retry_count: number;
  failure_reason: string | null;
  sent_at: string | null;
  created_at: string;
  updated_at: string;
}

interface DevH4WechatSettings {
  id: string;
  owner_id: string;
  corp_id: string;
  agent_id: string;
  secret_alias: string;
  callback_token_alias: string;
  aes_key_alias: string;
  callback_url: string;
  approval_callback_path: string;
  enabled: boolean;
  retry_max_attempts: number;
  retry_interval_seconds: number;
  created_at: string;
  updated_at: string;
  version: number;
}

const devCreatedH4Configs: DevH4NotificationConfig[] = [];
const devCreatedH4Records: DevH4NotificationRecord[] = [];
let devWechatSettings: DevH4WechatSettings | null = null;

export async function handleH4WechatNotifyDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
) {
  const searchParams = new URL(req.url ?? "", "http://wms.local").searchParams;
  if (req.method === "GET" && pathname === "/api/v1/wechat-notify/configs") {
    const eventType = searchParams.get("event_type")?.trim().toLowerCase() ?? "";
    const data = devH4Configs().filter((item) => !eventType || item.event_type.toLowerCase().includes(eventType));
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/configs") {
    const body = await readJsonBody(req);
    sendJson(res, 200, devSaveH4Config(body));
    return;
  }

  if (req.method === "GET" && pathname === "/api/v1/wechat-notify/settings") {
    sendJson(res, 200, { data: devWechatSettings ?? devSeedWechatSettings() });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/settings") {
    const body = await readJsonBody(req);
    devWechatSettings = devSaveWechatSettings(body);
    sendJson(res, 200, devWechatSettings);
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/send") {
    const body = await readJsonBody(req);
    sendJson(res, 200, devCreateH4Records(body));
    return;
  }

  const resend = pathname.match(/^\/api\/v1\/wechat-notify\/records\/([^/]+)\/resend$/);
  if (req.method === "POST" && resend) {
    const record = devResendH4Record(decodeURIComponent(resend[1]));
    if (!record) {
      sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Notification record not found", trace_id: "dev-mock" });
      return;
    }
    sendJson(res, 200, record);
    return;
  }

  if (req.method === "GET" && pathname === "/api/v1/wechat-notify/records") {
    const data = devH4Records().filter((item) => {
      const eventType = searchParams.get("event_type")?.trim().toLowerCase() ?? "";
      const recipient = searchParams.get("recipient")?.trim().toLowerCase() ?? "";
      const status = searchParams.get("status")?.trim() ?? "";
      const from = searchParams.get("from")?.slice(0, 10) ?? "";
      const to = searchParams.get("to")?.slice(0, 10) ?? "";
      const date = item.created_at.slice(0, 10);
      return (
        (!eventType || item.event_type.toLowerCase().includes(eventType)) &&
        (!recipient || item.recipient.toLowerCase().includes(recipient)) &&
        (!status || item.status === status) &&
        (!from || date >= from) &&
        (!to || date <= to)
      );
    });
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/approvals") {
    const body = await readJsonBody(req);
    const now = new Date().toISOString();
    sendJson(res, 200, {
      id: `00000000-0000-0000-0000-${String(Date.now()).slice(-12)}`,
      owner_id: devOwnerId,
      scenario: asString(body.scenario, "config_change"),
      business_ref: asString(body.business_ref, "H4-DEMO"),
      dedupe_key: asString(body.dedupe_key, `approval-${Date.now()}`),
      approver_user: asString(body.approver_user, "warehouse_manager"),
      process_id: asString(body.process_id, "ww-process-demo"),
      callback_path: asString(body.callback_path, "/api/v1/wechat-notify/approvals/callback"),
      summary: asString(body.summary, "H4 dev mock 审批"),
      status: "pending",
      opinion: null,
      external_approval_id: null,
      approved_by: null,
      approved_at: null,
      failure_reason: null,
      created_at: now,
      updated_at: now,
    });
    return;
  }

  sendJson(res, 404, {
    code: "DEV_MOCK_NOT_FOUND",
    message: "Wechat notify dev mock route not found",
    trace_id: "dev-mock",
  });
}

function devSeedWechatSettings(): DevH4WechatSettings {
  const now = "2026-07-08T08:30:00.000Z";
  return {
    id: "00000000-0000-0000-0000-000000004301",
    owner_id: devOwnerId,
    corp_id: "ww-demo-corp",
    agent_id: "1000002",
    secret_alias: "h4/wechat/agent_secret",
    callback_token_alias: "h4/wechat/callback_token",
    aes_key_alias: "h4/wechat/aes_key",
    callback_url: "https://wms.example.com/api/v1/wechat-notify/approvals/callback",
    approval_callback_path: "/api/v1/wechat-notify/approvals/{approval_id}/callback",
    enabled: true,
    retry_max_attempts: 3,
    retry_interval_seconds: 60,
    created_at: now,
    updated_at: now,
    version: 1,
  };
}

function devH4Configs(): DevH4NotificationConfig[] {
  const rows = [...devCreatedH4Configs];
  for (const config of devSeedH4Configs()) {
    if (!rows.some((item) => item.event_type === config.event_type)) rows.push(config);
  }
  return rows;
}

function devSeedH4Configs(): DevH4NotificationConfig[] {
  const now = "2026-07-08T09:00:00.000Z";
  return [
    {
      id: "00000000-0000-0000-0000-000000004401",
      owner_id: devOwnerId,
      event_type: "asn_arrived",
      enabled: true,
      template: "ASN {{asn_no}} 已到货，请仓库收货组处理。",
      recipient_rule: { users: ["receiving_lead"], roles: ["warehouse_manager"] },
      channels: ["wechat"],
      created_at: now,
      updated_at: now,
      version: 1,
    },
    {
      id: "00000000-0000-0000-0000-000000004402",
      owner_id: devOwnerId,
      event_type: "temperature_excursion",
      enabled: true,
      template: "温度超标：{{device_code}} / {{temperature}}，请质量负责人确认。",
      recipient_rule: { users: ["qa_lead"], roles: ["quality_manager"] },
      channels: ["wechat"],
      created_at: now,
      updated_at: now,
      version: 1,
    },
  ];
}

function devH4Records(): DevH4NotificationRecord[] {
  const rows = [...devCreatedH4Records];
  for (const record of devSeedH4Records()) {
    if (!rows.some((item) => item.id === record.id)) rows.push(record);
  }
  return rows;
}

function devSeedH4Records(): DevH4NotificationRecord[] {
  const now = "2026-07-08T09:15:00.000Z";
  return [
    {
      id: "00000000-0000-0000-0000-000000004501",
      owner_id: devOwnerId,
      config_id: "00000000-0000-0000-0000-000000004401",
      event_type: "asn_arrived",
      dedupe_key: "asn-arrived-demo-001",
      recipient: "receiving_lead",
      channel: "wechat",
      content_summary: "ASN ASN-DEMO-001 已到货，请仓库收货组处理。",
      status: "success",
      retry_count: 0,
      failure_reason: null,
      sent_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      id: "00000000-0000-0000-0000-000000004502",
      owner_id: devOwnerId,
      config_id: "00000000-0000-0000-0000-000000004402",
      event_type: "temperature_excursion",
      dedupe_key: "temperature-demo-001",
      recipient: "qa_lead",
      channel: "wechat",
      content_summary: "温度超标：DEV-COLD-01 / 12.5，请质量负责人确认。",
      status: "failed",
      retry_count: 1,
      failure_reason: "dev mock 企业微信返回临时错误",
      sent_at: null,
      created_at: now,
      updated_at: now,
    },
  ];
}

function devSaveH4Config(body: Record<string, unknown>): DevH4NotificationConfig {
  const now = new Date().toISOString();
  const eventType = asString(body.event_type, "custom_event");
  const existing = devH4Configs().find((item) => item.event_type === eventType);
  const saved: DevH4NotificationConfig = {
    id: existing?.id ?? `00000000-0000-0000-0000-${String(4400 + devCreatedH4Configs.length + 1).padStart(12, "0")}`,
    owner_id: devOwnerId,
    event_type: eventType,
    enabled: asBoolean(body.enabled, true),
    template: asString(body.template, "{{message}}"),
    recipient_rule: asRecord(body.recipient_rule),
    channels: asStringArray(body.channels, ["wechat"]),
    created_at: existing?.created_at ?? now,
    updated_at: now,
    version: (existing?.version ?? 0) + 1,
  };
  const index = devCreatedH4Configs.findIndex((item) => item.event_type === eventType);
  if (index >= 0) devCreatedH4Configs[index] = saved;
  else devCreatedH4Configs.unshift(saved);
  return saved;
}

function devSaveWechatSettings(body: Record<string, unknown>): DevH4WechatSettings {
  const now = new Date().toISOString();
  const existing = devWechatSettings ?? devSeedWechatSettings();
  return {
    ...existing,
    corp_id: asString(body.corp_id, existing.corp_id),
    agent_id: asString(body.agent_id, existing.agent_id),
    secret_alias: asString(body.secret_alias, existing.secret_alias),
    callback_token_alias: asString(body.callback_token_alias, existing.callback_token_alias),
    aes_key_alias: asString(body.aes_key_alias, existing.aes_key_alias),
    callback_url: asString(body.callback_url, existing.callback_url),
    approval_callback_path: asString(body.approval_callback_path, existing.approval_callback_path),
    enabled: asBoolean(body.enabled, existing.enabled),
    retry_max_attempts: asNumber(body.retry_max_attempts, existing.retry_max_attempts),
    retry_interval_seconds: asNumber(body.retry_interval_seconds, existing.retry_interval_seconds),
    updated_at: now,
    version: existing.version + 1,
  };
}

function devCreateH4Records(body: Record<string, unknown>): DevH4NotificationRecord[] {
  const now = new Date().toISOString();
  const eventType = asString(body.event_type, "custom_event");
  const config = devH4Configs().find((item) => item.event_type === eventType);
  const recipients = asStringArray(body.recipients, ["receiving_lead"]);
  const payload = asRecord(body.payload);
  const template = config?.template ?? "{{message}}";
  return recipients.map((recipient, index) => {
    const content = renderDevTemplate(template, payload);
    const record: DevH4NotificationRecord = {
      id: `00000000-0000-0000-0000-${String(Date.now() + index).slice(-12)}`,
      owner_id: devOwnerId,
      config_id: config?.id ?? null,
      event_type: eventType,
      dedupe_key: asString(body.dedupe_key, `dev-${Date.now()}`),
      recipient,
      channel: "wechat",
      content_summary: content.slice(0, 500),
      status: "success",
      retry_count: 0,
      failure_reason: null,
      sent_at: now,
      created_at: now,
      updated_at: now,
    };
    devCreatedH4Records.unshift(record);
    return record;
  });
}

function devResendH4Record(recordId: string): DevH4NotificationRecord | null {
  const now = new Date().toISOString();
  const existing = devH4Records().find((item) => item.id === recordId);
  if (!existing) return null;
  const next = {
    ...existing,
    status: "success",
    retry_count: existing.retry_count + 1,
    failure_reason: null,
    sent_at: now,
    updated_at: now,
  };
  const index = devCreatedH4Records.findIndex((item) => item.id === recordId);
  if (index >= 0) devCreatedH4Records[index] = next;
  else devCreatedH4Records.unshift(next);
  return next;
}

async function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown>> {
  let raw = "";
  for await (const chunk of req) {
    raw += String(chunk);
  }
  if (!raw) return {};
  const parsed: unknown = JSON.parse(raw);
  return asRecord(parsed);
}

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const record: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value)) {
    record[key] = item;
  }
  return record;
}

function asString(value: unknown, fallback: string) {
  return typeof value === "string" && value.trim() ? value.trim() : fallback;
}

function asStringArray(value: unknown, fallback: string[]) {
  if (!Array.isArray(value)) return fallback;
  const items = value.filter((item): item is string => typeof item === "string" && item.trim().length > 0);
  return items.length > 0 ? items : fallback;
}

function asBoolean(value: unknown, fallback: boolean) {
  return typeof value === "boolean" ? value : fallback;
}

function asNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function sendJson(res: ServerResponse, statusCode: number, body: unknown) {
  res.statusCode = statusCode;
  res.setHeader("content-type", "application/json; charset=utf-8");
  res.setHeader("cache-control", "no-store");
  res.end(JSON.stringify(body));
}

function renderDevTemplate(template: string, payload: Record<string, unknown>) {
  return template.replace(/\{\{\s*([a-zA-Z0-9_.-]+)\s*\}\}/g, (_match, key: string) => {
    const value = payload[key];
    return typeof value === "string" || typeof value === "number" || typeof value === "boolean" ? String(value) : "";
  });
}
