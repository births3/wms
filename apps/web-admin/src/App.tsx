import { Card, CardContent, StatusBadge } from "@wms/ui";
import { Activity, ClipboardList, KeyRound, ShieldCheck } from "lucide-react";

const foundations = [
  {
    id: "h1",
    title: "H1 权限与多租户",
    description: "AuthContext、JWT、多货主隔离和权限门控的生产入口。",
    status: "pending" as const,
    icon: ShieldCheck,
  },
  {
    id: "h2",
    title: "H2 审计追踪",
    description: "写操作接入 append-only 审计事件和审计查询链路。",
    status: "pending" as const,
    icon: ClipboardList,
  },
  {
    id: "h3",
    title: "H3 OpenAPI 契约",
    description: "后端 utoipa 生成 OpenAPI，前端通过 api-client 消费。",
    status: "pending" as const,
    icon: KeyRound,
  },
];

export function App() {
  return (
    <main className="min-h-screen bg-muted/30 text-foreground">
      <section className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-8">
        <header className="flex flex-col gap-3 border-b border-border pb-6">
          <div className="flex flex-wrap items-center gap-3">
            <StatusBadge status="in_progress" label="Wave 1 shell" />
            <span className="text-sm text-muted-foreground">ADR-0029: 原型走查后迁移生产</span>
          </div>
          <div>
            <h1 className="text-3xl font-semibold tracking-normal">WMS Web Admin</h1>
            <p className="mt-2 max-w-3xl text-base leading-7 text-muted-foreground">
              生产管理端壳工程只承载 H1/H2/H3 横向底座；业务页面从 prototypes 迁移前必须完成 checklist。
            </p>
          </div>
        </header>

        <div className="grid gap-4 md:grid-cols-3">
          {foundations.map((item) => {
            const Icon = item.icon;
            return (
              <Card key={item.id} className="rounded-lg shadow-sm">
                <CardContent className="flex h-full flex-col gap-4 p-5">
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex size-10 items-center justify-center rounded-md bg-primary/10 text-primary">
                      <Icon className="size-5" aria-hidden />
                    </div>
                    <StatusBadge status={item.status} label="待接入" size="sm" />
                  </div>
                  <div>
                    <h2 className="text-lg font-semibold tracking-normal">{item.title}</h2>
                    <p className="mt-2 text-sm leading-6 text-muted-foreground">{item.description}</p>
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>

        <Card className="rounded-lg shadow-sm">
          <CardContent className="grid gap-4 p-5 md:grid-cols-[auto_1fr]">
            <div className="flex size-10 items-center justify-center rounded-md bg-wms-success/10 text-wms-success">
              <Activity className="size-5" aria-hidden />
            </div>
            <div>
              <h2 className="text-lg font-semibold tracking-normal">生产边界</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                此壳工程不复制 prototypes 页面、不保留 mock-only 流程、不绕过 OpenAPI。业务页迁移时按
                docs/prototypes/prototype-to-production.md 验收。
              </p>
            </div>
          </CardContent>
        </Card>
      </section>
    </main>
  );
}
