import { useState, useMemo } from "react";
import { TABS, DEVICE_META, GROUP_ORDER, type Device } from "./Tabs";

/**
 * App — 原型预览主框架
 *
 * 布局：顶栏（端筛选）+ 左侧 sidebar（按层分组）+ 右侧内容区
 * URL：#<tab-value> 同步当前 tab；端筛选只影响 sidebar 视图
 */

const DEVICE_CHIPS: { value: Device | "all"; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "pc", label: "PC" },
  { value: "pda", label: "PDA" },
  { value: "pad", label: "PAD" },
];

export function App() {
  const initial = window.location.hash.replace("#", "") || "gallery";
  const [tabValue, setTabValue] = useState<string>(
    TABS.some((t) => t.value === initial) ? initial : "gallery"
  );
  const [deviceFilter, setDeviceFilter] = useState<Device | "all">("all");

  const currentTab = TABS.find((t) => t.value === tabValue) ?? TABS[0];

  // 按 group 分组 + 端过滤
  const groupedTabs = useMemo(() => {
    const filtered =
      deviceFilter === "all"
        ? TABS
        : TABS.filter((t) => t.device.includes(deviceFilter));
    const map = new Map<string, typeof TABS>();
    for (const tab of filtered) {
      const list = map.get(tab.group) ?? [];
      list.push(tab);
      map.set(tab.group, list);
    }
    return GROUP_ORDER.map((g) => ({ group: g, tabs: map.get(g) ?? [] }))
      .filter((s) => s.tabs.length > 0);
  }, [deviceFilter]);

  const switchTab = (v: string) => {
    setTabValue(v);
    window.location.hash = v;
  };

  return (
    <div className="min-h-screen bg-muted/30 font-sans">
      {/* 顶栏 */}
      <header className="bg-background border-b sticky top-0 z-10">
        <div className="px-6 py-3 flex items-center gap-6">
          <div>
            <h1 className="text-lg font-semibold">WMS 原型预览</h1>
            <p className="text-xs text-muted-foreground">
              P0 + 组件库 · ADR-0021/0022 · {TABS.length} 个 tab
            </p>
          </div>
          <div className="ml-auto flex items-center gap-1">
            <span className="text-xs text-muted-foreground mr-2">端筛选</span>
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
        {/* 左侧 sidebar */}
        <aside className="w-56 bg-background border-r min-h-[calc(100vh-65px)] py-4 sticky top-[65px] self-start">
          {groupedTabs.map((section) => (
            <div key={section.group} className="mb-4">
              <div className="px-4 py-1.5 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
                {section.group}
              </div>
              <div className="space-y-0.5">
                {section.tabs.map((tab) => (
                  <button
                    key={tab.value}
                    onClick={() => switchTab(tab.value)}
                    className={`w-full text-left px-4 py-1.5 text-sm flex items-center gap-2 transition-colors ${
                      tabValue === tab.value
                        ? "bg-primary/10 text-primary border-r-2 border-primary font-medium"
                        : "hover:bg-muted/60"
                    }`}
                  >
                    <span className="flex-1">{tab.label}</span>
                    <span className="flex gap-1">
                      {tab.device.map((d) => (
                        <span
                          key={d}
                          className={`text-[10px] px-1.5 py-0.5 rounded border ${DEVICE_META[d].color}`}
                        >
                          {DEVICE_META[d].label}
                        </span>
                      ))}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          ))}
          {groupedTabs.length === 0 && (
            <div className="px-4 py-8 text-xs text-muted-foreground text-center">
              该端暂无原型页
            </div>
          )}
        </aside>

        {/* 右侧主区 */}
        <main className="flex-1 p-6 min-w-0">
          {/* 当前 tab 标题条 */}
          <div className="mb-4 flex items-center gap-3">
            <h2 className="text-base font-medium">{currentTab.label}</h2>
            <span className="text-xs text-muted-foreground">{currentTab.group}</span>
            <span className="flex gap-1">
              {currentTab.device.map((d) => (
                <span
                  key={d}
                  className={`text-[10px] px-1.5 py-0.5 rounded border ${DEVICE_META[d].color}`}
                >
                  {DEVICE_META[d].label}
                </span>
              ))}
            </span>
            <span className="ml-auto text-xs text-muted-foreground font-mono">
              #{currentTab.value}
            </span>
          </div>

          {currentTab.render()}
        </main>
      </div>
    </div>
  );
}
