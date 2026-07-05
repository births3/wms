import * as React from "react";
import { Button, Card, CardContent, Input, PageHeader, StatusBadge, WorkspaceTabs, cn } from "@wms/ui";
import {
  Activity,
  BookOpen,
  ChevronDown,
  CheckCircle2,
  ClipboardList,
  KeyRound,
  Layers,
  LogOut,
  MapPinned,
  PackageCheck,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  Search,
  ShieldCheck,
  Users,
  Warehouse,
  type LucideIcon,
} from "lucide-react";

import { useCurrentUserQuery, useLogout, type CurrentUser } from "@/features/auth/auth-queries";
import { apiBaseUrl, wave1ContractPaths } from "@/lib/api";
import { clearAuthSession, hasActiveAuthSession } from "@/lib/auth-session";
import { LoginPage } from "@/pages/auth/LoginPage";
import { FeatureFlagConfigCenterPage } from "@/pages/config-center/FeatureFlagConfigCenterPage";
import { M2InboundPage, type M2InboundMode } from "@/pages/inbound/M2InboundPage";
import { M3BatchManagementPage } from "@/pages/inventory/M3BatchManagementPage";
import { M1MasterDataPage, type MasterDataViewId } from "@/pages/master-data/M1MasterDataPage";
import { M4OutboundPage, type M4OutboundMode } from "@/pages/outbound/M4OutboundPage";

type AdminView =
  | "dashboard"
  | MasterDataViewId
  | "m1-feature-flags"
  | "m2-receiving"
  | "m2-inspecting"
  | "m2-putaway"
  | "m3-batches"
  | "m4-orders"
  | "m4-waves"
  | "m4-review"
  | "m4-returns";

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

const menuSections: Array<{ label: string; items: MenuItem[] }> = [
  {
    label: "工作台",
    items: [{ id: "dashboard", title: "运营总览", subtitle: "系统基础状态", icon: Activity }],
  },
  {
    label: "基础档案",
    items: [
      { id: "m1-products", title: "M1 商品档案", subtitle: "商品编码 / 规格", icon: PackageCheck },
      { id: "m1-business-partners", title: "M1 客商档案", subtitle: "供应商 / 客户门店", icon: Users },
      { id: "m1-warehouses", title: "M1 仓库管理", subtitle: "仓库 / 状态", icon: Warehouse },
      { id: "m1-zones", title: "M1 库区管理", subtitle: "库区 / 仓库", icon: MapPinned },
      { id: "m1-locations", title: "M1 库位管理", subtitle: "库位 / 容量", icon: MapPinned },
      { id: "m1-system-dictionary", title: "M1 系统字典", subtitle: "单据类型 / 特殊药品分类", icon: BookOpen },
      { id: "m1-feature-flags", title: "M1 Feature Flag", subtitle: "配置中心 / 灰度", icon: KeyRound },
    ],
  },
  {
    label: "入库业务",
    items: [
      { id: "m2-receiving", title: "M2 收货管理", subtitle: "ASN / 到货确认", icon: CheckCircle2 },
      { id: "m2-inspecting", title: "M2 验收管理", subtitle: "批号 / 效期 / 签字", icon: ClipboardList },
      { id: "m2-putaway", title: "M2 上架管理", subtitle: "库位 / 数量确认", icon: PackageCheck },
    ],
  },
  {
    label: "出库业务",
    items: [
      { id: "m4-orders", title: "M4 出库订单管理", subtitle: "订单 / 校验 / 作废", icon: ClipboardList },
      { id: "m4-waves", title: "M4 波次规划", subtitle: "波次 / 路径 / 锁定", icon: PackageCheck },
      { id: "m4-review", title: "M4 复核发货", subtitle: "复核 / 打印 / 交接", icon: CheckCircle2 },
      { id: "m4-returns", title: "M4 采购退货出库", subtitle: "退供应商 / 审批 / 发货", icon: ClipboardList },
    ],
  },
  {
    label: "库内业务",
    items: [{ id: "m3-batches", title: "M3 批号管理", subtitle: "批号 / 效期 / 库位", icon: Layers }],
  },
  {
    label: "基础能力",
    items: [
      { title: "H1 权限租户", subtitle: "已接入接口", icon: ShieldCheck, disabled: true },
      { title: "H2 审计追踪", subtitle: "已接入接口", icon: ClipboardList, disabled: true },
      { title: "H3 OpenAPI", subtitle: "契约同步", icon: KeyRound, disabled: true },
    ],
  },
];

const WORKSPACE_TABS_STORAGE_KEY = "wms:web-admin:workspace-tabs:v1";

interface AdminWorkspaceTab {
  view: AdminView;
  label: string;
  subtitle: string;
  closable: boolean;
}

interface AdminWorkspaceState {
  view: AdminView;
  openTabs: AdminWorkspaceTab[];
}

function readWorkspaceTabs(): AdminWorkspaceState {
  if (typeof window === "undefined") return defaultWorkspaceState();
  try {
    const raw = window.localStorage.getItem(WORKSPACE_TABS_STORAGE_KEY);
    if (!raw) return defaultWorkspaceState();
    const stored = JSON.parse(raw) as { activeView?: unknown; openTabs?: unknown };
    const views = Array.isArray(stored.openTabs) ? stored.openTabs.filter(isAdminView) : [];
    const openViews = normalizeWorkspaceViews(views);
    const activeView = isAdminView(stored.activeView) && openViews.includes(stored.activeView)
      ? stored.activeView
      : openViews[0];
    return { view: activeView, openTabs: openViews.map(workspaceTabForView) };
  } catch {
    return defaultWorkspaceState();
  }
}

function writeWorkspaceTabs(state: AdminWorkspaceState) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(
    WORKSPACE_TABS_STORAGE_KEY,
    JSON.stringify({ activeView: state.view, openTabs: state.openTabs.map((tab) => tab.view) }),
  );
}

function defaultWorkspaceState(): AdminWorkspaceState {
  return { view: "dashboard", openTabs: [workspaceTabForView("dashboard")] };
}

function openWorkspaceTab(state: AdminWorkspaceState, nextView: AdminView): AdminWorkspaceState {
  const exists = state.openTabs.some((tab) => tab.view === nextView);
  return {
    view: nextView,
    openTabs: exists ? state.openTabs : [...state.openTabs, workspaceTabForView(nextView)],
  };
}

function closeWorkspaceTab(state: AdminWorkspaceState, targetView: AdminView): AdminWorkspaceState {
  if (targetView === "dashboard") return state;
  const targetIndex = state.openTabs.findIndex((tab) => tab.view === targetView);
  if (targetIndex < 0) return state;
  const openTabs = state.openTabs.filter((tab) => tab.view !== targetView);
  if (state.view !== targetView) return { ...state, openTabs };
  const nextActiveIndex = Math.max(0, targetIndex - 1);
  return { view: openTabs[nextActiveIndex]?.view ?? "dashboard", openTabs };
}

function closeOtherWorkspaceTabs(state: AdminWorkspaceState, targetView: AdminView): AdminWorkspaceState {
  const targetTab = state.openTabs.find((tab) => tab.view === targetView) ?? workspaceTabForView(targetView);
  const openTabs = targetView === "dashboard"
    ? [workspaceTabForView("dashboard")]
    : [workspaceTabForView("dashboard"), targetTab];
  return { view: targetView, openTabs };
}

function normalizeWorkspaceViews(views: AdminView[]) {
  const ordered = views.filter((view, index) => views.indexOf(view) === index);
  return ["dashboard", ...ordered.filter((view) => view !== "dashboard")] as AdminView[];
}

function workspaceTabForView(view: AdminView): AdminWorkspaceTab {
  const item = menuSections.flatMap((section) => section.items).find((menuItem) => menuItem.id === view);
  return {
    view,
    label: item?.title ?? "运营总览",
    subtitle: item?.subtitle ?? "系统基础状态",
    closable: view !== "dashboard",
  };
}

function isAdminView(value: unknown): value is AdminView {
  return typeof value === "string" && menuSections.some((section) => section.items.some((item) => item.id === value));
}

export function App() {
  const logout = useLogout();
  const [workspaceState, setWorkspaceState] = React.useState<AdminWorkspaceState>(readWorkspaceTabs);
  const [sessionVersion, setSessionVersion] = React.useState(0);
  const hasSession = React.useMemo(() => hasActiveAuthSession(), [sessionVersion]);
  const currentUserQuery = useCurrentUserQuery(hasSession);
  const view = workspaceState.view;
  const openTabs = workspaceState.openTabs;

  React.useEffect(() => {
    writeWorkspaceTabs(workspaceState);
  }, [workspaceState]);

  const navigateTo = React.useCallback((nextView: AdminView) => {
    setWorkspaceState((state) => openWorkspaceTab(state, nextView));
  }, []);

  const closeTab = React.useCallback((targetView: AdminView) => {
    setWorkspaceState((state) => closeWorkspaceTab(state, targetView));
  }, []);

  const closeOtherTabs = React.useCallback((targetView: AdminView) => {
    setWorkspaceState((state) => closeOtherWorkspaceTabs(state, targetView));
  }, []);

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

  const handleLogout = () => {
    logout();
    setWorkspaceState((state) => ({ ...state, view: "dashboard" }));
    setSessionVersion((value) => value + 1);
  };
  return (
    <AppShell
      currentUser={currentUserQuery.data}
      activeView={view}
      openTabs={openTabs}
      onNavigate={navigateTo}
      onCloseTab={closeTab}
      onCloseOtherTabs={closeOtherTabs}
      onLogout={handleLogout}
    >
      {openTabs.map((tab) => (
        <div key={tab.view} hidden={tab.view !== view}>
          {renderAdminView(tab.view, currentUserQuery.data, navigateTo)}
        </div>
      ))}
    </AppShell>
  );
}

function renderAdminView(
  view: AdminView,
  currentUser: CurrentUser,
  navigateTo: (view: AdminView) => void,
) {
  const inboundMode = inboundViewToMode(view);
  const outboundMode = outboundViewToMode(view);
  const masterDataViewId = masterDataViewToId(view);

  if (view === "m1-feature-flags") {
    return <FeatureFlagConfigCenterPage onBack={() => navigateTo("dashboard")} />;
  }
  if (masterDataViewId) {
    return <M1MasterDataPage viewId={masterDataViewId} onBack={() => navigateTo("dashboard")} />;
  }
  if (inboundMode) {
    return (
      <M2InboundPage
        mode={inboundMode}
        currentOwner={{ ownerId: currentUser.owner_id, ownerCode: currentUser.owner_code }}
        onBack={() => navigateTo("dashboard")}
      />
    );
  }
  if (view === "m3-batches") {
    return <M3BatchManagementPage onBack={() => navigateTo("dashboard")} />;
  }
  if (outboundMode) {
    return <M4OutboundPage mode={outboundMode} onBack={() => navigateTo("dashboard")} />;
  }
  return (
    <Dashboard
      currentUser={currentUser}
      onOpenM2Inbound={() => navigateTo("m2-receiving")}
      onOpenM4Outbound={() => navigateTo("m4-orders")}
    />
  );
}

function inboundViewToMode(view: AdminView): M2InboundMode | null {
  if (view === "m2-receiving") return "receiving";
  if (view === "m2-inspecting") return "inspecting";
  if (view === "m2-putaway") return "putaway";
  return null;
}

function masterDataViewToId(view: AdminView): MasterDataViewId | null {
  if (
    view === "m1-products" ||
    view === "m1-business-partners" ||
    view === "m1-warehouses" ||
    view === "m1-zones" ||
    view === "m1-locations" ||
    view === "m1-system-dictionary"
  ) {
    return view;
  }
  return null;
}

function outboundViewToMode(view: AdminView): M4OutboundMode | null {
  if (view === "m4-orders") return "orders";
  if (view === "m4-waves") return "waves";
  if (view === "m4-review") return "review";
  if (view === "m4-returns") return "returns";
  return null;
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

function AppShell({
  currentUser,
  activeView,
  openTabs,
  onNavigate,
  onCloseTab,
  onCloseOtherTabs,
  onLogout,
  children,
}: {
  currentUser: CurrentUser;
  activeView: AdminView;
  openTabs: AdminWorkspaceTab[];
  onNavigate: (view: AdminView) => void;
  onCloseTab: (view: AdminView) => void;
  onCloseOtherTabs: (view: AdminView) => void;
  onLogout: () => void;
  children: React.ReactNode;
}) {
  const [menuFilter, setMenuFilter] = React.useState("");
  const [menuFilterOpen, setMenuFilterOpen] = React.useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = React.useState(false);
  const sidebarRef = React.useRef<HTMLElement | null>(null);
  const closeMenuFilter = React.useCallback(() => {
    setMenuFilter("");
    setMenuFilterOpen(false);
  }, []);
  const navigateFromMenu = React.useCallback(
    (nextView: AdminView) => {
      closeMenuFilter();
      onNavigate(nextView);
    },
    [closeMenuFilter, onNavigate],
  );
  const normalizedMenuFilter = menuFilter.trim().toLowerCase();
  const visibleMenuSections = menuSections
    .map((section) => ({
      ...section,
      items: normalizedMenuFilter
        ? section.items.filter((item) => `${item.title} ${item.subtitle}`.toLowerCase().includes(normalizedMenuFilter))
        : section.items,
    }))
    .filter((section) => section.items.length > 0);

  React.useEffect(() => {
    if (!menuFilterOpen) return;

    function closeMenuFilterOnOutsidePointer(event: PointerEvent) {
      const target = event.target instanceof Element ? event.target : null;
      if (target && sidebarRef.current?.contains(target)) return;
      closeMenuFilter();
    }

    document.addEventListener("pointerdown", closeMenuFilterOnOutsidePointer);
    return () => document.removeEventListener("pointerdown", closeMenuFilterOnOutsidePointer);
  }, [closeMenuFilter, menuFilterOpen]);

  React.useEffect(() => {
    if (!menuFilterOpen) return;

    function closeMenuFilterByKeyboard(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      closeMenuFilter();
    }

    document.addEventListener("keydown", closeMenuFilterByKeyboard);
    return () => document.removeEventListener("keydown", closeMenuFilterByKeyboard);
  }, [closeMenuFilter, menuFilterOpen]);

  return (
    <div
      className={cn(
        "min-h-screen bg-muted/30 text-foreground lg:grid lg:grid-rows-[3.5rem_1fr]",
        sidebarCollapsed ? "lg:grid-cols-[4.5rem_1fr]" : "lg:grid-cols-[14rem_1fr]",
      )}
    >
      <header
        className={cn(
          "hidden border-b bg-background lg:col-span-2 lg:grid lg:h-14 lg:items-center",
          sidebarCollapsed ? "lg:grid-cols-[4.5rem_minmax(0,1fr)_auto]" : "lg:grid-cols-[14rem_minmax(0,1fr)_auto]",
        )}
      >
        <div className={cn("flex h-full items-center gap-3 border-r px-3", sidebarCollapsed ? "justify-center" : "justify-between")}>
          {!sidebarCollapsed && (
            <div className="flex min-w-0 items-center gap-2">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
                <Warehouse className="size-5" aria-hidden />
              </div>
              <div className="min-w-0">
                <div className="truncate text-sm font-semibold tracking-normal">WMS Admin</div>
                <div className="truncate text-[11px] text-muted-foreground">{currentUser.owner_code}</div>
              </div>
            </div>
          )}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={sidebarCollapsed ? "展开侧边栏" : "收缩侧边栏"}
            title={sidebarCollapsed ? "展开侧边栏" : "收缩侧边栏"}
            onClick={() => {
              setSidebarCollapsed((value) => !value);
              closeMenuFilter();
            }}
          >
            {sidebarCollapsed ? <PanelLeftOpen className="size-4" aria-hidden /> : <PanelLeftClose className="size-4" aria-hidden />}
          </Button>
        </div>

        <WorkspaceTabs
          className="min-w-0 border-0 bg-transparent"
          tabs={openTabs.map((tab) => ({
            value: tab.view,
            label: tab.label,
            subtitle: tab.subtitle,
            closable: tab.closable,
          }))}
          activeValue={activeView}
          onActiveValueChange={(nextView) => {
            if (isAdminView(nextView)) onNavigate(nextView);
          }}
          onCloseTab={(targetView) => {
            if (isAdminView(targetView)) onCloseTab(targetView);
          }}
          onCloseOtherTabs={(targetView) => {
            if (isAdminView(targetView)) onCloseOtherTabs(targetView);
          }}
        />

        <div className="flex h-full items-center gap-3 border-l px-4">
          <div className="min-w-0 text-right">
            <div className="truncate text-sm font-medium">{currentUser.display_name}</div>
            <div className="truncate text-xs text-muted-foreground">{currentUser.username}</div>
          </div>
          <Button type="button" variant="outline" size="sm" onClick={onLogout}>
            <LogOut className="size-4" aria-hidden />
            退出
          </Button>
        </div>
      </header>

      <aside ref={sidebarRef} className="hidden border-r bg-background lg:sticky lg:top-14 lg:flex lg:h-[calc(100vh-3.5rem)] lg:flex-col">

        <nav className={cn("flex-1 overflow-y-auto py-4", sidebarCollapsed ? "space-y-2 px-2" : "space-y-5 px-3")}>
          {!sidebarCollapsed && menuFilterOpen ? (
            <div className="space-y-2">
              <label className="relative block">
                <Search className="pointer-events-none absolute left-2.5 top-2.5 size-4 text-muted-foreground" aria-hidden />
                <Input
                  value={menuFilter}
                  onChange={(event) => setMenuFilter(event.target.value)}
                  placeholder="筛选菜单"
                  aria-label="筛选菜单"
                  className="h-9 pl-9 text-sm"
                  autoFocus
                />
              </label>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="w-full justify-start"
                onClick={closeMenuFilter}
              >
                收起筛选
              </Button>
            </div>
          ) : !sidebarCollapsed ? (
            <Button type="button" variant="outline" className="w-full justify-start" onClick={() => setMenuFilterOpen(true)}>
              <Search className="size-4" aria-hidden />
              筛选菜单
            </Button>
          ) : null}
          {visibleMenuSections.map((section) => (
            <MenuSection
              key={section.label}
              label={section.label}
              activeView={activeView}
              onNavigate={navigateFromMenu}
              items={section.items}
              forceOpen={Boolean(normalizedMenuFilter)}
              collapsed={sidebarCollapsed}
            />
          ))}
        </nav>
      </aside>

      <div className="min-w-0">
        <header className="flex items-center justify-between border-b bg-background px-4 py-3 lg:hidden">
          <div>
            <div className="text-base font-semibold tracking-normal">WMS Admin</div>
            <div className="text-xs text-muted-foreground">{currentUser.owner_code}</div>
          </div>
          <Button type="button" variant="outline" size="sm" onClick={onLogout}>
            <LogOut className="size-4" aria-hidden />
            退出
          </Button>
        </header>
        {children}
      </div>
    </div>
  );
}

interface MenuItem {
  id?: AdminView;
  title: string;
  subtitle: string;
  icon: LucideIcon;
  disabled?: boolean;
}

function MenuSection({
  label,
  items,
  activeView,
  onNavigate,
  forceOpen = false,
  collapsed = false,
}: {
  label: string;
  items: MenuItem[];
  activeView: AdminView;
  onNavigate: (view: AdminView) => void;
  forceOpen?: boolean;
  collapsed?: boolean;
}) {
  const hasActive = items.some((item) => item.id === activeView);
  const [open, setOpen] = React.useState(() => hasActive || label === "工作台" || label === "入库业务");
  const visible = collapsed || forceOpen || open;

  React.useEffect(() => {
    if (hasActive) setOpen(true);
  }, [hasActive]);

  if (collapsed) {
    return (
      <section aria-label={label}>
        <div className="space-y-1">
          {items.map((item) => {
            const Icon = item.icon;
            const active = item.id === activeView;
            return (
              <button
                key={item.title}
                type="button"
                aria-current={active ? "page" : undefined}
                aria-label={item.title}
                title={item.title}
                disabled={item.disabled}
                onClick={() => item.id && onNavigate(item.id)}
                className={cn(
                  "flex size-10 w-full items-center justify-center rounded-md disabled:cursor-not-allowed disabled:opacity-45",
                  active ? "bg-primary text-primary-foreground" : "text-foreground hover:bg-muted"
                )}
              >
                <Icon className="size-4" aria-hidden />
              </button>
            );
          })}
        </div>
      </section>
    );
  }

  return (
    <section>
      <button
        type="button"
        className="flex w-full items-center justify-between rounded-md px-2 py-1 text-left text-xs font-medium text-muted-foreground hover:bg-muted"
        aria-expanded={visible}
        onClick={() => setOpen((value) => !value)}
      >
        {label}
        <ChevronDown className={visible ? "size-3 transition-transform" : "size-3 -rotate-90 transition-transform"} aria-hidden />
      </button>
      {visible && <div className="mt-2 space-y-1">
        {items.map((item) => {
          const Icon = item.icon;
          const active = item.id === activeView;
          return (
            <button
              key={item.title}
              type="button"
              aria-current={active ? "page" : undefined}
              disabled={item.disabled}
              onClick={() => item.id && onNavigate(item.id)}
              className={
                active
                  ? "flex w-full items-center gap-3 rounded-md bg-primary px-3 py-2 text-left text-primary-foreground"
                  : "flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-45"
              }
            >
              <Icon className="size-4 shrink-0" aria-hidden />
              <span className="min-w-0">
                <span className="block truncate text-sm font-medium">{item.title}</span>
                <span className={active ? "block truncate text-xs text-primary-foreground/80" : "block truncate text-xs text-muted-foreground"}>
                  {item.subtitle}
                </span>
              </span>
            </button>
          );
        })}
      </div>}
    </section>
  );
}

interface DashboardProps {
  currentUser: CurrentUser;
  onOpenM2Inbound: () => void;
  onOpenM4Outbound: () => void;
}

function Dashboard({ currentUser, onOpenM2Inbound, onOpenM4Outbound }: DashboardProps) {
  return (
    <section className="flex w-full flex-col gap-6 px-4 py-8 lg:px-8">
      <PageHeader title="WMS Web Admin" subtitle={`${currentUser.owner_code} / ${currentUser.display_name}`} />

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
          <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
            <div>
              <h2 className="text-lg font-semibold tracking-normal">运行入口</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                API 基址：{apiBaseUrl || "当前域名"}
              </p>
            </div>
            <Button type="button" onClick={onOpenM2Inbound}>
              <PackageCheck className="size-4" aria-hidden />
              M2 收货管理
            </Button>
            <Button type="button" variant="outline" onClick={onOpenM4Outbound}>
              <ClipboardList className="size-4" aria-hidden />
              M4 出库订单管理
            </Button>
          </div>
        </CardContent>
      </Card>
    </section>
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
