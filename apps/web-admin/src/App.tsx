import * as React from "react";
import { Button, Card, CardContent, PageHeader, StatusBadge } from "@wms/ui";
import { Activity, ClipboardList, KeyRound, LogOut, RefreshCw, ShieldCheck } from "lucide-react";

import { useCurrentUserQuery, useLogout, type CurrentUser } from "@/features/auth/auth-queries";
import { apiBaseUrl, wave1ContractPaths } from "@/lib/api";
import { clearAuthSession, hasActiveAuthSession } from "@/lib/auth-session";
import { LoginPage } from "@/pages/auth/LoginPage";

const foundations = [
  {
    id: "h1",
    title: "H1 权限与多租户",
    description: "JWT 登录、AuthContext、货主隔离和权限集合。",
    status: "completed" as const,
    label: "可登录",
    meta: "login / me",
    icon: ShieldCheck,
  },
  {
    id: "h2",
    title: "H2 审计追踪",
    description: "登录和写操作进入 append-only 审计链路。",
    status: "completed" as const,
    label: "已接入",
    meta: "audit events",
    icon: ClipboardList,
  },
  {
    id: "h3",
    title: "H3 OpenAPI 契约",
    description: "前端通过 @wms/api-client 消费后端契约。",
    status: "completed" as const,
    label: "已同步",
    meta: `${wave1ContractPaths.length} 条基础路径`,
    icon: KeyRound,
  },
];

export function App() {
  const logout = useLogout();
  const [sessionVersion, setSessionVersion] = React.useState(0);
  const hasSession = React.useMemo(() => hasActiveAuthSession(), [sessionVersion]);
  const currentUserQuery = useCurrentUserQuery(hasSession);

  React.useEffect(() => {
    if (hasSession && currentUserQuery.isError) {
      clearAuthSession();
      setSessionVersion((value) => value + 1);
    }
  }, [currentUserQuery.isError, hasSession]);

  if (!hasSession) {
    return <LoginPage onLoggedIn={() => setSessionVersion((value) => value + 1)} />;
  }

  if (currentUserQuery.isPending) {
    return <LoadingShell />;
  }

  if (currentUserQuery.isError || !currentUserQuery.data) {
    return (
      <LoginPage
        onLoggedIn={() => setSessionVersion((value) => value + 1)}
        sessionMessage="登录状态已失效，请重新登录"
      />
    );
  }

  return (
    <Dashboard
      currentUser={currentUserQuery.data}
      onLogout={() => {
        logout();
        setSessionVersion((value) => value + 1);
      }}
    />
  );
}

function LoadingShell() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-muted/40 text-foreground">
      <div className="flex items-center gap-3 text-sm text-muted-foreground">
        <RefreshCw className="size-4 animate-spin" aria-hidden />
        加载当前用户
      </div>
    </main>
  );
}

interface DashboardProps {
  currentUser: CurrentUser;
  onLogout: () => void;
}

function Dashboard({ currentUser, onLogout }: DashboardProps) {
  return (
    <main className="min-h-screen bg-muted/30 text-foreground">
      <section className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-8">
        <PageHeader
          title="WMS Web Admin"
          subtitle={`${currentUser.owner_code} / ${currentUser.display_name}`}
          actions={
            <Button type="button" variant="outline" onClick={onLogout}>
              <LogOut className="size-4" aria-hidden />
              退出
            </Button>
          }
        />

        <div className="grid gap-4 lg:grid-cols-[18rem_1fr]">
          <Card className="rounded-lg shadow-sm">
            <CardContent className="flex flex-col gap-4 p-5">
              <div>
                <p className="text-xs font-medium text-muted-foreground">当前用户</p>
                <h2 className="mt-2 text-lg font-semibold tracking-normal">{currentUser.display_name}</h2>
                <p className="mt-1 text-sm text-muted-foreground">{currentUser.username}</p>
              </div>
              <div className="grid gap-3 text-sm">
                <InfoRow label="货主" value={currentUser.owner_code} />
                <InfoRow label="角色" value={currentUser.roles.join(" / ") || "未分配"} />
                <InfoRow label="权限数" value={`${currentUser.permissions.length}`} />
              </div>
            </CardContent>
          </Card>

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
                      <StatusBadge status={item.status} label={item.label} size="sm" />
                    </div>
                    <div>
                      <h2 className="text-lg font-semibold tracking-normal">{item.title}</h2>
                      <p className="mt-2 text-sm leading-6 text-muted-foreground">{item.description}</p>
                      <p className="mt-3 text-xs font-medium text-muted-foreground">{item.meta}</p>
                    </div>
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </div>

        <Card className="rounded-lg shadow-sm">
          <CardContent className="grid gap-4 p-5 md:grid-cols-[auto_1fr]">
            <div className="flex size-10 items-center justify-center rounded-md bg-wms-success/10 text-wms-success">
              <Activity className="size-5" aria-hidden />
            </div>
            <div>
              <h2 className="text-lg font-semibold tracking-normal">运行入口</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                API 基址：{apiBaseUrl || "当前域名"}
              </p>
            </div>
          </CardContent>
        </Card>
      </section>
    </main>
  );
}

interface InfoRowProps {
  label: string;
  value: string;
}

function InfoRow({ label, value }: InfoRowProps) {
  return (
    <div className="flex items-center justify-between gap-3 border-t border-border pt-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="truncate font-medium">{value}</span>
    </div>
  );
}
