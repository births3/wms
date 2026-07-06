export interface TreeCatalogNode {
  id: string;
  label: string;
  description?: string;
  badge?: string | number;
  disabled?: boolean;
  children?: TreeCatalogNode[];
}

export interface TreeCatalogFlatNode {
  node: TreeCatalogNode;
  depth: number;
}

export interface TreeCatalogPreference {
  selectedNodeId: string;
  expandedNodeIds: string[];
  query: string;
}

export function flattenTreeCatalogNodes(nodes: readonly TreeCatalogNode[], depth = 0): TreeCatalogFlatNode[] {
  return nodes.flatMap((node) => [
    { node, depth },
    ...flattenTreeCatalogNodes(node.children ?? [], depth + 1),
  ]);
}

export function collectTreeCatalogNodeIds(nodes: readonly TreeCatalogNode[]) {
  return flattenTreeCatalogNodes(nodes).map(({ node }) => node.id);
}

export function defaultTreeCatalogExpandedNodeIds(nodes: readonly TreeCatalogNode[]) {
  return flattenTreeCatalogNodes(nodes)
    .filter(({ node }) => Boolean(node.children?.length))
    .map(({ node }) => node.id);
}

export function findTreeCatalogNode(nodes: readonly TreeCatalogNode[], id?: string): TreeCatalogNode | undefined {
  if (!id) return undefined;
  for (const node of nodes) {
    if (node.id === id) return node;
    const child = findTreeCatalogNode(node.children ?? [], id);
    if (child) return child;
  }
  return undefined;
}

export function firstSelectableTreeCatalogNode(nodes: readonly TreeCatalogNode[]): TreeCatalogNode | undefined {
  for (const node of nodes) {
    if (!node.disabled) return node;
    const child = firstSelectableTreeCatalogNode(node.children ?? []);
    if (child) return child;
  }
  return undefined;
}

export function filterTreeCatalogNodes(nodes: readonly TreeCatalogNode[], query: string): TreeCatalogNode[] {
  const normalized = normalizeTreeCatalogQuery(query);
  if (!normalized) return [...nodes];
  return nodes.flatMap((node) => {
    const children = filterTreeCatalogNodes(node.children ?? [], normalized);
    if (treeCatalogNodeMatches(node, normalized) || children.length > 0) {
      return [{ ...node, children }];
    }
    return [];
  });
}

export function normalizeTreeCatalogPreference(
  value: unknown,
  nodes: readonly TreeCatalogNode[]
): TreeCatalogPreference {
  const record = isRecord(value) ? value : {};
  const validIds = new Set(collectTreeCatalogNodeIds(nodes));
  const selectedNodeId =
    typeof record.selectedNodeId === "string" && validIds.has(record.selectedNodeId)
      ? record.selectedNodeId
      : firstSelectableTreeCatalogNode(nodes)?.id ?? "";
  const expandedNodeIds = Array.isArray(record.expandedNodeIds)
    ? record.expandedNodeIds.filter((id): id is string => typeof id === "string" && validIds.has(id))
    : defaultTreeCatalogExpandedNodeIds(nodes);

  return {
    selectedNodeId,
    expandedNodeIds,
    query: normalizeTreeCatalogQuery(record.query),
  };
}

export function normalizeTreeCatalogQuery(value: unknown) {
  return typeof value === "string" ? value.trim().toLowerCase() : "";
}

function treeCatalogNodeMatches(node: TreeCatalogNode, query: string) {
  return [node.label, node.description, node.badge, node.id]
    .filter((value) => value !== undefined && value !== null)
    .some((value) => String(value).toLowerCase().includes(query));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
