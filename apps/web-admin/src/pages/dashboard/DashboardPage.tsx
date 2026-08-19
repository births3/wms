import * as React from "react";
import { Activity, CheckCircle2, ClipboardList, History, Layers, PackageCheck } from "lucide-react";
import { Button, Card, CardContent, DashboardPageTemplate } from "@wms/ui";

import type { AdminView } from "@/app-shell/admin-view";
import type { CurrentUser } from "@/features/auth/auth-queries";
import { COLUMN_OWNER } from "@/lib/ui-strings";

const operationKpis = [
  { id: "pending-receiving", label: "待收货", hint: "待办接口未接入", icon: PackageCheck },
  { id: "pending-inspecting", label: "待验收", hint: "待办接口未接入", icon: CheckCircle2 },
  { id: "pending-putaway", label: "待上架", hint: "待办接口未接入", icon: Layers },
  { id: "pending-review", label: "待复核", hint: "待办接口未接入", icon: History },
] as const;

interface DashboardProps {
  currentUser: CurrentUser;
  availableViews: ReadonlySet<AdminView>;
  onOpenM2Inbound: () => void;
  onOpenM4Outbound: () => void;
  onOpenM3Batches: () => void;
  onOpenH2Audit: () => void;
}

export function Dashboard({
  currentUser,
  availableViews,
  onOpenM2Inbound,
  onOpenM4Outbound,
  onOpenM3Batches,
  onOpenH2Audit,
}: DashboardProps) {
  return (
    <DashboardPageTemplate
      kpiSlot={
        <div className="grid gap-4 lg:grid-cols-[18rem_1fr]">
          <Card className="rounded-lg shadow-sm">
            <CardContent className="flex flex-col gap-4 p-5">
              <div>
                <p className="text-xs font-medium text-muted-foreground">当前用户</p>
                <h2 className="mt-2 text-lg font-semibold tracking-normal">{currentUser.display_name}</h2>
                <p className="mt-1 text-sm text-muted-foreground">{currentUser.username}</p>
              </div>
              <div className="grid gap-3 text-sm">
                <InfoRow label={COLUMN_OWNER} value={currentUser.owner_code} />
                <InfoRow label="角色" value={currentUser.roles.join(" / ") || "未分配"} />
                <InfoRow label="权限数" value={`${currentUser.permissions.length}`} />
              </div>
            </CardContent>
          </Card>

          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            {operationKpis.map((item) => {
              const Icon = item.icon;
              return (
                <Card key={item.id} className="rounded-lg shadow-sm">
                  <CardContent className="flex h-full flex-col gap-3 p-5">
                    <div className="flex items-center justify-between gap-3">
                      <div className="flex size-10 items-center justify-center rounded-md bg-primary/10 text-primary">
                        <Icon className="size-5" aria-hidden />
                      </div>
                      <p className="text-xs font-medium text-muted-foreground">{item.hint}</p>
                    </div>
                    <div>
                      <p className="text-sm text-muted-foreground">{item.label}</p>
                      <p className="mt-1 text-lg font-semibold tracking-tight">未接入</p>
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </div>
      }
      mainSlot={
        <Card className="rounded-lg shadow-sm">
          <CardContent className="grid gap-4 p-5 md:grid-cols-[auto_1fr]">
            <div className="flex size-10 items-center justify-center rounded-md bg-wms-success/10 text-wms-success">
              <Activity className="size-5" aria-hidden />
            </div>
            <div className="flex flex-col gap-4">
              <div>
                <h2 className="text-lg font-semibold tracking-normal">快捷入口</h2>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">进入常用作业页面，处理收货、出库、批号与审计查询</p>
              </div>
              <div className="flex flex-wrap gap-3">
                {availableViews.has("m2-receiving") ? <Button type="button" onClick={onOpenM2Inbound}><PackageCheck className="size-4" aria-hidden />M2 收货管理</Button> : null}
                {availableViews.has("m4-orders") ? <Button type="button" variant="outline" onClick={onOpenM4Outbound}><ClipboardList className="size-4" aria-hidden />M4 出库订单</Button> : null}
                {availableViews.has("m3-batches") ? <Button type="button" variant="outline" onClick={onOpenM3Batches}><Layers className="size-4" aria-hidden />M3 批号管理</Button> : null}
                {availableViews.has("h2-audit-trail") ? <Button type="button" variant="outline" onClick={onOpenH2Audit}><History className="size-4" aria-hidden />H2 审计</Button> : null}
                {availableViews.size === 1 ? <span className="text-sm text-muted-foreground">当前用户没有可用快捷入口</span> : null}
              </div>
            </div>
          </CardContent>
        </Card>
      }
    />
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 border-t border-border pt-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="truncate font-medium">{value}</span>
    </div>
  );
}
