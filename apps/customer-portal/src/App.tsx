import { useState } from "react";
import {
  Archive,
  Building2,
  FileCheck2,
  LogOut,
  PackageSearch,
  ShieldCheck,
  Users,
} from "lucide-react";
import { Button } from "@wms/ui";

import { LoginPage } from "./LoginPage";
import { OrdersPage } from "./OrdersPage";
import { ExportsPage } from "./ExportsPage";
import { UsersPage } from "./UsersPage";
import type { LoginResponse } from "./types";

const SESSION_KEY = "wms-customer-portal-session";

type PageKey = "orders" | "exports" | "users";

function readSession(): LoginResponse | null {
  const raw = sessionStorage.getItem(SESSION_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as LoginResponse;
  } catch {
    sessionStorage.removeItem(SESSION_KEY);
    return null;
  }
}

export function App() {
  const [session, setSession] = useState<LoginResponse | null>(readSession);
  const [page, setPage] = useState<PageKey>("orders");

  if (!session) {
    return (
      <LoginPage
        onLogin={(next) => {
          sessionStorage.setItem(SESSION_KEY, JSON.stringify(next));
          setSession(next);
        }}
      />
    );
  }

  const logout = () => {
    sessionStorage.removeItem(SESSION_KEY);
    setSession(null);
  };
  const isAdmin = session.user.role === "customer_admin";

  return (
    <div className="min-h-screen bg-[#f3f6f4] text-slate-900">
      <header className="portal-header">
        <div className="flex items-center gap-3">
          <div className="grid size-10 place-items-center rounded-xl bg-white/15">
            <FileCheck2 className="size-6" />
          </div>
          <div>
            <div className="text-lg font-semibold tracking-wide">药检资料服务平台</div>
            <div className="text-xs text-emerald-100">订单范围内的合规资料查询与下载</div>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <div className="hidden text-right sm:block">
            <div className="text-sm font-medium">{session.user.display_name}</div>
            <div className="text-xs text-emerald-100">
              {isAdmin ? "客户管理员 · 全部地址" : "客户账号 · 授权地址"}
            </div>
          </div>
          <Button
            type="button"
            variant="outline"
            className="border-white/30 bg-transparent text-white hover:bg-white/10 hover:text-white"
            onClick={logout}
          >
            <LogOut className="mr-2 size-4" />
            退出
          </Button>
        </div>
      </header>

      <div className="mx-auto grid max-w-[1480px] gap-6 px-4 py-6 lg:grid-cols-[220px_minmax(0,1fr)] lg:px-8">
        <aside className="h-fit rounded-2xl border border-slate-200 bg-white p-3 shadow-sm">
          <div className="mb-3 flex items-center gap-2 rounded-xl bg-emerald-50 p-3 text-sm text-emerald-900">
            <Building2 className="size-4" />
            <span className="truncate">客户资料空间</span>
          </div>
          <nav className="space-y-1" aria-label="客户平台导航">
            <NavButton
              active={page === "orders"}
              icon={<PackageSearch className="size-4" />}
              label="订单与药检单"
              onClick={() => setPage("orders")}
            />
            <NavButton
              active={page === "exports"}
              icon={<Archive className="size-4" />}
              label="导出中心"
              onClick={() => setPage("exports")}
            />
            {isAdmin ? (
              <NavButton
                active={page === "users"}
                icon={<Users className="size-4" />}
                label="客户账号"
                onClick={() => setPage("users")}
              />
            ) : null}
          </nav>
          <div className="mt-5 rounded-xl border border-slate-200 bg-slate-50 p-3 text-xs leading-5 text-slate-600">
            <div className="mb-1 flex items-center gap-1.5 font-medium text-slate-800">
              <ShieldCheck className="size-4 text-emerald-700" />
              数据范围受控
            </div>
            仅显示本账号获授权地址下已发货或已签收订单，不支持按批号跨订单检索。
          </div>
        </aside>

        <main className="min-w-0">
          {page === "orders" ? (
            <OrdersPage session={session} onOpenExports={() => setPage("exports")} />
          ) : null}
          {page === "exports" ? <ExportsPage session={session} /> : null}
          {page === "users" && isAdmin ? <UsersPage session={session} /> : null}
        </main>
      </div>
    </div>
  );
}

function NavButton(props: {
  active: boolean;
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`flex w-full items-center gap-2 rounded-xl px-3 py-2.5 text-left text-sm font-medium transition ${
        props.active
          ? "bg-emerald-700 text-white shadow-sm"
          : "text-slate-700 hover:bg-slate-100"
      }`}
      onClick={props.onClick}
    >
      {props.icon}
      {props.label}
    </button>
  );
}
