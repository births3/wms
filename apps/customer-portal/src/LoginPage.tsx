import { useState, type FormEvent } from "react";
import { FileCheck2, KeyRound, LockKeyhole, UserRound } from "lucide-react";
import { Button, Input, Label } from "@wms/ui";

import { login } from "./api";
import type { LoginResponse } from "./types";

export function LoginPage(props: { onLogin: (session: LoginResponse) => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
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
    <main className="login-shell">
      <section className="login-brand">
        <div className="inline-flex items-center gap-3 rounded-2xl bg-white/10 px-4 py-3">
          <FileCheck2 className="size-8" />
          <div>
            <div className="text-xl font-semibold">药检资料服务平台</div>
            <div className="text-sm text-emerald-100">独立、安全、按订单授权</div>
          </div>
        </div>
        <div className="max-w-xl">
          <h1 className="mt-10 text-4xl font-semibold leading-tight">
            每一份药检资料，
            <br />
            都有明确的订单边界。
          </h1>
          <p className="mt-5 max-w-lg text-base leading-7 text-emerald-50/90">
            查询已发货和已签收订单中的药检单，查看当前版本、资料处理状态，并安全完成单份或批量下载。
          </p>
        </div>
        <div className="grid gap-3 text-sm text-emerald-50 sm:grid-cols-3">
          <BrandPoint icon={<LockKeyhole className="size-4" />} text="客户独立认证" />
          <BrandPoint icon={<UserRound className="size-4" />} text="多账号地址范围" />
          <BrandPoint icon={<KeyRound className="size-4" />} text="15 分钟下载授权" />
        </div>
      </section>

      <section className="grid place-items-center bg-[#f3f6f4] px-6 py-10">
        <form
          className="w-full max-w-md rounded-3xl border border-slate-200 bg-white p-8 shadow-xl shadow-slate-900/5"
          onSubmit={submit}
        >
          <div className="text-sm font-medium text-emerald-700">客户登录</div>
          <h2 className="mt-1 text-2xl font-semibold">访问您的订单资料</h2>
          <p className="mt-2 text-sm leading-6 text-slate-500">
            使用客户管理员分配的独立平台账号。
          </p>

          <div className="mt-7 space-y-5">
            <div className="space-y-2">
              <Label htmlFor="username">用户名</Label>
              <Input
                id="username"
                autoComplete="username"
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                placeholder="请输入用户名"
                data-testid="portal-username"
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
            className="mt-6 h-11 w-full bg-emerald-700 hover:bg-emerald-800"
            disabled={pending || !username.trim() || !password}
            data-testid="portal-login"
          >
            {pending ? "正在验证…" : "登录平台"}
          </Button>
          <p className="mt-5 text-center text-xs leading-5 text-slate-400">
            登录、查询和下载均会写入客户平台审计记录。
          </p>
        </form>
      </section>
    </main>
  );
}

function BrandPoint(props: { icon: React.ReactNode; text: string }) {
  return (
    <div className="flex items-center gap-2 rounded-xl bg-white/10 px-3 py-2.5">
      {props.icon}
      {props.text}
    </div>
  );
}
