import { useQuery } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type AuditEvent = components["schemas"]["AuditEvent"];

export interface AuditEventQueryParams {
  keyword?: string;
  action?: string;
  resourceType?: string;
  actorId?: string;
  from?: string;
  to?: string;
  limit?: number;
}

export interface AuditEventRow {
  id: string;
  occurredAt: string;
  actorName: string;
  actorId: string;
  action: string;
  resourceType: string;
  resourceId: string;
  objectLabel: string;
  result: "success" | "failed";
  resultLabel: string;
  traceId: string;
  searchText: string;
}

export const auditQueryKey = ["audit"] as const;

export function useAuditEventsQuery(params: AuditEventQueryParams) {
  return useQuery<AuditEventRow[], ApiError>({
    queryKey: [...auditQueryKey, "events", params],
    queryFn: () => listAuditEvents(params),
  });
}

async function listAuditEvents(params: AuditEventQueryParams): Promise<AuditEventRow[]> {
  const result = await api.GET("/api/v1/audit/events", {
    params: {
      query: {
        resource_type: emptyToUndefined(params.resourceType),
        actor_id: emptyToUndefined(params.actorId),
        from: emptyToUndefined(params.from),
        to: emptyToUndefined(params.to),
        limit: params.limit ?? 100,
      },
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取审计事件失败", result.response.status);
  }
  const keyword = (params.keyword ?? "").trim().toLowerCase();
  const actionFilter = (params.action ?? "").trim().toLowerCase();
  return result.data.data
    .map(mapAuditEventRow)
    .filter((row) => {
      if (actionFilter && !row.action.toLowerCase().includes(actionFilter)) return false;
      if (!keyword) return true;
      return row.searchText.includes(keyword);
    });
}

function mapAuditEventRow(event: AuditEvent): AuditEventRow {
  const result = deriveResult(event.action, event.diff);
  const objectLabel = `${event.resource_type} / ${event.resource_id}`;
  return {
    id: String(event.id),
    occurredAt: event.occurred_at,
    actorName: event.actor.actor_name,
    actorId: event.actor.actor_id,
    action: event.action,
    resourceType: event.resource_type,
    resourceId: event.resource_id,
    objectLabel,
    result,
    resultLabel: result === "success" ? "成功" : "失败",
    traceId: event.trace_id,
    searchText: [
      event.action,
      event.actor.actor_name,
      event.actor.actor_id,
      event.resource_type,
      event.resource_id,
      event.trace_id,
    ]
      .join(" ")
      .toLowerCase(),
  };
}

function deriveResult(action: string, diff: Record<string, unknown>): "success" | "failed" {
  const text = `${action} ${JSON.stringify(diff)}`.toLowerCase();
  if (/(fail|failed|reject|rejected|error|驳回|失败)/.test(text)) return "failed";
  return "success";
}

function emptyToUndefined(value?: string) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}
