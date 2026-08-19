import type { IncomingMessage, ServerResponse } from "node:http";

import { sendError, sendJson } from "./web-admin-dev-mock-core-common";

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

interface DevH4ApprovalRecord {
  id: string;
  owner_id: string;
  scenario: string;
  business_ref: string;
  dedupe_key: string;
  approver_user: string;
  process_id: string;
  callback_path: string;
  summary: string;
  status: string;
  opinion: string | null;
  external_approval_id: string | null;
  approved_by: string | null;
  approved_at: string | null;
  failure_reason: string | null;
  created_at: string;
  updated_at: string;
}

interface DevH4IdempotencyEntry {
  pathname: string;
  requestBody: string;
  responseBody: unknown;
}

const devCreatedH4Configs: DevH4NotificationConfig[] = [];
const devCreatedH4Records: DevH4NotificationRecord[] = [];
const devCreatedH4Approvals: DevH4ApprovalRecord[] = [];
let devWechatSettings: DevH4WechatSettings | null = null;
const devH4Idempotency = new Map<string, DevH4IdempotencyEntry>();

export async function handleH4WechatNotifyDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
) {
  const searchParams = new URL(req.url ?? "", "http://wms.local").searchParams;
  const idempotencyKey = requiresIdempotencyKey(req.method, pathname) ? requireIdempotencyKey(req, res) : null;
  if (requiresIdempotencyKey(req.method, pathname) && !idempotencyKey) return;
  if (req.method === "GET" && pathname === "/api/v1/wechat-notify/configs") {
    const eventType = searchParams.get("event_type")?.trim().toLowerCase() ?? "";
    const data = devH4Configs().filter((item) => !eventType || item.event_type.toLowerCase().includes(eventType));
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/configs") {
    const body = await readJsonBody(req);
    if (replayIdempotentResponse(res, idempotencyKey, pathname, body)) return;
    const validationError = validateConfigInput(body);
    if (validationError) {
      sendError(res, 422, validationError.code, validationError.message);
      return;
    }
    sendIdempotentResponse(res, idempotencyKey, pathname, body, devSaveH4Config(body));
    return;
  }

  if (req.method === "GET" && pathname === "/api/v1/wechat-notify/settings") {
    sendJson(res, 200, { data: devWechatSettings ?? devSeedWechatSettings() });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/settings") {
    const body = await readJsonBody(req);
    if (replayIdempotentResponse(res, idempotencyKey, pathname, body)) return;
    const validationError = validateWechatSettingsInput(body);
    if (validationError) {
      sendError(res, 422, "H4_REQUEST_INVALID", validationError);
      return;
    }
    devWechatSettings = devSaveWechatSettings(body);
    sendIdempotentResponse(res, idempotencyKey, pathname, body, devWechatSettings);
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/settings/test") {
    const settings = devWechatSettings ?? devSeedWechatSettings();
    const validationError = validateWechatSettings(settings);
    if (validationError) {
      sendError(res, 422, "H4_REQUEST_INVALID", validationError);
      return;
    }
    sendJson(res, 200, {
      status: settings.enabled ? "success" : "warning",
      message: settings.enabled ? "企业微信参数校验通过" : "企业微信参数已保存但未启用",
      checked_at: new Date().toISOString(),
    });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/send") {
    const body = await readJsonBody(req);
    if (replayIdempotentResponse(res, idempotencyKey, pathname, body)) return;
    const validationError = validateSendInput(body);
    if (validationError) {
      sendError(res, 422, validationError.code, validationError.message);
      return;
    }
    const eventType = asString(body.event_type, "");
    const config = devH4Configs().find((item) => item.event_type === eventType && item.enabled);
    if (!config) {
      sendError(res, 404, "H4_EVENT_NOT_FOUND", "通知事件未配置或未启用");
      return;
    }
    const content = renderDevTemplate(config.template, body.payload);
    if (content === null) {
      sendError(res, 422, "H4_TEMPLATE_INVALID", "通知模板变量无法渲染");
      return;
    }
    sendIdempotentResponse(res, idempotencyKey, pathname, body, devCreateH4Records(body, config, content));
    return;
  }

  const resend = pathname.match(/^\/api\/v1\/wechat-notify\/records\/([^/]+)\/resend$/);
  if (req.method === "POST" && resend) {
    const recordId = decodeURIComponent(resend[1]);
    const requestBody = { record_id: recordId };
    if (replayIdempotentResponse(res, idempotencyKey, pathname, requestBody)) return;
    const existing = devH4Records().find((item) => item.id === recordId);
    if (!existing) {
      sendError(res, 404, "H4_RECORD_NOT_FOUND", "通知记录不存在");
      return;
    }
    if (!["failed", "retrying"].includes(existing.status)) {
      sendError(res, 422, "H4_RECORD_NOT_RESENDABLE", "仅失败或重试中的通知可以重发");
      return;
    }
    const record = devResendH4Record(existing);
    sendIdempotentResponse(res, idempotencyKey, pathname, requestBody, record);
    return;
  }

  if (req.method === "GET" && pathname === "/api/v1/wechat-notify/records") {
    const fromText = searchParams.get("from");
    const toText = searchParams.get("to");
    const from = timestamp(fromText);
    const to = timestamp(toText);
    if ((fromText && from === null) || (toText && to === null)) {
      sendError(res, 400, "DEV_MOCK_REQUEST_INVALID", "通知记录时间范围非法");
      return;
    }
    const data = devH4Records().filter((item) => {
      const eventType = searchParams.get("event_type")?.trim().toLowerCase() ?? "";
      const recipient = searchParams.get("recipient")?.trim().toLowerCase() ?? "";
      const status = searchParams.get("status")?.trim() ?? "";
      const createdAt = Date.parse(item.created_at);
      return (
        (!eventType || item.event_type.toLowerCase().includes(eventType)) &&
        (!recipient || item.recipient.toLowerCase().includes(recipient)) &&
        (!status || item.status === status) &&
        (from === null || createdAt >= from) &&
        (to === null || createdAt <= to)
      );
    });
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return;
  }

  const approvalCallback = pathname.match(/^\/api\/v1\/wechat-notify\/approvals\/([^/]+)\/callback$/);
  if (req.method === "POST" && approvalCallback) {
    const body = await readJsonBody(req);
    if (replayIdempotentResponse(res, idempotencyKey, pathname, body)) return;
    const approvalId = decodeURIComponent(approvalCallback[1]);
    const existing = devCreatedH4Approvals.find((item) => item.id === approvalId);
    if (!existing) {
      sendError(res, 404, "H4_APPROVAL_NOT_FOUND", "审批记录不存在");
      return;
    }
    const conclusion = approvalStatus(body.conclusion);
    if (!conclusion) {
      sendError(res, 422, "H4_APPROVAL_STATUS_INVALID", "审批结论非法");
      return;
    }
    const approvedBy = asString(body.approved_by, "").toLowerCase();
    const externalApprovalId = asString(body.external_approval_id, "");
    if (approvedBy !== devUserId || approvedBy !== existing.approver_user || !externalApprovalId) {
      sendError(res, 422, "H4_REQUEST_INVALID", "审批回写身份或外部审批 ID 非法");
      return;
    }
    if (existing.status !== "pending") {
      if (existing.status !== conclusion) {
        sendError(res, 409, "H4_IDEMPOTENCY_CONFLICT", "审批终态与已有记录冲突");
        return;
      }
      sendIdempotentResponse(res, idempotencyKey, pathname, body, existing);
      return;
    }
    const now = new Date().toISOString();
    Object.assign(existing, {
      status: conclusion,
      opinion: typeof body.opinion === "string" ? body.opinion : null,
      external_approval_id: externalApprovalId,
      approved_by: approvedBy,
      approved_at: now,
      updated_at: now,
    });
    sendIdempotentResponse(res, idempotencyKey, pathname, body, existing);
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/wechat-notify/approvals") {
    const body = await readJsonBody(req);
    if (replayIdempotentResponse(res, idempotencyKey, pathname, body)) return;
    const validationError = validateApprovalInput(body);
    if (validationError) {
      sendError(res, 422, "H4_REQUEST_INVALID", validationError);
      return;
    }
    const now = new Date().toISOString();
    const approval: DevH4ApprovalRecord = {
      id: `00000000-0000-0000-0000-${String(Date.now()).slice(-12)}`,
      owner_id: devOwnerId,
      scenario: asString(body.scenario, "config_change"),
      business_ref: asString(body.business_ref, "H4-DEMO"),
      dedupe_key: asString(body.dedupe_key, `approval-${Date.now()}`),
      approver_user: asString(body.approver_user, devUserId).toLowerCase(),
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
    };
    devCreatedH4Approvals.unshift(approval);
    sendIdempotentResponse(res, idempotencyKey, pathname, body, approval);
    return;
  }

  sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Wechat notify dev mock route not found");
}

function validateWechatSettingsInput(body: Record<string, unknown>) {
  if (typeof body.enabled !== "boolean") return "是否启用必须为布尔值";
  const settings = {
    corp_id: typeof body.corp_id === "string" ? body.corp_id : "",
    agent_id: typeof body.agent_id === "string" ? body.agent_id : "",
    secret_alias: typeof body.secret_alias === "string" ? body.secret_alias : "",
    callback_token_alias: typeof body.callback_token_alias === "string" ? body.callback_token_alias : "",
    aes_key_alias: typeof body.aes_key_alias === "string" ? body.aes_key_alias : "",
    callback_url: typeof body.callback_url === "string" ? body.callback_url : "",
    approval_callback_path: typeof body.approval_callback_path === "string" ? body.approval_callback_path : "",
    retry_max_attempts: typeof body.retry_max_attempts === "number" ? body.retry_max_attempts : Number.NaN,
    retry_interval_seconds: typeof body.retry_interval_seconds === "number" ? body.retry_interval_seconds : Number.NaN,
  };
  return validateWechatSettings(settings);
}

function validateWechatSettings(settings: Pick<DevH4WechatSettings,
  "corp_id" | "agent_id" | "secret_alias" | "callback_token_alias" | "aes_key_alias" |
  "callback_url" | "approval_callback_path" | "retry_max_attempts" | "retry_interval_seconds"
>) {
  const required = [settings.corp_id, settings.agent_id, settings.secret_alias, settings.callback_token_alias,
    settings.aes_key_alias, settings.callback_url, settings.approval_callback_path];
  if (required.some((value) => !value.trim())) return "企业微信参数不完整";
  if (!isHttpUrl(settings.callback_url) || !isCallbackPath(settings.approval_callback_path)) {
    return "回调地址或路径格式不正确";
  }
  if (!Number.isInteger(settings.retry_max_attempts) || settings.retry_max_attempts < 0 || settings.retry_max_attempts > 10) {
    return "最大重试次数必须在 0 到 10 之间";
  }
  if (!Number.isInteger(settings.retry_interval_seconds) || settings.retry_interval_seconds < 1 || settings.retry_interval_seconds > 3600) {
    return "重试间隔必须在 1 到 3600 秒之间";
  }
  return null;
}

function validateSendInput(body: Record<string, unknown>) {
  if (typeof body.event_type !== "string" || !body.event_type.trim()) {
    return { code: "H4_REQUEST_INVALID", message: "事件类型不能为空" };
  }
  if (typeof body.dedupe_key !== "string" || !body.dedupe_key.trim()) {
    return { code: "H4_REQUEST_INVALID", message: "去重键不能为空" };
  }
  if (!Array.isArray(body.recipients) || !body.recipients.some((value) => typeof value === "string" && value.trim())) {
    return { code: "H4_NO_RECIPIENTS", message: "通知接收人为空" };
  }
  return null;
}

function validateApprovalInput(body: Record<string, unknown>) {
  const required = ["scenario", "business_ref", "dedupe_key", "approver_user", "process_id", "callback_path", "summary"];
  if (required.some((key) => typeof body[key] !== "string" || !String(body[key]).trim())) {
    return "审批请求字段不完整";
  }
  if (!isUuid(String(body.approver_user)) || !isCallbackPath(String(body.callback_path))) {
    return "审批人或回调路径格式不正确";
  }
  return null;
}

function validateConfigInput(body: Record<string, unknown>) {
  if (typeof body.enabled !== "boolean") {
    return { code: "H4_REQUEST_INVALID", message: "是否启用必须为布尔值" };
  }
  if (!Array.isArray(body.channels) || body.channels.some((channel) => typeof channel !== "string")) {
    return { code: "H4_REQUEST_INVALID", message: "通知方式必须为字符串数组" };
  }
  if (typeof body.event_type !== "string" || !body.event_type.trim()) {
    return { code: "H4_REQUEST_INVALID", message: "事件类型不能为空" };
  }
  if (typeof body.template !== "string" || !body.template.trim()) {
    return { code: "H4_REQUEST_INVALID", message: "通知模板不能为空" };
  }
  if (!hasRecipientRule(body.recipient_rule)) {
    return { code: "H4_NO_RECIPIENTS", message: "通知接收人为空" };
  }
  return null;
}

function hasRecipientRule(value: unknown) {
  return Object.values(asRecord(value)).some((items) =>
    Array.isArray(items) && items.some((item) => typeof item === "string" && item.trim().length > 0));
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
      status: "failed",
      retry_count: 0,
      failure_reason: "企业微信外部发送能力尚未启用",
      sent_at: null,
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
      retry_count: 0,
      failure_reason: "企业微信外部发送能力尚未启用",
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

function devCreateH4Records(
  body: Record<string, unknown>,
  config: DevH4NotificationConfig,
  content: string,
): DevH4NotificationRecord[] {
  const now = new Date().toISOString();
  const eventType = asString(body.event_type, "custom_event");
  const recipients = asStringArray(body.recipients, ["receiving_lead"]);
  return recipients.map((recipient, index) => {
    const dedupeKey = asString(body.dedupe_key, `dev-${Date.now()}`);
    const existing = devH4Records().find((record) =>
      record.event_type === eventType && record.recipient === recipient && record.dedupe_key === dedupeKey);
    if (existing) return existing;
    const record: DevH4NotificationRecord = {
      id: `00000000-0000-0000-0000-${String(Date.now() + index).slice(-12)}`,
      owner_id: devOwnerId,
      config_id: config.id,
      event_type: eventType,
      dedupe_key: dedupeKey,
      recipient,
      channel: "wechat",
      content_summary: content.slice(0, 500),
      status: "failed",
      retry_count: 0,
      failure_reason: "企业微信外部发送能力尚未启用",
      sent_at: null,
      created_at: now,
      updated_at: now,
    };
    devCreatedH4Records.unshift(record);
    return record;
  });
}

function devResendH4Record(existing: DevH4NotificationRecord): DevH4NotificationRecord {
  const now = new Date().toISOString();
  const next = {
    ...existing,
    status: "failed",
    retry_count: existing.retry_count + 1,
    failure_reason: "企业微信外部发送能力尚未启用",
    sent_at: null,
    updated_at: now,
  };
  const index = devCreatedH4Records.findIndex((item) => item.id === existing.id);
  if (index >= 0) devCreatedH4Records[index] = next;
  else devCreatedH4Records.unshift(next);
  return next;
}

function requiresIdempotencyKey(method: string | undefined, pathname: string) {
  if (method !== "POST" || pathname === "/api/v1/wechat-notify/settings/test") return false;
  return pathname === "/api/v1/wechat-notify/configs"
    || pathname === "/api/v1/wechat-notify/settings"
    || pathname === "/api/v1/wechat-notify/send"
    || pathname === "/api/v1/wechat-notify/approvals"
    || /\/api\/v1\/wechat-notify\/approvals\/[^/]+\/callback$/.test(pathname)
    || /\/api\/v1\/wechat-notify\/records\/[^/]+\/resend$/.test(pathname);
}

function requireIdempotencyKey(req: IncomingMessage, res: ServerResponse) {
  const key = req.headers["idempotency-key"];
  const normalized = Array.isArray(key) ? key[0]?.trim() : key?.trim();
  if (normalized) return normalized;
  sendError(res, 400, "H4_IDEMPOTENCY_REQUIRED", "缺少 Idempotency-Key");
  return null;
}

function replayIdempotentResponse(
  res: ServerResponse,
  key: string | null,
  pathname: string,
  body: Record<string, unknown>,
) {
  if (!key) return false;
  const existing = devH4Idempotency.get(key);
  if (!existing) return false;
  if (existing.pathname !== pathname || existing.requestBody !== JSON.stringify(body)) {
    sendError(res, 409, "H4_IDEMPOTENCY_CONFLICT", "Idempotency-Key 已用于其他请求");
    return true;
  }
  sendJson(res, 200, existing.responseBody);
  return true;
}

function sendIdempotentResponse(
  res: ServerResponse,
  key: string | null,
  pathname: string,
  body: Record<string, unknown>,
  responseBody: unknown,
) {
  if (key) {
    devH4Idempotency.set(key, { pathname, requestBody: JSON.stringify(body), responseBody });
  }
  sendJson(res, 200, responseBody);
}

function isHttpUrl(value: string) {
  try {
    const url = new URL(value);
    return (url.protocol === "http:" || url.protocol === "https:") && Boolean(url.host);
  } catch {
    return false;
  }
}

function isCallbackPath(value: string) {
  const path = value.trim();
  return path.startsWith("/") && !path.startsWith("//") && !/\s/.test(path);
}

function isUuid(value: string) {
  return /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(value.trim());
}

function timestamp(value: string | null) {
  if (!value) return null;
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? null : parsed;
}

function approvalStatus(value: unknown) {
  const conclusion = asString(value, "");
  if (conclusion === "approved" || conclusion === "同意") return "approved";
  if (conclusion === "rejected" || conclusion === "拒绝") return "rejected";
  return null;
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

function renderDevTemplate(template: string, payload: unknown): string | null {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) return null;
  let content = template;
  for (const [key, value] of Object.entries(payload)) {
    const rendered = typeof value === "string" ? value : (JSON.stringify(value) ?? "null");
    content = content.replaceAll(`{{${key}}}`, rendered);
  }
  return content.includes("{{") || content.includes("}}") ? null : content;
}
