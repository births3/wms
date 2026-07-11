import { useQuery } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type AuditEvent = components["schemas"]["AuditEvent"];

export interface AuditEventQueryParams {
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
  traceId: string;
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
  return result.data.data.map(mapAuditEventRow);
}

function mapAuditEventRow(event: AuditEvent): AuditEventRow {
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
    traceId: event.trace_id,
  };
}

function emptyToUndefined(value?: string) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}
