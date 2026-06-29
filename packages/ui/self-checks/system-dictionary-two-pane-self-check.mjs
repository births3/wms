import assert from "node:assert/strict";
import {
  getSystemDictionarySelectedGroup,
  summarizeSystemDictionaryGroup,
  summarizeSystemDictionaryParams,
} from "../src/business/SystemDictionaryTwoPane/system-dictionary-two-pane-logic.ts";

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
