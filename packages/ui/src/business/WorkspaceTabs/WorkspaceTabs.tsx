import * as React from "react";
import { X } from "lucide-react";

import { cn } from "../../lib/utils";

/**
 * WorkspaceTabs — 管理端工作台多页签
 *
 * 层级：Layer 2 业务复合
 * 关联故事：PC 管理端工作台导航
 * Wave：Wave 6
 * 业务约束：只负责页签交互；打开页签、缓存和持久化由 AppShell 持有。
 *
 * @example
 *   <WorkspaceTabs tabs={tabs} activeValue={view} onActiveValueChange={setView} />
 */
export interface WorkspaceTabItem {
  value: string;
  label: React.ReactNode;
  subtitle?: React.ReactNode;
  closable?: boolean;
}

export interface WorkspaceTabsProps extends React.HTMLAttributes<HTMLDivElement> {
  tabs: WorkspaceTabItem[];
  activeValue: string;
  onActiveValueChange: (value: string) => void;
  onCloseTab?: (value: string) => void;
  onCloseOtherTabs?: (value: string) => void;
}

export const WorkspaceTabs = React.forwardRef<HTMLDivElement, WorkspaceTabsProps>(function WorkspaceTabs(
  {
    tabs,
    activeValue,
    onActiveValueChange,
    onCloseTab,
    onCloseOtherTabs,
    className,
    ...rest
  },
  ref,
) {
  const [contextTarget, setContextTarget] = React.useState<{ value: string; x: number; y: number } | null>(null);
  const targetTab = contextTarget ? tabs.find((tab) => tab.value === contextTarget.value) : null;

  React.useEffect(() => {
    if (!contextTarget) return;
    function closeByPointer(event: PointerEvent) {
      const target = event.target instanceof Element ? event.target : null;
      if (target?.closest("[data-workspace-tabs-menu]")) return;
      setContextTarget(null);
    }
    function closeByKeyboard(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      setContextTarget(null);
    }
    document.addEventListener("pointerdown", closeByPointer);
    document.addEventListener("keydown", closeByKeyboard);
    return () => {
      document.removeEventListener("pointerdown", closeByPointer);
      document.removeEventListener("keydown", closeByKeyboard);
    };
  }, [contextTarget]);

  return (
    <div ref={ref} className={cn("min-w-0 bg-transparent", className)} data-workspace-tabs {...rest}>
      <div className="flex max-w-full items-center gap-1 overflow-x-auto px-3">
        {tabs.map((tab) => {
          const active = tab.value === activeValue;
          return (
            <div
              key={tab.value}
              data-workspace-tab-value={tab.value}
              className={cn(
                "group flex h-8 min-w-20 max-w-44 items-center rounded-full border text-xs",
                active
                  ? "border-primary/20 bg-primary/10 text-primary"
                  : "border-transparent text-muted-foreground hover:bg-muted hover:text-foreground",
              )}
              onContextMenu={(event) => {
                event.preventDefault();
                setContextTarget({ value: tab.value, x: event.clientX, y: event.clientY });
              }}
            >
              <button
                type="button"
                className="min-w-0 flex-1 px-2 text-left"
                aria-current={active ? "page" : undefined}
                title={typeof tab.label === "string" ? tab.label : undefined}
                onClick={() => onActiveValueChange(tab.value)}
              >
                <span className="block truncate whitespace-nowrap font-medium">{tab.label}</span>
              </button>
              {tab.closable && onCloseTab && (
                <button
                  type="button"
                  className="mr-1 flex size-6 shrink-0 items-center justify-center rounded-full hover:bg-background/80"
                  aria-label={`关闭${typeof tab.label === "string" ? tab.label : "页签"}`}
                  title="关闭"
                  onClick={(event) => {
                    event.stopPropagation();
                    onCloseTab(tab.value);
                  }}
                >
                  <X className="size-3.5" aria-hidden />
                </button>
              )}
            </div>
          );
        })}
      </div>

      {contextTarget && targetTab && onCloseOtherTabs && (
        <div
          data-workspace-tabs-menu
          className="fixed z-50 w-32 rounded-md border bg-background p-1 text-sm shadow-lg"
          // 动态：右键菜单位置来自鼠标坐标。
          style={{ left: contextTarget.x, top: contextTarget.y }}
        >
          <button
            type="button"
            className="w-full rounded px-2 py-1.5 text-left hover:bg-muted"
            onClick={() => {
              onCloseOtherTabs(targetTab.value);
              setContextTarget(null);
            }}
          >
            关闭其他
          </button>
        </div>
      )}
    </div>
  );
});
WorkspaceTabs.displayName = "WorkspaceTabs";
