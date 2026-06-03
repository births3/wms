import { useMemo, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { Boxes, ChevronDown, PanelLeft, PanelLeftClose, Search, ShieldCheck, Store, Warehouse, Workflow } from "lucide-react";
import { TABS, DEVICE_META, GROUP_ORDER, type Device, type Group, type TabDef } from "./Tabs";

/**
 * App — 原型预览主框架
 *
 * 布局：顶栏（端筛选）+ 左侧层次导航（领域 / 模块 / 原型页）+ 右侧内容区
 * URL：#<tab-value> 同步当前 tab；筛选只影响导航视图，不卸载当前页面
 */

type DomainId = "all" | "kit" | "foundation" | "warehouse" | "portals" | "extensions";

interface DomainMeta {
  id: DomainId;
  label: string;
  icon: LucideIcon;
  groups?: Group[];
}

const DEVICE_CHIPS: { value: Device | "all"; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "pc", label: "PC" },
  { value: "pda", label: "PDA" },
  { value: "pad", label: "PAD" },
  { value: "h5", label: "H5" },
];

const DOMAINS: DomainMeta[] = [
  { id: "all", label: "全部", icon: Boxes },
  { id: "kit", label: "组件", icon: Boxes, groups: ["组件"] },
  {
    id: "foundation",
    label: "横向底座",
    icon: ShieldCheck,
    groups: ["H1 权限审计", "H2/H3 治理", "H4/H5 协同", "H-AL 告警"],
  },
  {
    id: "warehouse",
    label: "仓储主线",
    icon: Warehouse,
    groups: ["H-DOCK 月台", "M1 基础数据", "M2 采购入库", "M3 库存核心", "M4 销售出库", "M5 冷链监控", "M6 GSP 报表"],
  },
  {
    id: "portals",
    label: "协同端",
    icon: Store,
    groups: ["H-Driver 司机端", "H-Store 门店端"],
  },
  {
    id: "extensions",
    label: "运营扩展",
    icon: Workflow,
    groups: [
      "M8 连锁",
      "M9 计费",
      "M10 TMS",
      "M-TE 任务引擎",
      "M-RP 补货",
      "M-PK 包装站",
      "M-VR 规则",
      "M-QL 质量联系单",
      "M-SA 报损报溢",
      "M-RC 对账",
      "M-DI 药检单",
      "M-BA 批号调整",
      "M-TC 追溯码",
      "M-PM 参数对照",
      "M8 计费 / M10 TMS",
    ],
  },
];

const DOMAIN_BY_GROUP = DOMAINS.reduce((acc, domain) => {
  for (const group of domain.groups ?? []) {
    acc[group] = domain.id;
  }
  return acc;
}, {} as Partial<Record<Group, DomainId>>);

// 仓储主线序号：复用 architecture-dependencies.md §1.0 的 M1..M6 业务流顺序（收→存→发→报表）
const WAREHOUSE_SEQ = (DOMAINS.find((domain) => domain.id === "warehouse")?.groups ?? []).reduce(
  (acc, group, index) => {
    acc[group] = index + 1;
    return acc;
  },
  {} as Partial<Record<Group, number>>,
);

function domainForGroup(group: Group): DomainId {
  return DOMAIN_BY_GROUP[group] ?? "extensions";
}

function domainLabel(id: DomainId) {
  return DOMAINS.find((domain) => domain.id === id)?.label ?? id;
}

function tabMatchesQuery(tab: TabDef, query: string) {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return [tab.value, tab.label, tab.group, ...tab.device]
    .join(" ")
    .toLowerCase()
    .includes(q);
}

function tabDevices(tabs: TabDef[]) {
  const devices = new Set<Device>();
  for (const tab of tabs) {
    for (const device of tab.device) devices.add(device);
  }
  return Array.from(devices);
}

export function App() {
  const initial = window.location.hash.replace("#", "") || "gallery";
  const initialTab = TABS.find((tab) => tab.value === initial) ?? TABS[0];
  const [tabValue, setTabValue] = useState<string>(initialTab.value);
  const [deviceFilter, setDeviceFilter] = useState<Device | "all">("all");
  const [activeDomain, setActiveDomain] = useState<DomainId>(domainForGroup(initialTab.group));
  const [query, setQuery] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [collapsedGroups, setCollapsedGroups] = useState<Set<Group>>(new Set());

  const toggleGroup = (group: Group) =>
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      next.has(group) ? next.delete(group) : next.add(group);
      return next;
    });

  const currentTab = TABS.find((tab) => tab.value === tabValue) ?? TABS[0];

  const baseFilteredTabs = useMemo(() => {
    return TABS.filter((tab) => {
      const deviceOk = deviceFilter === "all" || tab.device.includes(deviceFilter);
      return deviceOk && tabMatchesQuery(tab, query);
    });
  }, [deviceFilter, query]);

  const domainCounts = useMemo(() => {
    const counts = new Map<DomainId, number>();
    counts.set("all", baseFilteredTabs.length);
    for (const tab of baseFilteredTabs) {
      const domain = domainForGroup(tab.group);
      counts.set(domain, (counts.get(domain) ?? 0) + 1);
    }
    return counts;
  }, [baseFilteredTabs]);

  const navigationSections = useMemo(() => {
    const scopedTabs =
      activeDomain === "all"
        ? baseFilteredTabs
        : baseFilteredTabs.filter((tab) => domainForGroup(tab.group) === activeDomain);
    const byGroup = new Map<Group, TabDef[]>();
    for (const tab of scopedTabs) {
      const tabs = byGroup.get(tab.group) ?? [];
      tabs.push(tab);
      byGroup.set(tab.group, tabs);
    }
    return GROUP_ORDER.map((group) => ({ group, tabs: byGroup.get(group) ?? [] }))
      .filter((section) => section.tabs.length > 0);
  }, [activeDomain, baseFilteredTabs]);

  const allCollapsed =
    navigationSections.length > 0 && navigationSections.every((section) => collapsedGroups.has(section.group));

  const toggleAllGroups = () =>
    setCollapsedGroups(allCollapsed ? new Set() : new Set(navigationSections.map((section) => section.group)));

  const switchTab = (value: string) => {
    const nextTab = TABS.find((tab) => tab.value === value);
    setTabValue(value);
    if (nextTab) setActiveDomain(domainForGroup(nextTab.group));
    window.location.hash = value;
  };

  return (
    <div className="min-h-screen bg-muted/30 font-sans">
      <header className="h-[72px] bg-background border-b sticky top-0 z-20">
        <div className="h-full px-6 flex items-center gap-6">
          <button
            onClick={() => setSidebarOpen((open) => !open)}
            aria-label={sidebarOpen ? "隐藏导航栏" : "显示导航栏"}
            aria-pressed={sidebarOpen}
            className="size-9 shrink-0 rounded-md border border-input flex items-center justify-center hover:bg-muted transition-colors"
          >
            {sidebarOpen ? <PanelLeftClose className="size-4" /> : <PanelLeft className="size-4" />}
          </button>

          <div className="min-w-[240px]">
            <h1 className="text-lg font-semibold">WMS 原型预览</h1>
            <p className="text-xs text-muted-foreground">
              P0 + 组件库 · ADR-0021/0022 · {TABS.length} 个 tab
            </p>
          </div>

          <div className="relative w-[360px] max-w-[34vw]">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索原型"
              className="h-9 w-full rounded-md border bg-background pl-8 pr-3 text-sm outline-none focus:border-primary"
            />
          </div>

          <div className="ml-auto flex items-center gap-1">
            <span className="text-xs text-muted-foreground mr-2">端</span>
            {DEVICE_CHIPS.map((chip) => (
              <button
                key={chip.value}
                onClick={() => setDeviceFilter(chip.value)}
                className={`text-xs px-2.5 py-1 rounded-md border transition-colors ${
                  deviceFilter === chip.value
                    ? "bg-primary text-primary-foreground border-primary"
                    : "bg-background hover:bg-muted border-input"
                }`}
              >
                {chip.label}
              </button>
            ))}
          </div>
        </div>
      </header>

      <div className="flex">
        {sidebarOpen && (
        <aside className="w-[360px] shrink-0 bg-background border-r sticky top-[72px] self-start max-h-[calc(100vh-72px)] overflow-y-auto">
          <div className="p-3 border-b">
            <div className="grid grid-cols-2 gap-1.5">
              {DOMAINS.map((domain) => {
                const Icon = domain.icon;
                const count = domainCounts.get(domain.id) ?? 0;
                const active = activeDomain === domain.id;
                return (
                  <button
                    key={domain.id}
                    onClick={() => setActiveDomain(domain.id)}
                    className={`h-10 rounded-md border px-2 flex items-center gap-2 transition-colors ${
                      active
                        ? "bg-primary/10 border-primary text-primary"
                        : "bg-background border-input hover:bg-muted/60"
                    }`}
                  >
                    <Icon className="size-4 shrink-0" />
                    <span className="min-w-0 flex-1 text-left text-xs font-medium truncate">{domain.label}</span>
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-mono">
                      {count}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>

          <div className="px-3 py-1.5 border-b flex items-center justify-between">
            <span className="text-[11px] text-muted-foreground">{navigationSections.length} 个分组</span>
            <button
              onClick={toggleAllGroups}
              disabled={navigationSections.length === 0}
              className="text-xs px-2 py-1 rounded-md border border-input hover:bg-muted transition-colors disabled:opacity-50"
            >
              {allCollapsed ? "全部展开" : "全部折叠"}
            </button>
          </div>

          <nav className="pb-4">
            {navigationSections.map((section) => {
              const devices = tabDevices(section.tabs);
              const collapsed = collapsedGroups.has(section.group);
              return (
                <section key={section.group} className="border-b last:border-b-0">
                  <button
                    onClick={() => toggleGroup(section.group)}
                    aria-expanded={!collapsed}
                    className="w-full px-3 py-2 bg-muted/35 flex items-center gap-2 text-left hover:bg-muted/60 transition-colors"
                  >
                    <ChevronDown className={`size-3.5 shrink-0 text-muted-foreground transition-transform ${collapsed ? "-rotate-90" : ""}`} />
                    {WAREHOUSE_SEQ[section.group] && (
                      <span className="size-5 shrink-0 rounded-full bg-primary/10 text-primary text-[11px] font-mono font-semibold flex items-center justify-center">
                        {WAREHOUSE_SEQ[section.group]}
                      </span>
                    )}
                    <div className="min-w-0 flex-1">
                      <div className="text-xs font-semibold truncate">{section.group}</div>
                      <div className="text-[11px] text-muted-foreground">
                        {domainLabel(domainForGroup(section.group))} · {section.tabs.length} 页
                      </div>
                    </div>
                    <div className="flex gap-1 shrink-0">
                      {devices.map((device) => (
                        <span key={device} className={`text-[10px] px-1.5 py-0.5 rounded border ${DEVICE_META[device].color}`}>
                          {DEVICE_META[device].label}
                        </span>
                      ))}
                    </div>
                  </button>
                  {!collapsed && (
                  <div className="py-1">
                    {section.tabs.map((tab) => (
                      <button
                        key={tab.value}
                        onClick={() => switchTab(tab.value)}
                        className={`w-full text-left px-3 py-2 grid grid-cols-[1fr_auto] gap-2 transition-colors ${
                          tabValue === tab.value
                            ? "bg-primary/10 text-primary border-r-2 border-primary"
                            : "hover:bg-muted/60"
                        }`}
                      >
                        <span className="min-w-0">
                          <span className="block text-sm font-medium truncate">{tab.label}</span>
                          <span className="block text-[11px] font-mono text-muted-foreground truncate">#{tab.value}</span>
                        </span>
                        <span className="flex gap-1 items-start">
                          {tab.device.map((device) => (
                            <span key={device} className={`text-[10px] px-1.5 py-0.5 rounded border ${DEVICE_META[device].color}`}>
                              {DEVICE_META[device].label}
                            </span>
                          ))}
                        </span>
                      </button>
                    ))}
                  </div>
                  )}
                </section>
              );
            })}

            {navigationSections.length === 0 && (
              <div className="px-4 py-10 text-xs text-muted-foreground text-center">
                无匹配原型
              </div>
            )}
          </nav>
        </aside>
        )}
        <main className="flex-1 p-6 min-w-0">
          <div className="mb-4 flex items-center gap-3">
            <h2 className="text-base font-medium truncate">{currentTab.label}</h2>
            <span className="text-xs text-muted-foreground shrink-0">
              {domainLabel(domainForGroup(currentTab.group))} / {currentTab.group}
            </span>
            <span className="flex gap-1 shrink-0">
              {currentTab.device.map((device) => (
                <span key={device} className={`text-[10px] px-1.5 py-0.5 rounded border ${DEVICE_META[device].color}`}>
                  {DEVICE_META[device].label}
                </span>
              ))}
            </span>
            <span className="ml-auto text-xs text-muted-foreground font-mono shrink-0">
              #{currentTab.value}
            </span>
          </div>

          {currentTab.render()}
        </main>
      </div>
    </div>
  );
}
