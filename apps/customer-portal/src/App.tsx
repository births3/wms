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
    <div className="portal-root">
      <header className="portal-header">
        <div className="portal-header-inner">
          <div className="portal-brand">
            <div className="portal-brand-mark">
              <FileCheck2 className="size-6" />
            </div>
            <div>
              <div className="portal-brand-title">药检资料服务平台</div>
              <div className="portal-brand-caption">订单范围内的合规资料查询与下载</div>
            </div>
          </div>

          <div className="portal-account">
            <div className="portal-account-avatar" aria-hidden="true">
              {session.user.display_name.slice(0, 1)}
            </div>
            <div className="portal-account-copy">
              <div className="portal-account-name">{session.user.display_name}</div>
              <div className="portal-account-role">
                {isAdmin ? "客户管理员 · 全部地址" : "客户账号 · 授权地址"}
              </div>
            </div>
            <Button
              type="button"
              variant="outline"
              className="portal-logout-button border-white/30 bg-transparent text-white hover:bg-white/10 hover:text-white"
              onClick={logout}
            >
              <LogOut className="mr-2 size-4" />
              退出
            </Button>
          </div>
        </div>
      </header>

      <div className="portal-workspace">
        <aside className="portal-sidebar">
          <div className="portal-space-label">
            <Building2 className="size-4" />
            <span className="truncate">客户资料空间</span>
          </div>
          <nav className="portal-navigation" aria-label="客户平台导航">
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
          <div className="portal-scope-card">
            <div className="portal-scope-title">
              <ShieldCheck className="size-4" />
              数据范围受控
            </div>
            仅显示本账号获授权地址下已发货或已签收订单，不支持按批号跨订单检索。
          </div>
        </aside>

        <main className="portal-main">
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
      className="portal-nav-button"
      data-active={props.active}
      aria-current={props.active ? "page" : undefined}
      onClick={props.onClick}
    >
      <span className="portal-nav-icon">{props.icon}</span>
      <span>{props.label}</span>
    </button>
  );
}
