import { useState, type FormEvent } from "react";
import { FileCheck2, KeyRound, LockKeyhole, UserRound } from "lucide-react";
import { Button, Input, Label } from "@wms/ui";

import { login } from "./api";
import type { LoginResponse } from "./types";

export function LoginPage(props: { onLogin: (session: LoginResponse) => void }) {
  const [username, setUsername] = useState(
    import.meta.env.DEV ? import.meta.env.VITE_PORTAL_DEV_USERNAME ?? "" : "",
  );
  const [password, setPassword] = useState(
    import.meta.env.DEV ? import.meta.env.VITE_PORTAL_DEV_PASSWORD ?? "" : "",
  );
  const [error, setError] = useState("");
  const [pending, setPending] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError("");
    setPending(true);
    try {
      props.onLogin(await login(username, password));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : "登录失败");
    } finally {
      setPending(false);
    }
  };

  return (
    <main className="login-shell portal-root">
      <section className="login-brand">
        <div className="login-brand-lockup">
          <div className="portal-brand-mark portal-brand-mark-large">
            <FileCheck2 className="size-7" />
          </div>
          <div>
            <div className="login-brand-name">药检资料服务平台</div>
            <div className="login-brand-caption">独立部署 · 订单授权 · 全程留痕</div>
          </div>
        </div>
        <div className="login-hero-copy">
          <div className="login-kicker">CUSTOMER DOCUMENT PORTAL</div>
          <h1>
            药检资料，
            <span>清楚、安全、随时可查。</span>
          </h1>
          <p>
            查询已发货和已签收订单中的药检单，查看当前版本、资料处理状态，并安全完成单份或批量下载。
          </p>
        </div>
        <div className="login-points">
          <BrandPoint icon={<LockKeyhole className="size-4" />} text="客户独立认证" />
          <BrandPoint icon={<UserRound className="size-4" />} text="多账号地址范围" />
          <BrandPoint icon={<KeyRound className="size-4" />} text="15 分钟下载授权" />
        </div>
      </section>

      <section className="login-form-pane">
        <form
          className="login-card"
          onSubmit={submit}
        >
          <div className="login-card-icon">
            <LockKeyhole className="size-5" />
          </div>
          <div className="portal-eyebrow">客户登录</div>
          <h2>访问订单资料</h2>
          <p className="login-card-description">
            使用客户管理员分配的独立平台账号。
          </p>

          <div className="login-fields">
            <div className="space-y-2">
              <Label htmlFor="username">用户名</Label>
              <Input
                id="username"
                autoComplete="username"
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                placeholder="请输入用户名"
                data-testid="portal-username"
                className="h-11"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">密码</Label>
              <Input
                id="password"
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                placeholder="请输入密码"
                data-testid="portal-password"
                className="h-11"
              />
            </div>
          </div>
          {error ? (
            <div
              role="alert"
              className="mt-4 rounded-xl bg-red-50 px-3 py-2 text-sm text-red-700"
            >
              {error}
            </div>
          ) : null}
          <Button
            type="submit"
            className="login-submit"
            disabled={pending || !username.trim() || !password}
            data-testid="portal-login"
          >
            {pending ? "正在验证…" : "登录平台"}
          </Button>
          <p className="login-audit-note">
            登录、查询和下载均会写入客户平台审计记录。
          </p>
        </form>
      </section>
    </main>
  );
}

function BrandPoint(props: { icon: React.ReactNode; text: string }) {
  return (
    <div className="login-point">
      <span>{props.icon}</span>
      <span>{props.text}</span>
    </div>
  );
}
