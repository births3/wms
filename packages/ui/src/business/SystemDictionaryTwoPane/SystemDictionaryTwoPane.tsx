import * as React from "react";
import { Card, CardContent } from "../../ui";
import { cn } from "../../lib/utils";
import { EmptyState } from "../EmptyState";
import { StatusBadge } from "../StatusBadge";
import {
  getSystemDictionarySelectedGroup,
  summarizeSystemDictionaryGroup,
  summarizeSystemDictionaryParams,
  systemDictionarySourceText,
  type SystemDictionaryTwoPaneGroup,
  type SystemDictionaryTwoPaneItem,
} from "./system-dictionary-two-pane-logic";

export interface SystemDictionaryTwoPaneProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  groups: SystemDictionaryTwoPaneGroup[];
  initialGroupCode?: string;
  emptyTitle?: string;
  emptyDescription?: string;
}

/**
 * SystemDictionaryTwoPane — 系统字典两层展示
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M1-011 系统字典 / 单据类型参数配置
 * Wave：Wave 3 管理端基础档案
 * 业务约束：左层只展示字典分类，右层只展示选中分类的字典项与参数摘要。
 *
 * @example
 *   <SystemDictionaryTwoPane groups={[{ code: "document_type", name: "单据类型", items: [] }]} />
 */
export const SystemDictionaryTwoPane = React.forwardRef<
  HTMLDivElement,
  SystemDictionaryTwoPaneProps
>(
  (
    {
      groups,
      initialGroupCode,
      emptyTitle = "暂无字典项",
      emptyDescription = "当前分类下还没有可展示的字典项。",
      className,
      ...rest
    },
    ref
  ) => {
    const [selectedCode, setSelectedCode] = React.useState(initialGroupCode);
    const selectedGroup = getSystemDictionarySelectedGroup(groups, selectedCode);

    if (groups.length === 0) {
      return (
        <Card ref={ref} className={cn("rounded-lg shadow-sm", className)} {...rest}>
          <CardContent className="p-0">
            <EmptyState title="暂无字典分类" description="请先确认系统字典接口是否返回分类。" />
          </CardContent>
        </Card>
      );
    }

    return (
      <Card ref={ref} className={cn("overflow-hidden rounded-lg shadow-sm", className)} {...rest}>
        <CardContent className="grid gap-0 p-0 md:grid-cols-[minmax(15rem,20rem)_1fr]">
          <aside className="border-b bg-muted/20 p-3 md:border-b-0 md:border-r">
            <div className="mb-2 text-xs font-medium text-muted-foreground">字典分类</div>
            <div className="flex flex-col gap-2">
              {groups.map((group) => {
                const summary = summarizeSystemDictionaryGroup(group);
                const selected = selectedGroup?.code === group.code;
                return (
                  <button
                    key={group.code}
                    type="button"
                    aria-pressed={selected}
                    onClick={() => setSelectedCode(group.code)}
                    className={cn(
                      "rounded-md border px-3 py-2 text-left transition-colors",
                      selected
                        ? "border-primary bg-primary/10 text-primary"
                        : "border-border bg-background hover:bg-muted/60"
                    )}
                  >
                    <span className="flex min-w-0 items-start justify-between gap-3">
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-medium">{summary.name}</span>
                        <span className="mt-0.5 block truncate font-mono text-xs text-muted-foreground">
                          {summary.code}
                        </span>
                      </span>
                      <span className="shrink-0 rounded-md bg-background px-2 py-1 text-xs font-medium text-foreground">
                        {summary.enabledCount}/{summary.totalCount}
                      </span>
                    </span>
                  </button>
                );
              })}
            </div>
          </aside>

          <section className="min-w-0 p-4">
            <div className="mb-3 flex items-center justify-between gap-3">
              <h2 className="text-sm font-semibold">字典项</h2>
              <span className="text-xs text-muted-foreground">
                共 {selectedGroup?.items.length ?? 0} 项
              </span>
            </div>

            {selectedGroup && selectedGroup.items.length > 0 ? (
              <ul className="flex flex-col gap-3">
                {selectedGroup.items.map((item) => (
                  <SystemDictionaryItemRow key={item.code} item={item} />
                ))}
              </ul>
            ) : (
              <EmptyState title={emptyTitle} description={emptyDescription} className="min-h-64" />
            )}
          </section>
        </CardContent>
      </Card>
    );
  }
);
SystemDictionaryTwoPane.displayName = "SystemDictionaryTwoPane";

function SystemDictionaryItemRow({ item }: { item: SystemDictionaryTwoPaneItem }) {
  const params = summarizeSystemDictionaryParams(item.params);

  return (
    <li className="rounded-md border bg-background p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">{item.name}</div>
          <div className="mt-0.5 font-mono text-xs text-muted-foreground">{item.code}</div>
        </div>
        <StatusBadge
          status={item.enabled ? "completed" : "isolated"}
          label={item.enabled ? "启用" : "停用"}
          size="sm"
        />
      </div>

      <div className="mt-3 text-xs text-muted-foreground">
        来源：<span className="text-foreground">{systemDictionarySourceText(item.source)}</span>
      </div>

      {params.length > 0 ? (
        <div className="mt-3 flex flex-wrap gap-2">
          {params.map((param) => (
            <span
              key={param.key}
              className="inline-flex max-w-full items-center gap-1 rounded-md bg-muted px-2 py-1 text-xs"
            >
              <span className="font-mono text-muted-foreground">{param.key}</span>
              <span className="break-all text-foreground">{param.value}</span>
            </span>
          ))}
        </div>
      ) : (
        <div className="mt-3 text-xs text-muted-foreground">参数：-</div>
      )}
    </li>
  );
}
