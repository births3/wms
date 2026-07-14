import type { IncomingMessage, ServerResponse } from "node:http";

import { asBoolean, asNumber, asString, readJsonBody, sendError, sendJson } from "./web-admin-dev-mock-core-common";

interface DevDocumentNumberRule {
  id: string;
  owner_id: string;
  document_type: string;
  rule_code: string;
  rule_name: string;
  template: string;
  reset_policy: string;
  sequence_width: number;
  sequence_mode: string;
  enabled: boolean;
  effective_from: string | null;
  effective_to: string | null;
  created_at: string;
  updated_at: string;
  version: number;
}

interface DevDocumentNumberAllocation {
  id: string;
  owner_id: string;
  rule_id: string;
  document_type: string;
  generated_no: string;
  sequence_value: number;
  counter_key: string;
  source_module: string;
  source_document_id: string | null;
  created_at: string;
}

const ownerId = "00000000-0000-0000-0000-000000000001";
const initialDate = "2026-07-12T08:00:00.000Z";
const rules: DevDocumentNumberRule[] = [
  createRule("purchase-inbound", "purchase_inbound", "采购入库单号", "{OWNER}-ASN-{YYYY}{MM}{DD}-{SEQ}"),
  createRule("sales-return", "sales_return", "销售退货入库单号", "{OWNER}-SR-{YYYY}{MM}{DD}-{SEQ}"),
  createRule("outbound-order", "outbound_order", "出库订单号", "{OWNER}-OUT-{YYYY}{MM}{DD}-{SEQ}"),
  createRule("stocktake", "stocktake", "盘点单号", "{OWNER}-PD-{YYYY}{MM}{DD}-{SEQ}"),
];
const allocations: DevDocumentNumberAllocation[] = rules.slice(0, 3).map((rule, index) => ({
  id: `00000000-0000-0000-0000-${String(8100 + index).padStart(12, "0")}`,
  owner_id: ownerId,
  rule_id: rule.id,
  document_type: rule.document_type,
  generated_no: `PY001-${rule.document_type === "purchase_inbound" ? "ASN" : rule.document_type === "sales_return" ? "SR" : "OUT"}-20260712-${String(index + 1).padStart(4, "0")}`,
  sequence_value: index + 1,
  counter_key: `${rule.rule_code}:20260712`,
  source_module: rule.document_type === "outbound_order" ? "M4" : "M2",
  source_document_id: null,
  created_at: initialDate,
}));
const idempotency = new Map<string, { body: string; response: DevDocumentNumberRule }>();

export async function handleDocumentNumberingDevMock(req: IncomingMessage, res: ServerResponse, pathname: string) {
  if (req.method === "GET" && pathname === "/api/v1/code-generator/document-number-rules") {
    const documentType = new URL(req.url ?? pathname, "http://wms.local").searchParams.get("document_type");
    const data = documentType ? rules.filter((rule) => rule.document_type === documentType) : rules;
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/code-generator/document-number-allocations") {
    const documentType = new URL(req.url ?? pathname, "http://wms.local").searchParams.get("document_type");
    const data = documentType ? allocations.filter((row) => row.document_type === documentType) : allocations;
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }

  const rulePath = pathname.match(/^\/api\/v1\/code-generator\/document-number-rules\/([^/]+)$/);
  if (req.method === "PUT" && rulePath) {
    const body = await readJsonBody(req);
    const response = upsertRule(req, res, decodeURIComponent(rulePath[1]), body);
    if (!response) return true;
    sendJson(res, 200, response);
    return true;
  }

  const enabledPath = pathname.match(/^\/api\/v1\/code-generator\/document-number-rules\/([^/]+)\/enabled$/);
  if (req.method === "PATCH" && enabledPath) {
    const body = await readJsonBody(req);
    const rule = rules.find((item) => item.rule_code === decodeURIComponent(enabledPath[1]));
    if (!rule) {
      sendError(res, 404, "MCG_DOCUMENT_NUMBERING_RULE_NOT_FOUND", "单据号规则不存在");
      return true;
    }
    rule.enabled = asBoolean(body.enabled, rule.enabled);
    rule.updated_at = new Date().toISOString();
    rule.version += 1;
    sendJson(res, 200, rule);
    return true;
  }

  return false;
}

function upsertRule(req: IncomingMessage, res: ServerResponse, ruleCode: string, body: Record<string, unknown>) {
  const key = req.headers["idempotency-key"];
  if (typeof key !== "string" || !key.trim()) {
    sendError(res, 400, "MCG_DOCUMENT_NUMBERING_IDEMPOTENCY_REQUIRED", "缺少 Idempotency-Key");
    return null;
  }
  const requestBody = JSON.stringify(body);
  const replay = idempotency.get(key);
  if (replay) {
    if (replay.body !== requestBody) {
      sendError(res, 409, "MCG_DOCUMENT_NUMBERING_IDEMPOTENCY_CONFLICT", "幂等键已被不同请求使用");
      return null;
    }
    return replay.response;
  }
  const documentType = asString(body.document_type, "");
  const ruleName = asString(body.rule_name, "");
  const template = asString(body.template, "");
  const width = asNumber(body.sequence_width, 0);
  if (!documentType || !ruleName || !template || width < 1 || width > 12) {
    sendError(res, 422, "MCG_DOCUMENT_NUMBERING_INVALID", "单据号规则参数非法");
    return null;
  }
  const now = new Date().toISOString();
  const existing = rules.find((item) => item.rule_code === ruleCode);
  const response: DevDocumentNumberRule = existing ?? createRule(ruleCode, documentType, ruleName, template);
  Object.assign(response, {
    document_type: documentType,
    rule_name: ruleName,
    template,
    reset_policy: asString(body.reset_policy, "daily"),
    sequence_width: width,
    sequence_mode: "no_gap",
    enabled: asBoolean(body.enabled, true),
    effective_from: typeof body.effective_from === "string" ? body.effective_from : null,
    effective_to: typeof body.effective_to === "string" ? body.effective_to : null,
    updated_at: now,
    version: response.version + 1,
  });
  if (!existing) rules.unshift(response);
  idempotency.set(key, { body: requestBody, response });
  return response;
}

function createRule(ruleCode: string, documentType: string, ruleName: string, template: string): DevDocumentNumberRule {
  return {
    id: crypto.randomUUID(),
    owner_id: ownerId,
    document_type: documentType,
    rule_code: ruleCode,
    rule_name: ruleName,
    template,
    reset_policy: "daily",
    sequence_width: 4,
    sequence_mode: "no_gap",
    enabled: true,
    effective_from: null,
    effective_to: null,
    created_at: initialDate,
    updated_at: initialDate,
    version: 1,
  };
}
