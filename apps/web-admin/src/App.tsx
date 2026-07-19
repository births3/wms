import * as React from "react";
import { Button, Card, CardContent, Input, PageHeader, WorkspaceTabs, cn } from "@wms/ui";
import {
  Activity,
  Bell,
  BookOpen,
  CheckCircle2,
  ClipboardList,
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
  ShieldCheck,
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
import { usePublishedAdminMenuQuery } from "@/features/admin-menu/admin-menu-queries";
import { useCurrentUserQuery, useLogout, type CurrentUser } from "@/features/auth/auth-queries";
import { clearAuthSession, hasActiveAuthSession } from "@/lib/auth-session";
import { LoginPage } from "@/pages/auth/LoginPage";

/** 作业 KPI 示例数据（尚未接入真实待办接口，勿当作生产统计） */
const operationKpis = [
  {
    id: "pending-receiving",
    label: "待收货",
    value: 12,
    hint: "示例 · 入库单待收货",
    icon: PackageCheck,
  },
  {
    id: "pending-inspecting",
    label: "待验收",
    value: 5,
    hint: "示例 · 收货后待验收",
    icon: CheckCircle2,
  },
  {
    id: "pending-putaway",
    label: "待上架",
    value: 8,
    hint: "示例 · 验收后待上架",
    icon: Layers,
  },
  {
    id: "pending-review",
    label: "待复核",
    value: 3,
    hint: "示例 · 出库复核 / 审批",
    icon: History,
  },
] as const;

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
      { id: "m1-system-dictionary", title: "M1 系统字典", subtitle: "单据类型 / 特殊药品分类", icon: BookOpen },
      { id: "m1-feature-flags", title: "M1 功能开关", subtitle: "配置中心 / Feature Flag", icon: KeyRound },
    ],
  },
  {
    label: "入库业务",
    items: [
      { id: "m2-receiving", title: "M2 收货管理", subtitle: "ASN / 到货确认", icon: CheckCircle2 },
      { id: "m2-inspecting", title: "M2 验收管理", subtitle: "批号 / 效期 / 签字", icon: ClipboardList },
      { id: "m2-putaway", title: "M2 上架管理", subtitle: "库位 / 数量确认", icon: PackageCheck },
      { id: "m2-putaway-strategy", title: "M2 上架策略", subtitle: "规则优先级 / 方案绑定", icon: ClipboardList },
      { id: "m-di-platforms", title: "M-DI 药检平台", subtitle: "平台 / 认证 / 状态", icon: KeyRound },
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
      { id: "m3-counts", title: "M3 库存盘点", subtitle: "盘点单 / 差异审批", icon: ClipboardList },
      { id: "m3-maintenance", title: "M3 在库养护", subtitle: "计划 / 任务执行", icon: ClipboardList },
      { id: "m3-relocations", title: "M3 库内移库", subtitle: "库位转移", icon: Layers },
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

const defaultMenuTree: SidebarMenuTreeSection<AdminView>[] = [
  { label: "工作台", groups: [{ label: "工作台概览", items: [menuItem("dashboard")] }] },
  {
    label: "基础档案",
    groups: [
      { label: "主数据", items: [menuItem("m1-products"), menuItem("m1-business-partners")] },
      { label: "仓储资料", items: [menuItem("m1-warehouses"), menuItem("m1-zones"), menuItem("m1-locations"), menuItem("dock-management")] },
      { label: "系统配置", items: [menuItem("m1-system-dictionary"), menuItem("m1-feature-flags")] },
    ],
  },
  {
    label: "入库业务",
    groups: [{ label: "入库作业", items: [menuItem("m2-receiving"), menuItem("m2-inspecting"), menuItem("m2-putaway"), menuItem("m2-putaway-strategy"), menuItem("m-di-platforms")] }],
  },
  {
    label: "出库业务",
    groups: [{ label: "出库作业", items: [menuItem("m4-orders"), menuItem("m4-waves"), menuItem("m4-review"), menuItem("m4-returns")] }],
  },
  { label: "库内业务", groups: [{ label: "库存管理", items: [menuItem("m3-batches"), menuItem("m3-location-history"), menuItem("m3-status-config"), menuItem("m3-counts"), menuItem("m3-maintenance"), menuItem("m3-relocations"), menuItem("mte-task-dispatch"), menuItem("mte-task-groups"), menuItem("mte-task-types")] }] },
  { label: "增值业务", groups: [{ label: "增值作业", items: [menuItem("m9-billing-rules"), menuItem("m10-route-plans")] }] },
  {
    label: "基础能力",
    groups: [
      { label: "H1 权限租户", items: [menuItem("h1-menu-management"), menuItem("h1-role-permission"), menuItem("h1-session-management"), menuItem("h1-api-keys")] },
      { label: "H2 审计能力", items: [menuItem("h2-audit-trail")] },
      { label: "H3 契约能力", items: [menuItem("h3-api-contract")] },
      { label: "H4 企业微信", items: [menuItem("h4-wechat-settings"), menuItem("h4-notify-configs"), menuItem("h4-notify-records")] },
      { label: "H-AL 告警能力", items: [menuItem("hal-alert-dashboard"), menuItem("hal-alert-definitions"), menuItem("hal-alert-escalations")] },
      { label: "H5 快递能力", items: [menuItem("h5-express")] },
      { label: "H8 集成中心", items: [menuItem("h8-erp-connectors"), menuItem("h8-erp-messages")] },
      { label: "H9 打印能力", items: [menuItem("h9-print-templates")] },
      { label: "M-CG 编码能力", items: [menuItem("mcg-numbering")] },
    ],
  },
];

const adminMenuIconByKey: Record<string, LucideIcon> = {
  Activity,
  Bell,
  BookOpen,
  CheckCircle2,
  ClipboardList,
  History,
  KeyRound,
  Layers,
  MapPinned,
  PackageCheck,
  PanelLeftOpen,
  Printer,
  ShieldCheck,
  Truck,
  Users,
  Warehouse,
};

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
    subtitle: item?.subtitle ?? "今日待办与快捷入口",
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
          {renderAdminView(tab.view, currentUserQuery.data, navigateTo) ?? (
            <Dashboard
              currentUser={currentUserQuery.data}
              onOpenM2Inbound={() => navigateTo("m2-receiving")}
              onOpenM4Outbound={() => navigateTo("m4-orders")}
              onOpenM3Batches={() => navigateTo("m3-batches")}
              onOpenH2Audit={() => navigateTo("h2-audit-trail")}
            />
          )}
        </div>
      ))}
    </AppShell>
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
  const [expandedMenuKeys, setExpandedMenuKeys] = React.useState<string[]>(readExpandedMenuKeys);
  const expandedForActiveViewRef = React.useRef<AdminView | null>(null);
  const sidebarRef = React.useRef<HTMLElement | null>(null);
  const publishedMenuQuery = usePublishedAdminMenuQuery(true);
  const menuTree = React.useMemo(() => {
    const publishedTree = publishedMenuQuery.data?.data;
    if (!publishedTree?.length) return defaultMenuTree;
    const parsed = menuTreeFromAdminNodes({ nodes: publishedTree, isView: isAdminView, iconByKey: adminMenuIconByKey });
    return parsed.length > 0 ? parsed : defaultMenuTree;
  }, [publishedMenuQuery.data?.data]);
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

interface DashboardProps {
  currentUser: CurrentUser;
  onOpenM2Inbound: () => void;
  onOpenM4Outbound: () => void;
  onOpenM3Batches: () => void;
  onOpenH2Audit: () => void;
}

function Dashboard({
  currentUser,
  onOpenM2Inbound,
  onOpenM4Outbound,
  onOpenM3Batches,
  onOpenH2Audit,
}: DashboardProps) {
  return (
    <section className="flex w-full flex-col gap-6 px-4 py-8 lg:px-8">
      <PageHeader
        title="运营总览"
        subtitle={`货主 ${currentUser.owner_code} · 下方 KPI 为示例数据，后续可接真实待办`}
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
                    <p className="mt-1 text-3xl font-semibold tracking-tight tabular-nums">{item.value}</p>
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
          <div className="flex flex-col gap-4">
            <div>
              <h2 className="text-lg font-semibold tracking-normal">快捷入口</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                进入常用作业页面，处理收货、出库、批号与审计查询
              </p>
            </div>
            <div className="flex flex-wrap gap-3">
              <Button type="button" onClick={onOpenM2Inbound}>
                <PackageCheck className="size-4" aria-hidden />
                M2 收货管理
              </Button>
              <Button type="button" variant="outline" onClick={onOpenM4Outbound}>
                <ClipboardList className="size-4" aria-hidden />
                M4 出库订单
              </Button>
              <Button type="button" variant="outline" onClick={onOpenM3Batches}>
                <Layers className="size-4" aria-hidden />
                M3 批号管理
              </Button>
              <Button type="button" variant="outline" onClick={onOpenH2Audit}>
                <History className="size-4" aria-hidden />
                H2 审计
              </Button>
            </div>
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
