import * as React from "react";
import { ChevronDown, ChevronRight, Search } from "lucide-react";
import { Button, Card, CardContent, Input } from "../../ui";
import { cn } from "../../lib/utils";
import { EmptyState } from "../EmptyState";
import {
  defaultTreeCatalogExpandedNodeIds,
  filterTreeCatalogNodes,
  findTreeCatalogNode,
  firstSelectableTreeCatalogNode,
  normalizeTreeCatalogPreference,
  type TreeCatalogNode,
} from "./tree-catalog-logic";

export interface TreeCatalogProps extends Omit<React.HTMLAttributes<HTMLDivElement>, "children"> {
  nodes: TreeCatalogNode[];
  title?: string;
  searchPlaceholder?: string;
  selectedNodeId?: string;
  onSelectedNodeIdChange?: (nodeId: string) => void;
  storageKey?: string;
  defaultExpandedNodeIds?: string[];
  emptyTitle?: string;
  emptyDescription?: string;
}

/**
 * TreeCatalog — 管理端树状导航组件
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-H9-001 打印模板类型字典 / US-H1-007 PC 管理端三层菜单管理
 * Wave：Wave 6 横向能力收口
 * 适用：模板类型树、菜单树、库区库位树等左侧导航。
 * 业务约束：只负责树的搜索、展开、选择和偏好保存，右侧明细继续由页面复用 DataGrid。
 *
 * @example
 *   <TreeCatalog nodes={[{ id: "type:asn", label: "ASN 单" }]} />
 */
export const TreeCatalog = React.forwardRef<HTMLDivElement, TreeCatalogProps>(
  (
    {
      nodes,
      title = "目录",
      searchPlaceholder = "搜索目录",
      selectedNodeId,
      onSelectedNodeIdChange,
      storageKey,
      defaultExpandedNodeIds,
      emptyTitle = "暂无目录",
      emptyDescription = "当前没有可展示的树节点。",
      className,
      ...rest
    },
    ref
  ) => {
    const storedPreference = React.useMemo(() => readPreference(storageKey, nodes), [nodes, storageKey]);
    const [internalSelectedId, setInternalSelectedId] = React.useState(storedPreference.selectedNodeId);
    const [expandedNodeIds, setExpandedNodeIds] = React.useState(
      defaultExpandedNodeIds ?? storedPreference.expandedNodeIds
    );
    const [query, setQuery] = React.useState(storedPreference.query);
    const selectedId = selectedNodeId ?? internalSelectedId;
    const selectedExists = Boolean(findTreeCatalogNode(nodes, selectedId));
    const visibleNodes = React.useMemo(() => filterTreeCatalogNodes(nodes, query), [nodes, query]);
    const queryActive = Boolean(query);

    React.useEffect(() => {
      if (selectedExists) return;
      const next = firstSelectableTreeCatalogNode(nodes)?.id ?? "";
      setInternalSelectedId(next);
      if (next) onSelectedNodeIdChange?.(next);
    }, [nodes, onSelectedNodeIdChange, selectedExists]);

    React.useEffect(() => {
      if (!storageKey || typeof window === "undefined") return;
      window.localStorage.setItem(
        storageKey,
        JSON.stringify({
          selectedNodeId: selectedId,
          expandedNodeIds,
          query,
        })
      );
    }, [expandedNodeIds, query, selectedId, storageKey]);

    function selectNode(node: TreeCatalogNode) {
      if (node.disabled) return;
      setInternalSelectedId(node.id);
      onSelectedNodeIdChange?.(node.id);
    }

    function toggleExpanded(nodeId: string) {
      setExpandedNodeIds((current) =>
        current.includes(nodeId) ? current.filter((id) => id !== nodeId) : [...current, nodeId]
      );
    }

    return (
      <Card ref={ref} className={cn("overflow-hidden rounded-lg border bg-background shadow-sm", className)} {...rest}>
        <CardContent className="p-0">
          <div className="border-b px-4 py-3">
            <div className="flex items-center justify-between gap-2">
              <h2 className="text-base font-semibold tracking-normal">{title}</h2>
              <span className="text-xs text-muted-foreground">{nodes.length}</span>
            </div>
          </div>
          <div className="border-b bg-muted/20 p-4">
            <div className="relative">
              <Search className="pointer-events-none absolute left-2 top-2.5 size-4 text-muted-foreground" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={searchPlaceholder}
                className="pl-8"
              />
            </div>
          </div>

          {nodes.length === 0 ? (
            <EmptyState title={emptyTitle} description={emptyDescription} className="min-h-64" />
          ) : visibleNodes.length === 0 ? (
            <EmptyState title="没有匹配目录" description="请调整搜索条件。" className="min-h-64" />
          ) : (
            <div className="space-y-1 p-3">
              {visibleNodes.map((node) => (
                <TreeNodeRow
                  key={node.id}
                  node={node}
                  depth={0}
                  selectedId={selectedId}
                  expandedNodeIds={expandedNodeIds}
                  queryActive={queryActive}
                  onSelect={selectNode}
                  onToggle={toggleExpanded}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    );
  }
);
TreeCatalog.displayName = "TreeCatalog";

function TreeNodeRow({
  node,
  depth,
  selectedId,
  expandedNodeIds,
  queryActive,
  onSelect,
  onToggle,
}: {
  node: TreeCatalogNode;
  depth: number;
  selectedId: string;
  expandedNodeIds: string[];
  queryActive: boolean;
  onSelect: (node: TreeCatalogNode) => void;
  onToggle: (nodeId: string) => void;
}) {
  const children = node.children ?? [];
  const hasChildren = children.length > 0;
  const expanded = queryActive || expandedNodeIds.includes(node.id);
  const selected = selectedId === node.id;

  return (
    <div>
      <div className="flex items-start gap-1" style={{ paddingLeft: `${depth * 0.875}rem` }}>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="mt-0.5 size-7 shrink-0"
          aria-label={expanded ? "折叠" : "展开"}
          aria-expanded={hasChildren ? expanded : undefined}
          disabled={!hasChildren}
          onClick={() => onToggle(node.id)}
        >
          {hasChildren ? (
            expanded ? (
              <ChevronDown className="size-4" aria-hidden />
            ) : (
              <ChevronRight className="size-4" aria-hidden />
            )
          ) : (
            <span className="size-4" />
          )}
        </Button>
        <button
          type="button"
          disabled={node.disabled}
          onClick={() => onSelect(node)}
          className={cn(
            "min-w-0 flex-1 rounded-md border px-3 py-2 text-left transition-colors",
            selected
              ? "border-primary/50 bg-primary/10 text-primary shadow-sm"
              : "border-transparent bg-background text-foreground hover:border-border hover:bg-muted/50",
            node.disabled && "cursor-not-allowed opacity-60"
          )}
        >
          <span className="flex min-w-0 items-start justify-between gap-3">
            <span className="min-w-0">
              <span className="block truncate text-sm font-medium">{node.label}</span>
              {node.description ? (
                <span className="mt-0.5 block truncate font-mono text-xs text-muted-foreground">
                  {node.description}
                </span>
              ) : null}
            </span>
            {node.badge !== undefined ? (
              <span className="shrink-0 rounded-full border bg-background px-2 py-0.5 text-xs font-medium text-foreground">
                {node.badge}
              </span>
            ) : null}
          </span>
        </button>
      </div>
      {hasChildren && expanded ? (
        <div className="mt-1 space-y-1">
          {children.map((child) => (
            <TreeNodeRow
              key={child.id}
              node={child}
              depth={depth + 1}
              selectedId={selectedId}
              expandedNodeIds={expandedNodeIds}
              queryActive={queryActive}
              onSelect={onSelect}
              onToggle={onToggle}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function readPreference(storageKey: string | undefined, nodes: TreeCatalogNode[]) {
  if (!storageKey || typeof window === "undefined") {
    return normalizeTreeCatalogPreference(
      { expandedNodeIds: defaultTreeCatalogExpandedNodeIds(nodes) },
      nodes
    );
  }
  try {
    const storedValue = window.localStorage.getItem(storageKey);
    return normalizeTreeCatalogPreference(storedValue ? JSON.parse(storedValue) : null, nodes);
  } catch {
    return normalizeTreeCatalogPreference(null, nodes);
  }
}
