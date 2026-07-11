/**
 * LoginPage — WMS 管理端登录页
 *
 * 层级：Layer 3 页面
 * 关联故事：US-H1-001
 * 业务约束：密码只存在于表单状态；提交后通过 @wms/api-client 调用登录接口。
 *
 * @example
 *   <LoginPage onLoggedIn={() => void 0} />
 */

import * as React from "react";
import { Button, Card, CardContent, CardHeader, CardTitle, Input, Label } from "@wms/ui";
import { Eye, EyeOff, Loader2, LogIn, ShieldCheck } from "lucide-react";

import { ApiError, useLoginMutation } from "@/features/auth/auth-queries";
import { apiBaseUrl } from "@/lib/api";

export interface LoginPageProps {
  onLoggedIn: () => void;
  sessionMessage?: string;
}

export function LoginPage({ onLoggedIn, sessionMessage }: LoginPageProps) {
  const loginMutation = useLoginMutation();
  const devLogin = __WMS_WEB_ADMIN_DEV_LOGIN__;
  const [ownerCode, setOwnerCode] = React.useState(devLogin.enabled ? devLogin.ownerCode : "");
  const [username, setUsername] = React.useState(devLogin.enabled ? devLogin.username : "");
  const [password, setPassword] = React.useState(devLogin.enabled ? devLogin.password : "");
  const [showPassword, setShowPassword] = React.useState(false);

  const canSubmit = ownerCode.trim() !== "" && username.trim() !== "" && password !== "";

  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit || loginMutation.isPending) {
      return;
    }
    loginMutation.mutate(
      {
        owner_code: ownerCode.trim(),
        username: username.trim(),
        password,
      },
      {
        onSuccess: () => {
          setPassword("");
          setShowPassword(false);
          onLoggedIn();
        },
        onError: () => {
          setPassword("");
          setShowPassword(false);
        },
      }
    );
  }

  const errorMessage =
    loginMutation.error instanceof ApiError ? loginMutation.error.message : sessionMessage;

  return (
    <main className="flex min-h-screen items-center justify-center bg-muted/40 px-4 py-8 text-foreground">
      <section className="grid w-full max-w-5xl gap-6 md:grid-cols-[minmax(0,1fr)_24rem]">
        <div className="flex min-h-[28rem] flex-col justify-between rounded-lg border border-border bg-background p-8">
          <div className="flex items-center gap-3">
            <div className="flex size-11 items-center justify-center rounded-md bg-primary text-primary-foreground">
              <ShieldCheck className="size-5" aria-hidden />
            </div>
            <div>
              <h1 className="text-2xl font-semibold tracking-normal">WMS Web Admin</h1>
              <p className="mt-1 text-sm text-muted-foreground">医药仓储管理端</p>
            </div>
          </div>
          <div className="max-w-xl">
            <h2 className="text-3xl font-semibold tracking-normal">货主隔离登录</h2>
            <p className="mt-4 text-base leading-7 text-muted-foreground">
              使用货主编码、账号与密码进入管理端。登录后系统按当前货主与角色权限加载工作台。
            </p>
          </div>
          {devLogin.enabled ? (
            <p className="text-xs text-muted-foreground/70">
              开发调试 · API 基址：{apiBaseUrl || "当前域名"}
            </p>
          ) : (
            <div aria-hidden className="h-4" />
          )}
        </div>

        <Card className="rounded-lg shadow-sm">
          <CardHeader>
            <CardTitle className="text-xl">登录</CardTitle>
          </CardHeader>
          <CardContent>
            <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
              <div className="flex flex-col gap-2">
                <Label htmlFor="owner-code">货主编码</Label>
                <Input
                  id="owner-code"
                  autoComplete="organization"
                  placeholder="例如 PY_OWNER"
                  value={ownerCode}
                  onChange={(event) => setOwnerCode(event.target.value)}
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="username">登录账号</Label>
                <Input
                  id="username"
                  autoComplete="username"
                  placeholder="请输入登录账号"
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                />
              </div>
              <div className="flex flex-col gap-2">
                <Label htmlFor="password">密码</Label>
                <div className="relative">
                  <Input
                    id="password"
                    type={showPassword ? "text" : "password"}
                    autoComplete="current-password"
                    placeholder="请输入密码"
                    className="pr-10"
                    value={password}
                    onChange={(event) => setPassword(event.target.value)}
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="absolute right-1 top-1/2 size-8 -translate-y-1/2 text-muted-foreground"
                    aria-label={showPassword ? "隐藏密码" : "显示密码"}
                    aria-pressed={showPassword}
                    onClick={() => setShowPassword((value) => !value)}
                  >
                    {showPassword ? (
                      <EyeOff className="size-4" aria-hidden />
                    ) : (
                      <Eye className="size-4" aria-hidden />
                    )}
                  </Button>
                </div>
              </div>

              {(errorMessage || sessionMessage) && (
                <p className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">
                  {errorMessage ?? sessionMessage}
                </p>
              )}

              <Button type="submit" size="lg" disabled={!canSubmit || loginMutation.isPending}>
                {loginMutation.isPending ? (
                  <Loader2 className="size-4 animate-spin" aria-hidden />
                ) : (
                  <LogIn className="size-4" aria-hidden />
                )}
                登录
              </Button>
            </form>
          </CardContent>
        </Card>
      </section>
    </main>
  );
}
