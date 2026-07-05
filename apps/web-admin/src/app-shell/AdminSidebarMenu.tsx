import * as React from "react";
import { cn } from "@wms/ui";
import { ChevronDown, ShieldCheck, type LucideIcon } from "lucide-react";

import type { AdminMenuNode } from "@/features/admin-menu/admin-menu-queries";

export interface SidebarMenuItem<TView extends string> {
  id?: TView;
  title: string;
  subtitle: string;
  icon: LucideIcon;
  disabled?: boolean;
}

export interface SidebarMenuGroup<TView extends string> {
  label: string;
  items: SidebarMenuItem<TView>[];
}

export interface SidebarMenuTreeSection<TView extends string> {
  label: string;
  groups: SidebarMenuGroup<TView>[];
}

export function AdminSidebarMenu<TView extends string>({
  sections,
  activeView,
  onNavigate,
  expandedKeys,
  onToggleKey,
  forceOpen = false,
  collapsed = false,
}: {
  sections: SidebarMenuTreeSection<TView>[];
  activeView: TView;
  onNavigate: (view: TView) => void;
  expandedKeys: Set<string>;
  onToggleKey: (key: string) => void;
  forceOpen?: boolean;
  collapsed?: boolean;
}) {
  return (
    <>
      {sections.map((section) => (
        <MenuSection
          key={section.label}
          section={section}
          activeView={activeView}
          onNavigate={onNavigate}
          expandedKeys={expandedKeys}
          onToggleKey={onToggleKey}
          forceOpen={forceOpen}
          collapsed={collapsed}
        />
      ))}
    </>
  );
}

function MenuSection<TView extends string>({
  section,
  activeView,
  onNavigate,
  expandedKeys,
  onToggleKey,
  forceOpen = false,
  collapsed = false,
}: {
  section: SidebarMenuTreeSection<TView>;
  activeView: TView;
  onNavigate: (view: TView) => void;
  expandedKeys: Set<string>;
  onToggleKey: (key: string) => void;
  forceOpen?: boolean;
  collapsed?: boolean;
}) {
  const items = section.groups.flatMap((group) => group.items);
  const hasActive = items.some((item) => item.id === activeView);
  const sectionKey = menuSectionKey(section.label);
  const visible = collapsed || forceOpen || expandedKeys.has(sectionKey) || hasActive;

  if (collapsed) {
    return (
      <section aria-label={section.label}>
        <div className="space-y-1">
          {items.map((item) => <MenuItemButton key={item.title} item={item} activeView={activeView} onNavigate={onNavigate} compact />)}
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
        onClick={() => onToggleKey(sectionKey)}
      >
        {section.label}
        <ChevronDown className={visible ? "size-3 transition-transform" : "size-3 -rotate-90 transition-transform"} aria-hidden />
      </button>
      {visible ? (
        <div className="mt-2 space-y-1">
          {section.groups.map((group) => (
            <MenuGroupBlock
              key={group.label}
              sectionLabel={section.label}
              group={group}
              activeView={activeView}
              onNavigate={onNavigate}
              expandedKeys={expandedKeys}
              onToggleKey={onToggleKey}
              forceOpen={forceOpen}
            />
          ))}
        </div>
      ) : null}
    </section>
  );
}

function MenuGroupBlock<TView extends string>({
  sectionLabel,
  group,
  activeView,
  onNavigate,
  expandedKeys,
  onToggleKey,
  forceOpen,
}: {
  sectionLabel: string;
  group: SidebarMenuGroup<TView>;
  activeView: TView;
  onNavigate: (view: TView) => void;
  expandedKeys: Set<string>;
  onToggleKey: (key: string) => void;
  forceOpen: boolean;
}) {
  const hasActive = group.items.some((item) => item.id === activeView);
  const groupKey = menuGroupKey(sectionLabel, group.label);
  const visible = forceOpen || expandedKeys.has(groupKey) || hasActive;

  return (
    <div className="space-y-1">
      <button
        type="button"
        className="flex w-full items-center justify-between rounded-md px-3 py-1.5 text-left text-xs font-medium text-muted-foreground hover:bg-muted/80"
        aria-expanded={visible}
        onClick={() => onToggleKey(groupKey)}
      >
        {group.label}
        <ChevronDown className={visible ? "size-3 transition-transform" : "size-3 -rotate-90 transition-transform"} aria-hidden />
      </button>
      {visible ? (
        <div className="space-y-1 pl-2">
          {group.items.map((item) => <MenuItemButton key={item.title} item={item} activeView={activeView} onNavigate={onNavigate} />)}
        </div>
      ) : null}
    </div>
  );
}

function MenuItemButton<TView extends string>({
  item,
  activeView,
  onNavigate,
  compact = false,
}: {
  item: SidebarMenuItem<TView>;
  activeView: TView;
  onNavigate: (view: TView) => void;
  compact?: boolean;
}) {
  const Icon = item.icon;
  const active = item.id === activeView;
  if (compact) {
    return (
      <button
        type="button"
        aria-current={active ? "page" : undefined}
        aria-label={item.title}
        title={item.title}
        disabled={item.disabled}
        onClick={() => item.id && onNavigate(item.id)}
        className={cn(
          "flex size-10 w-full items-center justify-center rounded-md disabled:cursor-not-allowed disabled:opacity-45",
          active ? "bg-primary text-primary-foreground" : "text-foreground hover:bg-muted",
        )}
      >
        <Icon className="size-4" aria-hidden />
      </button>
    );
  }

  return (
    <button
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
}

export function menuSectionKey(label: string) {
  return `section:${label}`;
}

export function menuGroupKey(sectionLabel: string, groupLabel: string) {
  return `group:${sectionLabel}:${groupLabel}`;
}

export function filterSidebarMenuTree<TView extends string>(
  sections: SidebarMenuTreeSection<TView>[],
  query: string,
) {
  if (!query) return sections;
  return sections.flatMap((section) => {
    const sectionMatches = textMatches(section.label, query);
    if (sectionMatches) return [section];
    const groups = section.groups.flatMap((group) => {
      const groupMatches = textMatches(group.label, query);
      if (groupMatches) return [group];
      const items = group.items.filter((item) => textMatches(`${item.title} ${item.subtitle}`, query));
      return items.length > 0 ? [{ ...group, items }] : [];
    });
    return groups.length > 0 ? [{ ...section, groups }] : [];
  });
}

export function menuTreeFromAdminNodes<TView extends string>({
  nodes,
  isView,
  iconByKey,
}: {
  nodes: AdminMenuNode[];
  isView: (viewId: string) => viewId is TView;
  iconByKey: Record<string, LucideIcon>;
}): SidebarMenuTreeSection<TView>[] {
  return nodes.flatMap((sectionNode) => {
    const groups = sectionNode.children.flatMap((groupNode) => {
      const items = groupNode.children.flatMap((node) => menuItemFromAdminNode(node, isView, iconByKey));
      return items.length > 0 ? [{ label: groupNode.title, items }] : [];
    });
    return groups.length > 0 ? [{ label: sectionNode.title, groups }] : [];
  });
}

function menuItemFromAdminNode<TView extends string>(
  node: AdminMenuNode,
  isView: (viewId: string) => viewId is TView,
  iconByKey: Record<string, LucideIcon>,
): SidebarMenuItem<TView>[] {
  if (!node.enabled || !node.view_id || !isView(node.view_id)) return [];
  return [{
    id: node.view_id,
    title: node.title,
    subtitle: node.code,
    icon: iconByKey[node.icon_key] ?? ShieldCheck,
  }];
}

function textMatches(value: string, query: string) {
  return value.toLowerCase().includes(query);
}
