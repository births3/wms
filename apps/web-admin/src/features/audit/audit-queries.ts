import { useQuery } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

export type AuditEvent = components["schemas"]["AuditEvent"];

export interface AuditEventQueryParams {
  resourceType?: string;
  action?: string;
  resourceId?: string;
  productCode?: string;
  batchNo?: string;
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
  ipAddress: string;
  objectLabel: string;
  diffBefore: string;
  diffAfter: string;
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
        action: emptyToUndefined(params.action),
        resource_id: emptyToUndefined(params.resourceId),
        product_code: emptyToUndefined(params.productCode),
        batch_no: emptyToUndefined(params.batchNo),
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
    ipAddress: event.ip ?? "-",
    objectLabel,
    diffBefore: formatDiffValue(event.diff, "before"),
    diffAfter: formatDiffValue(event.diff, "after"),
    traceId: event.trace_id,
  };
}

function formatDiffValue(diff: AuditEvent["diff"], key: "before" | "after") {
  if (!diff || typeof diff !== "object" || Array.isArray(diff)) return "-";
  const record = diff as Record<string, unknown>;
  const value = record[key] ?? Object.fromEntries(
    Object.entries(record)
      .filter(([, item]) => item && typeof item === "object" && key in (item as object))
      .map(([field, item]) => [field, (item as Record<string, unknown>)[key]]),
  );
  return value === undefined ? "-" : JSON.stringify(value);
}

function emptyToUndefined(value?: string) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}
