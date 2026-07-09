import { Card, CardContent, PageHeader, StatusBadge } from "@wms/ui";
import { ClipboardList, FileJson, ShieldCheck } from "lucide-react";

const h2AuditItems = [
  ["审计查询接口", "GET /api/v1/audit/events"],
  ["审计归档接口", "GET /api/v1/audit/archive/partitions"],
  ["事件总线接口", "GET /api/v1/event-bus/deliveries/pending"],
];

const h3ContractItems = [
  ["OpenAPI Spec", "GET /openapi.json"],
  ["Swagger UI", "GET /api-docs"],
  ["ReDoc", "GET /redoc"],
];

export function H2AuditTrailPage() {
  return (
    <PlatformPage
      title="H2 审计追踪"
      subtitle="append-only 审计、查询接口、归档保留与事件投递"
      statusLabel="已接入"
      items={h2AuditItems}
    />
  );
}

export function H3ApiContractPage() {
  return (
    <PlatformPage
      title="H3 OpenAPI 契约"
      subtitle="OpenAPI 文档、前端类型同步和运行时契约入口"
      statusLabel="已同步"
      items={h3ContractItems}
    />
  );
}

function PlatformPage({
  title,
  subtitle,
  statusLabel,
  items,
}: {
  title: string;
  subtitle: string;
  statusLabel: string;
  items: string[][];
}) {
  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader title={title} subtitle={subtitle} />
      <div className="grid gap-4 lg:grid-cols-[20rem_1fr]">
        <Card className="rounded-lg shadow-sm">
          <CardContent className="space-y-4 p-5">
            <div className="flex items-center justify-between gap-3">
              <div className="flex size-10 items-center justify-center rounded-md bg-primary/10 text-primary">
                <ShieldCheck className="size-5" aria-hidden />
              </div>
              <StatusBadge status="completed" label={statusLabel} size="sm" />
            </div>
            <div>
              <h2 className="text-base font-semibold tracking-normal">基础能力状态</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">该页面用于确认横向能力菜单入口、路由和契约证据可达。</p>
            </div>
          </CardContent>
        </Card>

        <Card className="rounded-lg shadow-sm">
          <CardContent className="space-y-3 p-5">
            <h2 className="text-base font-semibold tracking-normal">能力入口</h2>
            <div className="grid gap-3 md:grid-cols-3">
              {items.map(([label, value]) => (
                <div key={value} className="rounded-md border bg-background px-3 py-3">
                  <div className="flex items-center gap-2 text-sm font-medium">
                    {label.includes("OpenAPI") || label.includes("Swagger") || label.includes("ReDoc")
                      ? <FileJson className="size-4 text-primary" aria-hidden />
                      : <ClipboardList className="size-4 text-primary" aria-hidden />}
                    {label}
                  </div>
                  <div className="mt-2 truncate font-mono text-xs text-muted-foreground">{value}</div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </section>
  );
}
