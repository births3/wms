import assert from "node:assert/strict";
import {
  collectTreeCatalogNodeIds,
  defaultTreeCatalogExpandedNodeIds,
  filterTreeCatalogNodes,
  findTreeCatalogNode,
  firstSelectableTreeCatalogNode,
  flattenTreeCatalogNodes,
  normalizeTreeCatalogPreference,
} from "../src/business/TreeCatalog/tree-catalog-logic.ts";

const nodes = [
  {
    id: "type:asn",
    label: "ASN 单",
    description: "asn",
    children: [
      {
        id: "library:m2_asn",
        label: "M2 ASN 字段库",
        description: "m2_asn",
        children: [{ id: "version:v1", label: "v1", badge: 12 }],
      },
    ],
  },
  {
    id: "type:missing",
    label: "未发布模板",
    disabled: true,
  },
];

assert.deepEqual(collectTreeCatalogNodeIds(nodes), [
  "type:asn",
  "library:m2_asn",
  "version:v1",
  "type:missing",
]);
assert.equal(flattenTreeCatalogNodes(nodes)[2].depth, 2);
assert.deepEqual(defaultTreeCatalogExpandedNodeIds(nodes), ["type:asn", "library:m2_asn"]);
assert.equal(findTreeCatalogNode(nodes, "version:v1")?.label, "v1");
assert.equal(firstSelectableTreeCatalogNode([{ ...nodes[0], disabled: true }, nodes[1]])?.id, "library:m2_asn");

const filtered = filterTreeCatalogNodes(nodes, "字段库");
assert.equal(filtered.length, 1);
assert.equal(filtered[0].id, "type:asn");
assert.equal(filtered[0].children?.[0].id, "library:m2_asn");

assert.deepEqual(
  normalizeTreeCatalogPreference(
    {
      selectedNodeId: "missing",
      expandedNodeIds: ["type:asn", "ghost"],
      query: " ASN ",
    },
    nodes,
  ),
  {
    selectedNodeId: "type:asn",
    expandedNodeIds: ["type:asn"],
    query: "asn",
  },
);
