import * as React from "react";
import { Button, Card, CardContent, Input, WorkspaceTabs, cn } from "@wms/ui";
import {
  Activity,
  ArrowUpCircle,
  Bell,
  BellRing,
  BookOpen,
  CheckCircle2,
  ClipboardList,
  Database,
  History,
  Inbox,
  KeyRound,
  Layers,
  LogOut,
  MapPinned,
  PackageCheck,
  PanelLeftClose,
  PanelLeftOpen,
  Printer,
  RefreshCw,
  Search,
  Settings,
  ShieldCheck,
  Stamp,
  Truck,
  Users,
  Warehouse,
  type LucideIcon,
} from "lucide-react";

import {
  AdminSidebarMenu,
  filterSidebarMenuTree,
  menuGroupKey,
  menuSectionKey,
  menuTreeFromAdminNodes,
  type SidebarMenuItem,
  type SidebarMenuTreeSection,
} from "@/app-shell/AdminSidebarMenu";
import { renderAdminView } from "@/app-shell/AdminViewRenderer";
import type { AdminView } from "@/app-shell/admin-view";
import { usePublishedAdminMenuQuery, type AdminMenuNode } from "@/features/admin-menu/admin-menu-queries";
import { useCurrentUserQuery, useLogout, type CurrentUser } from "@/features/auth/auth-queries";
import { clearAuthSession, hasActiveAuthSession } from "@/lib/auth-session";
import { LoginPage } from "@/pages/auth/LoginPage";
import { Dashboard } from "@/pages/dashboard/DashboardPage";

const menuSections: Array<{ label: string; items: SidebarMenuItem<AdminView>[] }> = [
  {
    label: "工作台",
    items: [{ id: "dashboard", title: "运营总览", subtitle: "今日待办与快捷入口", icon: Activity }],
  },
  {
    label: "基础档案",
    items: [
      { id: "m1-products", title: "M1 商品档案", subtitle: "商品编码 / 规格", icon: PackageCheck },
      { id: "m1-business-partners", title: "M1 客商档案", subtitle: "供应商 / 客户门店", icon: Users },
      { id: "m1-warehouses", title: "M1 仓库管理", subtitle: "仓库 / 状态", icon: Warehouse },
      { id: "m1-zones", title: "M1 库区管理", subtitle: "库区 / 仓库", icon: MapPinned },
      { id: "m1-locations", title: "M1 库位管理", subtitle: "库位 / 容量", icon: MapPinned },
      { id: "dock-management", title: "M1 月台管理", subtitle: "月台 / 作业类型 / 温区", icon: MapPinned },
      { id: "m1-lpn-containers", title: "M1 容器管理", subtitle: "LPN / 类型策略", icon: PackageCheck },
      { id: "m1-system-dictionary", title: "M1 系统字典", subtitle: "单据类型 / 特殊药品分类", icon: BookOpen },
      { id: "m1-feature-flags", title: "M1 功能开关", subtitle: "配置中心 / Feature Flag", icon: KeyRound },
    ],
  },
  {
    label: "入库业务",
    items: [
      { id: "m2-receiving", title: "M2 收货管理", subtitle: "ASN / 到货确认", icon: CheckCircle2 },
      { id: "m2-inbound-documents", title: "入库资料录入", subtitle: "药检单 / 上游随货同行单", icon: ClipboardList },
      { id: "m-di-review", title: "药检单审核", subtitle: "逐份确认 / 版本记录", icon: ShieldCheck },
      { id: "m2-inspecting", title: "M2 验收管理", subtitle: "批号 / 效期 / 签字", icon: ClipboardList },
      { id: "m2-putaway", title: "M2 上架管理", subtitle: "库位 / 数量确认", icon: PackageCheck },
      { id: "m2-putaway-strategy", title: "M2 上架策略", subtitle: "规则优先级 / 方案绑定", icon: ClipboardList },
      { id: "m-di-platforms", title: "M-DI 药检平台", subtitle: "平台 / 认证 / 状态", icon: KeyRound },
      { id: "m-di-stamp", title: "药检图章配置", subtitle: "拖动设计 / 双人发布", icon: Stamp },
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
    items: [
      { id: "m3-batches", title: "M3 批号管理", subtitle: "批号 / 效期 / 库位", icon: Layers },
      { id: "m3-location-history", title: "M3 库位历史", subtitle: "库位流水 / 风险追踪", icon: Layers },
      { id: "m3-status-config", title: "M3 状态规则", subtitle: "状态转换 / 货主覆盖", icon: ClipboardList },
      { id: "m3-replenishment-strategies", title: "M3 补货策略", subtitle: "Min-Max / 动线 / 挂接", icon: ClipboardList },
      { id: "m3-replenishment-tasks", title: "M3 补货任务", subtitle: "大盘 / 重派 / 取消", icon: ClipboardList },
      { id: "m3-counts", title: "M3 库存盘点", subtitle: "盘点单 / 差异审批", icon: ClipboardList },
      { id: "m3-maintenance", title: "M3 在库养护", subtitle: "计划 / 任务执行", icon: ClipboardList },
      { id: "m3-relocations", title: "M3 库内移库", subtitle: "库位转移", icon: Layers },
      { id: "mrc-reconciliation", title: "M-RC 库存对账", subtitle: "ERP 差异 / 隔离 / 处置", icon: ClipboardList },
      { id: "mte-task-types", title: "M-TE 任务类型配置", subtitle: "任务 / 调度参数", icon: ClipboardList },
      { id: "mte-task-groups", title: "M-TE 任务组资格", subtitle: "仓库 / 类型 / 人员", icon: Users },
      { id: "mte-task-dispatch", title: "M-TE 任务调度", subtitle: "分派 / 下发 / 处置", icon: ClipboardList },
    ],
  },
  {
    label: "增值业务",
    items: [
      { id: "m9-billing-rules", title: "M9 计费规则", subtitle: "货主 / 合同 / 费率", icon: ClipboardList },
      { id: "m10-route-plans", title: "M10 路径规划接收", subtitle: "TMS / 路线 / 订单", icon: Truck },
    ],
  },
  {
    label: "基础能力",
    items: [
      { id: "h1-menu-management", title: "H1 菜单管理", subtitle: "三层菜单 / 权限点", icon: ShieldCheck },
      { id: "h1-role-permission", title: "H1 角色权限", subtitle: "角色 / 权限矩阵 / 批量授权", icon: ShieldCheck },
      { id: "h1-session-management", title: "H1 登录会话", subtitle: "Token / 设备 / 强制踢人", icon: ShieldCheck },
      { id: "h1-api-keys", title: "H1 API Key 管理", subtitle: "创建 / 轮换 / 吊销", icon: KeyRound },
      { id: "h2-audit-trail", title: "H2 审计追踪", subtitle: "审计 / 归档 / 事件", icon: ClipboardList },
      { id: "h3-api-contract", title: "H3 OpenAPI", subtitle: "契约 / 文档 / 类型", icon: KeyRound },
      { id: "h4-wechat-settings", title: "H4 参数设置", subtitle: "企业微信 / 回调 / 重试", icon: KeyRound },
      { id: "h4-notify-configs", title: "H4 通知配置", subtitle: "事件 / 模板 / 接收人", icon: Bell },
      { id: "h4-notify-records", title: "H4 发送记录", subtitle: "通知 / 重发 / 排查", icon: History },
      { id: "hal-alert-dashboard", title: "H-AL 告警看板", subtitle: "活动告警 / 统计 / 报表", icon: Activity },
      { id: "hal-alert-definitions", title: "H-AL 告警定义", subtitle: "条件 / 级别 / 接收人", icon: Bell },
      { id: "hal-alert-escalations", title: "H-AL 升级规则", subtitle: "三级升级 / 值班路由", icon: History },
      { id: "h5-express", title: "H5 快递对接", subtitle: "快递商 / 规则 / 面单", icon: Truck },
      { id: "h8-erp-connectors", title: "H8 ERP 连接", subtitle: "集成中心 / 通道 / 凭证引用", icon: KeyRound },
      { id: "h8-erp-messages", title: "H8 ERP 消息", subtitle: "集成中心 / 日志 / 死信重放", icon: Inbox },
      { id: "h8-erp-interface-tables", title: "H8 接口表探查", subtitle: "集成中心 / 只读 / 接口行", icon: Database },
      { id: "h9-delivery-note-aggregation", title: "作业·随货同行单归集", subtitle: "线路冻结 / 截单计划 / 归集结果", icon: Printer },
      { id: "h9-print-devices", title: "设备·Print Agent 管理", subtitle: "站点 / 打印机 / 纸盒 / 租约", icon: Printer },
      { id: "h9-print-templates", title: "H9 打印模板", subtitle: "字段库 / 模板类型", icon: Printer },
      { id: "mcg-numbering", title: "M-CG 单据号规则", subtitle: "单据类型 / 编码规则", icon: KeyRound },
    ],
  },
];

const WORKSPACE_TABS_STORAGE_KEY = "wms:web-admin:workspace-tabs:v1";
const MENU_EXPANDED_STORAGE_KEY = "wms:web-admin:menu-expanded:v1";
const menuItemById = new Map<AdminView, SidebarMenuItem<AdminView>>(
  menuSections.flatMap((section) => section.items).flatMap((item): Array<[AdminView, SidebarMenuItem<AdminView>]> => item.id ? [[item.id, item]] : []),
);

const dashboardMenuTree: SidebarMenuTreeSection<AdminView>[] = [
  { label: "工作台", groups: [{ label: "工作台概览", items: [menuItem("dashboard")] }] },
];

const adminMenuIconByKey: Record<string, LucideIcon> = {
  Activity,
  ArrowUpCircle,
  Bell,
  BellRing,
  BookOpen,
  CheckCircle2,
  ClipboardList,
  Database,
  History,
  Inbox,
  KeyRound,
  Layers,
  MapPinned,
  PackageCheck,
  PanelLeftOpen,
  Printer,
  Settings,
  ShieldCheck,
  Stamp,
  Truck,
  Users,
  Warehouse,
};

type PublishedMenuState =
  | { kind: "loading"; tree: SidebarMenuTreeSection<AdminView>[]; message: string }
  | { kind: "unavailable"; tree: SidebarMenuTreeSection<AdminView>[]; message: string }
  | { kind: "ready"; tree: SidebarMenuTreeSection<AdminView>[]; allowedViews: Set<AdminView> };

function resolvePublishedMenuState({
  isPending,
  isError,
  publishedTree,
}: {
  isPending: boolean;
  isError: boolean;
  publishedTree: AdminMenuNode[] | undefined;
}): PublishedMenuState {
  if (isPending) return { kind: "loading", tree: dashboardMenuTree, message: "正在加载已发布菜单" };
  if (isError) return { kind: "unavailable", tree: dashboardMenuTree, message: "已发布菜单读取失败，请重试" };
  if (!publishedTree?.length) return { kind: "unavailable", tree: dashboardMenuTree, message: "当前用户没有可用菜单" };

  const tree = menuTreeFromAdminNodes({ nodes: publishedTree, isView: isAdminView, iconByKey: adminMenuIconByKey });
  const allowedViews = viewsInMenuTree(tree);
  if (tree.length === 0 || !allowedViews.has("dashboard")) {
    return { kind: "unavailable", tree: dashboardMenuTree, message: "已发布菜单为空或缺少工作台入口" };
  }
  return { kind: "ready", tree, allowedViews };
}

function viewsInMenuTree(tree: SidebarMenuTreeSection<AdminView>[]) {
  const views = new Set<AdminView>();
  for (const section of tree) {
    for (const group of section.groups) {
      for (const item of group.items) {
        if (item.id) views.add(item.id);
      }
    }
  }
  return views;
}

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
  if (exists && state.view === nextView) return state;
  return {
    view: nextView,
    openTabs: exists ? state.openTabs : [...state.openTabs, workspaceTabForView(nextView)],
  };
}

/** 从 location.hash（形如 #/m4-orders）解析视图，非法值返回 null。 */
function viewFromLocationHash(): AdminView | null {
  if (typeof window === "undefined") return null;
  const candidate = window.location.hash.replace(/^#\/?/, "");
  return isAdminView(candidate) ? candidate : null;
}

function writeLocationHash(view: AdminView) {
  if (typeof window === "undefined") return;
  const next = `#/${view}`;
  if (window.location.hash !== next) window.location.hash = next;
}

/** 刷新 / 深链进入时：先恢复 localStorage 页签，再让 URL hash 指定的视图生效。 */
function initialWorkspaceState(): AdminWorkspaceState {
  const stored = readWorkspaceTabs();
  const hashView = viewFromLocationHash();
  return hashView ? openWorkspaceTab(stored, hashView) : stored;
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
    subtitle: item?.subtitle ?? "今日待办与快捷入口",
    closable: view !== "dashboard",
  };
}

function isAdminView(value: unknown): value is AdminView {
  return typeof value === "string" && menuSections.some((section) => section.items.some((item) => item.id === value));
}

export function App() {
  const logout = useLogout();
  const [workspaceState, setWorkspaceState] = React.useState<AdminWorkspaceState>(initialWorkspaceState);
  const [sessionVersion, setSessionVersion] = React.useState(0);
  const hasSession = React.useMemo(() => hasActiveAuthSession(), [sessionVersion]);
  const currentUserQuery = useCurrentUserQuery(hasSession);
  const publishedMenuQuery = usePublishedAdminMenuQuery(hasSession);
  const view = workspaceState.view;
  const openTabs = workspaceState.openTabs;
  const menuState = React.useMemo(
    () => resolvePublishedMenuState({
      isPending: publishedMenuQuery.isPending,
      isError: publishedMenuQuery.isError,
      publishedTree: publishedMenuQuery.data?.data,
    }),
    [publishedMenuQuery.data?.data, publishedMenuQuery.isError, publishedMenuQuery.isPending],
  );
  const viewAvailable = menuState.kind === "ready" ? menuState.allowedViews.has(view) : view === "dashboard";
  const safeView = viewAvailable ? view : "dashboard";
  const visibleOpenTabs = menuState.kind === "ready"
    ? openTabs.filter((tab) => menuState.allowedViews.has(tab.view))
    : [workspaceTabForView("dashboard")];

  React.useEffect(() => {
    writeWorkspaceTabs(workspaceState);
  }, [workspaceState]);

  const navigateTo = React.useCallback((nextView: AdminView) => {
    setWorkspaceState((state) => openWorkspaceTab(state, nextView));
  }, []);

  React.useLayoutEffect(() => {
    writeLocationHash(view);
  }, [view]);

  React.useEffect(() => {
    const onHashChange = () => {
      const next = viewFromLocationHash();
      if (next) navigateTo(next);
    };
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, [navigateTo]);

  const closeTab = React.useCallback((targetView: AdminView) => {
    setWorkspaceState((state) => closeWorkspaceTab(state, targetView));
  }, []);

  const closeOtherTabs = React.useCallback((targetView: AdminView) => {
    setWorkspaceState((state) => closeOtherWorkspaceTabs(state, targetView));
  }, []);

  React.useEffect(() => {
    if (menuState.kind !== "ready") return;
    if (safeView !== view) closeOtherTabs("dashboard");
  }, [closeOtherTabs, menuState.kind, safeView, view]);

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
      activeView={safeView}
      openTabs={visibleOpenTabs}
      menuState={menuState}
      onRetryMenu={() => void publishedMenuQuery.refetch()}
      onNavigate={navigateTo}
      onCloseTab={closeTab}
      onCloseOtherTabs={closeOtherTabs}
      onLogout={handleLogout}
    >
      {menuState.kind === "ready" ? visibleOpenTabs.map((tab) => (
        <div key={tab.view} hidden={tab.view !== safeView} className={tab.view === safeView ? "flex flex-1 flex-col min-h-0" : undefined}>
          <React.Suspense fallback={<PageLoading />}>
            {renderAdminView(tab.view, currentUserQuery.data, navigateTo) ?? (
              <Dashboard
                currentUser={currentUserQuery.data}
                availableViews={menuState.allowedViews}
                onOpenM2Inbound={() => navigateTo("m2-receiving")}
                onOpenM4Outbound={() => navigateTo("m4-orders")}
                onOpenM3Batches={() => navigateTo("m3-batches")}
                onOpenH2Audit={() => navigateTo("h2-audit-trail")}
              />
            )}
          </React.Suspense>
        </div>
      )) : (
        <MenuUnavailablePanel
          message={menuState.message}
          loading={menuState.kind === "loading"}
          onRetry={() => void publishedMenuQuery.refetch()}
        />
      )}
    </AppShell>
  );
}

function PageLoading() {
  return (
    <div className="flex min-h-[40vh] items-center justify-center text-sm text-muted-foreground">
      <RefreshCw className="mr-2 size-4 animate-spin" aria-hidden />
      页面加载中
    </div>
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

function AppShell({
  currentUser,
  activeView,
  openTabs,
  menuState,
  onRetryMenu,
  onNavigate,
  onCloseTab,
  onCloseOtherTabs,
  onLogout,
  children,
}: {
  currentUser: CurrentUser;
  activeView: AdminView;
  openTabs: AdminWorkspaceTab[];
  menuState: PublishedMenuState;
  onRetryMenu: () => void;
  onNavigate: (view: AdminView) => void;
  onCloseTab: (view: AdminView) => void;
  onCloseOtherTabs: (view: AdminView) => void;
  onLogout: () => void;
  children: React.ReactNode;
}) {
  const [menuFilter, setMenuFilter] = React.useState("");
  const [menuFilterOpen, setMenuFilterOpen] = React.useState(false);
  const [sidebarCollapsed, setSidebarCollapsed] = React.useState(false);
  const [expandedMenuKeys, setExpandedMenuKeys] = React.useState<string[]>(readExpandedMenuKeys);
  const expandedForActiveViewRef = React.useRef<AdminView | null>(null);
  const sidebarRef = React.useRef<HTMLElement | null>(null);
  const menuTree = menuState.tree;
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
  const visibleMenuSections = React.useMemo(
    () => filterSidebarMenuTree(menuTree, normalizedMenuFilter),
    [menuTree, normalizedMenuFilter],
  );
  const expandedMenuKeySet = React.useMemo(() => new Set(expandedMenuKeys), [expandedMenuKeys]);
  const toggleMenuKey = React.useCallback((key: string) => {
    setExpandedMenuKeys((current) => {
      if (current.includes(key)) {
        const groupPrefix = key.startsWith("section:") ? `group:${key.slice("section:".length)}:` : "";
        return current.filter((item) => item !== key && (!groupPrefix || !item.startsWith(groupPrefix)));
      }
      if (key.startsWith("section:")) return [key];
      return [...current.filter((item) => !item.startsWith("group:")), key];
    });
  }, []);

  React.useEffect(() => {
    if (expandedForActiveViewRef.current === activeView) return;
    const activeKeys = menuKeysForActiveView(menuTree, activeView);
    if (activeKeys.length === 0) return;
    expandedForActiveViewRef.current = activeView;
    setExpandedMenuKeys((current) => (
      current.length === activeKeys.length && activeKeys.every((key) => current.includes(key))
        ? current
        : activeKeys
    ));
  }, [activeView, menuTree]);

  React.useEffect(() => {
    writeExpandedMenuKeys(expandedMenuKeys);
  }, [expandedMenuKeys]);

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
        sidebarCollapsed ? "lg:grid-cols-[1fr]" : "lg:grid-cols-[14rem_1fr]",
      )}
    >
      <header
        className={cn(
          "hidden border-b bg-background lg:grid lg:h-14 lg:items-center",
          sidebarCollapsed
            ? "lg:col-span-1 lg:grid-cols-[minmax(0,1fr)_auto]"
            : "lg:col-span-2 lg:grid-cols-[14rem_minmax(0,1fr)_auto]",
        )}
      >
        {!sidebarCollapsed && (
          <div className="flex h-full items-center justify-between border-r px-3">
            <div className="flex min-w-0 items-center gap-2">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
                <Warehouse className="size-5" aria-hidden />
              </div>
              <div className="min-w-0">
                <div className="truncate text-sm font-semibold tracking-normal">WMS Admin</div>
                <div className="truncate text-[11px] text-muted-foreground">{currentUser.owner_code}</div>
              </div>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label="收缩侧边栏"
              title="完全收起侧边栏"
              onClick={() => {
                setSidebarCollapsed(true);
                closeMenuFilter();
              }}
            >
              <PanelLeftClose className="size-4" aria-hidden />
            </Button>
          </div>
        )}

        <WorkspaceTabs
          className="min-w-0 border-0 bg-transparent"
          leading={
            sidebarCollapsed ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-8 shrink-0 gap-1.5 rounded-full border-primary/30 bg-primary/10 px-3 text-xs font-semibold text-primary shadow-xs hover:bg-primary/20 hover:text-primary transition-colors"
                aria-label="展开侧边栏"
                title="展开主导航菜单"
                onClick={() => setSidebarCollapsed(false)}
              >
                <PanelLeftOpen className="size-4 text-primary" aria-hidden />
                <span>展开菜单</span>
              </Button>
            ) : undefined
          }
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

      {!sidebarCollapsed && (
        <aside ref={sidebarRef} className="hidden border-r bg-background lg:sticky lg:top-14 lg:flex lg:h-[calc(100vh-3.5rem)] lg:flex-col">
          <nav className="flex-1 overflow-y-auto py-4 space-y-5 px-3">
            {menuState.kind !== "ready" ? (
              <div className="space-y-2 rounded-md border border-amber-300/70 bg-amber-50 p-3 text-xs text-amber-950" role="status">
                <p>{menuState.message}</p>
                <Button type="button" variant="outline" size="sm" className="w-full" onClick={onRetryMenu}>
                  <RefreshCw className="size-3.5" aria-hidden />
                  重新加载菜单
                </Button>
              </div>
            ) : null}
            {menuFilterOpen ? (
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
          <AdminSidebarMenu
            sections={visibleMenuSections}
            activeView={activeView}
            onNavigate={navigateFromMenu}
            expandedKeys={expandedMenuKeySet}
            onToggleKey={toggleMenuKey}
            forceOpen={Boolean(normalizedMenuFilter)}
            collapsed={sidebarCollapsed}
          />
        </nav>
      </aside>
      )}

      <div className="min-w-0 flex flex-1 flex-col min-h-0">
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
        <h1 className="sr-only">
          {openTabs.find((tab) => tab.view === activeView)?.label ?? "WMS Admin"}
        </h1>
        {children}
      </div>
    </div>
  );
}

function menuItem(id: AdminView): SidebarMenuItem<AdminView> {
  const item = menuItemById.get(id);
  if (!item) throw new Error(`未注册菜单视图: ${id}`);
  return item;
}

function readExpandedMenuKeys() {
  const defaultKeys = [
    menuSectionKey("工作台"),
    menuGroupKey("工作台", "工作台概览"),
  ];
  if (typeof window === "undefined") return defaultKeys;
  try {
    const raw = window.localStorage.getItem(MENU_EXPANDED_STORAGE_KEY);
    if (!raw) return defaultKeys;
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) && parsed.every((item) => typeof item === "string") ? parsed : defaultKeys;
  } catch {
    return defaultKeys;
  }
}

function writeExpandedMenuKeys(keys: string[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(MENU_EXPANDED_STORAGE_KEY, JSON.stringify(keys));
}

function menuKeysForActiveView(sections: SidebarMenuTreeSection<AdminView>[], activeView: AdminView) {
  for (const section of sections) {
    for (const group of section.groups) {
      if (group.items.some((item) => item.id === activeView)) {
        return [menuSectionKey(section.label), menuGroupKey(section.label, group.label)];
      }
    }
  }
  return [];
}

function MenuUnavailablePanel({
  message,
  loading,
  onRetry,
}: {
  message: string;
  loading: boolean;
  onRetry: () => void;
}) {
  return (
    <section className="flex w-full flex-col gap-6 px-4 py-8 lg:px-8">
      <Card className="rounded-lg shadow-sm">
        <CardContent className="flex max-w-2xl flex-col gap-4 p-6" role={loading ? "status" : "alert"}>
          <div>
            <h2 className="text-lg font-semibold tracking-normal">当前菜单不可用</h2>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">{message}。为避免展示未授权页面，当前仅保留工作台。</p>
          </div>
          <Button type="button" variant="outline" className="w-fit" onClick={onRetry} disabled={loading}>
            <RefreshCw className={cn("size-4", loading && "animate-spin")} aria-hidden />
            {loading ? "加载菜单中" : "重新加载菜单"}
          </Button>
        </CardContent>
      </Card>
    </section>
  );
}
