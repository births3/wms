import assert from "node:assert/strict";
import {
  buildTwoPaneCatalogCopyTitle,
  filterTwoPaneCatalogGroups,
  filterTwoPaneCatalogItems,
  getTwoPaneCatalogSelectedGroup,
  normalizeTwoPaneCatalogFields,
  normalizeTwoPaneCatalogPreference,
  splitTwoPaneCatalogFields,
  toggleTwoPaneCatalogSelection,
} from "../src/business/TwoPaneCatalog/two-pane-catalog-logic.ts";

const groups = [
  {
    code: "dict",
    name: "系统字典",
    items: [
      { code: "document_type", name: "单据类型", source: "global", enabled: true },
      { code: "obsolete", name: "停用类型", source: "owner", enabled: false },
    ],
  },
  {
    code: "area",
    name: "库区分类",
    items: [{ code: "cold", name: "冷链库区", source: "global", enabled: true }],
  },
];

assert.equal(getTwoPaneCatalogSelectedGroup(groups, "missing")?.code, "dict");
assert.equal(filterTwoPaneCatalogGroups(groups, "库区")[0]?.code, "area");
assert.equal(
  filterTwoPaneCatalogItems(groups[0].items, "owner", (item) => [item.code, item.name, item.source]).length,
  1
);

assert.deepEqual(toggleTwoPaneCatalogSelection(["a"], "b", true), ["a", "b"]);
assert.deepEqual(toggleTwoPaneCatalogSelection(["a", "b"], "a", false), ["b"]);

assert.deepEqual(
  normalizeTwoPaneCatalogFields(
    [
      { key: "code", label: "编码" },
      { key: "params", label: "参数", layout: "detail" },
      { key: "source", label: "来源", defaultVisible: false },
    ],
    ["missing"]
  ),
  ["code", "params"]
);

assert.deepEqual(
  splitTwoPaneCatalogFields(
    [
      { key: "code", label: "编码" },
      { key: "params", label: "参数", layout: "detail" },
      { key: "source", label: "来源" },
    ],
    ["params", "code"]
  ),
  {
    columns: [{ key: "code", label: "编码" }],
    details: [{ key: "params", label: "参数", layout: "detail" }],
  }
);

assert.deepEqual(
  normalizeTwoPaneCatalogPreference(
    { selectedGroupCode: "area", groupQuery: " 库区 ", itemQuery: " 冷链 ", hiddenFieldKeys: ["source", "missing"] },
    groups,
    ["code", "source"]
  ),
  {
    selectedGroupCode: "area",
    groupQuery: "库区",
    itemQuery: "冷链",
    hiddenFieldKeys: ["source"],
  }
);

assert.equal(buildTwoPaneCatalogCopyTitle("purchase_inbound"), "复制 purchase_inbound");
assert.equal(buildTwoPaneCatalogCopyTitle(""), "复制");
