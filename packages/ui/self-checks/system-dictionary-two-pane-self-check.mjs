import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  getSystemDictionarySelectedGroup,
  summarizeSystemDictionaryGroup,
  summarizeSystemDictionaryParams,
} from "../src/business/SystemDictionaryTwoPane/system-dictionary-two-pane-logic.ts";

const __dirname = dirname(fileURLToPath(import.meta.url));
const twoPaneSource = readFileSync(resolve(__dirname, "../src/business/TwoPaneCatalog/TwoPaneCatalog.tsx"), "utf8");
const systemDictionarySource = readFileSync(resolve(__dirname, "../src/business/SystemDictionaryTwoPane/SystemDictionaryTwoPane.tsx"), "utf8");
const systemDictionaryPageSource = readFileSync(
  resolve(__dirname, "../../../apps/web-admin/src/pages/master-data/SystemDictionaryPage.tsx"),
  "utf8",
);

const groups = [
  {
    code: "document_type",
    name: "单据类型",
    items: [
      {
        code: "purchase_inbound",
        name: "采购入库",
        source: "global",
        enabled: true,
        params: {
          batch_policy: "required",
          direction: "inbound",
          workflow_template: "inbound_standard",
        },
      },
      {
        code: "obsolete",
        name: "停用类型",
        source: "owner_override",
        enabled: false,
        params: {},
      },
    ],
  },
];

assert.deepEqual(summarizeSystemDictionaryGroup(groups[0]), {
  code: "document_type",
  name: "单据类型",
  enabledCount: 1,
  totalCount: 2,
});

assert.equal(getSystemDictionarySelectedGroup(groups, "missing")?.code, "document_type");

assert.deepEqual(summarizeSystemDictionaryParams(groups[0].items[0].params), [
  { key: "direction", value: "inbound" },
  { key: "workflow_template", value: "inbound_standard" },
  { key: "batch_policy", value: "required" },
]);

assert.match(systemDictionaryPageSource, /defaultDictionaryGroupCode = "special_drug_category"/);
assert.match(twoPaneSource, /lg:grid-cols-\[20rem_minmax\(0,1fr\)\]/);
assert.match(twoPaneSource, /placeholder=\{`搜索\$\{groupTitle\}`\}/);
assert.match(twoPaneSource, /placeholder=\{`搜索\$\{itemTitle\}`\}/);
assert.match(systemDictionarySource, /rounded-md border bg-muted\/40/);
assert.match(systemDictionarySource, /rounded-full bg-background\/80/);
assert.match(systemDictionarySource, /font-medium text-muted-foreground">参数/);
